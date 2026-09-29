//! `typescript-ai-native tripwire` — list debt-registry entries whose
//! `touch:` tripwires fire on the current change set. Warn-only by
//! contract. The evaluation is `specmap_core::tripwire` (language-free
//! registry + glob matching); the change-set collection duplicates the
//! Rust stack's driver — a cross-package crate dep would couple the
//! stacks for sixty lines of git plumbing.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Collect the change set as repo-relative forward-slash paths.
fn changed_paths(root: &Path, base: Option<&str>) -> Result<Vec<String>> {
    let mut args_sets: Vec<Vec<String>> = Vec::new();
    match base {
        Some(rev) => {
            args_sets.push(vec![
                "diff".into(),
                "--name-only".into(),
                format!("{rev}...HEAD"),
            ]);
            args_sets.push(vec!["diff".into(), "--name-only".into(), "HEAD".into()]);
        }
        None => {
            args_sets.push(vec!["diff".into(), "--name-only".into(), "HEAD".into()]);
            args_sets.push(vec!["diff".into(), "--name-only".into(), "--cached".into()]);
        }
    }
    args_sets.push(vec![
        "ls-files".into(),
        "--others".into(),
        "--exclude-standard".into(),
    ]);

    let mut paths: Vec<String> = Vec::new();
    for args in args_sets {
        let out = Command::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .context("spawning git")?;
        if !out.status.success() {
            bail!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let p = line.trim().replace('\\', "/");
            if !p.is_empty() && !paths.contains(&p) {
                paths.push(p);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn run_tripwire(root: &Path, base: Option<&str>, debt_rel: &str) -> Result<()> {
    let debt_path = root.join(debt_rel);
    let debt_json = std::fs::read_to_string(&debt_path)
        .with_context(|| format!("reading {}", debt_path.display()))?;
    let changed = changed_paths(root, base)?;
    if changed.is_empty() {
        eprintln!("tripwire: change set is empty — nothing to match.");
        return Ok(());
    }
    let fired = specmap_core::tripwire::evaluate(&debt_json, &changed)?;
    if fired.is_empty() {
        eprintln!(
            "tripwire: no debt tripwires fire on {} changed path(s).",
            changed.len()
        );
        return Ok(());
    }
    eprintln!(
        "tripwire: {} debt entr{} fire on this change set — address each in the \
         PR description: pulled-in, re-dispositioned, or consciously deferred \
         (PLAYBOOK §7.5):",
        fired.len(),
        if fired.len() == 1 { "y" } else { "ies" }
    );
    for f in &fired {
        eprintln!(
            "  {} [{}] {} — disposition: {}",
            f.id, f.severity, f.title, f.disposition
        );
        for (pattern, paths) in &f.hits {
            for p in paths {
                eprintln!("      touch:{pattern} ← {p}");
            }
        }
        for un in &f.unevaluated {
            eprintln!("      (rev tripwire not yet evaluable: {un})");
        }
    }
    Ok(())
}
