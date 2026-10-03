//! Recent activity: walk the last 30 days of commits on every local branch and bucket their line churn.
//!
//! Each commit is diffed against its first parent once; the per-file result
//! is cached by commit id, since commits never change. Merge commits are
//! skipped so merged work is not counted twice. Times are committer times.
//! Only source files count (files tokei knows a language for, as in the
//! totals), so lock files and images do not swamp the numbers.

use std::collections::{BTreeMap, HashMap, HashSet};

use gix::prelude::TreeDiffChangeExt;
use gix::revision::walk::Sorting;
use gix::traverse::commit::simple::CommitTimeOrder;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::git::{GitError, Repo, read_err};
use crate::model::{Activity, ActivityWindow, Churn, DirActivity};
use crate::totals::dir_key;

/// The windows shown: label, length in seconds, sparkline buckets.
pub const WINDOWS: [(&str, i64, usize); 4] = [
    ("1h", 3_600, 12),
    ("24h", 86_400, 24),
    ("7d", 7 * 86_400, 28),
    ("30d", 30 * 86_400, 30),
];
/// Index into [`WINDOWS`] of the window used for directory trends (7 days).
const DIR_TREND_WINDOW: usize = 2;

/// Lines one commit changed in one file (repository-relative path).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChurn {
    /// Path in the repository, forward slashes.
    pub path: String,
    /// Lines added (0 for binary files).
    pub added: u64,
    /// Lines removed (0 for binary files).
    pub removed: u64,
}

/// One commit's diff against its first parent, as cached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitChurn {
    /// Committer time, seconds since the epoch.
    pub time: i64,
    /// Files changed.
    pub files: Vec<FileChurn>,
}

/// Commit id (hex) to its churn.
pub type CommitCache = HashMap<String, CommitChurn>;

/// Every local branch tip plus HEAD (which may be detached), deduplicated.
fn tips(repo: &gix::Repository) -> Result<Vec<gix::ObjectId>, GitError> {
    let mut tips = HashSet::new();
    let refs = repo.references().map_err(read_err("listing references"))?;
    for r in refs
        .local_branches()
        .map_err(read_err("listing branches"))?
    {
        let Ok(mut r) = r else { continue };
        match r.peel_to_id() {
            Ok(id) => {
                tips.insert(id.detach());
            }
            Err(e) => {
                warn!(branch = %r.name().as_bstr(), error = %e, "branch does not resolve; skipped")
            }
        }
    }
    if let Ok(id) = repo.head_id() {
        tips.insert(id.detach());
    }
    Ok(tips.into_iter().collect())
}

/// Diffs one commit against its first parent (or the empty tree for a root commit).
fn diff_commit(
    repo: &gix::Repository,
    id: gix::ObjectId,
    parent: Option<gix::ObjectId>,
    time: i64,
    blobs: &mut gix::diff::blob::Platform,
) -> Result<CommitChurn, GitError> {
    let tree_of = |id: gix::ObjectId| -> Result<gix::Tree<'_>, GitError> {
        repo.find_commit(id)
            .map_err(read_err("reading commit"))?
            .tree()
            .map_err(read_err("reading tree"))
    };
    let new_tree = tree_of(id)?;
    let old_tree = match parent {
        Some(p) => tree_of(p)?,
        None => repo.empty_tree(),
    };
    let changes = repo
        .diff_tree_to_tree(&old_tree, &new_tree, None)
        .map_err(read_err("diffing trees"))?;
    let mut files = Vec::new();
    for change in &changes {
        if !change.entry_mode().is_blob() {
            continue;
        }
        let attached = change.attach(repo, repo);
        let stats = attached
            .diff(blobs)
            .map_err(read_err("preparing blob diff"))?
            .line_counts()
            .map_err(|e| GitError::Read {
                context: "counting changed lines",
                source: e.into_error().into(),
            })?;
        let (added, removed) =
            stats.map_or((0, 0), |s| (u64::from(s.insertions), u64::from(s.removals)));
        files.push(FileChurn {
            path: change.location().to_string(),
            added,
            removed,
        });
    }
    blobs.clear_resource_cache_keep_allocation();
    Ok(CommitChurn { time, files })
}

