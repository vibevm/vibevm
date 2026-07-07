//! `tcg-typescript` — the agentic type oracle's delivery binary
//! (AGENTIC-TCG-TS-PLAN v0.1 D3; TCG-PROTOCOL-v0.1). Language-suffixed
//! like the other discipline binaries so several stacks share a PATH.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "tcg-typescript",
    about = "The agentic type oracle for TypeScript: a persistent enriching \
             relay (serve) over the language-service oracle, one-shot query \
             forms, and the bench harness"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// The persistent stdio relay an MCP host drives: TCG-PROTOCOL
    /// requests in, discipline-enriched responses out.
    Serve {
        /// Project root the oracle answers for.
        #[arg(long, default_value = ".")]
        root: String,
    },
    /// Validate one file (optionally with hypothetical content) —
    /// diagnostics + conform findings + advice; exit 1 when an error
    /// diagnostic or a non-baselined finding is present.
    Validate {
        /// Root-relative file path.
        file: String,
        /// Read hypothetical content from this path, or `-` for stdin.
        #[arg(long)]
        content_from: Option<String>,
        /// Emit the enriched result as JSON (default: human summary).
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = ".")]
        root: String,
    },
    /// In-scope symbols + cell/seam/branded context at a file (and
    /// optional L:C position).
    Scope {
        file: String,
        /// Position as `line:character` (1-based line, 0-based char).
        #[arg(long)]
        position: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = ".")]
        root: String,
    },
    /// Type-valid completions at a position (prefix-filtered).
    Complete {
        file: String,
        /// Position as `line:character`.
        #[arg(long)]
        position: String,
        /// Name prefix filter (the affordable-details cut).
        #[arg(long)]
        prefix: Option<String>,
        /// Entry cap after the prefix cut.
        #[arg(long, default_value_t = 50)]
        max: u64,
        /// Hypothetical content from a path or `-` for stdin.
        #[arg(long)]
        content_from: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = ".")]
        root: String,
    },
    /// Quick info (type display + docs) at a position.
    Type {
        file: String,
        /// Position as `line:character`.
        #[arg(long)]
        position: String,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = ".")]
        root: String,
    },
    /// Run the differential/latency corpus and write a report
    /// (research/tcg-bench feeds this; TCG-ORACLE §7 — measured, not
    /// CI-gated).
    Bench {
        /// Corpus directory (cases/*.json).
        #[arg(long)]
        corpus: PathBuf,
        /// Report JSON output path.
        #[arg(long)]
        report: PathBuf,
        #[arg(long, default_value = ".")]
        root: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Cmd::Serve { root } => tcg_cli_typescript::run_serve(std::path::Path::new(&root))?,
        Cmd::Validate {
            file,
            content_from,
            json,
            root,
        } => tcg_cli_typescript::run_validate(
            std::path::Path::new(&root),
            &file,
            content_from.as_deref(),
            json,
        )?,
        Cmd::Scope {
            file,
            position,
            json,
            root,
        } => tcg_cli_typescript::run_scope(
            std::path::Path::new(&root),
            &file,
            position.as_deref(),
            json,
        )?,
        Cmd::Complete {
            file,
            position,
            prefix,
            max,
            content_from,
            json,
            root,
        } => tcg_cli_typescript::run_complete(
            std::path::Path::new(&root),
            &file,
            &position,
            prefix.as_deref(),
            max,
            content_from.as_deref(),
            json,
        )?,
        Cmd::Type {
            file,
            position,
            json,
            root,
        } => tcg_cli_typescript::run_type(std::path::Path::new(&root), &file, &position, json)?,
        Cmd::Bench {
            corpus,
            report,
            root,
        } => tcg_cli_typescript::run_bench(std::path::Path::new(&root), &corpus, &report)?,
    };
    std::process::exit(code);
}
