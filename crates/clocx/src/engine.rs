//! The report engine: owns the caches of one root and computes a fresh [`Report`] on each refresh.
//!
//! Caches live in memory between refreshes (the live view refreshes often)
//! and are written back by [`Engine::persist`]. The baseline that the
//! change column compares with is loaded once, when the engine opens, so a
//! live session shows change since it started rather than since the last
//! refresh.

use std::path::PathBuf;
use std::sync::Arc;

use jiff::Timestamp;
use tracing::{debug, info, warn};

use crate::activity::{self, CommitCache};
use crate::cache::{Counts, Snapshot, Store};
use crate::cli::Args;
use crate::error::Error;
use crate::git::{GitError, Repo};
use crate::model::{Activity, Report, Section, Worktrees};
use crate::progress::{Phase, Progress};
use crate::{totals, worktrees};

/// Computes reports for one root, keeping per-file and per-commit caches warm.
pub struct Engine {
    root: PathBuf,
    depth: u16,
    base: Option<String>,
    store: Store,
    counts: Counts,
    commits: CommitCache,
    baseline: Option<Snapshot>,
    latest: Option<Snapshot>,
    progress: Arc<Progress>,
}

impl Engine {
    /// Opens the engine for the root in `args`, loading caches and the previous run's snapshot.
    ///
    /// # Errors
    /// Returns [`Error::Root`] when the root path cannot be resolved.
    pub fn open(args: &Args) -> Result<Self, Error> {
        let root = args.path.canonicalize().map_err(|source| Error::Root {
            path: args.path.clone(),
            source,
        })?;
        let store = if args.no_cache {
            Store::disabled()
        } else {
            Store::open(args.cache_dir.as_deref(), &root)
        };
        info!(root = %root.display(), depth = args.depth, "engine opened");
        Ok(Self {
            depth: args.depth,
            base: args.base.clone(),
            counts: store.load_counts(),
            commits: store.load_commits(),
            baseline: store.load_snapshot(),
            latest: None,
            progress: Arc::default(),
            store,
            root,
        })
    }

    /// The canonical root this engine reports on.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// The progress counters refreshes and [`Engine::persist`] advance, for a readout to poll.
    pub fn progress(&self) -> Arc<Progress> {
        Arc::clone(&self.progress)
    }

    /// Computes a fresh report; files and commits unchanged since the last refresh come from the caches.
    pub fn refresh(&mut self) -> Report {
        let now = Timestamp::now();
        let scan = totals::scan(
            &self.root,
            self.depth,
            &mut self.counts,
            self.baseline.as_ref(),
            now,
            &self.progress,
        );
        self.latest = Some(scan.snapshot);
        let repo = Repo::discover(&self.root);
        let activity = self.activity_section(&repo, now);
        let worktrees = worktrees_section(&repo, &self.root, self.base.as_deref(), &self.progress);
        self.progress.begin(Phase::Done, None);
        debug!("report refreshed");
        Report {
            root: self.root.display().to_string(),
            generated_at: now,
            totals: scan.totals,
            activity,
            worktrees,
        }
    }

    /// Writes the caches and the latest snapshot (the next run's baseline); failures are logged, never fatal.
    pub fn persist(&self) {
        self.progress.begin(Phase::Saving, None);
        if let Err(e) = self.store.save_counts(&self.counts) {
            warn!(error = %e, "count cache not saved");
        }
        if let Err(e) = self.store.save_commits(&self.commits) {
            warn!(error = %e, "commit cache not saved");
        }
        if let Some(snapshot) = &self.latest
            && let Err(e) = self.store.save_snapshot(snapshot)
        {
            warn!(error = %e, "snapshot not saved; the next run shows no change column");
        }
        self.progress.begin(Phase::Done, None);
    }

    /// The git activity section; outside a repository or on a git error it is unavailable, not fatal.
    fn activity_section(
        &mut self,
        repo: &Result<Repo, GitError>,
        now: Timestamp,
    ) -> Section<Activity> {
        let repo = match repo {
            Ok(r) => r,
            Err(e) => {
                info!(reason = %e, "no git activity");
                return Section::Unavailable {
                    reason: e.to_string(),
                };
            }
        };
        match activity::collect(repo, self.depth, now, &mut self.commits, &self.progress) {
            Ok(a) => Section::Ok(a),
            Err(e) => {
                warn!(error = %e, "git history could not be read");
                Section::Unavailable {
                    reason: e.to_string(),
                }
            }
        }
    }
}

/// The worktree section; outside a repository or on a git error it is unavailable, not fatal.
fn worktrees_section(
    repo: &Result<Repo, GitError>,
    root: &std::path::Path,
    base: Option<&str>,
    progress: &Progress,
) -> Section<Worktrees> {
    let repo = match repo {
        Ok(r) => r,
        Err(e) => {
            return Section::Unavailable {
                reason: e.to_string(),
            };
        }
    };
    match worktrees::collect(repo, root, base, progress) {
        Ok(w) => Section::Ok(w),
        Err(e) => {
            warn!(error = %e, "worktrees could not be read");
            Section::Unavailable {
                reason: e.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args(path: &std::path::Path, cache: &std::path::Path) -> Args {
        Args::parse_from([
            "clocx",
            "--cache-dir",
            cache.to_str().unwrap(),
            path.to_str().unwrap(),
        ])
    }

    #[test]
    fn baseline_stays_fixed_across_refreshes_until_persisted() {
        let tree = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(tree.path().join("a.rs"), "fn a() {}\n").unwrap();
        let mut first = Engine::open(&args(tree.path(), cache.path())).unwrap();
        first.refresh();
        first.persist();

        let mut live = Engine::open(&args(tree.path(), cache.path())).unwrap();
        std::fs::write(tree.path().join("b.rs"), "fn b() {}\n").unwrap();
        assert_eq!(live.refresh().totals.total.code_delta, Some(1));
        std::fs::write(tree.path().join("c.rs"), "fn c() {}\n").unwrap();
        assert_eq!(
            live.refresh().totals.total.code_delta,
            Some(2),
            "still against the opening baseline"
        );
        live.persist();

        let mut next = Engine::open(&args(tree.path(), cache.path())).unwrap();
        assert_eq!(next.refresh().totals.total.code_delta, Some(0));
    }

    #[test]
    fn refreshes_reuse_the_in_memory_cache() {
        let tree = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(tree.path().join("a.rs"), "fn a() {}\n").unwrap();
        let mut engine = Engine::open(&args(tree.path(), cache.path())).unwrap();
        assert_eq!(engine.refresh().totals.cache.counted, 1);
        let again = engine.refresh().totals.cache;
        assert_eq!((again.counted, again.unchanged), (0, 1));
    }
}
