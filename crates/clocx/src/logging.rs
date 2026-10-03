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
///
/// Events go through the progress readout's writer, which erases the
/// progress line first so a log line never lands in the middle of it.
pub fn init(verbose: u8) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_directive(verbose)));
    // A second install (tests calling run twice) is harmless; keep the first.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(crate::render::progress::LogWriter::default)
        .with_target(false)
        .without_time()
        .try_init();
}

/// Installs the subscriber for the live view: stderr would corrupt the screen, so logs go to
/// `live.log` in the cache directory, or nowhere when there is none.
pub fn init_live(verbose: u8, cache_dir: Option<&std::path::Path>, no_cache: bool) {
    let dir = if no_cache {
        None
    } else {
        cache_dir
            .map(std::path::Path::to_path_buf)
            .or_else(|| dirs::cache_dir().map(|d| d.join("clocx")))
    };
    let file = dir.and_then(|d| {
        std::fs::create_dir_all(&d).ok()?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(d.join("live.log"))
            .ok()
    });
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_directive(verbose)));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_target(false);
    let _ = match file {
        Some(f) => builder.with_writer(std::sync::Mutex::new(f)).try_init(),
        None => builder.with_writer(std::io::sink).try_init(),
    };
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
