//! `typescript-ai-native-conform` — the TypeScript structural gate, standalone.
//! Language-suffixed like `conform-rust` so several discipline checkers
//! share a PATH without shadowing one another (the Ф6 brief §4).

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "typescript-ai-native-conform",
    about = "AI-Native discipline conform gate for TypeScript (the ts-tsc frontend \
             over the language-neutral engine)"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Extract facts through the project's own `typescript` install,
    /// run the TS rules, emit SARIF, and gate new findings against the
    /// ratchet baseline.
    Check {
        /// Project root.
        #[arg(long, default_value = ".")]
        path: String,
        /// Path to the frozen-findings baseline, root-relative.
        #[arg(long, default_value = typescript_ai_native_conform::DEFAULT_TS_BASELINE)]
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
        #[arg(long, default_value = typescript_ai_native_conform::DEFAULT_TS_BASELINE)]
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
        } => typescript_ai_native_conform::run_check(
            std::path::Path::new(&path),
            &baseline,
            scope.as_deref(),
        ),
        Cmd::Freeze { path, baseline } => {
            typescript_ai_native_conform::run_freeze(std::path::Path::new(&path), &baseline)
        }
    }
}
