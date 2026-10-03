# clocx

Explanation and reference for clocx, the Rust tool in this fork of cloc.
clocx shows where code is, how much of it there is, and where work is
happening right now, across every git worktree of a repository.

The Perl `cloc` stays in the repository as a behaviour reference until
clocx reaches parity for what the owner uses.

## Usage

    clocx [PATH] [--depth N] [--rows N] [--color auto|always|never]
          [--cache-dir DIR] [--no-cache] [-v...]

| Flag | Meaning |
| --- | --- |
| `PATH` | Directory to report on; default the current directory. |
| `-d, --depth N` | Directory depth used to group totals and activity; default 1 (top-level directories). |
| `--rows N` | Most rows per text table; the rest fold into one dimmed `N more` row. Rows whose code changed are always shown. Default 12; `0` shows all. |
| `--cache-dir DIR` | Where the count cache and last-run snapshot live; default `<user cache dir>/clocx`. Also `CLOCX_CACHE_DIR`. |
| `--no-cache` | Read and write no cache or snapshot; the change column is never shown. |
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

The cache directory holds, per canonical root, `counts.json` (counts by
content key `<xxh3>:<language>`, and a size and mtime stamp per path)
and `snapshot.json` (the last run's code lines by language and
directory). A file whose size and mtime match its stamp is not read; a
file whose content hash is known is read but not recounted. The cache is
pruned to the files seen in each run. A missing or corrupt cache is
logged and rebuilt, never an error. Like git's index, an edit that keeps
both size and mtime unchanged is not noticed until the file changes
again.

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
- Diagnostics go through `tracing` to stderr, never to stdout.

## Layout

| Path | Role |
| --- | --- |
| `crates/clocx/src/main.rs` | Entry point; calls `clocx::run`. |
| `crates/clocx/src/cli.rs` | Command-line arguments (clap). |
| `crates/clocx/src/model.rs` | The typed report. |
| `crates/clocx/src/totals.rs` | Walk, count, group and diff the line totals. |
| `crates/clocx/src/cache.rs` | Count cache and last-run snapshot on disk. |
| `crates/clocx/src/render/` | All output: tables, styles, formatting. |
| `crates/clocx/src/logging.rs` | `tracing` subscriber setup. |
| `crates/clocx/src/error.rs` | The run's error type. |

## Licence

GPL-2.0-only, like cloc.
