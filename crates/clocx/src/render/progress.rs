//! The progress readout: one self-overwriting line on stderr while a one-shot report is computed.
//!
//! A drawer thread samples [`Progress`] every [`TICK`] and rewrites the line
//! in place (carriage return, then erase to end of line). Nothing is drawn
//! for runs shorter than [`SHOW_AFTER`], so warm-cache runs do not flicker.
//! Log lines go through [`LogWriter`], which erases the readout first so the
//! two never share a line; the next tick draws it again.

use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anstream::{AutoStream, ColorChoice};
use anstyle::Style;
use tracing::debug;
use unicode_width::UnicodeWidthStr;

use super::format;
use super::style::{Theme, paint};
use crate::cli::ColorWhen;
use crate::progress::{Phase, Progress, Sample};

/// How often the line is redrawn.
pub const TICK: Duration = Duration::from_millis(80);

/// How long a run must take before the line appears.
pub const SHOW_AFTER: Duration = Duration::from_millis(250);

/// Cells of the bar when the total is known.
const BAR_WIDTH: usize = 24;

/// Spinner frames (braille dots), cycled once per tick.
const SPINNER: [char; 10] = [
    '\u{280b}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283c}', '\u{2834}', '\u{2826}', '\u{2827}',
    '\u{2807}', '\u{280f}',
];

/// Moves to the start of the line and erases it.
const CLEAR: &str = "\r\u{1b}[2K";

/// Whether the readout currently occupies the cursor's line; held while anything writes to stderr.
static SHOWING: Mutex<bool> = Mutex::new(false);

/// A running readout; dropping it stops the drawer and erases the line.
pub struct Readout {
    stop: Sender<()>,
    drawer: Option<JoinHandle<()>>,
}

impl Drop for Readout {
    fn drop(&mut self) {
        // The drawer may already have stopped; then there is nobody to tell.
        let _ = self.stop.send(());
        if let Some(drawer) = self.drawer.take()
            && drawer.join().is_err()
        {
            debug!("progress drawer panicked");
        }
    }
}

/// Starts drawing `progress` on stderr; `None` when stderr is not a terminal that can redraw a line.
pub fn start(progress: Arc<Progress>, color: ColorWhen) -> Option<Readout> {
    let stderr = io::stderr();
    if !stderr.is_terminal() {
        debug!("stderr is not a terminal; no progress readout");
        return None;
    }
    if std::env::var_os("TERM").is_some_and(|t| t == "dumb") {
        debug!("dumb terminal; no progress readout");
        return None;
    }
    let colored = match color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => AutoStream::choice(&stderr) != ColorChoice::Never,
    };
    let theme = if colored {
        Theme::default()
    } else {
        Theme::plain()
    };
    let (stop, stopped) = mpsc::channel();
    match thread::Builder::new()
        .name("clocx-progress".to_owned())
        .spawn(move || draw_loop(&progress, &theme, &stopped))
    {
        Ok(drawer) => {
            debug!("progress readout started");
            Some(Readout {
                stop,
                drawer: Some(drawer),
            })
        }
        Err(e) => {
            debug!(error = %e, "progress drawer not started");
            None
        }
    }
}

/// Redraws the line every tick until told to stop (or the readout is gone), then erases it.
fn draw_loop(progress: &Progress, theme: &Theme, stopped: &Receiver<()>) {
    let started = Instant::now();
    let mut phase = (Phase::Idle, started);
    let mut frame = 0usize;
    while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(TICK) {
        let sample = progress.sample();
        if sample.phase != phase.0 {
            phase = (sample.phase, Instant::now());
        }
        if started.elapsed() < SHOW_AFTER {
            continue;
        }
        let text = line(&sample, phase.1.elapsed(), frame, width(), theme);
        draw(&text);
        frame = frame.wrapping_add(1);
    }
    erase();
}

/// The terminal width in columns, 80 when it cannot be read (or reads as zero, as on an unsized pty).
fn width() -> usize {
    match ratatui::crossterm::terminal::size() {
        Ok((w, _)) if w > 0 => usize::from(w),
        _ => 80,
    }
}

/// Locks the readout state; a panic elsewhere while holding it leaves it usable.
fn showing() -> std::sync::MutexGuard<'static, bool> {
    SHOWING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Replaces the current line with `text` (empty text just erases it).
fn draw(text: &str) {
    let mut showing = showing();
    // A gone stderr leaves nothing to show progress on; the report itself goes to stdout.
    let _ = draw_to(&mut io::stderr().lock(), &mut showing, text);
}

/// Erases the line if it is showing.
fn erase() {
    let mut showing = showing();
    if *showing {
        let _ = draw_to(&mut io::stderr().lock(), &mut showing, "");
    }
}

