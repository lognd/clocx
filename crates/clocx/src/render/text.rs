//! The one-shot text report: a header line followed by one table per section.

use super::style::{Theme, paint};
use crate::model::Report;

/// Builds the whole text report.
pub fn report(report: &Report, theme: &Theme) -> String {
    let mut out = String::new();
    header(report, theme, &mut out);
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
