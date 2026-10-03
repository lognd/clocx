//! The top-level error type of a clocx run; each variant maps to a user-facing message.

use std::io;
use std::path::PathBuf;

/// Why a clocx run failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The report root does not exist or cannot be resolved.
    #[error("cannot open {path}: {source}")]
    Root {
        /// The path as given on the command line.
        path: PathBuf,
        /// The underlying filesystem error.
        source: io::Error,
    },
    /// Writing the report to stdout failed (for example a closed pipe).
    #[error("cannot write output: {0}")]
    Output(#[source] io::Error),
}
