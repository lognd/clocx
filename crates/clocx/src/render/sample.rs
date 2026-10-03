//! A fixed report used by renderer tests.

use crate::model::{CacheUse, LineCounts, Report, Totals, TotalsRow};

/// Builds one row.
fn row(
    name: &str,
    files: u64,
    code: u64,
    comments: u64,
    blanks: u64,
    delta: Option<i64>,
) -> TotalsRow {
    TotalsRow {
        name: name.into(),
        counts: LineCounts {
            files,
            code,
            comments,
            blanks,
        },
        code_delta: delta,
    }
}

/// A small report with a baseline, so every column is populated.
pub fn report() -> Report {
    Report {
        root: "/tmp/x".into(),
        generated_at: "2026-10-03T12:00:00Z".parse().unwrap(),
        totals: Totals {
            depth: 1,
            languages: vec![
                row("Rust", 12, 3_421, 210, 402, Some(120)),
                row("Python", 1, 7, 1, 0, Some(-3)),
            ],
            directories: vec![
                row("crates", 12, 3_421, 210, 402, Some(120)),
                row(".", 1, 7, 1, 0, Some(0)),
            ],
            total: row("Total", 13, 3_428, 211, 402, Some(117)),
            baseline_at: Some("2026-10-03T10:00:00Z".parse().unwrap()),
            cache: CacheUse::default(),
        },
    }
}
