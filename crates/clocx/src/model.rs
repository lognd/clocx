//! Typed report data: computed by the counting and git modules, turned into output by `render`.

use jiff::Timestamp;
use serde::Serialize;

/// Everything one clocx run knows; the renderer is a pure function of this.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The directory the report covers, as given (canonicalised).
    pub root: String,
    /// When the report was computed.
    pub generated_at: Timestamp,
    /// Line totals by language and by directory.
    pub totals: Totals,
    /// Lines added and removed in recent git history; unavailable outside a repository.
    pub activity: Section<Activity>,
    /// Every worktree of the repository with its work in progress; unavailable outside a repository.
    pub worktrees: Section<Worktrees>,
}

/// A report section that may be unavailable (for example git data outside a repository).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Section<T> {
    /// The section was computed.
    Ok(T),
    /// The section could not be computed; `reason` says why, for people.
    Unavailable {
        /// Why, in a few words.
        reason: String,
    },
}

impl<T> Section<T> {
    /// The data when available.
    pub fn ok(&self) -> Option<&T> {
        match self {
            Section::Ok(t) => Some(t),
            Section::Unavailable { .. } => None,
        }
    }
}

/// Lines added and removed, files touched and commits made, over some span.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Churn {
    /// Lines added.
    pub added: u64,
    /// Lines removed.
    pub removed: u64,
    /// Distinct files touched.
    pub files: u64,
    /// Commits.
    pub commits: u64,
}

impl Churn {
    /// Lines added plus removed: how much changed.
    pub fn lines(&self) -> u64 {
        self.added + self.removed
    }
}

/// Activity over one window (for example the last hour) with a bucketed trend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivityWindow {
    /// Short label: `1h`, `24h`, `7d`, `30d`.
    pub label: String,
    /// Window length in seconds.
    pub seconds: i64,
    /// What changed in the window.
    pub churn: Churn,
    /// Lines changed per bucket, oldest first, for a sparkline.
    pub trend: Vec<u64>,
}

/// Activity in one directory, per window (same order as [`Activity::windows`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirActivity {
    /// Directory at the report depth (`.` for the root itself).
    pub name: String,
    /// Churn per window.
    pub windows: Vec<Churn>,
    /// Lines changed per bucket over the 7-day window, oldest first.
    pub trend: Vec<u64>,
}

/// Recent change activity from git history, across all local branches.
#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    /// Directory grouping depth.
    pub depth: u16,
    /// Windows from shortest to longest.
    pub windows: Vec<ActivityWindow>,
    /// Directories with activity in the longest window, busiest recently first.
    pub directories: Vec<DirActivity>,
    /// Branch tips the history walk started from.
    pub branches: u64,
    /// Time of the newest non-merge commit seen.
    pub last_commit_at: Option<Timestamp>,
}

/// Files and code, comment and blank lines of some group of files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LineCounts {
    /// Number of files.
    pub files: u64,
    /// Lines of code.
    pub code: u64,
    /// Comment lines.
    pub comments: u64,
    /// Blank lines.
    pub blanks: u64,
}

impl LineCounts {
    /// All lines: code, comments and blanks.
    pub fn lines(&self) -> u64 {
        self.code + self.comments + self.blanks
    }

    /// Adds one file's counts.
    pub fn add_file(&mut self, code: u64, comments: u64, blanks: u64) {
        self.files += 1;
        self.code += code;
        self.comments += comments;
        self.blanks += blanks;
    }
}

/// One row of a totals table: a language or a directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TotalsRow {
    /// Language name or directory path (`.` for the root itself).
    pub name: String,
    /// The group's counts now.
    pub counts: LineCounts,
    /// Change in code lines since the baseline; `None` when there is no comparable baseline.
    pub code_delta: Option<i64>,
}

/// How the per-file cache served this run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CacheUse {
    /// Files whose size and mtime matched, so they were not even read.
    pub unchanged: u64,
    /// Files read and hashed whose content was already counted.
    pub hash_hits: u64,
    /// Files actually counted this run.
    pub counted: u64,
    /// Files skipped because they could not be read.
    pub unreadable: u64,
}

/// Line totals by language and by directory, with change since the baseline.
#[derive(Debug, Clone, Serialize)]
pub struct Totals {
    /// Directory grouping depth (1 = top-level directories).
    pub depth: u16,
    /// Rows by language, most code first.
    pub languages: Vec<TotalsRow>,
    /// Rows by directory at `depth`, most code first.
    pub directories: Vec<TotalsRow>,
    /// The sum over all files.
    pub total: TotalsRow,
    /// When the baseline the deltas compare against was taken (the previous run).
    pub baseline_at: Option<Timestamp>,
    /// Cache statistics of this scan.
    pub cache: CacheUse,
}

/// One worktree and the work in progress in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorktreeStatus {
    /// Short name: the worktree directory's name.
    pub name: String,
    /// Absolute path of the worktree.
    pub path: String,
    /// Whether the report root lies in this worktree.
    pub current: bool,
    /// Checked-out branch, `None` when HEAD is detached or unborn.
    pub branch: Option<String>,
    /// Abbreviated HEAD commit id, `None` before the first commit.
    pub head: Option<String>,
    /// Uncommitted changes against HEAD: lines added and removed, files touched (`commits` is 0).
    pub uncommitted: Churn,
    /// Commits on HEAD that the base branch does not have; `None` without a base.
    pub ahead: Option<u64>,
    /// Everything since the merge base with the base branch, committed or not; `None` without a base.
    pub vs_base: Option<Churn>,
    /// Newest of the HEAD commit time and the mtimes of changed files.
    pub last_activity: Option<Timestamp>,
    /// Why this worktree could not be read, when it could not.
    pub problem: Option<String>,
}

/// All worktrees of the repository, most recently active first.
#[derive(Debug, Clone, Serialize)]
pub struct Worktrees {
    /// The base branch that `ahead` and `vs_base` compare with, if one was found.
    pub base: Option<String>,
    /// One entry per worktree.
    pub worktrees: Vec<WorktreeStatus>,
}
