//! `rust-ai-native-specmap` — the AI-Native traceability index binary (PROP-014 §2.5,
//! PROP-024 code-bearing packages). Builds (or `--check`s) the canonical
//! `specmap.json` over the project in the current directory (or `--path`),
//! driven by that project's `specmap.toml`. Installing the rust-ai-native
//! stack yields this binary, so a consumer gets the traceability engine, not a
//! description of it. The family prefix mirrors `rust-ai-native-conform`: the
//! TypeScript stack ships `typescript-ai-native-specmap`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "rust-ai-native-specmap",
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
    /// Run only the orphan-coverage gate (no committed `specmap.json` needed).
    /// For a package whose `scope!` targets are hosted in the consuming repo:
    /// gates that the code is tagged, not that the targets resolve here.
    #[arg(long, conflicts_with = "check")]
    gate: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.gate {
        rust_ai_native_specmap::run_gate(&cli.path)
    } else {
        rust_ai_native_specmap::run_specmap(&cli.path, cli.check)
    }
}
