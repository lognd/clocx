//! The one-shot text report: a header line, then the sections as aligned tables and dimmed notes.

use super::sections::{Part, panels};
use super::style::{Theme, paint};
use crate::model::Report;

/// Builds the whole text report, folding tables after `rows` rows (0 keeps all).
pub fn report(report: &Report, theme: &Theme, rows: usize) -> String {
    let mut out = String::new();
    header(report, theme, &mut out);
    let p = panels(report, theme, rows);
    // Groups are separated by a blank line; parts inside a group by one too, except notes.
    let groups = [
        vec![p.languages, p.directories],
        vec![p.activity],
        vec![p.work],
        vec![p.worktrees],
    ];
    for group in groups {
        for parts in group.into_iter().filter(|g| !g.is_empty()) {
            out.push('\n');
            for part in parts {
                match part {
                    Part::Table(t) => t.render(theme, &mut out),
                    Part::Note(n) => {
                        out.push_str(&paint(theme.dim, &n));
                        out.push('\n');
                    }
                }
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Section;
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
    fn worktrees_show_branch_work_and_progress() {
        let out = plain(&sample::report());
        let (_, wt) = out.split_once("Worktrees\n").unwrap();
        let header = wt.lines().next().unwrap();
        assert!(header.contains("vs master"), "{header:?}");
        let cur = wt.lines().find(|l| l.starts_with("* cloc")).unwrap();
        assert!(cur.contains("master"), "{cur:?}");
        let agent = wt.lines().find(|l| l.starts_with("  agent-1")).unwrap();
        assert!(
            agent.contains("ticket/x") && agent.contains("+12 -3") && agent.contains("+40 -3"),
            "{agent:?}"
        );
        assert!(agent.contains("2m ago"), "{agent:?}");
        let gone = wt.lines().find(|l| l.starts_with("  gone")).unwrap();
        assert!(gone.contains("worktree directory is missing"));
        let detached = wt.lines().find(|l| l.starts_with("  probe")).unwrap();
        assert!(detached.contains("(detached abc1234)"));
    }

    #[test]
    fn unavailable_worktrees_say_why() {
        let mut r = sample::report();
        r.worktrees = Section::Unavailable {
            reason: "not a git repository".into(),
        };
        assert!(plain(&r).contains("Worktrees: unavailable (not a git repository)."));
    }
}
