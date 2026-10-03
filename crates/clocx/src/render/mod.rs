//! The one module that writes output: report data in, text or JSON out.
//!
//! Everything else computes typed data (`crate::model`) and hands it here.
//! Text is always built with styles and written through `anstream`, which
//! strips escapes when stdout is not a terminal, `NO_COLOR` is set, or
//! `--color never` is given.
#![allow(clippy::print_stdout, clippy::print_stderr)]

pub mod format;
mod json;
pub mod live;
#[cfg(test)]
pub(crate) mod sample;
pub mod sections;
pub mod style;
pub mod table;
mod text;

use std::io::{self, Write};

use anstream::{AutoStream, ColorChoice};

use crate::cli::ColorWhen;
use crate::model::Report;
use style::Theme;

/// Which representation of the report to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Aligned, colored tables for people.
    #[default]
    Text,
    /// One JSON document for scripts; never colored.
    Json,
}

/// How the report should be written.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Text or JSON.
    pub format: Format,
    /// The user's color preference; `Auto` defers to the terminal and `NO_COLOR`.
    pub color: ColorWhen,
    /// Most rows per text table before the rest fold into one line; 0 shows all.
    pub rows: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: Format::Text,
            color: ColorWhen::Auto,
            rows: DEFAULT_ROWS,
        }
    }
}

/// Rows per text table unless `--rows` says otherwise.
pub const DEFAULT_ROWS: usize = 12;

/// Maps the CLI color flag onto anstream's choice.
fn choice(color: ColorWhen) -> ColorChoice {
    match color {
        ColorWhen::Auto => ColorChoice::Auto,
        ColorWhen::Always => ColorChoice::AlwaysAnsi,
        ColorWhen::Never => ColorChoice::Never,
    }
}

/// Renders the report as styled text, escapes included.
pub fn to_text(report: &Report, options: Options) -> String {
    text::report(report, &Theme::default(), options.rows)
}

/// Writes the report to `sink`: JSON as is, text with escapes stripped as the color choice demands.
///
/// # Errors
/// Returns the I/O error when the sink cannot be written (for example a closed pipe).
pub fn write_to<W: anstream::stream::RawStream + anstream::stream::AsLockedWrite>(
    mut sink: W,
    report: &Report,
    options: Options,
) -> io::Result<()> {
    if options.format == Format::Json {
        json::write(&mut sink, report)?;
        return sink.flush();
    }
    let mut stream = AutoStream::new(sink, choice(options.color));
    stream.write_all(to_text(report, options).as_bytes())?;
    stream.flush()
}

/// Writes the report to stdout.
///
/// # Errors
/// Returns the I/O error when stdout cannot be written.
pub fn emit(report: &Report, options: Options) -> io::Result<()> {
    let stdout = io::stdout().lock();
    write_to(stdout, report, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    use sample::report as sample;

    #[test]
    fn styled_text_has_escapes() {
        assert!(to_text(&sample(), Options::default()).contains('\u{1b}'));
    }

    #[test]
    fn auto_into_a_pipe_strips_escapes() {
        let mut buf: Vec<u8> = Vec::new();
        write_to(
            &mut buf,
            &sample(),
            Options {
                color: ColorWhen::Auto,
                ..Options::default()
            },
        )
        .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(!out.contains('\u{1b}'), "{out:?}");
        assert!(out.contains("/tmp/x"));
    }

    #[test]
    fn json_has_no_escapes_even_with_color_always() {
        let mut buf: Vec<u8> = Vec::new();
        let options = Options {
            format: Format::Json,
            color: ColorWhen::Always,
            ..Options::default()
        };
        write_to(&mut buf, &sample(), options).unwrap();
        assert!(!buf.contains(&0x1b));
        assert!(buf.starts_with(b"{"));
    }

    #[test]
    fn never_strips_and_always_keeps_escapes() {
        let mut never: Vec<u8> = Vec::new();
        write_to(
            &mut never,
            &sample(),
            Options {
                color: ColorWhen::Never,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(!never.contains(&0x1b));
        let mut always: Vec<u8> = Vec::new();
        write_to(
            &mut always,
            &sample(),
            Options {
                color: ColorWhen::Always,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(always.contains(&0x1b));
    }
}
