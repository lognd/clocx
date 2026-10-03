//! The clocx command-line entry point; all logic lives in the library.

use std::process::ExitCode;

fn main() -> ExitCode {
    clocx::run()
}
