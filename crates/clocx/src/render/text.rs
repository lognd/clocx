//! The one-shot text report: a header line followed by one table per section.

use super::format;
use super::style::{Theme, paint};
use super::table::{Align, Cell, Table};
use crate::model::{Activity, Churn, DirActivity, Report, Section, Totals, TotalsRow};

/// Builds the whole text report.
pub fn report(report: &Report, theme: &Theme, rows: usize) -> String {
    let mut out = String::new();
    header(report, theme, &mut out);
    totals(&report.totals, report, theme, rows, &mut out);
    activity(&report.activity, report, theme, rows, &mut out);
    out
}

/// The first line: tool name, root and timestamp.
fn header(report: &Report, theme: &Theme, out: &mut String) {
    let when = report
        .generated_at
        .strftime("%Y-%m-%d %H:%M UTC")
        .to_string();
    out.push_str(&paint(theme.title, "clocx"));
    out.push_str("  ");
    out.push_str(&paint(theme.name, &report.root));
    out.push_str("  ");
    out.push_str(&paint(theme.dim, &when));
    out.push('\n');
}

/// A signed change painted green or red; blank when zero or unknown.
pub(super) fn delta_cell(delta: Option<i64>, theme: &Theme) -> Cell {
    match delta {
        Some(d) if d > 0 => Cell::styled(format::delta(d), theme.added),
        Some(d) if d < 0 => Cell::styled(format::delta(d), theme.removed),
        _ => Cell::plain(""),
    }
}

/// Splits rows into those shown (the first `limit`, plus any later row `keep` accepts) and those folded away.
fn split<T>(rows: &[T], limit: usize, keep: impl Fn(&T) -> bool) -> (Vec<&T>, Vec<&T>) {
    if limit == 0 || rows.len() <= limit {
        return (rows.iter().collect(), Vec::new());
    }
    rows.iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |(mut shown, mut folded), (i, r)| {
            if i < limit || keep(r) {
                shown.push(r);
            } else {
                folded.push(r);
            }
            (shown, folded)
        },
    )
}

/// Keeps the first `limit` rows plus any later row that changed; the rest fold into one `N more` row.
fn fold(rows: &[TotalsRow], limit: usize) -> (Vec<&TotalsRow>, Option<TotalsRow>) {
    let (shown, folded) = split(rows, limit, |r| r.code_delta.is_some_and(|d| d != 0));
    if folded.is_empty() {
        return (shown, None);
    }
    let mut rest = TotalsRow {
        name: format!("{} more", folded.len()),
        counts: Default::default(),
        code_delta: folded[0].code_delta.map(|_| 0),
    };
    for r in folded {
        rest.counts.files += r.counts.files;
        rest.counts.code += r.counts.code;
        rest.counts.comments += r.counts.comments;
        rest.counts.blanks += r.counts.blanks;
    }
    (shown, Some(rest))
}

/// One totals table (languages or directories) with an optional change column.
fn totals_table(
    title: String,
    first: &str,
    rows: &[TotalsRow],
    total: &TotalsRow,
    with_delta: bool,
    limit: usize,
    theme: &Theme,
) -> Table {
    let mut columns = vec![
        (first, Align::Left),
        ("Files", Align::Right),
        ("Code", Align::Right),
        ("Comment", Align::Right),
        ("Blank", Align::Right),
        ("Lines", Align::Right),
    ];
    if with_delta {
        columns.push(("Change", Align::Right));
    }
    let mut table = Table::new(title, &columns);
    let cells = |r: &TotalsRow, name_style| {
        let mut c = vec![
            Cell::styled(r.name.clone(), name_style),
            Cell::plain(format::count(r.counts.files)),
            Cell::plain(format::count(r.counts.code)),
            Cell::plain(format::count(r.counts.comments)),
            Cell::plain(format::count(r.counts.blanks)),
            Cell::plain(format::count(r.counts.lines())),
        ];
        if with_delta {
            c.push(delta_cell(r.code_delta, theme));
        }
        c
    };
    let (shown, rest) = fold(rows, limit);
    for r in shown {
        table.row(cells(r, theme.name));
    }
    if let Some(rest) = rest {
        table.row(cells(&rest, theme.dim));
    }
    table.total(cells(total, theme.dim));
    table
}

