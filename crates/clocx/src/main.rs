//! The clocx command-line entry point; all logic lives in the library.

// frob:accept PROC001 because="ExitCode is the exit status of main, not a spawn; PROC001 targets frob's own crates"
use std::process::ExitCode;

fn main() -> ExitCode {
    clocx::run()
}
