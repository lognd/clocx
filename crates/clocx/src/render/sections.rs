//! Report sections as renderer-neutral tables and notes, shared by the text report and the live view.
//!
//! Every number, label and color decision lives here once; `text` prints
//! the parts as aligned lines and `live` draws them as dashboard panels.

use super::format;
use super::style::Theme;
use super::table::{Align, Cell, Table};
use crate::model::{
    Activity, Churn, DirActivity, Report, Section, Totals, TotalsRow, WorktreeStatus, Worktrees,
};

/// One piece of a panel: a table, or a dimmed one-line note.
#[derive(Debug, Clone)]
pub enum Part {
    /// An aligned table.
    Table(Table),
    /// A dimmed explanatory line (what a column compares with, why a section is missing).
    Note(String),
}

/// The report split into the panels the views lay out.
#[derive(Debug, Clone)]
pub struct Panels {
    /// Totals by language.
    pub languages: Vec<Part>,
    /// Totals by directory, with the change note.
    pub directories: Vec<Part>,
    /// Activity windows, with the branches note.
    pub activity: Vec<Part>,
    /// Activity by directory ("where work is happening").
    pub work: Vec<Part>,
    /// Worktrees, with the comparison note.
    pub worktrees: Vec<Part>,
}

/// Builds every panel of `report`, folding tables after `limit` rows (0 keeps all).
pub fn panels(report: &Report, theme: &Theme, limit: usize) -> Panels {
    let (languages, directories) = totals(&report.totals, report, theme, limit);
    let (activity, work) = activity(&report.activity, report, theme, limit);
    Panels {
        languages,
        directories,
        activity,
        work,
        worktrees: worktrees(&report.worktrees, report, theme),
    }
}

/// A signed change painted green or red; blank when zero or unknown.
fn delta_cell(delta: Option<i64>, theme: &Theme) -> Cell {
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

/// The languages and directories panels; the directories panel carries the change note.
fn totals(totals: &Totals, report: &Report, theme: &Theme, limit: usize) -> (Vec<Part>, Vec<Part>) {
    if totals.languages.is_empty() {
        return (
            vec![Part::Note("No source files found.".into())],
            Vec::new(),
        );
    }
    let with_delta = totals.baseline_at.is_some();
    let languages = totals_table(
        "Languages".into(),
        "Language",
        &totals.languages,
        &totals.total,
        with_delta,
        limit,
        theme,
    );
    let title = if totals.depth == 1 {
        "Directories".to_owned()
    } else {
        format!("Directories (depth {})", totals.depth)
    };
    let directories = totals_table(
        title,
        "Directory",
        &totals.directories,
        &totals.total,
        with_delta,
        limit,
        theme,
    );
    let mut dir_parts = vec![Part::Table(directories)];
    if let Some(at) = totals.baseline_at {
        dir_parts.push(Part::Note(format!(
            "Change: code lines since the previous run, {}.",
            format::age(at, report.generated_at)
        )));
    }
    (vec![Part::Table(languages)], dir_parts)
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

/// The activity windows panel and the per-directory "where work is happening" panel.
fn activity(
    section: &Section<Activity>,
    report: &Report,
    theme: &Theme,
    limit: usize,
) -> (Vec<Part>, Vec<Part>) {
    let a = match section {
        Section::Ok(a) => a,
        Section::Unavailable { reason } => {
            return (
                vec![Part::Note(format!("Activity: unavailable ({reason})."))],
                Vec::new(),
            );
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
    let last = a.last_commit_at.map_or_else(
        || "no commits yet".to_owned(),
        |t| format!("last commit {}", format::age(t, report.generated_at)),
    );
    let plural = if a.branches == 1 { "" } else { "es" };
    let activity = vec![
        Part::Table(windows),
        Part::Note(format!("Across {} branch{plural}; {last}.", a.branches)),
    ];

    if a.directories.is_empty() {
        return (activity, Vec::new());
    }
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
    (activity, vec![Part::Table(dirs)])
}

/// One worktree's row: name (current marked), branch, uncommitted, ahead, vs base, last activity.
fn worktree_row(w: &WorktreeStatus, report: &Report, theme: &Theme) -> Vec<Cell> {
    let name = if w.current {
        format!("* {}", w.name)
    } else {
        format!("  {}", w.name)
    };
    let name_style = if w.current { theme.header } else { theme.name };
    if let Some(problem) = &w.problem {
        return vec![
            Cell::styled(name, name_style),
            Cell::styled(problem.clone(), theme.warn),
            Cell::plain(""),
            Cell::plain(""),
            Cell::plain(""),
            Cell::plain(""),
            Cell::plain(""),
        ];
    }
    let branch = match (&w.branch, &w.head) {
        (Some(b), _) => Cell::styled(b.clone(), theme.name),
        (None, Some(h)) => Cell::styled(format!("(detached {h})"), theme.dim),
        (None, None) => Cell::styled("(no commits)", theme.dim),
    };
    let ahead = match w.ahead {
        Some(0) | None => Cell::plain(""),
        Some(n) => Cell::plain(format::count(n)),
    };
    vec![
        Cell::styled(name, name_style),
        branch,
        churn_cell(&w.uncommitted, theme),
        quiet_count(w.uncommitted.files),
        ahead,
        w.vs_base
            .as_ref()
            .map_or_else(|| Cell::plain(""), |c| churn_cell(c, theme)),
        Cell::styled(
            w.last_activity
                .map_or_else(String::new, |t| format::age(t, report.generated_at)),
            theme.dim,
        ),
    ]
}

/// The worktrees panel: every worktree and its work in progress, with the comparison note.
fn worktrees(section: &Section<Worktrees>, report: &Report, theme: &Theme) -> Vec<Part> {
    let w = match section {
        Section::Ok(w) => w,
        Section::Unavailable { reason } => {
            return vec![Part::Note(format!("Worktrees: unavailable ({reason})."))];
        }
    };
    let vs = w
        .base
        .as_ref()
        .map_or_else(|| "vs base".to_owned(), |b| format!("vs {b}"));
    let mut table = Table::new(
        "Worktrees",
        &[
            ("Worktree", Align::Left),
            ("Branch", Align::Left),
            ("Uncommitted", Align::Right),
            ("Files", Align::Right),
            ("Ahead", Align::Right),
            (vs.as_str(), Align::Right),
            ("Last activity", Align::Left),
        ],
    );
    for wt in &w.worktrees {
        table.row(worktree_row(wt, report, theme));
    }
    let note = match &w.base {
        Some(b) => format!(
            "Uncommitted: against HEAD. Ahead and vs {b}: since the merge base with {b}, committed or not."
        ),
        None => "No base branch found (main or master; set one with --base).".to_owned(),
    };
    vec![Part::Table(table), Part::Note(note)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::style::paint;

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