/// Writes `text` over the current line of `sink` and records whether a line is now showing.
fn draw_to(sink: &mut impl Write, showing: &mut bool, text: &str) -> io::Result<()> {
    *showing = !text.is_empty();
    write!(sink, "{CLEAR}{text}")?;
    sink.flush()
}

/// Writes one log event to `sink`, erasing a showing readout first.
fn log_to(sink: &mut impl Write, showing: &mut bool, event: &[u8]) -> io::Result<()> {
    if *showing {
        sink.write_all(CLEAR.as_bytes())?;
        *showing = false;
    }
    sink.write_all(event)?;
    sink.flush()
}

/// What a phase is called on the line, and what it counts.
fn words(phase: Phase) -> (&'static str, &'static str) {
    match phase {
        Phase::Idle => ("Starting", ""),
        Phase::Walking => ("Finding", "files"),
        Phase::Counting => ("Counting", "files"),
        Phase::History => ("History", "commits"),
        Phase::Worktrees => ("Worktrees", "worktrees"),
        Phase::Saving => ("Saving", "cache"),
        Phase::Done => ("Done", ""),
    }
}

/// Styled pieces of a line, measured before they are painted.
#[derive(Default)]
struct Pieces(Vec<(Style, String)>);

impl Pieces {
    /// Appends `text` in `style`, separated from what came before by `gap` spaces.
    fn push(&mut self, gap: usize, style: Style, text: String) {
        if !self.0.is_empty() {
            self.0.push((Style::new(), " ".repeat(gap)));
        }
        self.0.push((style, text));
    }

    /// Display width in terminal columns.
    fn width(&self) -> usize {
        self.0.iter().map(|(_, t)| t.width()).sum()
    }

    /// The pieces with their escape codes.
    fn paint(&self) -> String {
        self.0.iter().map(|(s, t)| paint(*s, t)).collect()
    }
}

/// The filled and empty cells of a bar `done / total` of the way full.
fn bar(done: u64, total: u64) -> (String, String) {
    let filled = if total == 0 {
        BAR_WIDTH
    } else {
        (done.min(total) as u128 * BAR_WIDTH as u128 / total as u128) as usize
    };
    (
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(BAR_WIDTH - filled),
    )
}

/// Builds the line for `sample`, `elapsed` into its phase, dropping detail until it fits in `width` columns.
fn line(sample: &Sample, elapsed: Duration, frame: usize, width: usize, theme: &Theme) -> String {
    // The cursor sits after the text; a line filling the last column would wrap.
    let room = width.saturating_sub(1);
    let tiers = [(true, true), (false, true), (false, false)];
    let mut built = Pieces::default();
    for (with_bar, with_rate) in tiers {
        built = pieces(sample, elapsed, frame, theme, with_bar, with_rate);
        if built.width() <= room {
            break;
        }
    }
    built.paint()
}

/// The pieces of one line, with or without the bar and the rate and ETA.
fn pieces(
    sample: &Sample,
    elapsed: Duration,
    frame: usize,
    theme: &Theme,
    with_bar: bool,
    with_rate: bool,
) -> Pieces {
    let (label, noun) = words(sample.phase);
    let mut p = Pieces::default();
    p.push(0, theme.spark, SPINNER[frame % SPINNER.len()].to_string());
    p.push(1, theme.title, format!("{label:<9}"));
    if matches!(sample.phase, Phase::Idle | Phase::Saving | Phase::Done) {
        if !noun.is_empty() {
            p.push(1, theme.dim, noun.to_owned());
        }
        return p;
    }
    let secs = elapsed.as_secs_f64();
    // Under one item a second the rate says nothing the count does not.
    let rate = (secs > 0.0)
        .then(|| sample.done as f64 / secs)
        .filter(|r| *r >= 1.0);
    match sample.total {
        Some(total) => {
            if with_bar {
                let (filled, empty) = bar(sample.done, total);
                p.push(1, theme.added, filled);
                p.0.push((theme.dim, empty));
            }
            p.push(
                1,
                theme.name,
                format!(
                    "{}/{} {noun}",
                    format::count(sample.done),
                    format::count(total)
                ),
            );
            let pct = (sample.done.min(total) * 100)
                .checked_div(total)
                .unwrap_or(100);
            p.push(2, theme.header, format!("{pct:>3}%"));
            if with_rate {
                if let Some(rate) = rate {
                    p.push(2, theme.dim, format!("{}/s", format::count(rate as u64)));
                }
                let eta = if sample.done > 0 && secs > 0.0 {
                    let left = total.saturating_sub(sample.done) as f64 * secs / sample.done as f64;
                    format::duration(Duration::from_secs_f64(left))
                } else {
                    "--".to_owned()
                };
                p.push(2, theme.dim, format!("ETA {eta}"));
            }
        }
        None => {
            p.push(
                1,
                theme.name,
                format!("{} {noun}", format::count(sample.done)),
            );
            if with_rate && let Some(rate) = rate {
                p.push(2, theme.dim, format!("{}/s", format::count(rate as u64)));
            }
        }
    }
    p
}

