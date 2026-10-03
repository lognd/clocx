//! Typed report data: computed by the counting and git modules, turned into output by `render`.

use jiff::Timestamp;
use serde::Serialize;

/// Everything one clocx run knows; the renderer is a pure function of this.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The directory the report covers, as given (canonicalised).
    pub root: String,
    /// When the report was computed.
    pub generated_at: Timestamp,
}
