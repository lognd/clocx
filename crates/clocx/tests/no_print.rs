//! Enforces that only the renderer writes output: no print macros elsewhere in `src/`.
//!
//! Clippy denies `print_stdout` and `print_stderr` workspace-wide too; this
//! test also catches `dbg!`-style escapes when clippy is not run.

use std::fs;
use std::path::{Path, PathBuf};

/// Macros that write straight to stdout or stderr.
const FORBIDDEN: &[&str] = &["println!", "print!", "eprintln!", "eprint!", "dbg!"];

/// Collects every `.rs` file under `dir`.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn print_macros_only_in_render() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let render = src.join("render");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty());

    let mut offenders = Vec::new();
    for file in files.iter().filter(|f| !f.starts_with(&render)) {
        let text = fs::read_to_string(file).expect("utf-8 source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if FORBIDDEN.iter().any(|m| code.contains(m)) {
                offenders.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "print macros outside render/:\n{}",
        offenders.join("\n")
    );
}
