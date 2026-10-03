//! The live view's event loop: watch files and git refs, refresh in the background, redraw.
//!
//! Three sources feed one channel: a file watcher, a key reader, and the
//! refresh worker that owns the [`Engine`]. File events are debounced here:
//! a refresh starts once events have been quiet for [`DEBOUNCE`]. Access
//! events (files being read, including by our own refresh) are dropped, or
//! every refresh would trigger the next. Refresh requests that pile up while
//! one is running collapse into a single refresh. A periodic tick refreshes
//! too, so the time windows and "ago" columns keep moving.
//! All drawing is done by [`crate::render::live`].

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{EventKind, RecursiveMode, Watcher};
use ratatui::crossterm::event::{
    self, Event as TermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use tracing::{debug, info, warn};

use crate::cli::{Args, ColorWhen};
use crate::engine::Engine;
use crate::error::Error;
use crate::model::Report;
use crate::render::Options;
use crate::render::live::{Status, draw};
use crate::render::style::Theme;

/// Quiet time after the last file event before a refresh starts.
const DEBOUNCE: Duration = Duration::from_millis(400);
/// Refresh at least this often, so windows slide and ages update.
const TICK: Duration = Duration::from_secs(30);
/// How long the key reader waits before checking whether to stop.
const KEY_POLL: Duration = Duration::from_millis(200);
/// Git files whose change means new commits, ref moves, checkouts or staging.
const GIT_FILES: [&str; 4] = ["HEAD", "packed-refs", "index", "ORIG_HEAD"];

/// Everything the main loop reacts to.
enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// The terminal was resized.
    Resize,
    /// Relevant files or refs changed.
    Changed,
    /// The worker finished a refresh.
    Report(Box<Report>),
}

/// What a key asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Leave the live view.
    Quit,
    /// Refresh now.
    Refresh,
}

/// Maps a key press to an action: q, Esc or Ctrl-C quit; r refreshes.
pub fn action(key: KeyEvent) -> Option<Action> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}

/// Decides which changed paths matter: source files not ignored by git, and git refs, HEAD and index.
pub struct Filter {
    roots: Vec<(PathBuf, Gitignore)>,
}

impl Filter {
    /// A filter with no watched roots yet.
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    /// Adds a worktree root, reading its top-level `.gitignore`.
    pub fn add_root(&mut self, root: &Path) {
        let mut builder = GitignoreBuilder::new(root);
        if let Some(e) = builder.add(root.join(".gitignore")) {
            debug!(root = %root.display(), error = %e, "no usable .gitignore");
        }
        let gi = builder.build().unwrap_or_else(|_| Gitignore::empty());
        self.roots.push((root.to_path_buf(), gi));
    }

    /// Whether a change at `path` should trigger a refresh.
    pub fn relevant(&self, path: &Path) -> bool {
        let in_git = path.components().any(|c| c.as_os_str() == ".git");
        if in_git {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".lock") {
                return false;
            }
            let under_refs = path.components().any(|c| c.as_os_str() == "refs");
            return under_refs || GIT_FILES.contains(&name);
        }
        // Linked worktrees keep their git files outside `.git/`, under `.git/worktrees/<id>/` of the
        // main repository, which the check above already covers.
        self.roots
            .iter()
            .filter(|(root, _)| path.starts_with(root))
            .max_by_key(|(root, _)| root.components().count())
            .is_some_and(|(_, gi)| {
                !gi.matched_path_or_any_parents(path, path.is_dir())
                    .is_ignore()
            })
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether a watcher event can mean content changed: reads cannot, but closing a file after writing can.
fn relevant_kind(kind: &EventKind) -> bool {
    use notify::event::{AccessKind, AccessMode};
    match kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        EventKind::Access(_) => false,
        _ => true,
    }
}

/// The theme the live view uses: plain when color is off by flag or `NO_COLOR`.
fn theme(color: ColorWhen) -> Theme {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    match color {
        ColorWhen::Never => Theme::plain(),
        ColorWhen::Auto if no_color => Theme::plain(),
        _ => Theme::default(),
    }
}

/// Paths to watch for a report: the root, the repository's git directory, and every worktree.
fn watch_targets(report: &Report, git_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut targets = vec![PathBuf::from(&report.root)];
    if let Some(g) = git_dir {
        targets.push(g.to_path_buf());
    }
    if let Some(w) = report.worktrees.ok() {
        targets.extend(
            w.worktrees
                .iter()
                .filter(|s| s.problem.is_none())
                .map(|s| PathBuf::from(&s.path)),
        );
    }
    targets
}

