# clocx

Count lines of code and see where work is happening: totals by language
and directory, recent change activity from git history, and the work in
progress in every git worktree, in one colored report or a live
dashboard.

clocx is a Rust tool that started as a fork of
[cloc](https://github.com/AlDanial/cloc) by Al Danial. It is a separate
project now; the Perl cloc is no longer part of this repository (see the
git history before the removal if you need it).

## Install

    cargo install --path crates/clocx

## Use

    clocx              # report on the current directory and exit
    clocx path/to/dir  # report on another directory
    clocx --live       # full-screen dashboard that refreshes on changes (q quits)
    clocx --json       # the same report as JSON, for scripts

The report has four parts:

- **Languages** and **Directories**: files and code, comment and blank
  lines, with the change in code lines since the previous run.
- **Activity**: lines added and removed, files touched and commits over
  the last hour, day, 7 days and 30 days, with trend sparklines.
- **Where work is happening**: the same activity per directory, current
  work on top.
- **Worktrees**: every worktree of the repository with its branch,
  uncommitted lines, commits ahead of the base branch, total lines
  against the base, and time of last activity.

Common flags: `--depth N` groups by deeper directories, `--rows N` caps
table length, `--base BRANCH` picks what worktrees compare with,
`--color never` (or `NO_COLOR`) turns color off.

The full reference, including how counts differ from cloc, is in
[docs/clocx.md](docs/clocx.md).

## Develop

    cargo test
    cargo clippy --all-targets -- -D warnings

Counting uses [tokei](https://github.com/XAMPPRocky/tokei), git access
uses [gix](https://github.com/GitoxideLabs/gitoxide), the dashboard uses
[ratatui](https://ratatui.rs), and file watching uses
[notify](https://github.com/notify-rs/notify).

## Licence

GPL-2.0-only, as cloc. See [LICENSE](LICENSE).
