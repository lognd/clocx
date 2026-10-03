//! The color palette: one place that says what each meaning looks like.

use anstyle::{AnsiColor, Effects, Style};

/// Styles by meaning; text is always styled here and stripped later when color is off.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Section titles.
    pub title: Style,
    /// Column headers.
    pub header: Style,
    /// Rules, totals and other secondary text.
    pub dim: Style,
    /// Lines added, positive deltas.
    pub added: Style,
    /// Lines removed, negative deltas.
    pub removed: Style,
    /// Names that identify a row (languages, directories, branches).
    pub name: Style,
    /// Trend sparklines.
    pub spark: Style,
    /// Warnings shown inside the report (a section that could not be computed).
    pub warn: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            title: Style::new().bold().fg_color(Some(AnsiColor::Cyan.into())),
            header: Style::new().bold(),
            dim: Style::new().effects(Effects::DIMMED),
            added: Style::new().fg_color(Some(AnsiColor::Green.into())),
            removed: Style::new().fg_color(Some(AnsiColor::Red.into())),
            name: Style::new(),
            spark: Style::new().fg_color(Some(AnsiColor::Blue.into())),
            warn: Style::new().fg_color(Some(AnsiColor::Yellow.into())),
        }
    }
}

impl Theme {
    /// A theme with no styles at all: for tests, and for the live view when color is off.
    pub fn plain() -> Self {
        let p = Style::new();
        Self {
            title: p,
            header: p,
            dim: p,
            added: p,
            removed: p,
            name: p,
            spark: p,
            warn: p,
        }
    }
}

/// Wraps `text` in `style`'s escape codes; plain styles add nothing.
pub fn paint(style: Style, text: &str) -> String {
    if style == Style::new() {
        return text.to_owned();
    }
    format!("{}{text}{}", style.render(), style.render_reset())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_style_adds_no_escapes() {
        assert_eq!(paint(Style::new(), "abc"), "abc");
    }

    #[test]
    fn styled_text_is_wrapped_and_reset() {
        let s = paint(Theme::default().added, "+3");
        assert!(s.starts_with("\u{1b}["));
        assert!(s.contains("+3"));
        assert!(s.ends_with("\u{1b}[0m"));
    }
}