/// Runs the refresh worker: refresh on each request (collapsing a backlog), persist the caches on exit.
fn worker(mut engine: Engine, requests: Receiver<()>, events: Sender<Event>) {
    while requests.recv().is_ok() {
        while requests.try_recv().is_ok() {}
        let started = Instant::now();
        let report = engine.refresh();
        debug!(ms = started.elapsed().as_millis() as u64, "live refresh");
        if events.send(Event::Report(Box::new(report))).is_err() {
            break;
        }
    }
    engine.persist();
    info!("live view caches saved");
}

/// Forwards key presses and resizes until `stop` is set.
fn key_reader(events: Sender<Event>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match event::poll(KEY_POLL) {
            Ok(true) => match event::read() {
                Ok(TermEvent::Key(k)) => {
                    if events.send(Event::Key(k)).is_err() {
                        break;
                    }
                }
                Ok(TermEvent::Resize(..)) => {
                    let _ = events.send(Event::Resize);
                }
                Ok(_) => {}
                Err(e) => warn!(error = %e, "terminal read failed"),
            },
            Ok(false) => {}
            Err(e) => {
                warn!(error = %e, "terminal poll failed");
                break;
            }
        }
    }
}

/// Runs the live view until the user quits; the terminal is restored even on error.
///
/// # Errors
/// [`Error::Terminal`] when the terminal cannot be set up or drawn on, [`Error::Watch`] when files cannot be watched.
pub fn run(engine: Engine, args: &Args, options: Options) -> Result<(), Error> {
    let git_dir = gix::discover(engine.root())
        .ok()
        .map(|r| r.common_dir().to_path_buf());
    let mut filter = Filter::new();
    filter.add_root(engine.root());
    let filter = Arc::new(std::sync::Mutex::new(filter));

    let (events_tx, events) = mpsc::channel::<Event>();
    let (requests, requests_rx) = mpsc::channel::<()>();
    let worker_events = events_tx.clone();
    let worker = thread::spawn(move || worker(engine, requests_rx, worker_events));

    let watch_events = events_tx.clone();
    let watch_filter = Arc::clone(&filter);
    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(ev) if relevant_kind(&ev.kind) => {
                let f = watch_filter
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(p) = ev.paths.iter().find(|p| f.relevant(p)) {
                    debug!(path = %p.display(), kind = ?ev.kind, "change detected");
                    let _ = watch_events.send(Event::Changed);
                }
            }
            Ok(_) => {}
            Err(e) => warn!(error = %e, "watch error"),
        })
        .map_err(Error::Watch)?;

    let stop = Arc::new(AtomicBool::new(false));
    let keys = {
        let (tx, stop) = (events_tx.clone(), Arc::clone(&stop));
        thread::spawn(move || key_reader(tx, stop))
    };
    drop(events_tx);

    let mut terminal = ratatui::try_init().map_err(Error::Terminal)?;
    let theme = theme(args.color);
    let mut status = Status {
        refreshing: true,
        report: None,
        watching: 0,
    };
    let mut watched: HashSet<PathBuf> = HashSet::new();
    let _ = requests.send(());
    let mut last_request = Instant::now();
    // When the first file event of a burst arrived; cleared when its refresh is requested.
    let mut pending: Option<Instant> = None;

    let result = (|| -> Result<(), Error> {
        loop {
            terminal
                .draw(|f| draw(f, &status, &theme, options.rows))
                .map_err(Error::Terminal)?;
            let wait = match pending {
                Some(since) => DEBOUNCE.saturating_sub(since.elapsed()),
                None => TICK.saturating_sub(last_request.elapsed()),
            };
            let mut refresh = false;
            match events.recv_timeout(wait) {
                Ok(Event::Key(k)) => match action(k) {
                    Some(Action::Quit) => return Ok(()),
                    Some(Action::Refresh) => refresh = true,
                    None => {}
                },
                Ok(Event::Resize) => {}
                // Restart the quiet period on every event of a burst.
                Ok(Event::Changed) => pending = Some(Instant::now()),
                Ok(Event::Report(report)) => {
                    status.refreshing = false;
                    for target in watch_targets(&report, git_dir.as_deref()) {
                        if watched.contains(&target) {
                            continue;
                        }
                        match watcher.watch(&target, RecursiveMode::Recursive) {
                            Ok(()) => {
                                info!(path = %target.display(), "watching");
                                if git_dir.as_deref() != Some(target.as_path()) {
                                    filter
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                                        .add_root(&target);
                                }
                                watched.insert(target);
                            }
                            Err(e) => warn!(path = %target.display(), error = %e, "cannot watch"),
                        }
                    }
                    status.watching = watched.len();
                    status.report = Some(*report);
                }
                Err(RecvTimeoutError::Timeout) => {
                    if pending.is_some_and(|since| since.elapsed() >= DEBOUNCE) || pending.is_none()
                    {
                        refresh = true;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
            if refresh {
                pending = None;
                status.refreshing = true;
                last_request = Instant::now();
                let _ = requests.send(());
            }
        }
    })();

    let restored = ratatui::try_restore().map_err(Error::Terminal);
    stop.store(true, Ordering::Relaxed);
    drop(watcher);
    drop(requests);
    drop(events);
    if worker.join().is_err() {
        warn!("refresh worker panicked; caches not saved");
    }
    let _ = keys.join();
    result.and(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventState;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn keys_map_to_actions() {
        assert_eq!(
            action(key(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Action::Quit)
        );
        assert_eq!(
            action(key(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Action::Quit)
        );
        assert_eq!(
            action(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
        assert_eq!(
            action(key(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::Refresh)
        );
        assert_eq!(action(key(KeyCode::Char('x'), KeyModifiers::NONE)), None);
        let mut release = key(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(action(release), None);
    }

    #[test]
    fn filter_keeps_sources_and_refs_and_drops_ignored_and_objects() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join(".gitignore"), "target/\n*.log\n").unwrap();
        let mut f = Filter::new();
        f.add_root(root);
        assert!(f.relevant(&root.join("src/main.rs")));
        assert!(!f.relevant(&root.join("target/debug/x.o")));
        assert!(!f.relevant(&root.join("build.log")));
        assert!(f.relevant(&root.join(".git/refs/heads/main")));
        assert!(f.relevant(&root.join(".git/HEAD")));
        assert!(f.relevant(&root.join(".git/index")));
        assert!(f.relevant(&root.join(".git/worktrees/w1/HEAD")));
        assert!(!f.relevant(&root.join(".git/objects/ab/cdef")));
        assert!(!f.relevant(&root.join(".git/index.lock")));
        assert!(!f.relevant(Path::new("/elsewhere/file.rs")));
    }

    #[test]
    fn worker_collapses_a_backlog_and_persists_on_exit() {
        use clap::Parser;
        let tree = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(tree.path().join("a.rs"), "fn a() {}\n").unwrap();
        let args = Args::parse_from([
            "clocx",
            "--cache-dir",
            cache.path().to_str().unwrap(),
            tree.path().to_str().unwrap(),
        ]);
        let engine = Engine::open(&args).unwrap();
        let (req_tx, req_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        for _ in 0..5 {
            req_tx.send(()).unwrap();
        }
        drop(req_tx);
        worker(engine, req_rx, ev_tx);
        let reports = ev_rx
            .try_iter()
            .filter(|e| matches!(e, Event::Report(_)))
            .count();
        assert_eq!(reports, 1, "five queued requests give one refresh");
        let saved = std::fs::read_dir(cache.path()).unwrap().count();
        assert_eq!(saved, 1, "caches written on exit");
    }

    #[test]
    fn reads_are_not_changes() {
        use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind};
        assert!(!relevant_kind(&EventKind::Access(AccessKind::Open(
            AccessMode::Read
        ))));
        assert!(!relevant_kind(&EventKind::Access(AccessKind::Close(
            AccessMode::Read
        ))));
        assert!(relevant_kind(&EventKind::Access(AccessKind::Close(
            AccessMode::Write
        ))));
        assert!(relevant_kind(&EventKind::Modify(ModifyKind::Any)));
        assert!(relevant_kind(&EventKind::Create(CreateKind::File)));
    }

    #[test]
    fn color_off_gives_the_plain_theme() {
        let plain = theme(ColorWhen::Never);
        assert_eq!(plain.added, anstyle::Style::new());
        assert_ne!(theme(ColorWhen::Always).added, anstyle::Style::new());
    }
}
