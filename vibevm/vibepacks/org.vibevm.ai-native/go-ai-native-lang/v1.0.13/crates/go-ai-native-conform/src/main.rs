//! `go-ai-native-conform` — the Go structural gate, standalone.
//! Family-prefixed like its siblings so several discipline checkers
//! share a PATH without shadowing one another (PROP-028 §2.4).

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "go-ai-native-conform",
    about = "AI-Native discipline conform gate for Go (the go-extract frontend \
             over the language-neutral engine)"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Extract facts through the stdlib-only go-extract sidecar, run
    /// the Go rules, emit SARIF, and gate new findings against the
    /// ratchet baseline.
    Check {
        /// Project root.
        #[arg(long, default_value = ".")]
        path: String,
        /// Path to the frozen-findings baseline, root-relative.
        #[arg(long, default_value = go_ai_native_conform::DEFAULT_GO_BASELINE)]
        baseline: String,
        /// Report findings only under this root-relative path prefix.
        #[arg(long)]
        scope: Option<String>,
    },
    /// Rewrite the baseline to the current finding set.
    Freeze {
        /// Project root.
        #[arg(long, default_value = ".")]
        path: String,
        /// Path to the baseline to rewrite, root-relative.
        #[arg(long, default_value = go_ai_native_conform::DEFAULT_GO_BASELINE)]
        baseline: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Check {
            path,
            baseline,
            scope,
        } => go_ai_native_conform::run_check(
            std::path::Path::new(&path),
            &baseline,
            scope.as_deref(),
        ),
        Cmd::Freeze { path, baseline } => {
            go_ai_native_conform::run_freeze(std::path::Path::new(&path), &baseline)
        }
    }
}
