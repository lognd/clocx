//! On-disk state of one report root: per-file counts keyed by content hash, and the last run's snapshot.
//!
//! The cache is an optimisation, so a missing or corrupt file is logged and
//! treated as empty; it never fails a run. Files live under
//! `<cache dir>/clocx/<hash of the canonical root>/`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Bumped when the on-disk shape changes; older files are discarded.
const VERSION: u32 = 1;
const COUNTS_FILE: &str = "counts.json";
const SNAPSHOT_FILE: &str = "snapshot.json";
const COMMITS_FILE: &str = "commits.json";

/// Per-commit churn by commit id, as stored on disk.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Commits {
    version: u32,
    commits: crate::activity::CommitCache,
}

/// Code, comment and blank lines of one file's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counted {
    /// Lines of code.
    pub code: u64,
    /// Comment lines.
    pub comments: u64,
    /// Blank lines.
    pub blanks: u64,
}

/// What a path looked like when it was last hashed; a match skips reading it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// File size in bytes.
    pub size: u64,
    /// Modification time in nanoseconds since the epoch.
    pub mtime_ns: i128,
    /// The content key (`<xxh3 hex>:<language>`) the file had at this stamp.
    pub key: String,
}

/// Per-file counts by content key, plus the stat shortcut by path.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Counts {
    version: u32,
    /// Content key to counts.
    pub by_key: HashMap<String, Counted>,
    /// Relative path to its last stamp.
    pub by_path: HashMap<String, Stamp>,
}

/// Code lines per language and per directory at the end of a run; the baseline for deltas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    version: u32,
    /// When the snapshot was taken.
    pub taken_at: Option<Timestamp>,
    /// Directory depth the directory map was grouped at; a different depth makes it incomparable.
    pub depth: u16,
    /// Code lines by language name.
    pub languages: BTreeMap<String, u64>,
    /// Code lines by directory.
    pub directories: BTreeMap<String, u64>,
    /// Total code lines.
    pub total: u64,
}

impl Snapshot {
    /// A snapshot of the given values, stamped with the current format version.
    pub fn new(
        taken_at: Timestamp,
        depth: u16,
        languages: BTreeMap<String, u64>,
        directories: BTreeMap<String, u64>,
        total: u64,
    ) -> Self {
        Self {
            version: VERSION,
            taken_at: Some(taken_at),
            depth,
            languages,
            directories,
            total,
        }
    }
}

/// Why saving cache state failed.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    /// The cache directory or file could not be written.
    #[error("cannot write {path}: {source}")]
    Io {
        /// The file or directory being written.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The state could not be encoded (a bug, kept typed rather than panicking).
    #[error("cannot encode {path}: {source}")]
    Encode {
        /// The file being written.
        path: PathBuf,
        /// The encoder error.
        source: serde_json::Error,
    },
}

/// The cache directory of one root, or nowhere when caching is off.
#[derive(Debug, Clone)]
pub struct Store {
    dir: Option<PathBuf>,
}

impl Store {
    /// A store under `base` (default: the user cache dir) keyed by the canonical `root`.
    pub fn open(base: Option<&Path>, root: &Path) -> Self {
        let base = base
            .map(Path::to_path_buf)
            .or_else(|| dirs::cache_dir().map(|d| d.join("clocx")));
        let key = format!(
            "{:016x}",
            xxhash_rust::xxh3::xxh3_64(root.to_string_lossy().as_bytes())
        );
        let dir = base.map(|b| b.join(key));
        debug!(dir = ?dir, "cache store");
        Self { dir }
    }

    /// A store that reads nothing and writes nothing (`--no-cache`).
    pub fn disabled() -> Self {
        Self { dir: None }
    }

    /// Loads the counts cache; empty when absent, stale or unreadable.
    pub fn load_counts(&self) -> Counts {
        self.load::<Counts>(COUNTS_FILE)
            .filter(|c| c.version == VERSION)
            .unwrap_or_else(|| Counts {
                version: VERSION,
                ..Counts::default()
            })
    }

    /// Loads the previous run's snapshot, if there is a usable one.
    pub fn load_snapshot(&self) -> Option<Snapshot> {
        self.load::<Snapshot>(SNAPSHOT_FILE)
            .filter(|s| s.version == VERSION)
    }

