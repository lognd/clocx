# clocx

Explanation and reference for clocx, the Rust tool in this fork of cloc.
clocx shows where code is, how much of it there is, and where work is
happening right now, across every git worktree of a repository.

The Perl `cloc` stays in the repository as a behaviour reference until
clocx reaches parity for what the owner uses.

## Usage

    clocx [PATH] [--depth N] [--color auto|always|never] [-v...]

| Flag | Meaning |
| --- | --- |
| `PATH` | Directory to report on; default the current directory. |
| `-d, --depth N` | Directory depth used to group totals and activity; default 1 (top-level directories). |
| `--color WHEN` | `auto` (default) colors only when stdout is a terminal and `NO_COLOR` is unset. |
| `-v` | More diagnostics on stderr (`-v` info, `-vv` debug, `-vvv` trace). `RUST_LOG` overrides. |

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
| `crates/clocx/src/render/` | All output: tables, styles, formatting. |
| `crates/clocx/src/logging.rs` | `tracing` subscriber setup. |
| `crates/clocx/src/error.rs` | The run's error type. |

## Licence

GPL-2.0-only, like cloc.
