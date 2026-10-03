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
