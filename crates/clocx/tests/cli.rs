//! End-to-end tests of the clocx binary: exit codes and plain output into a pipe.

// frob:accept PROC001 because="integration tests must spawn the built binary; PROC001 targets frob's own crates"
use std::process::Command;

/// Runs clocx with a throwaway cache dir so tests never touch the user's cache.
fn clocx(args: &[&str]) -> std::process::Output {
    let cache = tempfile::tempdir().expect("temp cache dir");
    clocx_cached(args, cache.path())
}

/// Runs the built clocx binary with `args` and `cache` as its cache dir, stdout captured (so not a terminal).
fn clocx_cached(args: &[&str], cache: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clocx"))
        .args(args)
        .env("CLOCX_CACHE_DIR", cache)
        .env_remove("NO_COLOR")
        .env_remove("RUST_LOG")
        .output()
        .expect("clocx binary runs")
}

// frob:tests crates/clocx/src/lib.rs::run kind=integration
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

// frob:tests crates/clocx/src/render/mod.rs::emit kind=integration
#[test]
fn color_always_forces_escapes_into_a_pipe() {
    let out = clocx(&["--color", "always", env!("CARGO_MANIFEST_DIR")]);
    assert!(out.status.success());
    assert!(out.stdout.contains(&0x1b));
}

// frob:tests crates/clocx/src/logging.rs::init kind=integration
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

/// The line of `out` that starts with `prefix`.
fn line<'a>(out: &'a str, prefix: &str) -> &'a str {
    out.lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starting {prefix:?} in:\n{out}"))
}

// frob:tests crates/clocx/src/totals.rs::scan kind=integration
#[test]
fn second_run_shows_change_since_the_first() {
    let tree = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let root = tree.path().to_str().unwrap();
    std::fs::create_dir(tree.path().join("src")).unwrap();
    std::fs::write(tree.path().join("src/a.rs"), "fn a() {}\n").unwrap();

    let first = String::from_utf8(clocx_cached(&[root], cache.path()).stdout).unwrap();
    assert!(
        !first.contains("Change"),
        "no baseline on the first run:\n{first}"
    );
    assert!(line(&first, "Rust").contains("1"));

    std::fs::write(tree.path().join("src/b.rs"), "fn b() {}\nfn c() {}\n").unwrap();
    let second = String::from_utf8(clocx_cached(&[root], cache.path()).stdout).unwrap();
    assert!(line(&second, "Rust").trim_end().ends_with("+2"), "{second}");
    assert!(line(&second, "src").trim_end().ends_with("+2"), "{second}");
    assert!(second.contains("Change: code lines since the previous run"));
}

#[test]
fn no_cache_never_shows_change() {
    let tree = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let root = tree.path().to_str().unwrap();
    std::fs::write(tree.path().join("a.rs"), "fn a() {}\n").unwrap();
    for _ in 0..2 {
        let out = clocx_cached(&["--no-cache", root], cache.path());
        assert!(!String::from_utf8(out.stdout).unwrap().contains("Change"));
    }
    assert_eq!(
        std::fs::read_dir(cache.path()).unwrap().count(),
        0,
        "nothing written"
    );
}

// frob:tests crates/clocx/src/render/json.rs::write kind=integration
#[test]
fn json_flag_prints_one_parseable_document() {
    let out = clocx(&["--json", "--color", "always", env!("CARGO_MANIFEST_DIR")]);
    assert!(out.status.success());
    assert!(!out.stdout.contains(&0x1b), "JSON is never colored");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.trim_start().starts_with('{') && text.trim_end().ends_with('}'));
    assert!(text.contains("\"schema_version\": 1"));
    assert!(text.contains("\"languages\""));
}

/// Runs git in `dir` with an isolated config; panics on failure.
fn git(dir: &std::path::Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// frob:tests crates/clocx/src/activity.rs::collect kind=integration
#[test]
fn fresh_commit_shows_in_the_last_hour() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path();
    git(root, &["init", "-q", "-b", "main"]);
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n").unwrap();
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "first"]);

    let out = clocx(&[root.to_str().unwrap()]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let hour = line(&text, "last 1h");
    assert!(hour.contains("+3"), "{text}");
    let (_, work) = text
        .split_once("Where work is happening\n")
        .expect("directory activity");
    assert!(line(work, "src").contains("+3"), "{text}");
    assert!(
        text.contains("Across 1 branch; last commit just now."),
        "{text}"
    );

    let json = String::from_utf8(clocx(&["--json", root.to_str().unwrap()]).stdout).unwrap();
    assert!(
        json.contains("\"activity\": {\n    \"status\": \"ok\""),
        "{json}"
    );
}

#[test]
fn outside_git_activity_is_unavailable_not_an_error() {
    let tree = tempfile::tempdir().unwrap();
    std::fs::write(tree.path().join("a.rs"), "fn a() {}\n").unwrap();
    let out = clocx(&[tree.path().to_str().unwrap()]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    // A temp dir inside some repository (odd machines) would have activity; only assert the common case.
    if !text.contains("Where work is happening") && !text.contains("last 1h") {
        assert!(
            text.contains("Activity: unavailable (not a git repository)."),
            "{text}"
        );
    }
}
