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

    /// When to color the output.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, value_name = "WHEN")]
    pub color: ColorWhen,

    /// More diagnostics on stderr (-v info, -vv debug, -vvv trace); `RUST_LOG` overrides.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