    /// Loads the per-commit churn cache; empty when absent, stale or unreadable.
    pub fn load_commits(&self) -> crate::activity::CommitCache {
        self.load::<Commits>(COMMITS_FILE)
            .filter(|c| c.version == VERSION)
            .map(|c| c.commits)
            .unwrap_or_default()
    }

    /// Writes the per-commit churn cache.
    ///
    /// # Errors
    /// Returns [`SaveError`] when the file cannot be encoded or written.
    pub fn save_commits(&self, commits: &crate::activity::CommitCache) -> Result<(), SaveError> {
        self.save(
            COMMITS_FILE,
            &Commits {
                version: VERSION,
                commits: commits.clone(),
            },
        )
    }

    /// Writes the counts cache.
    ///
    /// # Errors
    /// Returns [`SaveError`] when the file cannot be encoded or written.
    pub fn save_counts(&self, counts: &Counts) -> Result<(), SaveError> {
        self.save(COUNTS_FILE, counts)
    }

    /// Writes the snapshot that the next run compares against.
    ///
    /// # Errors
    /// Returns [`SaveError`] when the file cannot be encoded or written.
    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<(), SaveError> {
        self.save(SNAPSHOT_FILE, snapshot)
    }

    /// Reads and decodes one file; any failure is logged and yields `None`.
    fn load<T: for<'de> Deserialize<'de>>(&self, name: &str) -> Option<T> {
        let path = self.dir.as_ref()?.join(name);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                debug!(path = %path.display(), "no cache file yet");
                return None;
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "cache unreadable; starting fresh");
                return None;
            }
        };
        match serde_json::from_slice(&bytes) {
            Ok(v) => Some(v),
            Err(e) => {
                warn!(path = %path.display(), error = %e, "cache corrupt; starting fresh");
                None
            }
        }
    }

    /// Encodes and writes one file atomically (temp file then rename).
    fn save<T: Serialize>(&self, name: &str, value: &T) -> Result<(), SaveError> {
        let Some(dir) = &self.dir else { return Ok(()) };
        let path = dir.join(name);
        fs::create_dir_all(dir).map_err(|source| SaveError::Io {
            path: dir.clone(),
            source,
        })?;
        let bytes = serde_json::to_vec(value).map_err(|source| SaveError::Encode {
            path: path.clone(),
            source,
        })?;
        let tmp = dir.join(format!("{name}.tmp"));
        fs::write(&tmp, bytes).map_err(|source| SaveError::Io {
            path: tmp.clone(),
            source,
        })?;
        fs::rename(&tmp, &path).map_err(|source| SaveError::Io {
            path: path.clone(),
            source,
        })?;
        debug!(path = %path.display(), "cache saved");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_counts_and_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(Some(tmp.path()), Path::new("/some/root"));
        assert!(store.load_snapshot().is_none());
        assert!(store.load_counts().by_key.is_empty());

        let mut counts = store.load_counts();
        counts.by_key.insert(
            "ab:Rust".into(),
            Counted {
                code: 3,
                comments: 1,
                blanks: 0,
            },
        );
        store.save_counts(&counts).unwrap();
        assert_eq!(store.load_counts().by_key["ab:Rust"].code, 3);

        let snap = Snapshot::new(
            Timestamp::now(),
            1,
            BTreeMap::from([("Rust".into(), 3)]),
            BTreeMap::new(),
            3,
        );
        store.save_snapshot(&snap).unwrap();
        assert_eq!(store.load_snapshot(), Some(snap));
    }

    #[test]
    fn corrupt_files_are_treated_as_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(Some(tmp.path()), Path::new("/r"));
        store.save_counts(&store.load_counts()).unwrap();
        let dir = store.dir.clone().unwrap();
        fs::write(dir.join(COUNTS_FILE), b"{not json").unwrap();
        fs::write(dir.join(SNAPSHOT_FILE), b"[]").unwrap();
        assert!(store.load_counts().by_key.is_empty());
        assert!(store.load_snapshot().is_none());
    }

    #[test]
    fn disabled_store_reads_and_writes_nothing() {
        let store = Store::disabled();
        store.save_counts(&Counts::default()).unwrap();
        assert!(store.load_snapshot().is_none());
    }
}
