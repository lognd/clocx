//! Diagnostics setup: `tracing` events go to stderr, filtered by `-v` or `RUST_LOG`.

use tracing_subscriber::EnvFilter;

/// Maps the `-v` count to a default filter directive.
fn default_directive(verbose: u8) -> &'static str {
    match verbose {
        0 => "clocx=warn",
        1 => "clocx=info",
        2 => "clocx=debug",
        _ => "clocx=trace",
    }
}

/// Installs the global subscriber writing to stderr; `RUST_LOG` wins over `-v`.
pub fn init(verbose: u8) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_directive(verbose)));
    // A second install (tests calling run twice) is harmless; keep the first.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_maps_to_levels() {
        assert_eq!(default_directive(0), "clocx=warn");
        assert_eq!(default_directive(1), "clocx=info");
        assert_eq!(default_directive(2), "clocx=debug");
        assert_eq!(default_directive(9), "clocx=trace");
    }
}
