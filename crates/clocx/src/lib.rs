//! clocx: count lines of code and show where work is happening, across git worktrees.
//!
//! Data flows one way: the compute modules build a [`model::Report`], and
//! [`render`] alone turns it into output. Diagnostics go through `tracing`.

pub mod cli;
pub mod error;
pub mod logging;
pub mod model;
pub mod render;

use std::process::ExitCode;

use clap::Parser;
use jiff::Timestamp;
use tracing::{debug, error, info};

use cli::Args;
use error::Error;
use model::Report;

/// Computes the report for the parsed arguments.
///
/// # Errors
/// Returns [`Error::Root`] when the root path cannot be resolved.
pub fn build_report(args: &Args) -> Result<Report, Error> {
    let root = args.path.canonicalize().map_err(|source| Error::Root {
        path: args.path.clone(),
        source,
    })?;
    info!(root = %root.display(), depth = args.depth, "building report");
    Ok(Report {
        root: root.display().to_string(),
        generated_at: Timestamp::now(),
    })
}

/// Runs clocx with already-parsed arguments.
///
/// # Errors
/// Returns the first error of computing or writing the report.
pub fn run_with(args: &Args) -> Result<(), Error> {
    let report = build_report(args)?;
    debug!("rendering text report");
    render::emit(&report, render::Options { color: args.color }).map_err(Error::Output)
}

/// Parses the command line, runs, and maps the outcome to an exit code.
pub fn run() -> ExitCode {
    let args = Args::parse();
    logging::init(args.verbose);
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
