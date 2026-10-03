//! Worktree status: for every worktree of the repository, its branch, uncommitted work, and progress against the base.
//!
//! Changed paths come from git status (index stat cache, gitignore and
//! line-ending filters as git applies them). Line counts compare file
//! contents with the HEAD blob (uncommitted) and the merge-base blob (vs
//! base), ignoring `\r\n` versus `\n`. Untracked, non-ignored files count as
//! additions, so new files an agent creates show up before they are staged.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use gix::status::UntrackedFiles;
use jiff::Timestamp;
use rayon::prelude::*;
use tracing::{debug, info, warn};

use crate::git::{GitError, Repo, line_churn, read_err};
use crate::model::{Churn, WorktreeStatus, Worktrees};

/// Branch names tried, in order, when no base is given.
const DEFAULT_BASES: [&str; 2] = ["main", "master"];

/// Resolves the base branch: the given name (branch or revision), else `main`, else `master`.
fn resolve_base(repo: &gix::Repository, wanted: Option<&str>) -> Option<(String, gix::ObjectId)> {
    let lookup = |name: &str| -> Option<gix::ObjectId> {
        if let Ok(mut r) = repo.find_reference(format!("refs/heads/{name}").as_str()) {
            return r.peel_to_id().ok().map(|id| id.detach());
        }
        repo.rev_parse_single(name).ok().map(|id| id.detach())
    };
    match wanted {
        Some(name) => {
            let found = lookup(name).map(|id| (name.to_owned(), id));
            if found.is_none() {
                warn!(
                    base = name,
                    "base branch not found; ahead and vs-base are left out"
                );
            }
            found
        }
        None => DEFAULT_BASES.iter().find_map(|n| {
            let r = repo
                .find_reference(format!("refs/heads/{n}").as_str())
                .ok()?;
            let mut r = r;
            Some(((*n).to_owned(), r.peel_to_id().ok()?.detach()))
        }),
    }
}

/// Paths of the main worktree and every linked one; a linked worktree whose path is unknown is skipped.
fn worktree_paths(main: &gix::Repository) -> Result<Vec<PathBuf>, GitError> {
    let mut paths = Vec::new();
    if let Some(w) = main.workdir() {
        paths.push(w.to_path_buf());
    }
    for proxy in main.worktrees().map_err(read_err("listing worktrees"))? {
        match proxy.base() {
            Ok(p) => paths.push(p),
            Err(e) => warn!(id = %proxy.id(), error = %e, "worktree path unreadable; skipped"),
        }
    }
    Ok(paths)
}

/// The bytes of the blob at `path` in `tree`, or empty when absent or not a file.
fn blob_at(tree: Option<&gix::Tree<'_>>, path: &str) -> Result<Vec<u8>, GitError> {
    let Some(tree) = tree else {
        return Ok(Vec::new());
    };
    let Some(entry) = tree
        .lookup_entry_by_path(path)
        .map_err(read_err("looking up a path in a tree"))?
    else {
        return Ok(Vec::new());
    };
    if !entry.mode().is_blob() {
        return Ok(Vec::new());
    }
    let object = entry.object().map_err(read_err("reading a blob"))?;
    Ok(object.detach().data)
}

/// The tree of a commit.
fn tree_of(repo: &gix::Repository, id: gix::ObjectId) -> Result<gix::Tree<'_>, GitError> {
    repo.find_commit(id)
        .map_err(read_err("reading commit"))?
        .tree()
        .map_err(read_err("reading tree"))
}

/// Adds the change from `old` to `new` to `churn`; identical content (line endings aside) adds nothing.
fn add_change(churn: &mut Churn, old: &[u8], new: &[u8]) {
    match line_churn(old, new) {
        Some((0, 0)) => {}
        Some((added, removed)) => {
            churn.added += added;
            churn.removed += removed;
            churn.files += 1;
        }
        // Binary: the file changed if the bytes did; lines are not counted.
        None if old != new => churn.files += 1,
        None => {}
    }
}

