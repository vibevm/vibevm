//! Sub-command implementations. Each module keeps `pub fn run(&Context, args) -> anyhow::Result<()>`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

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
pub mod cache;
pub mod check;
pub mod clean;
/// The command-level compile-trace owner and the one command-exit join. Not
/// `pub`: nothing outside the CLI may hold a recorder, and the four report
/// consumers that will use it are all in this crate.
pub(crate) mod compile_trace;
pub mod deploy;
pub mod explain;
pub mod extensions;
pub mod extensions_analyze;
pub mod facts;
pub(crate) mod facts_check;
pub mod friends;
pub mod init;
pub mod install;
pub mod lifecycle;
pub mod list;
pub mod mcp;
pub mod outdated;
pub mod prefs;
pub mod progress;
pub mod progress_evidence;
pub mod query;
pub mod refactor;
pub mod registry;
pub mod reinstall;
pub mod requirements;
pub mod search;
pub mod select;
pub mod short_name;
pub mod show;
pub mod skill;
pub mod specmap;
pub mod term;
pub mod tools;
pub mod trace;
pub mod tree;
pub mod uninstall;
pub mod update;
pub mod vars;
pub mod vvm;
pub mod why;
pub mod workspace;
