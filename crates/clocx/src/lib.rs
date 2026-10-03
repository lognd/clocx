//! clocx: count lines of code and show where work is happening, across git worktrees.
//!
//! Data flows one way: the compute modules build a [`model::Report`], and
//! [`render`] alone turns it into output. Diagnostics go through `tracing`.

pub mod cache;
pub mod cli;
pub mod error;
pub mod logging;
pub mod model;
pub mod render;
pub mod totals;

// frob:accept PROC001 because="ExitCode is the exit status of main, not a spawn; PROC001 targets frob's own crates"
use std::process::ExitCode;

use clap::Parser;
use jiff::Timestamp;
use tracing::{debug, error, info, warn};

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
    let store = if args.no_cache {
        cache::Store::disabled()
    } else {
        cache::Store::open(args.cache_dir.as_deref(), &root)
    };
    let now = Timestamp::now();
    let mut counts = store.load_counts();
    let baseline = store.load_snapshot();
    let scan = totals::scan(&root, args.depth, &mut counts, baseline.as_ref(), now);
    // The cache only speeds up later runs; failing to write it is not fatal.
    if let Err(e) = store.save_counts(&counts) {
        warn!(error = %e, "count cache not saved");
    }
    if let Err(e) = store.save_snapshot(&scan.snapshot) {
        warn!(error = %e, "snapshot not saved; the next run shows no change column");
    }
    Ok(Report {
        root: root.display().to_string(),
        generated_at: now,
        totals: scan.totals,
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
}
