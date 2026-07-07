//! `vibe bin` — build and dispatch the tools installed packages declare
//! via `[[binary]]` (PROP-025 §§3–4). The resolution/build cell lives in
//! `vibe_workspace::bins` (shared with the tcg oracle registry,
//! PROP-026 §4 — one implementation, two consumers); this file is the
//! CLI's thin verbs over it.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-025#dispatch");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_workspace::bins::{build_binary, collect_binaries, find_binary};

/// `vibe bin list`.
pub fn run_list(project_root: &Path) -> Result<()> {
    let bins = collect_binaries(project_root)?;
    if bins.is_empty() {
        eprintln!("bin list: no installed package declares a [[binary]].");
        return Ok(());
    }
    for bin in &bins {
        let state = if bin.artifact().exists() {
            "built"
        } else {
            "not built"
        };
        println!(
            "{}\t{}\t{}\t{}",
            bin.decl.name,
            bin.package,
            state,
            bin.decl.description.as_deref().unwrap_or("")
        );
    }
    eprintln!(
        "{} binar{} declared.",
        bins.len(),
        if bins.len() == 1 { "y" } else { "ies" }
    );
    Ok(())
}

/// `vibe bin build [<names>…]`.
pub fn run_build(project_root: &Path, names: &[String], assume_yes: bool) -> Result<()> {
    let bins = collect_binaries(project_root)?;
    if bins.is_empty() {
        bail!("bin build: no installed package declares a [[binary]]");
    }
    let selected: Vec<_> = if names.is_empty() {
        bins.iter().collect()
    } else {
        let mut chosen = Vec::new();
        for name in names {
            chosen.push(find_binary(&bins, name)?);
        }
        chosen
    };
    for bin in selected {
        build_binary(bin, assume_yes)?;
    }
    Ok(())
}

/// `vibe bin path <name>` — the artifact path; non-zero when unbuilt.
pub fn run_path(project_root: &Path, name: &str) -> Result<()> {
    let bins = collect_binaries(project_root)?;
    let bin = find_binary(&bins, name)?;
    let artifact = bin.artifact();
    if !artifact.exists() {
        bail!(
            "`{name}` is declared by {} but not built — run `vibe bin build {name}`",
            bin.package
        );
    }
    println!("{}", artifact.display());
    Ok(())
}

/// `vibe bin exec <name> -- <args…>` — build-if-missing, then exec with
/// the exit code passed through.
pub fn run_exec(project_root: &Path, name: &str, args: &[String], assume_yes: bool) -> Result<i32> {
    let bins = collect_binaries(project_root)?;
    let bin = find_binary(&bins, name)?;
    if !bin.artifact().exists() {
        build_binary(bin, assume_yes)?;
    }
    let status = std::process::Command::new(bin.artifact())
        .args(args)
        .status()
        .with_context(|| format!("spawning {}", bin.artifact().display()))?;
    Ok(status.code().unwrap_or(1))
}
