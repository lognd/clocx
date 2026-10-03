//! Command-line arguments, parsed with clap.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// When to color text output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ColorWhen {
    /// Color when stdout is a terminal and `NO_COLOR` is unset.
    #[default]
    Auto,
    /// Always color, even into a pipe.
    Always,
    /// Never color.
    Never,
}

/// Count lines of code and show where work is happening.
#[derive(Debug, Clone, Parser)]
#[command(name = "clocx", version, about, long_about = None)]
pub struct Args {
    /// Directory to report on (default: the current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Directory depth used to group totals and activity (1 = top-level directories).
    #[arg(short, long, default_value_t = 1, value_parser = clap::value_parser!(u16).range(1..))]
    pub depth: u16,

    /// Base branch that worktrees are compared with (default: `main`, else `master`).
    #[arg(long, value_name = "BRANCH")]
    pub base: Option<String>,

    /// Most rows per text table; the rest fold into one line (0 shows every row).
    #[arg(long, default_value_t = crate::render::DEFAULT_ROWS, value_name = "N")]
    pub rows: usize,

    /// Print the report as one JSON document instead of tables.
    #[arg(long, conflicts_with = "live")]
    pub json: bool,

    /// Live view: a full-screen dashboard that refreshes on file and git changes (q quits).
    #[arg(short, long)]
    pub live: bool,

    /// When to color the output.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, value_name = "WHEN")]
    pub color: ColorWhen,

    /// Directory for the per-file count cache and the last-run snapshot (default: the user cache dir).
    #[arg(long, env = "CLOCX_CACHE_DIR", value_name = "DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Neither read nor write the cache or the last-run snapshot; no change column is shown.
    #[arg(long)]
    pub no_cache: bool,

    /// More diagnostics on stderr (-v info, -vv debug, -vvv trace); `RUST_LOG` overrides.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