/// The languages and directories tables, and a note on what the change column compares with.
fn totals(totals: &Totals, report: &Report, theme: &Theme, limit: usize, out: &mut String) {
    let with_delta = totals.baseline_at.is_some();
    out.push('\n');
    if totals.languages.is_empty() {
        out.push_str(&paint(theme.dim, "No source files found.\n"));
        return;
    }
    totals_table(
        "Languages".into(),
        "Language",
        &totals.languages,
        &totals.total,
        with_delta,
        limit,
        theme,
    )
    .render(theme, out);
    out.push('\n');
    let title = if totals.depth == 1 {
        "Directories".to_owned()
    } else {
        format!("Directories (depth {})", totals.depth)
    };
    totals_table(
        title,
        "Directory",
        &totals.directories,
        &totals.total,
        with_delta,
        limit,
        theme,
    )
    .render(theme, out);
    if let Some(at) = totals.baseline_at {
        let note = format!(
            "Change: code lines since the previous run, {}.\n",
            format::age(at, report.generated_at)
        );
        out.push_str(&paint(theme.dim, &note));
    }
}

/// Lines added and removed as `+a -r` in green and red; blank when nothing changed.
fn churn_cell(c: &Churn, theme: &Theme) -> Cell {
    let sep = if c.added > 0 && c.removed > 0 {
        " "
    } else {
        ""
    };
    Cell::spans(vec![
        (format::added(c.added), theme.added),
        (sep.to_owned(), anstyle::Style::new()),
        (format::removed(c.removed), theme.removed),
    ])
}

/// A count that is blank when zero, so quiet rows stay quiet.
fn quiet_count(n: u64) -> Cell {
    Cell::plain(if n == 0 {
        String::new()
    } else {
        format::count(n)
    })
}

