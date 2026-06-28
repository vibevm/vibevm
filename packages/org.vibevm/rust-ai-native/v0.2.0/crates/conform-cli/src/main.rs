//! `conform` — the AI-Native discipline gate binary (ENGINE-CONFORM v0.1,
//! PROP-024 code-bearing packages). Runs the gate over the project in the
//! current directory (or `--path`), driven by that project's `conform.toml`.
//! Installing the rust-ai-native stack yields this binary, so a consumer gets
//! the checker, not a description of it.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "conform",
    about = "The AI-Native discipline gate (ENGINE-CONFORM v0.1)"
)]
struct Cli {
    /// Project root — where `conform.toml` lives. Defaults to the current dir.
    #[arg(long, global = true, default_value = ".")]
    path: PathBuf,
    /// The ratchet baseline file, relative to the project root.
    #[arg(long, global = true, default_value = "conform-baseline.json")]
    baseline: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract facts, run the rules, and fail on any new finding past the baseline.
    Check {
        /// Limit the gate to one crate by name.
        #[arg(long)]
        scope: Option<String>,
    },
    /// Rewrite the baseline to the current finding set (a NEW rule landing, or
    /// a re-freeze after the set shrank).
    Freeze,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { scope } => {
            conform_cli::run_check(&cli.path, &cli.baseline, scope.as_deref())
        }
        Command::Freeze => conform_cli::run_freeze(&cli.path, &cli.baseline),
    }
}
