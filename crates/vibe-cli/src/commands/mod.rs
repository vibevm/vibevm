//! Sub-command implementations. Each module keeps `pub fn run(&Context, args) -> anyhow::Result<()>`.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#cli-surface");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::Manifest;

/// Resolve and validate a project root: canonicalise `path`, strip the
/// Windows `\\?\` verbatim prefix, and require a `vibe.toml` (the commands
/// that operate on a project — `agentic`, `skill` — share this guard so the
/// "run `vibe init` first" message and the UNC handling stay in one place).
pub(crate) fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

pub mod agentic;
pub mod aiui;
pub mod bin;
pub mod check;
pub mod init;
pub mod install;
pub mod list;
pub mod mcp;
pub mod outdated;
pub mod prefs;
pub mod registry;
pub mod reinstall;
pub mod search;
pub mod short_name;
pub mod show;
pub mod skill;
pub mod trace;
pub mod tree;
pub mod uninstall;
pub mod update;
pub mod vars;
pub mod vvm;
pub mod workspace;