/// Walks the last 30 days of history and returns each non-merge commit's churn, using and refilling `cache`.
///
/// The cache is pruned to the commits seen, so it stays bounded by the window.
///
/// # Errors
/// Returns [`GitError::Read`] when references, commits or trees cannot be read.
pub fn walk(
    repo: &gix::Repository,
    now: Timestamp,
    cache: &mut CommitCache,
) -> Result<(Vec<CommitChurn>, u64), GitError> {
    let tips = tips(repo)?;
    let branches = tips.len() as u64;
    if tips.is_empty() {
        debug!("no commits yet");
        cache.clear();
        return Ok((Vec::new(), 0));
    }
    let longest = WINDOWS[WINDOWS.len() - 1].1;
    let cutoff = now.as_second() - longest;
    let walk = repo
        .rev_walk(tips)
        .sorting(Sorting::ByCommitTimeCutoff {
            order: CommitTimeOrder::NewestFirst,
            seconds: cutoff,
        })
        .all()
        .map_err(read_err("starting history walk"))?;
    let mut blobs = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(read_err("preparing diff cache"))?;
    let mut fresh = CommitCache::new();
    let (mut hits, mut diffed) = (0u64, 0u64);
    for info in walk {
        let info = info.map_err(read_err("walking history"))?;
        if info.parent_ids.len() > 1 {
            continue;
        }
        let key = info.id.to_hex().to_string();
        let churn = match cache.remove(&key) {
            Some(c) => {
                hits += 1;
                c
            }
            None => {
                diffed += 1;
                diff_commit(
                    repo,
                    info.id,
                    info.parent_ids.first().copied(),
                    info.commit_time(),
                    &mut blobs,
                )?
            }
        };
        fresh.insert(key, churn);
    }
    *cache = fresh;
    info!(
        commits = cache.len(),
        hits, diffed, branches, "history walked"
    );
    Ok((cache.values().cloned().collect(), branches))
}

/// Whether a repository path is source code by the same rule as the totals (tokei knows its language).
fn code_path(repo: &Repo, path: &str, config: &tokei::Config) -> bool {
    tokei::LanguageType::from_path(repo.workdir.join(path), config).is_some()
}

/// One directory's running totals while bucketing: churn and distinct files per window, and the 7-day trend.
struct DirAcc<'a> {
    churn: Vec<Churn>,
    files: Vec<HashSet<&'a str>>,
    trend: Vec<u64>,
}

impl DirAcc<'_> {
    fn new() -> Self {
        Self {
            churn: vec![Churn::default(); WINDOWS.len()],
            files: vec![HashSet::new(); WINDOWS.len()],
            trend: vec![0; WINDOWS[DIR_TREND_WINDOW].2],
        }
    }
}