/// A tracing writer that erases the readout before each log event so the two never share a line.
///
/// The event is buffered and written in one piece when the writer drops,
/// which tracing does once per event.
#[derive(Default)]
pub struct LogWriter {
    buf: Vec<u8>,
}

impl Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for LogWriter {
    fn drop(&mut self) {
        let mut showing = showing();
        // Nothing sensible is left to do if stderr itself is gone.
        let _ = log_to(&mut io::stderr().lock(), &mut showing, &self.buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(phase: Phase, done: u64, total: Option<u64>) -> Sample {
        Sample { phase, done, total }
    }

    fn plain(s: &Sample, secs: u64, width: usize) -> String {
        line(s, Duration::from_secs(secs), 0, width, &Theme::plain())
    }

    #[test]
    fn a_known_total_shows_bar_fraction_rate_and_eta() {
        let l = plain(&sample(Phase::Counting, 250, Some(1_000)), 5, 120);
        assert!(l.starts_with("\u{280b} Counting "), "{l}");
        assert!(l.contains(&"\u{2588}".repeat(6)), "{l}");
        assert!(l.contains(&"\u{2591}".repeat(18)), "{l}");
        assert!(l.contains("250/1,000 files"), "{l}");
        assert!(l.contains(" 25%"), "{l}");
        assert!(l.contains("50/s"), "{l}");
        assert!(l.ends_with("ETA 15s"), "{l}");
    }

    #[test]
    fn an_unknown_total_shows_a_running_count() {
        let l = plain(&sample(Phase::History, 42, None), 2, 120);
        assert_eq!(l, "\u{280b} History   42 commits  21/s");
    }

    #[test]
    fn slow_phases_hide_the_rate_but_keep_the_eta() {
        let l = plain(&sample(Phase::Worktrees, 1, Some(4)), 3, 120);
        assert!(!l.contains("/s"), "{l}");
        assert!(l.ends_with("ETA 9s"), "{l}");
    }

    #[test]
    fn eta_is_unknown_before_anything_is_done() {
        let l = plain(&sample(Phase::Worktrees, 0, Some(3)), 0, 120);
        assert!(l.contains("0/3 worktrees"), "{l}");
        assert!(l.ends_with("ETA --"), "{l}");
    }

    #[test]
    fn narrow_terminals_drop_the_bar_then_the_rate() {
        let s = sample(Phase::Counting, 250, Some(1_000));
        let wide = plain(&s, 5, 120);
        let mid = plain(&s, 5, 50);
        let narrow = plain(&s, 5, 30);
        assert!(wide.contains('\u{2588}'));
        assert!(!mid.contains('\u{2588}') && mid.contains("ETA"), "{mid}");
        assert!(
            !narrow.contains("ETA") && narrow.contains("25%"),
            "{narrow}"
        );
        assert!(mid.width() < 50, "{mid}");
    }

    #[test]
    fn phases_without_counts_show_only_their_name() {
        assert_eq!(
            plain(&sample(Phase::Saving, 0, None), 1, 80),
            "\u{280b} Saving    cache"
        );
        assert_eq!(
            plain(&sample(Phase::Idle, 0, None), 1, 80),
            "\u{280b} Starting "
        );
    }

    #[test]
    fn the_bar_is_full_at_or_past_the_total() {
        assert_eq!(bar(5, 5).1, "");
        assert_eq!(bar(9, 5).1, "");
        assert_eq!(bar(0, 0).1, "");
        assert_eq!(bar(0, 7).0, "");
    }

    #[test]
    fn a_log_event_erases_the_readout_before_it_is_written() {
        let mut out: Vec<u8> = Vec::new();
        let mut showing = false;
        draw_to(&mut out, &mut showing, "Counting 1/2").unwrap();
        assert!(showing);
        log_to(&mut out, &mut showing, b"WARN cannot read x\n").unwrap();
        assert!(!showing, "the next tick draws the line again");
        log_to(&mut out, &mut showing, b"WARN cannot read y\n").unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!("{CLEAR}Counting 1/2{CLEAR}WARN cannot read x\nWARN cannot read y\n")
        );
    }

    #[test]
    fn drawing_nothing_erases_the_line() {
        let mut out: Vec<u8> = Vec::new();
        let mut showing = true;
        draw_to(&mut out, &mut showing, "").unwrap();
        assert!(!showing);
        assert_eq!(out, CLEAR.as_bytes());
    }

    #[test]
    fn colored_lines_carry_escapes() {
        let s = sample(Phase::Counting, 1, Some(2));
        let l = line(&s, Duration::from_secs(1), 3, 120, &Theme::default());
        assert!(l.contains('\u{1b}'));
        assert!(l.contains(SPINNER[3]));
    }
}
