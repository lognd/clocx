//! Aligned text tables with a title, bold headers, rules and a dimmed total row.

use anstyle::Style;
use unicode_width::UnicodeWidthStr;

use super::style::{Theme, paint};

/// Horizontal rule glyph.
const RULE: char = '\u{2500}';
/// Spaces between columns.
const GAP: usize = 2;

/// Which side of its column a cell hugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Text columns.
    Left,
    /// Number columns.
    Right,
}

/// One table cell: one or more runs of text, each with its own style.
#[derive(Debug, Clone)]
pub struct Cell {
    spans: Vec<(String, Style)>,
}

/// Replaces control characters (from odd file names) so cells cannot inject terminal escapes.
fn sanitize(text: String) -> String {
    if text.chars().any(char::is_control) {
        text.chars()
            .map(|c| if c.is_control() { '?' } else { c })
            .collect()
    } else {
        text
    }
}

impl Cell {
    /// An unstyled cell.
    pub fn plain(text: impl Into<String>) -> Self {
        Self::styled(text, Style::new())
    }

    /// A cell painted with `style`.
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self::spans(vec![(text.into(), style)])
    }

    /// A cell of several styled runs, drawn side by side (empty runs are dropped).
    pub fn spans(spans: Vec<(String, Style)>) -> Self {
        Self {
            spans: spans
                .into_iter()
                .filter(|(t, _)| !t.is_empty())
                .map(|(t, s)| (sanitize(t), s))
                .collect(),
        }
    }

    /// Display width of all runs together.
    fn width(&self) -> usize {
        self.spans.iter().map(|(t, _)| t.width()).sum()
    }

    /// The runs painted; `over` replaces unstyled runs (used for the total row).
    fn painted(&self, over: Option<Style>) -> String {
        self.spans
            .iter()
            .map(|(t, s)| match over {
                Some(o) if *s == Style::new() => paint(o, t),
                _ => paint(*s, t),
            })
            .collect()
    }

    /// The text without styles.
    #[cfg(test)]
    fn text(&self) -> String {
        self.spans.iter().map(|(t, _)| t.as_str()).collect()
    }
}

/// A table under construction; `render` lays it out.
#[derive(Debug, Clone)]
pub struct Table {
    title: String,
    columns: Vec<(String, Align)>,
    rows: Vec<Vec<Cell>>,
    total: Option<Vec<Cell>>,
}

impl Table {
    /// Starts a table with a title and `(header, alignment)` columns.
    pub fn new(title: impl Into<String>, columns: &[(&str, Align)]) -> Self {
        Self {
            title: title.into(),
            columns: columns.iter().map(|(h, a)| ((*h).to_owned(), *a)).collect(),
            rows: Vec::new(),
            total: None,
        }
    }

    /// Appends a data row; it must have one cell per column.
    pub fn row(&mut self, cells: Vec<Cell>) {
        debug_assert_eq!(
            cells.len(),
            self.columns.len(),
            "row width must match columns"
        );
        self.rows.push(cells);
    }

    /// Sets the total row, drawn dimmed under a rule.
    pub fn total(&mut self, cells: Vec<Cell>) {
        debug_assert_eq!(
            cells.len(),
            self.columns.len(),
            "total width must match columns"
        );
        self.total = Some(cells);
    }

    /// Whether the table has no data rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Lays the table out into `out`, one line per row, padded by display width.
    pub fn render(&self, theme: &Theme, out: &mut String) {
        let widths = self.widths();
        let full: usize = widths.iter().sum::<usize>() + GAP * widths.len().saturating_sub(1);

        out.push_str(&paint(theme.title, &self.title));
        out.push('\n');

        let header: Vec<Cell> = self
            .columns
            .iter()
            .map(|(h, _)| Cell::styled(h.clone(), theme.header))
            .collect();
        self.line(&header, &widths, None, out);
        let rule: String = std::iter::repeat_n(RULE, full).collect();
        out.push_str(&paint(theme.dim, &rule));
        out.push('\n');
        for row in &self.rows {
            self.line(row, &widths, None, out);
        }
        if let Some(total) = &self.total {
            out.push_str(&paint(theme.dim, &rule));
            out.push('\n');
            self.line(total, &widths, Some(theme.dim), out);
        }
    }

    /// Column widths: the widest of header and every cell, by display width.
    fn widths(&self) -> Vec<usize> {
        let mut widths: Vec<usize> = self.columns.iter().map(|(h, _)| h.width()).collect();
        for row in self.rows.iter().chain(self.total.iter()) {
            for (w, cell) in widths.iter_mut().zip(row) {
                *w = (*w).max(cell.width());
            }
        }
        widths
    }

    /// Writes one padded row; `over` replaces plain cell styles (used for the total row).
    fn line(&self, cells: &[Cell], widths: &[usize], over: Option<Style>, out: &mut String) {
        let mut line = String::new();
        for (i, ((cell, width), (_, align))) in
            cells.iter().zip(widths).zip(&self.columns).enumerate()
        {
            let pad = width.saturating_sub(cell.width());
            let painted = cell.painted(over);
            if i > 0 {
                line.push_str(&" ".repeat(GAP));
            }
            match align {
                Align::Left => {
                    line.push_str(&painted);
                    // No trailing padding on the last column.
                    if i + 1 < cells.len() {
                        line.push_str(&" ".repeat(pad));
                    }
                }
                Align::Right => {
                    line.push_str(&" ".repeat(pad));
                    line.push_str(&painted);
                }
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_are_replaced() {
        assert_eq!(Cell::plain("a\u{1b}[31mb\n").text(), "a?[31mb?");
        assert_eq!(Cell::plain("plain").text(), "plain");
    }

    #[test]
    fn spans_are_measured_together_and_painted_separately() {
        let t = Theme::default();
        let c = Cell::spans(vec![
            ("+3".into(), t.added),
            (" ".into(), Style::new()),
            ("-1".into(), t.removed),
            (String::new(), t.dim),
        ]);
        assert_eq!(c.width(), 5);
        assert_eq!(
            c.painted(None),
            format!("{} {}", paint(t.added, "+3"), paint(t.removed, "-1"))
        );
    }

    #[test]
    fn columns_align_by_display_width() {
        let mut t = Table::new(
            "Languages",
            &[("Language", Align::Left), ("Code", Align::Right)],
        );
        t.row(vec![Cell::plain("Rust"), Cell::plain("1,234")]);
        t.row(vec![Cell::plain("C"), Cell::plain("7")]);
        t.total(vec![Cell::plain("Total"), Cell::plain("1,241")]);
        let mut out = String::new();
        t.render(&Theme::plain(), &mut out);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "Languages");
        assert_eq!(lines[1], "Language   Code");
        assert_eq!(lines[3], "Rust      1,234");
        assert_eq!(lines[4], "C             7");
        assert_eq!(lines[6], "Total     1,241");
        assert_eq!(lines[2].chars().count(), 15);
    }
}