/// Buckets commits into the windows and per-directory rows, keeping only paths under the report root.
pub fn summarise(
    repo: &Repo,
    commits: &[CommitChurn],
    depth: u16,
    now: Timestamp,
    branches: u64,
) -> Activity {
    let now_s = now.as_second();
    let mut windows: Vec<ActivityWindow> = WINDOWS
        .iter()
        .map(|(label, seconds, buckets)| ActivityWindow {
            label: (*label).to_owned(),
            seconds: *seconds,
            churn: Churn::default(),
            trend: vec![0; *buckets],
        })
        .collect();
    let mut window_files: Vec<HashSet<&str>> = vec![HashSet::new(); WINDOWS.len()];
    let mut dirs: BTreeMap<String, DirAcc> = BTreeMap::new();
    let mut last: Option<i64> = None;
    let config = tokei::Config::default();
    let mut is_code: HashMap<&str, bool> = HashMap::new();

    for commit in commits {
        let files: Vec<(&str, &FileChurn)> = commit
            .files
            .iter()
            .filter(|f| {
                *is_code
                    .entry(f.path.as_str())
                    .or_insert_with(|| code_path(repo, &f.path, &config))
            })
            .filter_map(|f| repo.relative(&f.path).map(|rel| (rel, f)))
            .collect();
        if files.is_empty() {
            continue;
        }
        last = last.max(Some(commit.time));
        // Clock skew can put a commit in the future; treat it as just now.
        let age = (now_s - commit.time).max(0);
        let lines: u64 = files.iter().map(|(_, f)| f.added + f.removed).sum();
        let mut commit_dirs: HashSet<String> = HashSet::new();

        for (w, (_, seconds, buckets)) in WINDOWS.iter().enumerate() {
            if age >= *seconds {
                continue;
            }
            let win = &mut windows[w];
            win.churn.commits += 1;
            let bucket = (age * *buckets as i64 / *seconds) as usize;
            win.trend[*buckets - 1 - bucket] += lines;
            for (rel, f) in &files {
                win.churn.added += f.added;
                win.churn.removed += f.removed;
                window_files[w].insert(rel);
                let key = dir_key(rel, depth);
                let entry = dirs.entry(key.clone()).or_insert_with(DirAcc::new);
                entry.churn[w].added += f.added;
                entry.churn[w].removed += f.removed;
                entry.files[w].insert(rel);
                if w == DIR_TREND_WINDOW {
                    entry.trend[*buckets - 1 - bucket] += f.added + f.removed;
                }
                commit_dirs.insert(key);
            }
            for key in &commit_dirs {
                if let Some(entry) = dirs.get_mut(key) {
                    entry.churn[w].commits += 1;
                }
            }
            commit_dirs.clear();
        }
    }
    for (w, files) in window_files.iter().enumerate() {
        windows[w].churn.files = files.len() as u64;
    }
    let mut directories: Vec<DirActivity> = dirs
        .into_iter()
        .map(|(name, mut acc)| {
            for (c, f) in acc.churn.iter_mut().zip(&acc.files) {
                c.files = f.len() as u64;
            }
            DirActivity {
                name,
                windows: acc.churn,
                trend: acc.trend,
            }
        })
        .collect();
    // Busiest in the shortest window first, then the next, so current work rises to the top.
    directories.sort_by(|a, b| {
        let key = |d: &DirActivity| d.windows.iter().map(Churn::lines).collect::<Vec<_>>();
        key(b).cmp(&key(a)).then_with(|| a.name.cmp(&b.name))
    });
    Activity {
        depth,
        windows,
        directories,
        branches,
        last_commit_at: last.and_then(|s| Timestamp::from_second(s).ok()),
    }
}

