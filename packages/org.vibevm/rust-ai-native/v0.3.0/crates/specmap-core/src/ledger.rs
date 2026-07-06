//! The intent ledger, local slice (LEDGER-INTENT v0.1; PLAYBOOK
//! Phase 5). Interpretations class only — the facts class is the
//! conform engine's store (ENGINE-CONFORM §3), which this module
//! deliberately does not touch: facts are keyed by
//! `(file content-hash, producer)` and survive every epoch change.
//!
//! One query kind ships: `explain.item` — the prose render behind
//! `discipline-rust trace explain --prose`. The producer is a
//! deterministic template (the tool MUST be fully useful without an
//! LLM; an LLM prose producer slots in later under its own producer
//! id + model id). Interpretations are keyed by
//! `(subject subgraph, epoch, producer)` per LEDGER §2; entries under
//! an old epoch are simply never looked up again — hard invalidation.
//!
//! Storage: `.ledger/objects/<sha256[0..2]>/<sha256>` plus
//! `.ledger/telemetry.json` (hit rate, cost, rot-rate plumbing).
//! Local per checkout; never shipped, never signed, never exposed —
//! `.ledger/` is git-ignored.

specmark::scope!("spec://vibevm/discipline/LEDGER-INTENT-v0.1#classes");

use std::path::{Path, PathBuf};

use crate::generated::specmap::Specmap;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::content_hash;

/// The contextual-invalidation epoch (LEDGER §3): a hash over the
/// context of meaning — dependency lockfiles, toolchain, the
/// discipline package in effect, and the metamodel wire schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epoch(pub String);

impl Epoch {
    /// Short display form for provenance lines.
    pub fn short(&self) -> &str {
        let hex = self.0.strip_prefix("sha256:").unwrap_or(&self.0);
        &hex[..8.min(hex.len())]
    }
}

/// Compute the epoch for a checkout. Inputs that exist contribute
/// their bytes; absent ones contribute their absence (the hash input
/// names them either way, so adding a lockfile later changes the
/// epoch — correctly).
pub fn epoch(root: &Path) -> Epoch {
    let mut acc = String::new();
    for rel in [
        "Cargo.lock",
        "vibe.lock",
        "schemas/specmap.jtd.json",
        "vibevm.discipline.lock",
    ] {
        acc.push_str(rel);
        acc.push('\n');
        match std::fs::read_to_string(root.join(rel)) {
            Ok(text) => acc.push_str(&content_hash(&text)),
            Err(_) => acc.push_str("<absent>"),
        }
        acc.push('\n');
    }
    acc.push_str("toolchain\n");
    let toolchain = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
    acc.push_str(&toolchain);
    Epoch(content_hash(&acc))
}

/// Telemetry counters (LEDGER §5): hit rate and cost feed the
/// Charter's headline metric; the rot counters are plumbing for the
/// contextual-rot rate, incremented when a re-verification of an
/// epoch-invalidated entry runs (none do yet — the template producer
/// recomputes from scratch, cost ~0).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Telemetry {
    pub hits: u64,
    pub misses: u64,
    pub rot_checks: u64,
    pub rot_changed: u64,
}

fn telemetry_path(root: &Path) -> PathBuf {
    root.join(".ledger").join("telemetry.json")
}

