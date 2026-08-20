//! `cargo xtask specmap` — regenerate (or `--check`) the canonical
//! `specmap.json` traceability index (PROP-014 §2.5), plus the gates that
//! ride every run: the engine's orphan ratchet, and the host-resolution
//! gate (B-076) added here. A thin shim over the `rust-ai-native-specmap`
//! library, which ships in stack:org.vibevm.ai-native/rust-ai-native-lang
//! (PROP-024 code-bearing packages) — the same relationship `cargo xtask
//! conform` has with `rust-ai-native-conform`.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::repo_root;

pub(crate) fn run_specmap(check: bool) -> Result<()> {
    let root = repo_root()?;
    rust_ai_native_specmap::run_specmap(&root, check)?;
    run_resolve_gate(&root, check)
}

/// The host-resolution gate (B-076): every map edge whose far end is an
/// address in THIS project's own `spec://` namespace must resolve to a
/// unit the map actually carries. A host symbol citing a host anchor that
/// no spec unit mints is a dead citation (a typo'd or renamed anchor), and
/// until this gate it rode the index green — the `--check` byte-compare
/// measures freshness and the ratchet measures orphans, neither measures
/// resolution.
///
/// Jurisdiction is a coordinate, never an enumerated address list: an edge
/// is host-space iff its URI starts with `spec://<namespace>/`, the
/// `<namespace>` being the `specmap.toml` field that defines what this
/// project mints. Far ends in any other namespace — installed packages'
/// specs (`spec://core-ai-native/…`, `…/ENGINE-CONFORM…` and kin) — are
/// outside this map's jurisdiction and pass SILENTLY: they ride the
/// committed index unresolved by design (resolution-only external specs
/// never serialise, so the map stays byte-reproducible), and a hardcoded
/// list of "legitimate" non-host addresses would rot the day a package
/// changes its URIs. The namespace prefix cannot rot: it is the same data
/// the minter itself reads.
///
/// The ratchet idiom: reported in both modes, blocking only under
/// `--check`. An absent `specmap.toml` leaves the gate off (the engine has
/// already said so for the ratchet, and the placeholder namespace would
/// test nothing real).
fn run_resolve_gate(root: &Path, blocking: bool) -> Result<()> {
    let cfg_path = root.join(specmap_core::config::Config::REL_PATH);
    if !cfg_path.exists() {
        return Ok(());
    }
    // The engine has already loaded (and warned about) this same file one
    // call ago; parse it directly for the namespace rather than paying the
    // `Config::load` warning block a second time.
    let text = std::fs::read_to_string(&cfg_path)
        .with_context(|| format!("reading specmap config {}", cfg_path.display()))?;
    let cfg: specmap_core::config::Config = toml::from_str(&text)
        .with_context(|| format!("parsing specmap config {}", cfg_path.display()))?;
    let map = specmap_core::index::build(root, &cfg);
    let host_prefix = format!("spec://{}/", cfg.namespace);
    let carried: BTreeSet<&str> = map.specUnits.iter().map(|u| u.uri.as_str()).collect();
    let (mut blockers, mut outside) = (0usize, 0usize);
    for e in &map.edges {
        if e.uri.starts_with(&host_prefix) {
            if carried.contains(e.uri.as_str()) {
                continue;
            }
            blockers += 1;
            eprintln!(
                "  resolve: `{}` --{}--> `{}` at {}:{} — no unit of this map carries \
                 that address; fix the citation in code, never add a dead anchor to \
                 the spec",
                e.fromSymbol,
                verb_str(e),
                e.uri,
                e.file,
                e.line
            );
        } else if !carried.contains(e.uri.as_str()) {
            outside += 1;
        }
    }
    eprintln!(
        "specmap: resolve gate — {blockers} unresolved host edge(s), {outside} \
         non-host edge(s) outside this map's jurisdiction."
    );
    if blocking && blockers > 0 {
        bail!(
            "specmap resolve: {blockers} host edge(s) resolve to nothing in this map — \
             see the list above (every `spec://{}/…` a host symbol cites must exist)",
            cfg.namespace
        );
    }
    Ok(())
}

/// The wire verb of an edge, in the engine's own `dangling-edge` wording,
/// so a red resolve line reads exactly like the warning it upgrades.
fn verb_str(e: &specmap_core::generated::specmap::Edge) -> &'static str {
    use specmap_core::generated::specmap::EdgeVerb::*;
    match e.verb {
        Implements => "implements",
        Verifies => "verifies",
        Documents => "documents",
        Deviates => "deviates",
        Informs => "informs",
    }
}