/// Collects the activity section for an opened repository.
///
/// # Errors
/// Returns [`GitError::Read`] when history cannot be read.
pub fn collect(
    repo: &Repo,
    depth: u16,
    now: Timestamp,
    cache: &mut CommitCache,
) -> Result<Activity, GitError> {
    let (commits, branches) = walk(&repo.repo, now, cache)?;
    Ok(summarise(repo, &commits, depth, now, branches))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testrepo::TestRepo;

    const NOW: i64 = 1_790_000_000;

    fn now() -> Timestamp {
        Timestamp::from_second(NOW).unwrap()
    }

    fn window<'a>(a: &'a Activity, label: &str) -> &'a ActivityWindow {
        a.windows.iter().find(|w| w.label == label).unwrap()
    }

    fn dir<'a>(a: &'a Activity, name: &str) -> &'a DirActivity {
        a.directories
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("no dir {name}"))
    }

    /// Three commits: 40 days ago, 3 days ago, 10 minutes ago.
    fn history() -> TestRepo {
        let t = TestRepo::new();
        t.write("old/a.rs", "1\n2\n3\n");
        t.commit("old", NOW - 40 * 86_400);
        t.write("src/a.rs", "1\n2\n");
        t.write("docs/x.md", "x\n");
        t.commit("mid", NOW - 3 * 86_400);
        t.write("src/a.rs", "1\nchanged\n3\n");
        t.commit("recent", NOW - 600);
        t
    }

    #[test]
    fn windows_count_lines_files_and_commits() {
        let t = history();
        let repo = t.open();
        let a = collect(&repo, 1, now(), &mut CommitCache::new()).unwrap();

        let hour = window(&a, "1h");
        assert_eq!(
            hour.churn,
            Churn {
                added: 2,
                removed: 1,
                files: 1,
                commits: 1
            }
        );
        assert_eq!(window(&a, "24h").churn, hour.churn);
        let week = window(&a, "7d");
        assert_eq!(
            week.churn,
            Churn {
                added: 5,
                removed: 1,
                files: 2,
                commits: 2
            }
        );
        assert_eq!(
            window(&a, "30d").churn,
            week.churn,
            "40-day-old commit is outside every window"
        );
        assert_eq!(
            a.last_commit_at,
            Some(Timestamp::from_second(NOW - 600).unwrap())
        );
        assert_eq!(a.branches, 1);
    }

    #[test]
    fn directories_are_ranked_by_recent_work() {
        let t = history();
        let a = collect(&t.open(), 1, now(), &mut CommitCache::new()).unwrap();
        assert_eq!(a.directories[0].name, "src");
        let src = dir(&a, "src");
        assert_eq!(
            src.windows[0],
            Churn {
                added: 2,
                removed: 1,
                files: 1,
                commits: 1
            }
        );
        assert_eq!(src.windows[2].commits, 2);
        let docs = dir(&a, "docs");
        assert_eq!(docs.windows[0], Churn::default());
        assert_eq!(docs.windows[2].added, 1);
        assert!(a.directories.iter().all(|d| d.name != "old"));
    }

    #[test]
    fn trends_put_recent_changes_in_the_last_bucket() {
        let t = history();
        let a = collect(&t.open(), 1, now(), &mut CommitCache::new()).unwrap();
        let day = window(&a, "24h");
        assert_eq!(day.trend.len(), 24);
        assert_eq!(*day.trend.last().unwrap(), 3);
        assert_eq!(day.trend.iter().sum::<u64>(), 3);
        let week = &dir(&a, "src").trend;
        assert_eq!(week.len(), 28);
        assert_eq!(*week.last().unwrap(), 3);
        assert_eq!(week.iter().sum::<u64>(), 5);
    }

    #[test]
    fn other_branches_count_and_merges_do_not_double_count() {
        let t = history();
        t.git(&["checkout", "-q", "-b", "feature"]);
        t.write("feat/f.rs", "a\nb\n");
        t.commit("feature work", NOW - 300);
        t.git(&["checkout", "-q", "main"]);
        let before = collect(&t.open(), 1, now(), &mut CommitCache::new()).unwrap();
        assert_eq!(
            window(&before, "1h").churn.commits,
            2,
            "feature branch is walked"
        );
        assert_eq!(before.branches, 2);

        t.merge("feature", NOW - 60);
        let after = collect(&t.open(), 1, now(), &mut CommitCache::new()).unwrap();
        assert_eq!(window(&after, "1h").churn, window(&before, "1h").churn);
    }

    #[test]
    fn non_code_files_do_not_count() {
        let t = history();
        t.write("Cargo.lock", "a\nb\nc\nd\n");
        t.write("logo.png", "\u{1}\u{2}");
        t.commit("lock", NOW - 120);
        let a = collect(&t.open(), 1, now(), &mut CommitCache::new()).unwrap();
        assert_eq!(
            window(&a, "1h").churn.commits,
            1,
            "a commit of only non-code files is not activity"
        );
        assert!(a.directories.iter().all(|d| d.name != "."));
    }

    #[test]
    fn commits_are_cached_and_pruned() {
        let t = history();
        let repo = t.open();
        let mut cache = CommitCache::new();
        collect(&repo, 1, now(), &mut cache).unwrap();
        assert_eq!(cache.len(), 2, "only commits inside 30 days are kept");
        let first = cache.clone();
        let again = collect(&repo, 1, now(), &mut cache).unwrap();
        assert_eq!(cache, first);
        assert_eq!(window(&again, "7d").churn.added, 5);
    }

    #[test]
    fn a_subdirectory_root_sees_only_its_paths() {
        let t = history();
        let sub = t.open_at("src");
        let a = collect(&sub, 1, now(), &mut CommitCache::new()).unwrap();
        assert_eq!(window(&a, "7d").churn.files, 1);
        assert_eq!(a.directories.len(), 1);
        assert_eq!(a.directories[0].name, ".");
    }

    #[test]
    fn empty_repository_has_no_activity() {
        let t = TestRepo::new();
        let a = collect(&t.open(), 1, now(), &mut CommitCache::new()).unwrap();
        assert!(a.directories.is_empty());
        assert_eq!(a.windows.len(), 4);
        assert_eq!(a.last_commit_at, None);
    }
}
