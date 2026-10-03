//! Shared git access: opening the repository of a report root and the typed error for git reads.

use std::path::{Path, PathBuf};

use tracing::debug;

/// Why reading git state failed.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The root is not inside a git repository (or it cannot be opened).
    #[error("not a git repository")]
    NotARepo,
    /// The repository is bare, so there is no working tree to report on.
    #[error("bare repository has no working tree")]
    Bare,
    /// A git read failed; `context` says which.
    #[error("{context}: {source}")]
    Read {
        /// What was being read.
        context: &'static str,
        /// The gix error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Attaches context to a gix error, for `map_err`.
pub fn read_err<E: std::error::Error + Send + Sync + 'static>(
    context: &'static str,
) -> impl FnOnce(E) -> GitError {
    move |e| GitError::Read {
        context,
        source: Box::new(e),
    }
}

/// An opened repository plus where the report root sits inside its working tree.
pub struct Repo {
    /// The repository handle.
    pub repo: gix::Repository,
    /// The working tree root.
    pub workdir: PathBuf,
    /// The report root relative to the working tree, with forward slashes; empty at the top.
    pub prefix: String,
}

impl Repo {
    /// Discovers the repository containing `root` (which must be canonical).
    ///
    /// # Errors
    /// [`GitError::NotARepo`] outside a repository, [`GitError::Bare`] for a bare one.
    pub fn discover(root: &Path) -> Result<Self, GitError> {
        let repo = gix::discover(root).map_err(|e| {
            debug!(error = %e, "git discovery failed");
            GitError::NotARepo
        })?;
        let workdir = repo.workdir().ok_or(GitError::Bare)?.to_path_buf();
        let workdir = workdir.canonicalize().unwrap_or(workdir);
        let prefix = root
            .strip_prefix(&workdir)
            .map(|p| {
                p.components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();
        debug!(workdir = %workdir.display(), prefix, "opened repository");
        Ok(Self {
            repo,
            workdir,
            prefix,
        })
    }

    /// The path of `repo_path` relative to the report root, or `None` when it lies outside it.
    pub fn relative<'a>(&self, repo_path: &'a str) -> Option<&'a str> {
        if self.prefix.is_empty() {
            return Some(repo_path);
        }
        repo_path
            .strip_prefix(self.prefix.as_str())?
            .strip_prefix('/')
    }
}

/// Throwaway repositories with commits at chosen times, for tests.
#[cfg(test)]
pub mod testrepo {
    use std::fs;
    use std::path::{Path, PathBuf};
    // frob:accept PROC001 because="test fixtures drive the git CLI to build histories with fixed dates; PROC001 targets frob's own crates"
    use std::process::Command;

    use super::Repo;

    /// A temporary repository on branch `main`.
    pub struct TestRepo {
        dir: tempfile::TempDir,
    }

    impl TestRepo {
        /// Initialises an empty repository.
        pub fn new() -> Self {
            let t = Self {
                dir: tempfile::tempdir().unwrap(),
            };
            t.git(&["init", "-q", "-b", "main"]);
            t
        }

        /// The working tree root, canonical.
        pub fn path(&self) -> PathBuf {
            self.dir.path().canonicalize().unwrap()
        }

        /// Runs git in the repository and returns stdout; panics on failure.
        pub fn git(&self, args: &[&str]) -> String {
            self.git_at(&self.path(), args, None)
        }

        /// Runs git in `dir` with an optional fixed date for author and committer.
        pub fn git_at(&self, dir: &Path, args: &[&str], date: Option<i64>) -> String {
            let mut cmd = Command::new("git");
            cmd.current_dir(dir)
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.com",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1");
            if let Some(d) = date {
                let d = format!("@{d} +0000");
                cmd.env("GIT_AUTHOR_DATE", &d).env("GIT_COMMITTER_DATE", &d);
            }
            let out = cmd.output().expect("git runs");
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap()
        }

        /// Writes a file (creating directories) in the working tree.
        pub fn write(&self, rel: &str, text: &str) {
            let p = self.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }

        /// Stages everything and commits at `time` (seconds since the epoch).
        pub fn commit(&self, msg: &str, time: i64) {
            self.git(&["add", "-A"]);
            self.git_at(&self.path(), &["commit", "-q", "-m", msg], Some(time));
        }

        /// Merges `branch` into the current branch with a merge commit at `time`.
        pub fn merge(&self, branch: &str, time: i64) {
            self.git_at(
                &self.path(),
                &["merge", "-q", "--no-ff", "-m", "merge", branch],
                Some(time),
            );
        }

        /// Opens the repository with the report root at the top.
        pub fn open(&self) -> Repo {
            Repo::discover(&self.path()).unwrap()
        }

        /// Opens the repository with the report root at `rel`.
        pub fn open_at(&self, rel: &str) -> Repo {
            Repo::discover(&self.path().join(rel)).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_prefix(prefix: &str) -> Repo {
        let t = testrepo::TestRepo::new();
        let mut repo = t.open();
        repo.prefix = prefix.into();
        repo
    }

    #[test]
    fn relative_paths_respect_the_prefix() {
        let top = with_prefix("");
        assert_eq!(top.relative("src/a.rs"), Some("src/a.rs"));
        let sub = with_prefix("crates/x");
        assert_eq!(sub.relative("crates/x/src/a.rs"), Some("src/a.rs"));
        assert_eq!(sub.relative("crates/xy/a.rs"), None);
        assert_eq!(sub.relative("docs/a.md"), None);
    }

    #[test]
    fn outside_a_repository_is_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        // A temp dir may sit inside some repository on odd machines; only assert when it does not.
        if gix::discover(tmp.path()).is_err() {
            assert!(matches!(
                Repo::discover(tmp.path()),
                Err(GitError::NotARepo)
            ));
        }
    }
}
