# Changelog

All notable changes to clocx are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and clocx adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Changes not yet released are kept as one fragment per change in
[`changelog.d/`](changelog.d/); they are compiled into a section here when a
version is cut (see [docs/releasing.md](docs/releasing.md)).

## [0.1.1] - 2026-10-03

### Fixed

- Linux release binaries are static musl builds that run on any distribution; the 0.1.0 Linux archives needed glibc 2.39 and failed on older systems such as Ubuntu 22.04.

## [0.1.0] - 2026-10-03

First release of clocx, a Rust tool that began as a fork of cloc and is now
its own project at https://github.com/lognd/clocx.

### Added

- `clocx` prints one colored report and exits: aligned tables, green for
  lines added and red for lines removed, dimmed totals, plain output into
  a pipe or with `NO_COLOR`, and `--color auto|always|never`.
- Totals by language and by directory (`--depth N`): files and code,
  comment and blank lines, with the change in code lines since the
  previous run. Long tables fold after `--rows N` rows; rows that changed
  always show.
- Recent activity from git history across every local branch: lines added
  and removed, files touched and commits over the last hour, 24 hours,
  7 days and 30 days, with trend sparklines, and the same per directory
  ("Where work is happening").
- Every git worktree of the repository with its branch, uncommitted lines
  against HEAD, commits ahead of the base branch, everything it changed
  since branching (committed or not), and its last activity; `--base
  BRANCH` picks the base (default `main`, else `master`).
- `--live` (`-l`): a full-screen dashboard of the same report that
  refreshes when files change (debounced, ignoring `.gitignore`d paths),
  when commits, refs, `HEAD` or the index change in any worktree, and every
  30 seconds; `q` quits, `r` refreshes.
- `--json`: the whole report as one JSON document (`schema_version` 1).
- Per-file counts cached by content hash and per-commit diffs cached by
  commit id, so refreshes only recount what changed (`--cache-dir`,
  `--no-cache`).
- Prebuilt binaries for Linux (x86_64, arm64), macOS (arm64, x86_64) and
  Windows (x86_64) on GitHub releases, and `cargo install clocx`.

### Changed

- Counting matches cloc closely for code: hidden files such as
  `.github/workflows` count (never `.git`), and Python docstrings,
  one-line ones included, count as comments. The remaining differences are
  listed in docs/clocx.md, Differences from cloc.

### Removed

- The Perl cloc, its test suite, Docker packaging and upstream CI.

