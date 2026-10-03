//! clocx: count lines of code and show where work is happening, across git worktrees.
//!
//! Data flows one way: the compute modules build a [`model::Report`] (driven
//! by [`engine::Engine`]), and [`render`] alone turns it into output. Diagnostics
//! go through `tracing`.

pub mod activity;
pub mod cache;
pub mod cli;
pub mod engine;
pub mod error;
pub mod git;
pub mod live;
pub mod logging;
pub mod model;
pub mod render;
pub mod totals;
pub mod worktrees;

// frob:accept PROC001 because="ExitCode is the exit status of main, not a spawn; PROC001 targets frob's own crates"
use std::process::ExitCode;

use clap::Parser;
use tracing::{debug, error};

use cli::Args;
use engine::Engine;
use error::Error;
use model::Report;

/// Computes one report for the parsed arguments and saves the caches and baseline.
///
/// # Errors
/// Returns [`Error::Root`] when the root path cannot be resolved.
pub fn build_report(args: &Args) -> Result<Report, Error> {
    let mut engine = Engine::open(args)?;
    let report = engine.refresh();
    engine.persist();
    Ok(report)
}

/// The render options the arguments ask for.
fn render_options(args: &Args) -> render::Options {
    render::Options {
        format: if args.json {
            render::Format::Json
        } else {
            render::Format::Text
        },
        color: args.color,
        rows: args.rows,
    }
}

/// Runs clocx with already-parsed arguments: the live view with `--live`, else one report.
///
/// # Errors
/// Returns the first error of computing, writing or (live) drawing the report.
pub fn run_with(args: &Args) -> Result<(), Error> {
    if args.live {
        let engine = Engine::open(args)?;
        return live::run(engine, args, render_options(args));
    }
    let report = build_report(args)?;
    debug!("rendering report");
    render::emit(&report, render_options(args)).map_err(Error::Output)
}

/// Parses the command line, runs, and maps the outcome to an exit code.
pub fn run() -> ExitCode {
    let args = Args::parse();
    if args.live {
        logging::init_live(args.verbose, args.cache_dir.as_deref(), args.no_cache);
    } else {
        logging::init(args.verbose);
    }
    match run_with(&args) {
        Ok(()) => ExitCode::SUCCESS,
        // A closed pipe (clocx | head) is a normal way to stop reading.
        Err(Error::Output(e)) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(path: &str) -> Args {
        Args::parse_from(["clocx", "--color", "never", "--no-cache", path])
    }

    #[test]
    fn build_report_canonicalises_the_root() {
        let report = build_report(&args(env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(std::path::Path::new(&report.root).is_absolute());
    }

    #[test]
    fn build_report_rejects_a_missing_root() {
        let err = build_report(&args("/definitely/not/here")).unwrap_err();
        assert!(matches!(err, Error::Root { .. }), "{err}");
    }

    #[test]
    fn run_with_writes_the_report() {
        run_with(&args(env!("CARGO_MANIFEST_DIR"))).unwrap();
    }

    #[test]
    fn live_conflicts_with_json() {
        assert!(Args::try_parse_from(["clocx", "--live", "--json"]).is_err());
        assert!(Args::try_parse_from(["clocx", "-l"]).unwrap().live);
    }
}
