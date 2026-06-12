//! `vibe show <subcommand>` — inspect computed project state.
//!
//! v0 ships two pure-inspection subcommands; the M1.5+ runner-aware
//! ones (`graph`, `node`, `plan`) land alongside the LLM-build
//! pipeline.
//!
//! - `vibe show effective` — concatenate `spec/boot/*.md` (sorted by
//!   the canonical `NN-` prefix) and every installed package's
//!   `files_written` (in lockfile order), each preceded by a
//!   `spec://` provenance header so a cold reader knows which
//!   package contributed which content.
//! - `vibe show config` — dump the effective configuration: every
//!   `[[registry]]`, `[[mirror]]`, `[[override]]` from `vibe.toml`,
//!   plus the runtime knobs read from environment variables, each
//!   tagged with `provenance` so the operator sees where a value
//!   actually came from.
//!
//! Spec: `VIBEVM-SPEC.md` §9.5 (configuration sources / provenance),
//! §4.6 (effective spec), ROADMAP §M1.4.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

mod config;
mod effective;
mod features;
mod purls;
mod subskills;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::Manifest;

use crate::cli::{ShowArgs, ShowSubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: ShowArgs) -> Result<()> {
    match args.command {
        ShowSubcommand::Effective(sub) => effective::run_effective(ctx, sub),
        ShowSubcommand::Config(sub) => config::run_config(ctx, sub),
        ShowSubcommand::Features(sub) => features::run_features(ctx, sub),
        ShowSubcommand::Subskills(sub) => subskills::run_subskills(ctx, sub),
        ShowSubcommand::Purls(sub) => purls::run_purls(ctx, sub),
    }
}

// ===================== shared =====================

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first or pass `--path <dir>` pointing at a project root",
            stripped.display()
        );
    }
    Ok(stripped)
}
