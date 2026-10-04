<p align="center"><img src="https://raw.githubusercontent.com/lognd/clocx/master/docs/assets/clocx-banner.svg" alt="clocx: count lines of code and see where work is happening" width="100%"/></p>

# clocx

clocx counts lines of code and shows where work is happening right now:
totals by language and directory, lines added and removed over the last
hour, day, week and month from git history, and the work in progress in
every git worktree of the repository. It prints one colored report and
exits, or stays open as a live dashboard that refreshes as files and
commits change. It is built for a repository where several people or
coding agents work in parallel worktrees and you want to see all of it
at a glance.

[![CI](https://github.com/lognd/clocx/actions/workflows/ci.yml/badge.svg)](https://github.com/lognd/clocx/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/clocx.svg)](https://crates.io/crates/clocx)
[![PyPI](https://img.shields.io/pypi/v/clocx.svg)](https://pypi.org/project/clocx/)
[![GitHub release](https://img.shields.io/github/v/release/lognd/clocx?include_prereleases&sort=semver)](https://github.com/lognd/clocx/releases)
[![License: GPL-2.0](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](#license)
[![MSRV 1.88](https://img.shields.io/badge/MSRV-1.88-orange.svg)](#versioning-and-compatibility)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20macos%20%7C%20windows-lightgrey.svg)](#install)

## Install

```bash
cargo install clocx
```

Or, without a Rust toolchain, the prebuilt binary from PyPI (the wheel
holds only the `clocx` executable):

```bash
uv tool install clocx     # or: pipx install clocx
uvx clocx                 # run once without installing
```

Prebuilt binaries for Linux (x86_64, arm64; static, any distribution),
macOS (arm64, x86_64) and Windows (x86_64) are attached to every
[GitHub release](https://github.com/lognd/clocx/releases), each with a
SHA-256 checksum. Download the archive for your platform, check it, and put
`clocx` on your `PATH`.

From a checkout:

```bash
cargo install --path crates/clocx
```

## Sixty-second tour

```bash
clocx              # report on the current directory and exit
clocx path/to/repo # report on another directory
clocx --live       # full-screen dashboard; q quits
clocx --json       # the same report as one JSON document
```

<p align="center"><img src="https://raw.githubusercontent.com/lognd/clocx/master/docs/assets/clocx-report.svg" alt="An example clocx report: languages, directories, activity, where work is happening, and worktrees" width="760"/></p>

Read top to bottom:

- **Languages** and **Directories**: files and code, comment and blank
  lines, with the change in code lines since you last ran clocx.
- **Activity**: lines added and removed, files touched and commits over
  the last hour, 24 hours, 7 days and 30 days, across every local branch,
  each with a sparkline of when the changes happened.
- **Where work is happening**: the same activity per directory, with
  current work on top.
- **Worktrees**: every worktree with its branch, uncommitted lines against
  its HEAD, commits ahead of the base branch, everything it changed since
  branching (committed or not), and its last activity. The worktree you
  ran clocx in is marked `*`.

Green is lines added, red is lines removed, totals are dimmed. Output is
plain when it goes to a pipe or a file, or when `NO_COLOR` is set.

## Live view

`clocx --live` (or `-l`) draws the same report full screen and keeps it
current. It refreshes when a source file changes (debounced, ignoring what
`.gitignore` ignores), when commits, branches, `HEAD` or the index change
in any worktree, and every 30 seconds so the time windows keep moving.
Only changed files are recounted and only new commits are diffed, so it is
cheap to leave open on a second screen. `q` quits, `r` refreshes.

## Options

| Flag | Meaning |
| --- | --- |
| `-l, --live` | Live dashboard instead of a one-shot report. |
| `--json` | One JSON document with every section (`schema_version` 1). |
| `-d, --depth N` | Group directories N levels deep (default 1). |
| `--rows N` | Fold tables after N rows (default 12, `0` for all); changed rows always show. |
| `--base BRANCH` | What worktrees compare with (default `main`, else `master`). |
| `--color WHEN` | `auto`, `always` or `never`. |
| `--cache-dir DIR`, `--no-cache` | Where counts are cached, or no cache at all. |
| `--no-progress` | No progress line on stderr while the report is computed. |
| `-v` | Diagnostics on stderr (`-vv` debug); `RUST_LOG` also works. |

The full reference, with the JSON shape and the exact rules for each
section, is [docs/clocx.md](docs/clocx.md).

## How it counts

Counting is done by [tokei](https://github.com/XAMPPRocky/tokei): files git
would not ignore are counted, hidden directories such as `.github` count,
and documentation counts as comment (Python docstrings and Markdown prose;
code blocks in Markdown are code). clocx started as a fork of
[cloc](https://github.com/AlDanial/cloc) and agrees with it within a few
percent on code lines for the languages checked; the differences, and why,
are listed in
[docs/clocx.md, Differences from cloc](docs/clocx.md#differences-from-cloc).

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

The toolchain is pinned in `rust-toolchain.toml`. All output goes through
one module, `crates/clocx/src/render`; everything else computes typed data
and hands it over. Git access uses
[gix](https://github.com/GitoxideLabs/gitoxide), the dashboard uses
[ratatui](https://ratatui.rs), and file watching uses
[notify](https://github.com/notify-rs/notify). CI runs formatting, clippy,
the MSRV build, a crates.io package dry run and the tests on Linux, macOS
and Windows for every push.

## Versioning and compatibility

clocx follows [Semantic Versioning](https://semver.org). Before 1.0, a
minor version may change flags, output layout or the JSON shape; the JSON
carries a `schema_version` that changes whenever its shape does. The
minimum supported Rust version is 1.88 and is checked in CI; raising it is
a minor-version change. Changes are listed in [CHANGELOG.md](CHANGELOG.md),
and releases are cut as described in [docs/releasing.md](docs/releasing.md).

## Contributing

Contributions are welcome, from a typo fix to a new section of the report.
Read [CONTRIBUTING.md](https://github.com/lognd/clocx/blob/master/CONTRIBUTING.md)
before opening a pull request; it covers the local setup, the commit
format and the
[AI-assisted contributions policy](https://github.com/lognd/clocx/blob/master/CONTRIBUTING.md#ai-assisted-contributions).
Everyone taking part is expected to follow the
[Code of Conduct](https://github.com/lognd/clocx/blob/master/CODE_OF_CONDUCT.md).

## Security

See [SECURITY.md](https://github.com/lognd/clocx/blob/master/SECURITY.md)
for how to report a vulnerability; please do not open a public issue for
one.

## License

GPL-2.0-only, like cloc, from which clocx grew. See [LICENSE](LICENSE).

Logan Dapp <logan@logand.app>
