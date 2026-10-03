//! End-to-end test of the live view in a real pseudo-terminal (via util-linux `script`).
//!
//! Starts `clocx --live`, edits a file, makes a commit, presses `q`, and
//! checks from the debug log that each change caused exactly one refresh
//! and that the view exited cleanly with its caches saved.
#![cfg(target_os = "linux")]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Runs git in `dir` with an isolated config; panics on failure.
fn git(dir: &Path, args: &[&str]) {
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

/// Number of finished refreshes recorded in the live log so far.
fn refreshes(log: &Path) -> usize {
    std::fs::read_to_string(log).map_or(0, |s| s.matches("live refresh").count())
}

/// Waits up to 10 s for the log to show at least `n` refreshes.
fn wait_for(log: &Path, n: usize) -> usize {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        let got = refreshes(log);
        if got >= n {
            return got;
        }
        sleep(Duration::from_millis(100));
    }
    refreshes(log)
}

// frob:tests crates/clocx/src/live.rs::run kind=integration
#[test]
fn live_view_refreshes_on_edits_and_commits_and_quits_on_q() {
    if Command::new("script").arg("--version").output().is_err() {
        // No util-linux `script` to provide a pseudo-terminal: nothing to drive the view with.
        return;
    }
    let tree = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let root = tree.path();
    git(root, &["init", "-q", "-b", "main"]);
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.rs"), "fn a() {}\n").unwrap();
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "init"]);
    let log = cache.path().join("live.log");

    let cmd = format!(
        "COLUMNS=160 LINES=50 {} -vv --live --cache-dir {} {}",
        env!("CARGO_BIN_EXE_clocx"),
        cache.path().display(),
        root.display()
    );
    let mut child = Command::new("script")
        .args(["-qfec", &cmd, "/dev/null"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env_remove("NO_COLOR")
        .spawn()
        .expect("script runs");

    assert_eq!(wait_for(&log, 1), 1, "first refresh on start");
    // Give the watcher time to be armed after the first report.
    sleep(Duration::from_millis(500));

    std::fs::write(root.join("src/b.rs"), "fn b() {}\n").unwrap();
    assert_eq!(wait_for(&log, 2), 2, "one refresh for the edit");

    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "b"]);
    assert_eq!(
        wait_for(&log, 3),
        3,
        "one refresh for the commit's burst of ref and index changes"
    );
    // Our own refreshes read files; reads must not trigger further refreshes.
    sleep(Duration::from_secs(2));
    assert_eq!(refreshes(&log), 3, "no feedback loop");

    child.stdin.as_mut().unwrap().write_all(b"q").unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "q did not quit");
        sleep(Duration::from_millis(100));
    };
    assert!(status.success(), "{status:?}");
    let saved: Vec<String> = std::fs::read_dir(cache.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(saved.len() >= 2, "a cache directory and the log: {saved:?}");
}
