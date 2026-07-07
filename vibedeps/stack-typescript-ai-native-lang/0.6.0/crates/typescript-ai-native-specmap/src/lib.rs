//! The specmap driver for TypeScript trees (the `specmap-rust` twin):
//! build (or `--check`) the canonical `specmap.json` through the
//! `ts-tsc` scanner, then run the TS orphan gate. One extractor run
//! feeds both halves — the index's markers and the gate's export
//! inventory come from the same records.

use std::path::Path;

use anyhow::{Result, bail};
use specmap_core::config::Config;
use typescript_ai_native_specmap_scan::RecordsScanner;

fn extract(root: &Path) -> Result<Vec<typescript_ai_native_extract_bridge::FileRecord>> {
    let extractor = typescript_ai_native_extract_bridge::materialise_extractor(root)?;
    typescript_ai_native_extract_bridge::extract_tree(root, &extractor, None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Build (or `--check`) the index, then the orphan gate — blocking
/// only under `check`, mirroring `specmap-rust`. An absent
/// `specmap.toml` yields the default scan and leaves the gate off.
pub fn run_specmap_typescript(root: &Path, check: bool) -> Result<()> {
    let cfg = Config::load(root)?;
    if cfg.is_none() {
        eprintln!(
            "typescript-ai-native-specmap: NO specmap.toml — placeholder namespace `project` in force \
             and the orphan gate is off; run `typescript-ai-native init` to write a starting \
             policy."
        );
    }
    let scan_cfg = cfg.clone().unwrap_or_default();
    let records = extract(root)?;
    let scanner = RecordsScanner::new(&records, "src");
    let summary = if check {
        match specmap_core::index::check_with_scanner(root, &scan_cfg, &scanner)? {
            Ok(summary) => {
                eprintln!("typescript-ai-native-specmap --check: clean ({summary}).");
                summary
            }
            Err(msg) => bail!("{msg}"),
        }
    } else {
        let (path, summary) = specmap_core::index::write_with_scanner(root, &scan_cfg, &scanner)?;
        eprintln!(
            "typescript-ai-native-specmap: wrote {} ({summary}).",
            path.display()
        );
        summary
    };
    // The vacuity warning rides only a CONFIGURED scan (mirroring the Rust
    // driver, where it fires from the policy-gated ratchet path): an absent
    // specmap.toml already announced itself above, and a default scan with
    // nothing tagged is the normal pre-adoption state.
    if cfg.is_some()
        && let Some(w) = specmap_core::index::vacuity_warning(&summary)
    {
        eprintln!("typescript-ai-native-specmap: WARNING — {w}.");
    }
    match cfg {
        Some(cfg) => run_gate_over(&records, &cfg, check),
        None => Ok(()),
    }
}

/// Orphan-coverage gate only (`--gate`), no committed index read or
/// written — the package-self-trace twin of `specmap-rust --gate`.
pub fn run_gate(root: &Path) -> Result<()> {
    match Config::load(root)? {
        Some(cfg) => {
            let records = extract(root)?;
            run_gate_over(&records, &cfg, true)
        }
        None => Ok(()),
    }
}

fn run_gate_over(
    records: &[typescript_ai_native_extract_bridge::FileRecord],
    cfg: &Config,
    blocking: bool,
) -> Result<()> {
    let orphans = typescript_ai_native_specmap_scan::orphans(records, cfg);
    for o in &orphans {
        eprintln!(
            "  ratchet: ORPHAN `{}` ({}) at {}:{} — tag it with a JSDoc spec marker, \
             `@scope` its file, or exempt the root in specmap.toml",
            o.symbol, o.item_kind, o.file, o.line
        );
    }
    eprintln!(
        "typescript-ai-native-specmap: ratchet gate — {} orphan(s) ({} root(s) exempt).",
        orphans.len(),
        cfg.exempt.len()
    );
    if blocking && !orphans.is_empty() {
        bail!(
            "typescript-ai-native-specmap: {} untagged export(s) — the orphan ratchet blocks",
            orphans.len()
        );
    }
    Ok(())
}
