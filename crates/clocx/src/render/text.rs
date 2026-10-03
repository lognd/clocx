//! The one-shot text report: a header line followed by one table per section.

use super::format;
use super::style::{Theme, paint};
use super::table::{Align, Cell, Table};
use crate::model::{Report, Totals, TotalsRow};

/// Builds the whole text report.
pub fn report(report: &Report, theme: &Theme, rows: usize) -> String {
    let mut out = String::new();
    header(report, theme, &mut out);
    totals(&report.totals, report, theme, rows, &mut out);
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

/// Keeps the first `limit` rows plus any later row that changed; the rest fold into one `N more` row.
fn fold(rows: &[TotalsRow], limit: usize) -> (Vec<&TotalsRow>, Option<TotalsRow>) {
    if limit == 0 || rows.len() <= limit {
        return (rows.iter().collect(), None);
    }
    let changed = |r: &TotalsRow| r.code_delta.is_some_and(|d| d != 0);
    let (mut shown, mut folded) = (Vec::new(), Vec::new());
    for (i, r) in rows.iter().enumerate() {
        if i < limit || changed(r) {
            shown.push(r);
        } else {
            folded.push(r);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sample;
    use anstyle::Style;

    fn plain(report: &Report) -> String {
        let p = Style::new();
        let theme = Theme {
            title: p,
            header: p,
            dim: p,
            added: p,
            removed: p,
            name: p,
            spark: p,
            warn: p,
        };
        super::report(report, &theme, 0)
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
