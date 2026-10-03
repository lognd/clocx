# clocx

Explanation and reference for clocx, the Rust tool in this fork of cloc.
clocx shows where code is, how much of it there is, and where work is
happening right now, across every git worktree of a repository.

The Perl `cloc` stays in the repository as a behaviour reference until
clocx reaches parity for what the owner uses.

## Usage

    clocx [PATH] [--live] [--depth N] [--rows N] [--color auto|always|never]
          [--base BRANCH] [--cache-dir DIR] [--no-cache] [--json] [-v...]

By default clocx prints the report once and exits. `--live` (`-l`)
keeps it open as a dashboard instead.

| Flag | Meaning |
| --- | --- |
| `PATH` | Directory to report on; default the current directory. |
| `-d, --depth N` | Directory depth used to group totals and activity; default 1 (top-level directories). |
| `--base BRANCH` | Base branch worktrees are compared with; a branch name or any revision. Default `main`, else `master`. |
| `--rows N` | Most rows per text table; the rest fold into one dimmed `N more` row. Rows whose code changed are always shown. Default 12; `0` shows all. |
| `--cache-dir DIR` | Where the count cache and last-run snapshot live; default `<user cache dir>/clocx`. Also `CLOCX_CACHE_DIR`. |
| `--no-cache` | Read and write no cache or snapshot; the change column is never shown. |
| `-l, --live` | Full-screen dashboard that refreshes on changes (see Live view). Needs a terminal; not with `--json`. |
| `--json` | Print the report as one JSON document instead of tables (see JSON below). |
| `--color WHEN` | `auto` (default) colors only when stdout is a terminal and `NO_COLOR` is unset. |
| `-v` | More diagnostics on stderr (`-v` info, `-vv` debug, `-vvv` trace). `RUST_LOG` overrides. |

## Totals

clocx walks `PATH` honouring `.gitignore`, `.ignore` and hidden-file
rules (also outside a git repository), detects each file's language with
tokei, and counts code, comment and blank lines. Lines of languages
embedded in another file (for example code blocks in Markdown) count
toward the outer file's language. Files tokei does not recognise are
not code and are left out.

Two tables follow: by language, and by directory at `--depth` (`.` is
the root itself; files shallower than the depth group under their own
directory). Both are sorted by code lines.

The Change column is the difference in code lines from the previous run
on the same root, read from the snapshot each run saves at its end. It
is absent on the first run and with `--no-cache`. Directory changes are
shown only when the previous run used the same `--depth`. A language or
directory that disappeared shows as an empty row with a negative change.

### Cache

The cache directory holds, per canonical root, `commits.json` (see
Activity), `counts.json` (counts by
content key `<xxh3>:<language>`, and a size and mtime stamp per path)
and `snapshot.json` (the last run's code lines by language and
directory). A file whose size and mtime match its stamp is not read; a
file whose content hash is known is read but not recounted. The cache is
pruned to the files seen in each run. A missing or corrupt cache is
logged and rebuilt, never an error. Like git's index, an edit that keeps
both size and mtime unchanged is not noticed until the file changes
again.

## Activity

Recent change activity comes from git history. clocx walks every local
branch and `HEAD` back 30 days (by committer time), diffs each non-merge
commit against its first parent, and counts lines added and removed per
file. Merge commits are skipped, so work merged from a branch is counted
once. Renames follow the repository's git rename settings. Only source
files count, by the same rule as the totals (tokei knows the language),
so lock files and images do not swamp the numbers. When `PATH` is a
subdirectory, only paths under it count.

The Activity table has one row per window: last 1h, 24h, 7d and 30d,
with lines added and removed, distinct files touched, commits, and a
sparkline of lines changed per bucket (5 minutes, 1 hour, 6 hours and 1
day; oldest left). The note under it gives the number of branch tips
walked and the time of the newest commit.

"Where work is happening" has one row per directory (at `--depth`) with
activity in the last 30 days, showing `+added -removed` per window and a
7-day sparkline. Rows are sorted by the last hour, then the last day,
7 and 30 days, so current work is on top. Rows with activity in the last
day are never folded away.

Each commit's per-file result is cached by commit id in `commits.json`
(commits never change), pruned to the 30-day window. Outside a git
repository, or when history cannot be read, the section says it is
unavailable; that is not an error.

## Worktrees

The owner and coding agents work in many git worktrees at once; this
table shows the work in each before it is merged. One row per worktree
of the repository (the main checkout and every linked worktree, found
from whichever worktree `PATH` is in), most recently active first. The
worktree `PATH` is in is marked `*`.

| Column | Meaning |
| --- | --- |
| Branch | The checked-out branch, `(detached <id>)`, or `(no commits)`. |
| Uncommitted | Lines added and removed against the worktree's HEAD, staged or not. Untracked files that are not ignored count as added, so new files show before they are staged. |
| Files | Files with uncommitted changes. |
| Ahead | Commits on HEAD that the base branch does not have. |
| vs BASE | Lines added and removed since the merge base with the base branch, committed or not: the whole of the branch's work so far. Work that landed on the base after the branch point does not count. |
| Last activity | The newest of the HEAD commit time and the modification times of changed files. |

