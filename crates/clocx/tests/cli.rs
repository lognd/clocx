//! End-to-end tests of the clocx binary: exit codes and plain output into a pipe.

// frob:accept PROC001 because="integration tests must spawn the built binary; PROC001 targets frob's own crates"
use std::process::Command;

/// Runs the built clocx binary with `args`, stdout captured (so not a terminal).
fn clocx(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clocx"))
        .args(args)
        .env_remove("NO_COLOR")
        .env_remove("RUST_LOG")
        .output()
        .expect("clocx binary runs")
}

// frob:tests crates/clocx/src/lib.rs::run kind=integration
// frob:tests crates/clocx/src/lib.rs::run_with kind=integration
// frob:tests crates/clocx/src/lib.rs::build_report kind=integration
// frob:tests crates/clocx/src/render/mod.rs::emit kind=integration
// frob:tests crates/clocx/src/logging.rs::init kind=integration
#[test]
fn report_into_a_pipe_is_plain_and_succeeds() {
    let out = clocx(&[env!("CARGO_MANIFEST_DIR")]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).expect("utf-8 output");
    assert!(stdout.starts_with("clocx  "), "{stdout:?}");
    assert!(
        !stdout.contains('\u{1b}'),
        "escapes into a pipe: {stdout:?}"
    );
}

#[test]
fn color_always_forces_escapes_into_a_pipe() {
    let out = clocx(&["--color", "always", env!("CARGO_MANIFEST_DIR")]);
    assert!(out.status.success());
    assert!(out.stdout.contains(&0x1b));
}

#[test]
fn missing_root_fails_with_a_message_on_stderr() {
    let out = clocx(&["/definitely/not/here"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot open /definitely/not/here"),
        "{stderr}"
    );
}
