//! The live dashboard: the same panels as the text report, drawn full screen with ratatui.
//!
//! Layout, top to bottom: a header line; Activity beside Languages; "Where
//! work is happening" beside Directories; Worktrees across the full width;
//! a footer with the keys. Colors come from the shared [`Theme`].

use anstyle::{AnsiColor, Effects};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Row, Table as TuiTable};

use super::sections::{Part, panels};
use super::style::Theme;
use super::table::{Align, Cell, GAP, Table};
use crate::model::Report;

/// What the dashboard shows besides the report itself.
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// A refresh is running in the background.
    pub refreshing: bool,
    /// The newest report, once the first refresh finished.
    pub report: Option<Report>,
    /// Paths being watched for changes.
    pub watching: usize,
}

/// Maps one anstyle color onto ratatui's.
fn color(c: anstyle::Color) -> Option<Color> {
    let anstyle::Color::Ansi(a) = c else {
        return None;
    };
    Some(match a {
        AnsiColor::Black => Color::Black,
        AnsiColor::Red => Color::Red,
        AnsiColor::Green => Color::Green,
        AnsiColor::Yellow => Color::Yellow,
        AnsiColor::Blue => Color::Blue,
        AnsiColor::Magenta => Color::Magenta,
        AnsiColor::Cyan => Color::Cyan,
        AnsiColor::White => Color::Gray,
        AnsiColor::BrightBlack => Color::DarkGray,
        AnsiColor::BrightRed => Color::LightRed,
        AnsiColor::BrightGreen => Color::LightGreen,
        AnsiColor::BrightYellow => Color::LightYellow,
        AnsiColor::BrightBlue => Color::LightBlue,
        AnsiColor::BrightMagenta => Color::LightMagenta,
        AnsiColor::BrightCyan => Color::LightCyan,
        AnsiColor::BrightWhite => Color::White,
    })
}

/// Maps a theme style onto ratatui's, so both views share one palette.
pub fn style(s: anstyle::Style) -> Style {
    let mut out = Style::default();
    if let Some(fg) = s.get_fg_color().and_then(color) {
        out = out.fg(fg);
    }
    let effects = s.get_effects();
    if effects.contains(Effects::BOLD) {
        out = out.add_modifier(Modifier::BOLD);
    }
    if effects.contains(Effects::DIMMED) {
        out = out.add_modifier(Modifier::DIM);
    }
    out
}

/// One table cell as a ratatui line, aligned like its column.
fn line(cell: &Cell, align: Align, over: Option<Style>) -> Line<'static> {
    let spans: Vec<Span<'static>> = cell
        .runs()
        .iter()
        .map(|(t, s)| {
            let st = match over {
                Some(o) if *s == anstyle::Style::new() => o,
                _ => style(*s),
            };
            Span::styled(t.clone(), st)
        })
        .collect();
    let l = Line::from(spans);
    match align {
        Align::Left => l,
        Align::Right => l.alignment(Alignment::Right),
    }
}

/// A shared table as a ratatui widget; the total row (if any) is dimmed and last.
fn widget(table: &Table, theme: &Theme) -> (TuiTable<'static>, Vec<u16>) {
    let aligns: Vec<Align> = table.columns().iter().map(|(_, a)| *a).collect();
    let widths: Vec<u16> = table.widths().iter().map(|w| *w as u16).collect();
    let header = Row::new(
        table
            .columns()
            .iter()
            .map(|(h, a)| line(&Cell::styled(h.clone(), theme.header), *a, None))
            .collect::<Vec<_>>(),
    );
    let to_row = |cells: &[Cell], over: Option<Style>| {
        Row::new(
            cells
                .iter()
                .zip(&aligns)
                .map(|(c, a)| line(c, *a, over))
                .collect::<Vec<_>>(),
        )
    };
    let mut rows: Vec<Row<'static>> = table.rows().iter().map(|r| to_row(r, None)).collect();
    if let Some(total) = table.total_row() {
        rows.push(to_row(total, Some(style(theme.dim))));
    }
    let tui = TuiTable::new(rows, widths.iter().map(|w| Constraint::Length(*w)))
        .header(header)
        .column_spacing(GAP as u16);
    (tui, widths)
}

/// Height and natural width a panel needs, borders included.
fn size(parts: &[Part]) -> (u16, u16) {
    let mut height = 2u16;
    let mut width = 0u16;
    for part in parts {
        match part {
            Part::Table(t) => {
                let rows = t.rows().len() + usize::from(t.total_row().is_some()) + 1;
                height += rows as u16;
                let w: usize =
                    t.widths().iter().sum::<usize>() + GAP * t.columns().len().saturating_sub(1);
                width = width.max(w as u16 + 2);
            }
            Part::Note(n) => width = width.max(n.len() as u16 + 4),
        }
    }
    (height, width)
}

