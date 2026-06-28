//! `specmap-rust` — the AI-Native traceability index binary (PROP-014 §2.5,
//! PROP-024 code-bearing packages). Builds (or `--check`s) the canonical
//! `specmap.json` over the project in the current directory (or `--path`),
//! driven by that project's `specmap.toml`. Installing the rust-ai-native
//! stack yields this binary, so a consumer gets the traceability engine, not a
//! description of it. Per-language suffix (`-rust`) mirrors `conform-rust`: a
//! future TypeScript frontend ships `specmap-typescript`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "specmap-rust",
    about = "The AI-Native traceability index (PROP-014 §2.5, Rust frontend)"
)]
struct Cli {
    /// Project root — where `specmap.toml` and `specmap.json` live. Defaults to
    /// the current dir.
    #[arg(long, default_value = ".")]
    path: PathBuf,
    /// Byte-compare against the committed `specmap.json` and fail on drift,
    /// instead of rewriting it.
    #[arg(long)]
    check: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    specmap_cli::run_specmap(&cli.path, cli.check)
}
