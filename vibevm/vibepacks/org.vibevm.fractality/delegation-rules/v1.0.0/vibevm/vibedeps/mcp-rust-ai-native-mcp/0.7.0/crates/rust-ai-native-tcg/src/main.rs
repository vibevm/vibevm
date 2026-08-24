//! bin `rust-ai-native-tcg` — the agentic type oracle's CLI face
//! (TCG-PROTOCOL-RUST v0.1): the persistent `serve` relay, the
//! one-shot validate/scope/complete/type forms (the agent-without-MCP
//! path and the debug surface), and the bench harness.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rust_ai_native_tcg::{
    Policy, ScopeAnswer, derive_crate_module, detect_newtypes, enrich_validate,
    finalise_completions, parse_position, read_content_from, seam_file_for, spawn_oracle,
    validate_exit_code,
};

#[derive(Parser)]
#[command(
    name = "rust-ai-native-tcg",
    about = "The agentic type oracle for Rust: rust-analyzer answers, \
             discipline-enriched by the same conform rules as the gate. \
             The floor stays the truth (TCG-ORACLE-RUST §5)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// The persistent enriching relay an MCP host drives (stdio).
    Serve {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Type-check a file (optionally hypothetical content, never
    /// touching disk); exit 1 = an error diagnostic OR a non-baselined
    /// finding.
    Validate {
        file: String,
        /// Hypothetical content: a path, or `-` for stdin.
        #[arg(long)]
        content_from: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// In-scope symbols + the file's cell, seam, and newtype brands.
    Scope {
        file: String,
        /// `L:C` — 1-based line, 0-based character.
        #[arg(long)]
        position: Option<String>,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Type-valid completions at a position (prefix-filtered).
    Complete {
        file: String,
        #[arg(long)]
        position: String,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long, default_value_t = 50)]
        max: usize,
        #[arg(long)]
        content_from: Option<String>,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Quick info (type display + docs) at a position.
    Type {
        file: String,
        #[arg(long)]
        position: String,
        #[arg(long)]
        content_from: Option<String>,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// The differential corpus + latency harness (the REPORT is the
    /// ratchet; targets are recorded, never CI-gated).
    Bench {
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        report: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

fn resolve_root(root: &std::path::Path) -> PathBuf {
    rust_ai_native_tcg_bridge::verbatim_free(
        &root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let code = match cli.cmd {
        Cmd::Serve { root } => rust_ai_native_tcg::serve::run_serve(&resolve_root(&root))?,
        Cmd::Validate {
            file,
            content_from,
            json,
            root,
        } => {
            let root = resolve_root(&root);
            let policy = Policy::load(&root)?;
            let content = content_from.as_deref().map(read_content_from).transpose()?;
            let effective = match &content {
                Some(c) => c.clone(),
                None => std::fs::read_to_string(root.join(&file))?,
            };
            let mut oracle = spawn_oracle(&root)?;
            let raw = oracle
                .validate(&file, Some(effective.clone()))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let enriched = enrich_validate(&policy, &file, &effective, raw);
            let exit = validate_exit_code(&enriched);
            if json {
                println!("{}", serde_json::to_string_pretty(&enriched)?);
            } else {
                for d in &enriched.diagnostics {
                    println!(
                        "{}:{}:{} {} {} — {}",
                        file, d.line, d.character, d.category, d.code, d.message
                    );
                }
                for f in &enriched.conform_findings {
                    println!(
                        "finding {}:{} {}{} — {}",
                        file,
                        f.line,
                        f.rule,
                        if f.baselined { " [baselined]" } else { "" },
                        f.message
                    );
                }
                for a in &enriched.advice {
                    println!("advice: {a}");
                }
                println!(
                    "validate: {} diagnostic(s), {} finding(s) ({} new){}",
                    enriched.diagnostics.len(),
                    enriched.conform_findings.len(),
                    enriched
                        .conform_findings
                        .iter()
                        .filter(|f| !f.baselined)
                        .count(),
                    if enriched.degraded { " [degraded]" } else { "" },
                );
            }
            let _ = oracle.shutdown();
            exit
        }
        Cmd::Scope {
            file,
            position,
            root,
        } => {
            let root = resolve_root(&root);
            let policy = Policy::load(&root)?;
            let pos = position
                .as_deref()
                .map(parse_position)
                .transpose()?
                .unwrap_or(rust_ai_native_tcg_bridge::position::OuterPosition {
                    line: 1,
                    character: 0,
                });
            let mut oracle = spawn_oracle(&root)?;
            let entries = oracle
                .complete(&file, pos, None)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let symbols = finalise_completions(entries, &file, None, 200);
            let (_crate_name, module) = derive_crate_module(&policy.config.roots, &file);
            let seam = seam_file_for(&root, &file);
            let mut branded = std::fs::read_to_string(root.join(&file))
                .map(|t| detect_newtypes(&t, &file))
                .unwrap_or_default();
            if seam != file
                && let Ok(seam_text) = std::fs::read_to_string(root.join(&seam))
            {
                branded.extend(detect_newtypes(&seam_text, &seam));
            }
            let answer = ScopeAnswer {
                symbols,
                cell: module,
                seam_file: seam,
                branded,
            };
            println!("{}", serde_json::to_string_pretty(&answer)?);
            let _ = oracle.shutdown();
            0
        }
        Cmd::Complete {
            file,
            position,
            prefix,
            max,
            content_from,
            root,
        } => {
            let root = resolve_root(&root);
            let pos = parse_position(&position)?;
            let content = content_from.as_deref().map(read_content_from).transpose()?;
            let mut oracle = spawn_oracle(&root)?;
            let entries = oracle
                .complete(&file, pos, content)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let out = finalise_completions(entries, &file, prefix.as_deref(), max.max(1));
            println!("{}", serde_json::to_string_pretty(&out)?);
            let _ = oracle.shutdown();
            0
        }
        Cmd::Type {
            file,
            position,
            content_from,
            root,
        } => {
            let root = resolve_root(&root);
            let pos = parse_position(&position)?;
            let content = content_from.as_deref().map(read_content_from).transpose()?;
            let mut oracle = spawn_oracle(&root)?;
            let (display, documentation) = oracle
                .hover(&file, pos, content)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!(
                "{}",
                serde_json::json!({ "display": display, "documentation": documentation })
            );
            let _ = oracle.shutdown();
            0
        }
        Cmd::Bench {
            corpus,
            report,
            root,
        } => rust_ai_native_tcg::bench::run_bench(&corpus, &report, &resolve_root(&root))?,
    };
    std::process::exit(code);
}
