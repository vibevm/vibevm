//! `vibe agentic …` (relay producers) and `vibe command` (the relay
//! consumer) — the agentic relay (PROP-018 §2.7, §2.10).
//!
//! A `vibe agentic <op>` command composes an [`Intent`] and hands it to the
//! relay backend, which parks it in `.vibe/agentic/command.md` rather than
//! acting — vibevm has no inference engine of its own yet, so the reasoning
//! is delegated back to the calling agent. The agent then runs `vibe
//! command`, which prints the parked instruction to stdout and clears the
//! slot. The library (`vibe-mcp::agentic`) owns the relay; this module is
//! the CLI dispatch and rendering.
//!
//! [`Intent`]: vibe_mcp::agentic::Intent

specmark::scope!("spec://vibevm/common/PROP-018#relay");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::Manifest;
use vibe_mcp::agentic::{
    BackendOutcome, InferenceBackend, RelayBackend, drain_intent, explain_intent, relay_dir,
};

use crate::cli::{AgenticArgs, AgenticExplainArgs, AgenticSubcommand, CommandArgs};
use crate::output;

pub fn run(ctx: &output::Context, args: AgenticArgs) -> Result<()> {
    match args.command {
        AgenticSubcommand::Explain(sub) => run_explain(ctx, sub),
    }
}

fn run_explain(ctx: &output::Context, args: AgenticExplainArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let intent = explain_intent(&project_root);
    let backend = RelayBackend::for_project(&project_root);
    let outcome = backend.submit(&intent)?;
    let mailbox = relay_dir(&project_root)
        .join("command.md")
        .display()
        .to_string()
        .replace('\\', "/");

    match outcome {
        BackendOutcome::Delegated { pointer } => {
            if ctx.is_json() {
                ctx.emit_json(&serde_json::json!({
                    "ok": true,
                    "command": "agentic:explain",
                    "delegated": true,
                    "mailbox": mailbox,
                    "next": "vibe command",
                }))?;
            } else {
                ctx.summary(&pointer);
            }
        }
        // No built-in backend in the MVP — handled for forward-compat only.
        BackendOutcome::Completed(result) => {
            if ctx.is_json() {
                ctx.emit_json(&serde_json::json!({
                    "ok": true,
                    "command": "agentic:explain",
                    "delegated": false,
                    "result": result,
                }))?;
            } else {
                ctx.summary(&result);
            }
        }
    }
    Ok(())
}

/// `vibe command` — drain the relay: print the pending instruction parked
/// by a `vibe agentic …` command and clear the slot (PROP-018 §2.7). The
/// instruction goes to stdout verbatim so the agent can act on it; an empty
/// mailbox is a clean, exit-0 "no pending command".
pub fn run_command(ctx: &output::Context, args: CommandArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let dir = relay_dir(&project_root);
    match drain_intent(&dir)? {
        Some(content) => {
            if ctx.is_json() {
                ctx.emit_json(&serde_json::json!({
                    "ok": true,
                    "command": "command",
                    "pending": true,
                    "intent": content,
                }))?;
            } else {
                // Emit the instruction verbatim for the agent to execute.
                print!("{content}");
            }
        }
        None => {
            if ctx.is_json() {
                ctx.emit_json(&serde_json::json!({
                    "ok": true,
                    "command": "command",
                    "pending": false,
                }))?;
            } else {
                ctx.summary("no pending command");
            }
        }
    }
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}