/// Draws one panel: a bordered block titled by its table, notes along the bottom edge.
fn panel(frame: &mut Frame<'_>, area: Rect, parts: &[Part], fallback_title: &str, theme: &Theme) {
    let table = parts.iter().find_map(|p| {
        if let Part::Table(t) = p {
            Some(t)
        } else {
            None
        }
    });
    let title = table.map_or(fallback_title, Table::title);
    let mut block = Block::bordered()
        .title(Span::styled(format!(" {title} "), style(theme.title)))
        .border_style(style(theme.dim));
    for part in parts {
        if let Part::Note(n) = part {
            block = block.title_bottom(Span::styled(format!(" {n} "), style(theme.dim)));
        }
    }
    match table {
        Some(t) => frame.render_widget(widget(t, theme).0.block(block), area),
        None => frame.render_widget(block, area),
    }
}

/// Draws the whole dashboard into `frame`.
pub fn draw(frame: &mut Frame<'_>, status: &Status, theme: &Theme, rows: usize) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let when = status.report.as_ref().map_or_else(
        || "computing...".to_owned(),
        |r| format!("updated {}", r.generated_at.strftime("%H:%M:%S UTC")),
    );
    let root = status.report.as_ref().map_or("", |r| r.root.as_str());
    let mut head = vec![
        Span::styled("clocx", style(theme.title)),
        Span::raw("  "),
        Span::styled(root.to_owned(), style(theme.name)),
        Span::raw("  "),
        Span::styled(when, style(theme.dim)),
    ];
    if status.refreshing {
        head.push(Span::styled("  refreshing", style(theme.warn)));
    }
    frame.render_widget(Paragraph::new(Line::from(head)), header);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("q", style(theme.header)),
            Span::styled(" quit  ", style(theme.dim)),
            Span::styled("r", style(theme.header)),
            Span::styled(
                format!(" refresh  watching {} paths for changes", status.watching),
                style(theme.dim),
            ),
        ])),
        footer,
    );

    let Some(report) = &status.report else { return };
    let p = panels(report, theme, rows);
    let (act_h, act_w) = size(&p.activity);
    let (lang_h, _) = size(&p.languages);
    let (work_h, work_w) = size(&p.work);
    let (dir_h, _) = size(&p.directories);
    let left_w = act_w.max(work_w);
    let [top, middle, bottom] = Layout::vertical([
        Constraint::Length(act_h.max(lang_h)),
        Constraint::Length(work_h.max(dir_h)),
        Constraint::Min(3),
    ])
    .areas(body);
    let split = |area: Rect| -> [Rect; 2] {
        Layout::horizontal([Constraint::Length(left_w), Constraint::Min(0)]).areas(area)
    };
    let [act, lang] = split(top);
    panel(frame, act, &p.activity, "Activity", theme);
    panel(frame, lang, &p.languages, "Languages", theme);
    let [work, dirs] = split(middle);
    panel(frame, work, &p.work, "Where work is happening", theme);
    panel(frame, dirs, &p.directories, "Directories", theme);
    panel(frame, bottom, &p.worktrees, "Worktrees", theme);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sample;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn screen(status: &Status, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, status, &Theme::default(), 12))
            .unwrap();
        let buf = term.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn dashboard_shows_every_panel() {
        let status = Status {
            refreshing: false,
            report: Some(sample::report()),
            watching: 3,
        };
        let s = screen(&status, 180, 50);
        for needle in [
            "clocx",
            "/tmp/x",
            " Activity ",
            " Languages ",
            " Where work is happening ",
            " Directories ",
            " Worktrees ",
            "last 1h",
            "Rust",
            "agent-1",
            "q quit",
            "watching 3 paths",
        ] {
            assert!(s.contains(needle), "missing {needle:?} in:\n{s}");
        }
    }

    #[test]
    fn before_the_first_report_it_says_computing() {
        let status = Status {
            refreshing: true,
            report: None,
            watching: 0,
        };
        let s = screen(&status, 80, 10);
        assert!(s.contains("computing..."));
        assert!(s.contains("refreshing"));
    }

    #[test]
    fn theme_styles_map_onto_ratatui() {
        let t = Theme::default();
        assert_eq!(style(t.added).fg, Some(Color::Green));
        assert!(style(t.dim).add_modifier.contains(Modifier::DIM));
        assert!(style(t.header).add_modifier.contains(Modifier::BOLD));
        assert_eq!(style(anstyle::Style::new()), Style::default());
    }
}