/// The activity windows table and the per-directory "where work is happening" table.
fn activity(
    section: &Section<Activity>,
    report: &Report,
    theme: &Theme,
    limit: usize,
    out: &mut String,
) {
    out.push('\n');
    let a = match section {
        Section::Ok(a) => a,
        Section::Unavailable { reason } => {
            out.push_str(&paint(
                theme.dim,
                &format!("Activity: unavailable ({reason}).\n"),
            ));
            return;
        }
    };
    let mut windows = Table::new(
        "Activity",
        &[
            ("Window", Align::Left),
            ("Added", Align::Right),
            ("Removed", Align::Right),
            ("Files", Align::Right),
            ("Commits", Align::Right),
            ("Trend", Align::Left),
        ],
    );
    for w in &a.windows {
        windows.row(vec![
            Cell::styled(format!("last {}", w.label), theme.name),
            Cell::styled(format::added(w.churn.added), theme.added),
            Cell::styled(format::removed(w.churn.removed), theme.removed),
            quiet_count(w.churn.files),
            quiet_count(w.churn.commits),
            Cell::styled(format::sparkline(&w.trend), theme.spark),
        ]);
    }
    windows.render(theme, out);
    let last = a.last_commit_at.map_or_else(
        || "no commits yet".to_owned(),
        |t| format!("last commit {}", format::age(t, report.generated_at)),
    );
    let plural = if a.branches == 1 { "" } else { "es" };
    out.push_str(&paint(
        theme.dim,
        &format!("Across {} branch{plural}; {last}.\n", a.branches),
    ));

    if a.directories.is_empty() {
        return;
    }
    out.push('\n');
    let mut columns: Vec<(String, Align)> = vec![("Directory".into(), Align::Left)];
    columns.extend(a.windows.iter().map(|w| (w.label.clone(), Align::Right)));
    columns.push(("7d trend".into(), Align::Left));
    let columns: Vec<(&str, Align)> = columns.iter().map(|(h, al)| (h.as_str(), *al)).collect();
    let mut dirs = Table::new("Where work is happening", &columns);
    // Anything touched in the last day stays visible however long the table is.
    let (shown, folded) = split(&a.directories, limit, |d: &DirActivity| {
        d.windows.iter().take(2).any(|c| c.lines() > 0)
    });
    let row = |name: Cell, d: &DirActivity| {
        let mut cells = vec![name];
        cells.extend(d.windows.iter().map(|c| churn_cell(c, theme)));
        cells.push(Cell::styled(format::sparkline(&d.trend), theme.spark));
        cells
    };
    for d in shown {
        dirs.row(row(Cell::styled(d.name.clone(), theme.name), d));
    }
    if !folded.is_empty() {
        let mut rest = DirActivity {
            name: String::new(),
            windows: vec![Churn::default(); a.windows.len()],
            trend: vec![0; folded[0].trend.len()],
        };
        for d in &folded {
            for (r, c) in rest.windows.iter_mut().zip(&d.windows) {
                r.added += c.added;
                r.removed += c.removed;
            }
            for (r, t) in rest.trend.iter_mut().zip(&d.trend) {
                *r += t;
            }
        }
        dirs.row(row(
            Cell::styled(format!("{} more", folded.len()), theme.dim),
            &rest,
        ));
    }
    dirs.render(theme, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sample;

    fn plain(report: &Report) -> String {
        super::report(report, &Theme::plain(), 0)
    }

    #[test]
    fn totals_tables_show_counts_and_change() {
        let out = plain(&sample::report());
        assert!(out.contains("Languages\n"));
        assert!(out.contains("Directories\n"));
        let rust = out.lines().find(|l| l.starts_with("Rust")).unwrap();
        assert!(rust.contains("3,421") && rust.ends_with("+120"), "{rust:?}");
        let py = out.lines().find(|l| l.starts_with("Python")).unwrap();
        assert!(py.ends_with("-3"), "{py:?}");
        let dot = out.lines().find(|l| l.starts_with(". ")).unwrap();
        assert!(
            dot.trim_end().ends_with('0') || !dot.contains('+'),
            "zero change stays blank: {dot:?}"
        );
        assert!(out.contains("Change: code lines since the previous run, 2h ago."));
    }

    #[test]
    fn no_baseline_means_no_change_column() {
        let mut r = sample::report();
        r.totals.baseline_at = None;
        let out = plain(&r);
        assert!(!out.contains("Change"));
    }

    #[test]
    fn deeper_grouping_is_named_in_the_title() {
        let mut r = sample::report();
        r.totals.depth = 2;
        assert!(plain(&r).contains("Directories (depth 2)"));
    }

    #[test]
    fn empty_tree_says_so() {
        let mut r = sample::report();
        r.totals.languages.clear();
        assert!(plain(&r).contains("No source files found."));
    }

    #[test]
    fn activity_shows_windows_and_directories() {
        let out = plain(&sample::report());
        assert!(out.contains("Activity\n"));
        let hour = out.lines().find(|l| l.starts_with("last 1h")).unwrap();
        assert!(hour.contains("+42") && hour.contains("-10"), "{hour:?}");
        assert!(out.contains("Across 3 branches; last commit 5m ago."));
        let (_, work) = out.split_once("Where work is happening\n").unwrap();
        let crates = work.lines().find(|l| l.starts_with("crates")).unwrap();
        assert!(crates.contains("+42 -10"), "{crates:?}");
        let docs = work.lines().find(|l| l.starts_with("docs")).unwrap();
        assert!(docs.contains("+5") && !docs.contains("+5 -"), "{docs:?}");
    }

    #[test]
    fn unavailable_activity_says_why() {
        let mut r = sample::report();
        r.activity = Section::Unavailable {
            reason: "not a git repository".into(),
        };
        let out = plain(&r);
        assert!(out.contains("Activity: unavailable (not a git repository)."));
        assert!(!out.contains("Where work is happening"));
    }

    #[test]
    fn split_keeps_rows_the_predicate_wants() {
        let rows = [1, 2, 30, 4];
        let (shown, folded) = split(&rows, 1, |r| *r > 10);
        assert_eq!(shown, [&1, &30]);
        assert_eq!(folded, [&2, &4]);
        assert_eq!(split(&rows, 0, |_| false).1.len(), 0);
    }

    #[test]
    fn long_tables_fold_but_keep_changed_rows() {
        let row = |name: &str, code, delta| TotalsRow {
            name: name.into(),
            counts: crate::model::LineCounts {
                files: 1,
                code,
                comments: 0,
                blanks: 0,
            },
            code_delta: Some(delta),
        };
        let rows = vec![
            row("a", 40, 0),
            row("b", 30, 0),
            row("c", 20, 5),
            row("d", 10, 0),
            row("e", 5, 0),
        ];
        let (shown, rest) = fold(&rows, 1);
        let names: Vec<&str> = shown.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["a", "c"]);
        let rest = rest.unwrap();
        assert_eq!(
            (rest.name.as_str(), rest.counts.files, rest.counts.code),
            ("3 more", 3, 45)
        );
        assert_eq!(fold(&rows, 0).0.len(), 5);
        assert!(fold(&rows, 9).1.is_none());
    }

    #[test]
    fn deltas_are_colored_by_sign() {
        let t = Theme::default();
        let up = delta_cell(Some(3), &t);
        let down = delta_cell(Some(-3), &t);
        let mut a = Table::new("t", &[("x", Align::Right)]);
        a.row(vec![up]);
        a.row(vec![down]);
        let mut out = String::new();
        a.render(&t, &mut out);
        assert!(out.contains(&paint(t.added, "+3")));
        assert!(out.contains(&paint(t.removed, "-3")));
    }
}
