//! `cargo xtask specmap` — regenerate (or `--check`) the canonical
//! `specmap.json` traceability index (PROP-014 §2.5), plus the Phase 2
//! orphan ratchet gate that rides every run.

use anyhow::{Result, bail};

use specmap_core::config::Config;

use crate::repo_root;

pub(crate) fn run_specmap(check: bool) -> Result<()> {
    let root = repo_root()?;
    // The policy file drives the scan + the ratchet (Traceability Relocation
    // Plan, Phase 2). Absent `specmap.toml`: default scan roots and the
    // ratchet gate off — the pre-config behaviour.
    let cfg = Config::load(&root)?;
    let scan_cfg = cfg.clone().unwrap_or_default();
    if check {
        match specmap_core::index::check(&root, &scan_cfg)? {
            Ok(summary) => {
                eprintln!("xtask specmap --check: clean ({summary}).");
            }
            Err(msg) => bail!("{msg}"),
        }
    } else {
        let (path, summary) = specmap_core::index::write(&root, &scan_cfg)?;
        eprintln!("xtask specmap: wrote {} ({summary}).", path.display());
    }
    match cfg {
        Some(cfg) => run_ratchet_gate(&root, &cfg, check),
        None => Ok(()),
    }
}

/// The Phase 2 ratchet: the orphan gate over non-exempt crates
/// (PLAYBOOK #phase2 "flip the ratchet"). Reported in both modes;
/// blocking only under `--check`. Only runs when a `specmap.toml` is
/// present (an absent policy file leaves the gate off).
fn run_ratchet_gate(root: &std::path::Path, cfg: &Config, blocking: bool) -> Result<()> {
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
        "xtask specmap: ratchet gate — {} gated orphan(s), {} dispositioned ({} crate(s) exempt).",
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