/// Paths git status reports as changed (staged, unstaged or untracked), repository-relative.
fn status_paths(repo: &gix::Repository) -> Result<BTreeSet<String>, GitError> {
    let iter = repo
        .status(gix::progress::Discard)
        .map_err(read_err("preparing status"))?
        .untracked_files(UntrackedFiles::Files)
        .index_worktree_rewrites(None)
        .into_iter(None)
        .map_err(read_err("starting status"))?;
    let mut paths = BTreeSet::new();
    for item in iter {
        let item = item.map_err(read_err("reading status"))?;
        paths.insert(item.location().to_string());
    }
    Ok(paths)
}

/// Modification time of a file as a timestamp, if it exists.
fn mtime(path: &Path) -> Option<Timestamp> {
    let t = fs::metadata(path).ok()?.modified().ok()?;
    let d = t.duration_since(UNIX_EPOCH).ok()?;
    Timestamp::from_second(d.as_secs() as i64).ok()
}

/// Computes one worktree's status; any git failure becomes the worktree's `problem`.
fn status_of(path: &Path, base: Option<gix::ObjectId>) -> WorktreeStatus {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut status = WorktreeStatus {
        name,
        path: path.display().to_string(),
        current: false,
        branch: None,
        head: None,
        uncommitted: Churn::default(),
        ahead: None,
        vs_base: None,
        last_activity: None,
        problem: None,
    };
    if !path.exists() {
        status.problem = Some("worktree directory is missing".to_owned());
        return status;
    }
    if let Err(e) = fill(path, base, &mut status) {
        warn!(worktree = %path.display(), error = %e, "worktree status incomplete");
        status.problem = Some(e.to_string());
    }
    status
}

/// Fills `status` for the worktree at `path`.
fn fill(
    path: &Path,
    base: Option<gix::ObjectId>,
    status: &mut WorktreeStatus,
) -> Result<(), GitError> {
    let repo = gix::open(path).map_err(read_err("opening worktree"))?;
    let head = repo.head().map_err(read_err("reading HEAD"))?;
    status.branch = head
        .referent_name()
        .filter(|_| !head.is_detached())
        .map(|n| n.shorten().to_string());
    let head_id = repo.head_id().ok().map(|id| id.detach());
    status.head = head_id.map(|id| id.to_hex_with_len(7).to_string());
    let head_tree = head_id.map(|id| tree_of(&repo, id)).transpose()?;
    let head_time = match head_id {
        Some(id) => {
            let commit = repo
                .find_commit(id)
                .map_err(read_err("reading HEAD commit"))?;
            let seconds = commit
                .time()
                .map_err(read_err("reading commit time"))?
                .seconds;
            Timestamp::from_second(seconds).ok()
        }
        None => None,
    };

    let changed = status_paths(&repo)?;
    let mut newest = head_time;
    let mut disk = std::collections::HashMap::new();
    for rel in &changed {
        let file = path.join(rel);
        if file.is_dir() {
            continue;
        }
        let new = fs::read(&file).unwrap_or_default();
        let old = blob_at(head_tree.as_ref(), rel)?;
        add_change(&mut status.uncommitted, &old, &new);
        newest = newest.max(mtime(&file));
        disk.insert(rel.clone(), new);
    }
    status.last_activity = newest;
    debug!(worktree = %path.display(), changed = changed.len(), "status read");

    let (Some(base), Some(head_id)) = (base, head_id) else {
        return Ok(());
    };
    let ahead = repo
        .rev_walk([head_id])
        .with_hidden([base])
        .all()
        .map_err(read_err("counting commits ahead"))?
        .filter_map(Result::ok)
        .count();
    status.ahead = Some(ahead as u64);
    let Ok(merge_base) = repo.merge_base(head_id, base) else {
        debug!(worktree = %path.display(), "no merge base with the base branch");
        return Ok(());
    };
    let base_tree = tree_of(&repo, merge_base.detach())?;
    let mut paths: BTreeSet<String> = disk.keys().cloned().collect();
    if let Some(head_tree) = &head_tree {
        let changes = repo
            .diff_tree_to_tree(&base_tree, head_tree, None)
            .map_err(read_err("diffing base and HEAD"))?;
        for change in &changes {
            if change.entry_mode().is_blob() {
                paths.insert(change.location().to_string());
            }
            if let gix::object::tree::diff::ChangeDetached::Rewrite {
                source_location, ..
            } = change
            {
                paths.insert(source_location.to_string());
            }
        }
    }
    let mut vs_base = Churn::default();
    for rel in &paths {
        let old = blob_at(Some(&base_tree), rel)?;
        let new = match disk.get(rel) {
            Some(bytes) => bytes.clone(),
            None if changed.contains(rel) => Vec::new(),
            None => blob_at(head_tree.as_ref(), rel)?,
        };
        add_change(&mut vs_base, &old, &new);
    }
    vs_base.commits = ahead as u64;
    status.vs_base = Some(vs_base);
    Ok(())
}

