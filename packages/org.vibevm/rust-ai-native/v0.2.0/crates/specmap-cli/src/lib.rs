//! The specmap engine driver (PROP-014 §2.5): build (or `--check`) the
//! canonical `specmap.json` traceability index over a project tree, plus the
//! orphan ratchet gate that rides every run.
//!
//! This was extracted out of vibevm's `xtask` so the rust-ai-native package
//! ships a *runnable* engine, not a description of one (PROP-024 code-bearing
//! packages). vibevm's `cargo xtask specmap` is now a thin shim over this
//! library, and the `specmap-rust` binary (`src/main.rs`) is what an installed
//! consumer runs in its own project. The policy is data (`specmap.toml`),
//! never hardcoded here — the same engine runs on any layout, exactly as
//! `conform-cli` ships the conform gate.

use std::path::Path;

use anyhow::{Result, bail};
use specmap_core::config::Config;

/// Build (or `--check`) the index over the tree at `root`, then run the orphan
/// ratchet. `check` byte-compares against the committed `specmap.json` and
/// fails on drift; otherwise it rewrites the index. The ratchet runs in both
/// modes, blocking only under `check`. An absent `specmap.toml` yields the
/// default scan and leaves the gate off.
pub fn run_specmap(root: &Path, check: bool) -> Result<()> {
    let cfg = Config::load(root)?;
    let scan_cfg = cfg.clone().unwrap_or_default();
    if check {
        match specmap_core::index::check(root, &scan_cfg)? {
            Ok(summary) => eprintln!("specmap --check: clean ({summary})."),
            Err(msg) => bail!("{msg}"),
        }
    } else {
        let (path, summary) = specmap_core::index::write(root, &scan_cfg)?;
        eprintln!("specmap: wrote {} ({summary}).", path.display());
    }
    match cfg {
        Some(cfg) => run_ratchet_gate(root, &cfg, check),
        None => Ok(()),
    }
}

/// The Phase 2 ratchet: the orphan gate over non-exempt crates (PLAYBOOK
/// `#phase2`). Reported in both modes; blocking only under `--check`. Only
/// runs when a `specmap.toml` is present (an absent policy leaves it off).
fn run_ratchet_gate(root: &Path, cfg: &Config, blocking: bool) -> Result<()> {
    let map = specmap_core::index::build(root, cfg);
    let orphans = specmap_core::ratchet::orphans(root, &map, cfg);
    let mut blockers = 0usize;
    for o in &orphans {
        match &o.disposition {
            Some(debt) => eprintln!(
                "  ratchet: orphan dispositioned ({debt}): `{}` ({}) at {}:{}",
                o.symbol, o.item_kind, o.file, o.line
            ),
            None => {
                blockers += 1;
                eprintln!(
                    "  ratchet: ORPHAN `{}` ({}) at {}:{} — tag it, scope! its module, \
                     or disposition it in specmap.toml with a debt id",
                    o.symbol, o.item_kind, o.file, o.line
                );
            }
        }
    }
    eprintln!(
        "specmap: ratchet gate — {} gated orphan(s), {} dispositioned ({} crate(s) exempt).",
        blockers,
        orphans.len() - blockers,
        cfg.exempt.len()
    );
    if blocking && blockers > 0 {
        bail!(
            "specmap ratchet: {blockers} orphan(s) in gated crates — \
             see the list above (PLAYBOOK #phase2 acceptance: empty or dispositioned)"
        );
    }
    Ok(())
}
