//! CLI surface for `vector-headless` (D-36).

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "vector-headless",
    version,
    about = "Headless pass-through terminal proxy (D-36)"
)]
pub struct Cli {
    /// Override grid columns. Default: read from parent terminal, fall back to 80.
    #[arg(long)]
    pub cols: Option<u16>,

    /// Override grid rows. Default: read from parent terminal, fall back to 24.
    #[arg(long)]
    pub rows: Option<u16>,

    /// Enable trace-level logging for the parser (writes to stderr; may break raw-mode display).
    #[arg(long)]
    pub debug_parser: bool,

    /// Scrollback line capacity (CORE-03 minimum 10_000).
    #[arg(long, default_value_t = 10_000)]
    pub scrollback: usize,
}