Changed paths come from git status, so `.gitignore`, the index stat
cache and line-ending conversion behave as in git. Line counts compare
file contents with the blob in HEAD or the merge base, ignoring `\r\n`
versus `\n`, so a `core.autocrlf` checkout does not look modified.
Binary files count as files touched with no lines. A worktree that
cannot be read (for example its directory was deleted) shows the reason
in place of its branch; it does not fail the report.

## Live view

`clocx --live` draws the same report full screen, meant to be left open
on a second screen:

- top: Activity beside Languages; middle: Where work is happening
  beside Directories; bottom: Worktrees across the full width; the notes
  of each table sit on its bottom border;
- header: root, time of the last refresh, and `refreshing` while one
  runs; footer: the keys and how many paths are watched.

Keys: `q`, `Esc` or `Ctrl-C` quit; `r` refreshes now.

It refreshes when:

- a file under the root or under any worktree changes and is not
  ignored by that tree's top-level `.gitignore` (events are debounced:
  the refresh starts once they have been quiet for 0.4 s);
- git state changes: refs, `HEAD`, `packed-refs` or the index, in the
  repository or any worktree (new commits, branch moves, checkouts,
  staging);
- 30 seconds pass without one, so the time windows and "ago" columns
  keep moving.

File reads are not changes (only writes are), so the view's own
refreshes never trigger more refreshes. Refreshes run in the
background; requests that arrive during one collapse into one more.
Caches stay in memory between refreshes, so only changed files are
recounted and only new commits are diffed. The Change column compares
with the snapshot from before the view started, and the snapshot is
saved when the view exits. Diagnostics go to `live.log` in the cache
directory instead of the screen. Color follows `--color` and
`NO_COLOR`.

## JSON

`--json` writes the same report the tables are drawn from, pretty-printed,
followed by a newline. It is never colored, whatever `--color` says.

- `schema_version` (1) comes first; it is bumped on any incompatible
  change to the shape.
- `root`, `generated_at` (RFC 3339, UTC).
- `totals`: `depth`, `languages` and `directories` (every row, not
  folded; each has `name`, `counts` with `files`, `code`, `comments`,
  `blanks`, and `code_delta`), `total`, `baseline_at`, and `cache`
  (`unchanged`, `hash_hits`, `counted`, `unreadable`).
- `activity`: `{"status": "unavailable", "reason": ...}` or
  `{"status": "ok", ...}` with `depth`, `windows` (each `label`,
  `seconds`, `churn` with `added`, `removed`, `files`, `commits`, and
  `trend`, oldest bucket first), `directories` (each `name`, `windows`
  as churn in the same order, `trend` over 7 days), `branches` and
  `last_commit_at`.
- `worktrees`: unavailable, or `{"status": "ok", "base": ..., "worktrees":
  [...]}` with each worktree's `name`, `path`, `current`, `branch`,
  `head`, `uncommitted` (churn, `commits` 0), `ahead`, `vs_base` (churn,
  `commits` = ahead), `last_activity` and `problem`.
- Values that are unknown are `null`, never absent: `code_delta` and
  `baseline_at` without a previous run.

## Output rules

- One module, `crates/clocx/src/render`, writes all output. Every other
  module computes typed data (`crates/clocx/src/model.rs`) and hands it
  to the renderer.
- `clippy::print_stdout` and `clippy::print_stderr` are denied for the
  whole workspace; only `render` opts back in. The test
  `crates/clocx/tests/no_print.rs` also fails on print macros outside
  `render`.
- Text is always built with styles and written through `anstream`,
  which strips escape codes when stdout is not a terminal, `NO_COLOR` is
  set, or `--color never` is given.
- Colors carry meaning: green for lines added, red for lines removed,
  dimmed rules and totals, cyan section titles, blue sparklines.
- Numbers are exact up to 99,999 (with thousands separators), then
  abbreviated (`123.5k`, `12.3M`). Zero deltas are left blank.
- Control characters in names (odd file names) are shown as `?` in
  tables, so a file name cannot inject terminal escapes; JSON escapes
  them.
- Diagnostics go through `tracing` to stderr (to `live.log` in the live
  view), never to stdout. A failed run ends with one line on stderr,
  `clocx: error: ...`, and exit status 1.

## Layout

| Path | Role |
| --- | --- |
| `crates/clocx/src/main.rs` | Entry point; calls `clocx::run`. |
| `crates/clocx/src/engine.rs` | Owns the caches of one root; computes a report per refresh. |
| `crates/clocx/src/live.rs` | Live view event loop: watcher, keys, background refresh. |
| `crates/clocx/src/cli.rs` | Command-line arguments (clap). |
| `crates/clocx/src/model.rs` | The typed report. |
| `crates/clocx/src/totals.rs` | Walk, count, group and diff the line totals. |
| `crates/clocx/src/cache.rs` | Count cache, commit cache and last-run snapshot on disk. |
| `crates/clocx/src/git.rs` | Open the repository of a root; git error type. |
| `crates/clocx/src/activity.rs` | Walk recent history and bucket line churn. |
| `crates/clocx/src/worktrees.rs` | Status of every worktree against HEAD and the base. |
| `crates/clocx/src/render/` | All output: `sections` builds the tables once; `text`, `json` and `live` (ratatui) draw them; `style` is the one palette. |
| `crates/clocx/src/logging.rs` | `tracing` subscriber setup. |
| `crates/clocx/src/error.rs` | The run's error type. |

## Licence

GPL-2.0-only, like cloc.