/// Collects the status of every worktree of the repository `repo` belongs to.
///
/// `root` (canonical) marks the worktree the report is about as current;
/// `base` overrides the base branch (default `main`, else `master`).
///
/// # Errors
/// Returns [`GitError::Read`] when the worktree list cannot be read; a single unreadable worktree is reported in its row instead.
pub fn collect(repo: &Repo, root: &Path, base: Option<&str>) -> Result<Worktrees, GitError> {
    let main = repo
        .repo
        .main_repo()
        .map_err(read_err("opening the main repository"))?;
    let base = resolve_base(&main, base);
    let paths = worktree_paths(&main)?;
    let base_id = base.as_ref().map(|(_, id)| *id);
    let mut worktrees: Vec<WorktreeStatus> =
        paths.par_iter().map(|p| status_of(p, base_id)).collect();

    // The deepest worktree containing the root is the current one (worktrees can nest).
    let canonical = |p: &str| {
        PathBuf::from(p)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(p))
    };
    if let Some(current) = worktrees
        .iter_mut()
        .filter(|w| root.starts_with(canonical(&w.path)))
        .max_by_key(|w| canonical(&w.path).components().count())
    {
        current.current = true;
    }
    worktrees.sort_by(|a, b| {
        a.problem
            .is_some()
            .cmp(&b.problem.is_some())
            .then_with(|| b.last_activity.cmp(&a.last_activity))
            .then_with(|| a.name.cmp(&b.name))
    });
    info!(count = worktrees.len(), base = ?base.as_ref().map(|b| &b.0), "worktrees read");
    Ok(Worktrees {
        base: base.map(|(name, _)| name),
        worktrees,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo::TestRepo;

    const T0: i64 = 1_790_000_000;

    fn find<'a>(w: &'a Worktrees, name: &str) -> &'a WorktreeStatus {
        w.worktrees
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no worktree {name}: {w:?}"))
    }

    /// A repo on `main` with one commit, plus a linked worktree `feat` on branch `feature`.
    fn setup() -> (TestRepo, tempfile::TempDir) {
        let t = TestRepo::new();
        t.write("src/a.rs", "1\n2\n3\n");
        t.commit("base", T0);
        let wts = tempfile::tempdir().unwrap();
        let feat = wts.path().join("feat");
        t.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature",
            feat.to_str().unwrap(),
        ]);
        (t, wts)
    }

    #[test]
    fn every_worktree_is_listed_with_its_branch() {
        let (t, _wts) = setup();
        let w = collect(&t.open(), &t.path(), None).unwrap();
        assert_eq!(w.base.as_deref(), Some("main"));
        assert_eq!(w.worktrees.len(), 2);
        let main = find(&w, t.path().file_name().unwrap().to_str().unwrap());
        assert_eq!(main.branch.as_deref(), Some("main"));
        assert!(main.current);
        let feat = find(&w, "feat");
        assert_eq!(feat.branch.as_deref(), Some("feature"));
        assert!(!feat.current);
        assert_eq!(feat.ahead, Some(0));
        assert_eq!(feat.uncommitted, Churn::default());
        assert_eq!(feat.last_activity, Timestamp::from_second(T0).ok());
    }

    #[test]
    fn uncommitted_edits_and_new_files_count_against_head() {
        let (t, wts) = setup();
        let feat = wts.path().join("feat");
        fs::write(feat.join("src/a.rs"), "1\nTWO\n3\n4\n").unwrap();
        fs::write(feat.join("new.rs"), "x\ny\n").unwrap();
        fs::write(feat.join(".gitignore"), "target/\n").unwrap();
        fs::create_dir(feat.join("target")).unwrap();
        fs::write(feat.join("target/junk.rs"), "ignored\n").unwrap();
        let w = collect(&t.open(), &t.path(), None).unwrap();
        let s = find(&w, "feat");
        assert_eq!(
            s.uncommitted,
            Churn {
                added: 5,
                removed: 1,
                files: 3,
                commits: 0
            }
        );
        assert!(
            s.last_activity > Timestamp::from_second(T0).ok(),
            "edits move last activity"
        );
    }

    #[test]
    fn line_ending_only_changes_are_not_work() {
        let (t, wts) = setup();
        let feat = wts.path().join("feat");
        fs::write(feat.join("src/a.rs"), "1\r\n2\r\n3\r\n").unwrap();
        let w = collect(&t.open(), &t.path(), None).unwrap();
        assert_eq!(find(&w, "feat").uncommitted, Churn::default());
    }

    #[test]
    fn commits_ahead_and_lines_against_base() {
        let (t, wts) = setup();
        let feat = wts.path().join("feat");
        fs::write(feat.join("src/b.rs"), "b\nb\n").unwrap();
        t.git_at(&feat, &["add", "-A"], None);
        t.git_at(&feat, &["commit", "-q", "-m", "b"], Some(T0 + 60));
        fs::write(feat.join("src/a.rs"), "1\n2\n3\n4\n").unwrap();
        // Work landing on main after the branch point must not count against the branch.
        t.write("src/main_only.rs", "m\n");
        t.commit("main moves", T0 + 120);

        let w = collect(&t.open(), &t.path(), None).unwrap();
        let s = find(&w, "feat");
        assert_eq!(s.ahead, Some(1));
        assert_eq!(
            s.uncommitted,
            Churn {
                added: 1,
                removed: 0,
                files: 1,
                commits: 0
            }
        );
        assert_eq!(
            s.vs_base,
            Some(Churn {
                added: 3,
                removed: 0,
                files: 2,
                commits: 1
            })
        );
        let main = w.worktrees.iter().find(|x| x.current).unwrap();
        assert_eq!(main.ahead, Some(0));
    }

    #[test]
    fn explicit_and_missing_bases() {
        let (t, _wts) = setup();
        let w = collect(&t.open(), &t.path(), Some("feature")).unwrap();
        assert_eq!(w.base.as_deref(), Some("feature"));
        let w = collect(&t.open(), &t.path(), Some("nope")).unwrap();
        assert_eq!(w.base, None);
        assert!(
            w.worktrees
                .iter()
                .all(|s| s.ahead.is_none() && s.vs_base.is_none())
        );
    }

    #[test]
    fn report_from_a_linked_worktree_marks_it_current() {
        let (_t, wts) = setup();
        let feat = wts.path().join("feat").canonicalize().unwrap();
        let repo = Repo::discover(&feat).unwrap();
        let w = collect(&repo, &feat, None).unwrap();
        assert_eq!(w.worktrees.len(), 2);
        assert!(find(&w, "feat").current);
        assert_eq!(w.worktrees.iter().filter(|s| s.current).count(), 1);
    }

    #[test]
    fn a_removed_worktree_directory_is_a_problem_not_an_error() {
        let (t, wts) = setup();
        fs::remove_dir_all(wts.path().join("feat")).unwrap();
        let w = collect(&t.open(), &t.path(), None).unwrap();
        let s = find(&w, "feat");
        assert_eq!(s.problem.as_deref(), Some("worktree directory is missing"));
        assert_eq!(
            w.worktrees.last().unwrap().name,
            "feat",
            "problems sort last"
        );
    }
}
