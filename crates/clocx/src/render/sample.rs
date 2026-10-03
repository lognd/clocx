//! A fixed report used by renderer tests.

use crate::model::{
    Activity, ActivityWindow, CacheUse, Churn, DirActivity, LineCounts, Report, Section, Totals,
    TotalsRow,
};

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
        activity: Section::Ok(activity()),
    }
}

/// Churn shorthand.
fn churn(added: u64, removed: u64, files: u64, commits: u64) -> Churn {
    Churn {
        added,
        removed,
        files,
        commits,
    }
}

/// Activity with work in the last hour in `crates` and older work in `docs`.
fn activity() -> Activity {
    let hour = churn(42, 10, 3, 2);
    let week = churn(1_300, 210, 20, 9);
    let window = |label: &str, seconds, c, n| ActivityWindow {
        label: label.into(),
        seconds,
        churn: c,
        trend: vec![0, 1, 5, n],
    };
    Activity {
        depth: 1,
        windows: vec![
            window("1h", 3_600, hour, 52),
            window("24h", 86_400, hour, 52),
            window("7d", 604_800, week, 52),
            window("30d", 2_592_000, week, 52),
        ],
        directories: vec![
            DirActivity {
                name: "crates".into(),
                windows: vec![
                    hour,
                    hour,
                    churn(1_295, 210, 18, 8),
                    churn(1_295, 210, 18, 8),
                ],
                trend: vec![0, 3, 9],
            },
            DirActivity {
                name: "docs".into(),
                windows: vec![
                    Churn::default(),
                    Churn::default(),
                    churn(5, 0, 2, 1),
                    churn(5, 0, 2, 1),
                ],
                trend: vec![5, 0, 0],
            },
        ],
        branches: 3,
        last_commit_at: Some("2026-10-03T11:55:00Z".parse().unwrap()),
    }
}
