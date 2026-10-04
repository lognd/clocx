//! The JSON report: the typed report as one pretty-printed document with a schema version.

use std::io::{self, Write};

use serde::Serialize;

use crate::model::Report;

/// Bumped on any incompatible change to the JSON shape.
pub const SCHEMA_VERSION: u32 = 1;

/// The top-level JSON object: the version first, then the report's fields.
#[derive(Serialize)]
struct Document<'a> {
    schema_version: u32,
    #[serde(flatten)]
    report: &'a Report,
}

/// Writes `report` as JSON followed by a newline.
///
/// # Errors
/// Returns the I/O error when `sink` cannot be written.
pub fn write<W: Write>(sink: &mut W, report: &Report) -> io::Result<()> {
    let doc = Document {
        schema_version: SCHEMA_VERSION,
        report,
    };
    serde_json::to_writer_pretty(&mut *sink, &doc).map_err(io::Error::from)?;
    sink.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sample;

    fn parsed(report: &Report) -> serde_json::Value {
        let mut buf = Vec::new();
        write(&mut buf, report).unwrap();
        assert!(buf.ends_with(b"}\n"));
        serde_json::from_slice(&buf).unwrap()
    }

    #[test]
    fn document_carries_version_and_every_section() {
        let v = parsed(&sample::report());
        assert_eq!(v["schema_version"], SCHEMA_VERSION);
        assert_eq!(v["root"], "/tmp/x");
        assert_eq!(v["generated_at"], "2026-10-03T12:00:00Z");
        let rust = &v["totals"]["languages"][0];
        assert_eq!(rust["name"], "Rust");
        assert_eq!(rust["counts"]["code"], 3421);
        assert_eq!(rust["code_delta"], 120);
        assert_eq!(v["totals"]["total"]["counts"]["files"], 13);
        assert_eq!(v["totals"]["depth"], 1);
    }

    #[test]
    fn activity_says_whether_the_clone_is_shallow() {
        let mut r = sample::report();
        assert_eq!(parsed(&r)["activity"]["shallow"], false);
        if let crate::model::Section::Ok(a) = &mut r.activity {
            a.shallow = true;
        }
        assert_eq!(parsed(&r)["activity"]["shallow"], true);
    }

    #[test]
    fn missing_baseline_is_null_not_absent() {
        let mut r = sample::report();
        r.totals.baseline_at = None;
        r.totals.languages[0].code_delta = None;
        let v = parsed(&r);
        assert!(v["totals"]["baseline_at"].is_null());
        assert!(v["totals"]["languages"][0]["code_delta"].is_null());
    }

    #[test]
    fn control_characters_in_names_are_escaped() {
        let mut r = sample::report();
        r.totals.directories[0].name = "odd\u{1b}[31mdir".into();
        let mut buf = Vec::new();
        write(&mut buf, &r).unwrap();
        assert!(!buf.contains(&0x1b));
        assert_eq!(
            parsed(&r)["totals"]["directories"][0]["name"],
            "odd\u{1b}[31mdir"
        );
    }
}
