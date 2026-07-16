//! Argument structs for `vibe aiui …` — the agent-facing observation surface
//! (PROP-042). Split from the `cli` hub along command-family lines; the hub
//! re-exports everything, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

#[derive(Debug, Args)]
pub struct AiuiArgs {
    #[command(subcommand)]
    pub command: AiuiSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AiuiSubcommand {
    /// Render the `vibe tree` TUI **headlessly** to a symbolic snapshot — no
    /// terminal, deterministic (PROP-042 §1/§4). Optionally drive a key script
    /// with `--send` (e.g. "F2 Down Enter"; `F4`/`F6` are refused), set the grid
    /// with `--size COLSxROWS`, and pick `--format text|cells`. Read-only.
    Render(AiuiRenderArgs),
}

#[derive(Debug, Args)]
pub struct AiuiRenderArgs {
    /// Project root to analyse — the same resolver `vibe tree` uses.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Terminal grid as `COLSxROWS`.
    #[arg(long, default_value = "80x24")]
    pub size: String,

    /// A space-separated key script to drive before snapshotting (PROP-042 §3),
    /// e.g. "F2 Down Enter". `F4`/`F6` are refused (side effects).
    #[arg(long, default_value = "")]
    pub send: String,

    /// Snapshot format: `text` (the glyph grid, golden-friendly) or `cells`
    /// (JSON runs with style).
    #[arg(long, value_enum, default_value_t = SnapFormat::Text)]
    pub format: SnapFormat,
}

/// The `vibe aiui render --format` choice (PROP-042 §2).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SnapFormat {
    /// The glyph grid, one line per row.
    Text,
    /// JSON: run-length-encoded rows carrying fg/bg/modifiers.
    Cells,
}
