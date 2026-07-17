//! `go-ai-native-tcg` — the agentic type oracle for Go: the persistent
//! enriching relay (`serve`) an MCP host drives, one-shot
//! validate/scope/complete/type forms, and the differential/latency
//! bench harness (TCG-PROTOCOL-GO v0.1).

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "go-ai-native-tcg",
    about = "The agentic type oracle for Go: a persistent enriching relay over \
             the consumer's gopls, one-shot validate/scope/complete/type, and \
             the bench harness"
)]
struct Cli {
    /// Project root.
    #[arg(long, global = true, default_value = ".")]
    root: String,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// The persistent NDJSON relay (TCG-PROTOCOL-GO §1): frames on
    /// stdin, frames on stdout; self-inits so the first frame may be
    /// any op.
    Serve,
    /// Validate one file (optionally hypothetical content) and print
    /// the enriched answer. Exit 1 = an error-grade diagnostic or a
    /// non-baselined conform finding.
    Validate {
        /// Repo-relative file.
        file: String,
        /// Read the hypothetical content from this path (`-` = stdin).
        #[arg(long)]
        content_from: Option<String>,
    },
    /// In-scope symbols + cell/seam context + defined-type brands.
    Scope {
        file: String,
        /// `L:C` (1-based line, 0-based character).
        #[arg(long)]
        position: Option<String>,
        #[arg(long)]
        content_from: Option<String>,
    },
    /// Type-valid completions at a position.
    Complete {
        file: String,
        /// `L:C` (1-based line, 0-based character).
        #[arg(long)]
        position: String,
        #[arg(long)]
        content_from: Option<String>,
        /// Keep only names with this prefix.
        #[arg(long)]
        prefix: Option<String>,
        /// Cap the entry count (default 50).
        #[arg(long, default_value_t = 50)]
        max: usize,
    },
    /// Quick info (hover) at a position.
    Type {
        file: String,
        /// `L:C` (1-based line, 0-based character).
        #[arg(long)]
        position: String,
        #[arg(long)]
        content_from: Option<String>,
    },
    /// The differential/latency harness over a corpus (ORACLE-GO §5,
    /// §8): oracle vs `go build`, existence-grain, targets recorded
    /// never gated.
    Bench {
        /// The corpus JSON.
        #[arg(long)]
        corpus: String,
        /// Where to write the report JSON.
        #[arg(long)]
        report: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = std::path::PathBuf::from(&cli.root);
    match cli.command {
        Cmd::Serve => {
            let code = go_ai_native_tcg::serve::run_serve(&root)?;
            std::process::exit(code);
        }
        Cmd::Validate { file, content_from } => {
            let policy = go_ai_native_tcg::Policy::load(&root)?;
            let mut oracle = go_ai_native_tcg::spawn_oracle(&root)?;
            let text = match content_from {
                Some(spec) => go_ai_native_tcg::read_content_from(&spec)?,
                None => std::fs::read_to_string(root.join(&file))?,
            };
            let raw = oracle
                .validate(&file, Some(text.clone()))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let enriched = go_ai_native_tcg::enrich_validate(&policy, &file, &text, raw);
            println!("{}", serde_json::to_string_pretty(&enriched)?);
            let code = go_ai_native_tcg::validate_exit_code(&enriched);
            let _ = oracle.shutdown();
            std::process::exit(code);
        }
        Cmd::Scope {
            file,
            position,
            content_from,
        } => {
            let policy = go_ai_native_tcg::Policy::load(&root)?;
            let mut oracle = go_ai_native_tcg::spawn_oracle(&root)?;
            let content = content_from
                .map(|s| go_ai_native_tcg::read_content_from(&s))
                .transpose()?;
            let pos = match position {
                Some(p) => go_ai_native_tcg::parse_position(&p)?,
                None => go_ai_native_tcg_bridge::position::OuterPosition {
                    line: 1,
                    character: 0,
                },
            };
            let entries = oracle
                .complete(&file, pos, content)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let symbols =
                go_ai_native_tcg::finalise_completions(&policy.config, entries, &file, None, 200);
            let cell = go_ai_native_tcg::cell_of(&policy.config, &file);
            let seam = go_ai_native_tcg::seam_file_for(&policy.config, &file);
            let mut branded = Vec::new();
            if let Ok(records) = go_ai_native_extract_bridge::extract_tree(
                &policy.root,
                &policy.extractor,
                Some(&[file.clone()]),
            ) {
                for record in &records {
                    branded.extend(go_ai_native_tcg::brands_of(record));
                }
            }
            let answer = go_ai_native_tcg::ScopeAnswer {
                symbols,
                cell,
                seam_file: seam,
                branded,
            };
            println!("{}", serde_json::to_string_pretty(&answer)?);
            let _ = oracle.shutdown();
            Ok(())
        }
        Cmd::Complete {
            file,
            position,
            content_from,
            prefix,
            max,
        } => {
            let policy = go_ai_native_tcg::Policy::load(&root)?;
            let mut oracle = go_ai_native_tcg::spawn_oracle(&root)?;
            let content = content_from
                .map(|s| go_ai_native_tcg::read_content_from(&s))
                .transpose()?;
            let pos = go_ai_native_tcg::parse_position(&position)?;
            let entries = oracle
                .complete(&file, pos, content)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let out = go_ai_native_tcg::finalise_completions(
                &policy.config,
                entries,
                &file,
                prefix.as_deref(),
                max.max(1),
            );
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "entries": out }))?);
            let _ = oracle.shutdown();
            Ok(())
        }
        Cmd::Type {
            file,
            position,
            content_from,
        } => {
            let mut oracle = go_ai_native_tcg::spawn_oracle(&root)?;
            let content = content_from
                .map(|s| go_ai_native_tcg::read_content_from(&s))
                .transpose()?;
            let pos = go_ai_native_tcg::parse_position(&position)?;
            let (display, documentation) = oracle
                .hover(&file, pos, content)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "display": display, "documentation": documentation,
                }))?
            );
            let _ = oracle.shutdown();
            Ok(())
        }
        Cmd::Bench { corpus, report } => go_ai_native_tcg::bench::run_bench(&root, &corpus, &report),
    }
}