pub fn load_telemetry(root: &Path) -> Telemetry {
    std::fs::read_to_string(telemetry_path(root))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_telemetry(root: &Path, t: &Telemetry) -> Result<()> {
    let path = telemetry_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(t)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// One served prose render with its cache verdict.
#[derive(Debug)]
pub struct ProseRender {
    pub text: String,
    pub cached: bool,
    pub epoch: Epoch,
}

fn object_path(root: &Path, key_hex: &str) -> PathBuf {
    root.join(".ledger")
        .join("objects")
        .join(&key_hex[..2])
        .join(key_hex)
}

/// `explain.item` with a prose render (LEDGER §6 query kind 2): the
/// structured subgraph is the ground truth; the prose cites URIs; the
/// stored entry is keyed by `(subgraph, epoch, producer)` so an
/// epoch change makes yesterday's render unreachable while the
/// conform facts store stays untouched.
pub fn prose_explain(root: &Path, map: &Specmap, target: &str) -> Result<ProseRender> {
    const PRODUCER: &str = "explain.item/prose-template-1";
    let subgraph = crate::explain::explain_json(map, target)?;
    let subject = serde_json::to_string(&subgraph)?;
    let epoch = epoch(root);
    let key = content_hash(&format!("{PRODUCER}\n{}\n{subject}", epoch.0));
    let key_hex = key.strip_prefix("sha256:").unwrap_or(&key).to_string();
    let slot = object_path(root, &key_hex);

    let mut telemetry = load_telemetry(root);
    if let Ok(text) = std::fs::read_to_string(&slot) {
        telemetry.hits += 1;
        save_telemetry(root, &telemetry)?;
        return Ok(ProseRender {
            text,
            cached: true,
            epoch,
        });
    }

    let text = render_prose(&subgraph, target, &epoch, PRODUCER);
    if let Some(parent) = slot.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&slot, &text).with_context(|| format!("writing {}", slot.display()))?;
    telemetry.misses += 1;
    save_telemetry(root, &telemetry)?;
    Ok(ProseRender {
        text,
        cached: false,
        epoch,
    })
}

/// Deterministic template prose over the explain subgraph. Every
/// render ends with the provenance line (LEDGER §4) — the last line
/// of defense against staleness.
fn render_prose(
    subgraph: &serde_json::Value,
    target: &str,
    epoch: &Epoch,
    producer: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {target}\n\n"));
    let mut cited: Vec<String> = Vec::new();
    if let Some(edges) = subgraph.get("edges").and_then(|e| e.as_array()) {
        for e in edges {
            let verb = e.get("verb").and_then(|v| v.as_str()).unwrap_or("?");
            let uri = e.get("uri").and_then(|v| v.as_str()).unwrap_or("?");
            let from = e.get("from_symbol").and_then(|v| v.as_str()).unwrap_or("?");
            let line = e.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
            let file = e.get("file").and_then(|v| v.as_str()).unwrap_or("?");
            let pin = e
                .get("pinned_r")
                .and_then(|v| v.as_u64())
                .map(|r| format!(" (pinned r{r})"))
                .unwrap_or_default();
            out.push_str(&format!("- `{from}` {verb} {uri}{pin} — {file}:{line}\n"));
            if let Some(reason) = e.get("reason").and_then(|v| v.as_str()) {
                out.push_str(&format!("  deviation: {reason}\n"));
            }
            let uri_pinned = format!(
                "{uri}{}",
                e.get("pinned_r")
                    .and_then(|v| v.as_u64())
                    .map(|r| format!("~r{r}"))
                    .unwrap_or_default()
            );
            if !cited.contains(&uri_pinned) {
                cited.push(uri_pinned);
            }
        }
    }
    if let Some(units) = subgraph.get("units").and_then(|u| u.as_array()) {
        out.push('\n');
        for u in units {
            let uri = u.get("uri").and_then(|v| v.as_str()).unwrap_or("?");
            let heading = u.get("heading").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!("Unit {uri}: {heading}\n"));
        }
    }
    out.push_str(&format!(
        "\n— provenance: computed at {}, epoch {}, producer {producer}\n",
        if cited.is_empty() {
            "<no spec inputs>".to_string()
        } else {
            cited.join(", ")
        },
        epoch.short()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Specmap};

    fn mini_map() -> Specmap {
        Specmap {
            schema: 2,
            codeItems: vec![CodeItem {
                symbol: "demo::thing".into(),
                itemKind: "fn".into(),
                crateName: "demo".into(),
                file: "crates/demo/src/lib.rs".into(),
                line: 3,
            }],
            edges: vec![Edge {
                fromSymbol: "demo::thing".into(),
                verb: EdgeVerb::Implements,
                uri: "spec://vibevm/common/PROP-000#root".into(),
                provenance: EdgeProvenance::Authored,
                file: "crates/demo/src/lib.rs".into(),
                line: 3,
                pinnedR: None,
                reason: None,
            }],
            specUnits: vec![],
            suspects: vec![],
            warnings: vec![],
        }
    }

    fn seed_epoch_inputs(root: &Path) {
        std::fs::write(root.join("Cargo.lock"), "lock v1\n").unwrap();
    }

    #[test]
    fn second_identical_prose_call_is_a_cache_hit() {
        let tmp = tempfile::tempdir().unwrap();
        seed_epoch_inputs(tmp.path());
        let map = mini_map();

        let first = prose_explain(tmp.path(), &map, "demo::thing").unwrap();
        assert!(!first.cached);
        let second = prose_explain(tmp.path(), &map, "demo::thing").unwrap();
        assert!(second.cached, "second identical call must hit the cache");
        assert_eq!(first.text, second.text);

        let t = load_telemetry(tmp.path());
        assert_eq!((t.hits, t.misses), (1, 1));
        assert!(first.text.contains("— provenance:"));
        assert!(first.text.contains("epoch"));
    }

    #[test]
    fn editing_cargo_lock_invalidates_the_render() {
        let tmp = tempfile::tempdir().unwrap();
        seed_epoch_inputs(tmp.path());
        let map = mini_map();

        let first = prose_explain(tmp.path(), &map, "demo::thing").unwrap();
        std::fs::write(tmp.path().join("Cargo.lock"), "lock v2 — dep bumped\n").unwrap();
        let after = prose_explain(tmp.path(), &map, "demo::thing").unwrap();
        assert!(
            !after.cached,
            "an epoch change must make the old render unreachable"
        );
        assert_ne!(first.epoch, after.epoch);

        let t = load_telemetry(tmp.path());
        assert_eq!((t.hits, t.misses), (0, 2));
    }

    #[test]
    fn epoch_is_stable_for_unchanged_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        seed_epoch_inputs(tmp.path());
        assert_eq!(epoch(tmp.path()), epoch(tmp.path()));
    }
}
