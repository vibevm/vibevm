//! The intent ledger, local slice (LEDGER-INTENT v0.1; PLAYBOOK
//! Phase 5). Interpretations class only — the facts class is the
//! conform engine's store (ENGINE-CONFORM §3), which this module
//! deliberately does not touch: facts are keyed by
//! `(file content-hash, producer)` and survive every epoch change.
//!
//! One query kind ships: `explain.item` — the prose render behind
//! `rust-ai-native trace explain --prose`. The producer is a
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

specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/LEDGER-INTENT-v0.1#classes"
);

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

/// The closed query-kind register (LEDGER-INTENT §6/§8). Adding a
/// variant is a reviewed PR, not a string; the kind is part of every
/// cache key. One kind ships in v0.1 — `explain.item` — behind the
/// `trace explain --prose` path. The kind that previously lived as an
/// in-function `const PRODUCER` string (`ledger.rs:132` before B-022)
/// is now a typed value the key composes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    ExplainItem,
}

impl QueryKind {
    /// The wire/provenance name, e.g. "explain.item".
    pub fn name(self) -> &'static str {
        match self {
            QueryKind::ExplainItem => "explain.item",
        }
    }

    /// The producer id for this kind's shipped producer, e.g.
    /// "explain.item/prose-template-1".
    pub fn producer(self) -> &'static str {
        match self {
            QueryKind::ExplainItem => "explain.item/prose-template-1",
        }
    }
}

/// The stored ledger entry (LEDGER-INTENT §4, the deterministic
/// subset). The LLM-only fields — `model_id`, `prompt_rev`, `cost`,
/// `confidence` — wait for a producer that carries them (B-020); a
/// deterministic template cannot populate them, so they are absent
/// rather than zero-valued. The slot's content is the JSON
/// serialisation of this struct; an old bare-prose object fails to
/// parse as `LedgerEntry` and is treated as a MISS — graceful, no
/// migration code. Carrying `producer`/`kind`/`inputs_hash` on the
/// entry (not only folded into the key) is what makes §8's
/// cache-poisoning predicate — "select a bad producer's entries" —
/// possible at all.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LedgerEntry {
    /// The entry-shape schema version (bump ⇒ wholesale invalidate).
    pub schema: u32,
    /// `QueryKind::name()` — the kind, readable off the entry.
    pub kind: String,
    /// The producer id (`QueryKind::producer()`).
    pub producer: String,
    /// The epoch the entry was computed under (`Epoch.0`).
    pub epoch: String,
    /// `content_hash(subject_json)` — readable, not only key-folded.
    pub inputs_hash: String,
    /// UNIX seconds at compute time (R2: `std::time`, no chrono).
    pub created_at_unix: u64,
    /// The rendered prose body.
    pub body: String,
}

/// Canonical, versioned key material (LEDGER-INTENT §2/§8). The `v=1`
/// prefix is the key-schema version (bumping it wholesale-invalidates
/// the cache); the remaining fields are the structured tuple §8 says
/// must be reviewable per kind, so a bad producer's entries can be
/// selected by predicate rather than only by deleting `.ledger/`
/// wholesale. R1: a hand-built stable string is chosen over
/// `serde_json::to_string` of a struct — its bytes are exactly what a
/// test can assert on, so stability is trivially verifiable and free
/// of any struct-field-ordering assumption the serialiser would carry.
fn cache_key(kind: QueryKind, producer: &str, epoch: &str, subject_hash: &str) -> String {
    format!(
        "v=1\nk={}\np={}\ne={}\ns={}",
        kind.name(),
        producer,
        epoch,
        subject_hash,
    )
}

/// Wall-clock seconds since the UNIX epoch (R2). `Cargo.toml` carries
/// no `chrono`, so this uses the std clock; the `unwrap_or(0)` only
/// fires if the clock is somehow before the epoch — not a real case.
fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `explain.item` with a prose render (LEDGER §6 query kind 2): the
/// structured subgraph is the ground truth; the prose cites URIs; the
/// stored entry is keyed by `(subgraph, epoch, producer)` so an
/// epoch change makes yesterday's render unreachable while the
/// conform facts store stays untouched.
pub fn prose_explain(root: &Path, map: &Specmap, target: &str) -> Result<ProseRender> {
    let kind = QueryKind::ExplainItem;
    let producer = kind.producer();
    let subgraph = crate::explain::explain_json(map, target)?;
    let subject = serde_json::to_string(&subgraph)?;
    let subject_hash = content_hash(&subject);
    let epoch = epoch(root);
    let key = content_hash(&cache_key(kind, producer, &epoch.0, &subject_hash));
    let key_hex = key.strip_prefix("sha256:").unwrap_or(&key).to_string();
    let slot = object_path(root, &key_hex);

    let mut telemetry = load_telemetry(root);
    // Hit path: a slot exists AND parses as the structured entry
    // (LEDGER §4). An old bare-prose object (or anything that fails
    // to parse) is a graceful MISS — counted as a miss, recomputed
    // into the new shape, no migration code. The key-schema bump
    // (`v=1` in `cache_key`) already moved the old opaque-hash slots
    // to unreachable paths, so this branch mostly guards a slot the
    // new key still resolves to but a pre-B-022 binary wrote.
    if let Ok(bytes) = std::fs::read_to_string(&slot) {
        if let Ok(entry) = serde_json::from_str::<LedgerEntry>(&bytes) {
            telemetry.hits += 1;
            save_telemetry(root, &telemetry)?;
            return Ok(ProseRender {
                text: entry.body,
                cached: true,
                epoch,
            });
        }
    }

    // Miss path (slot absent, unreadable, or old-format): recompute.
    let text = render_prose(&subgraph, target, &epoch, producer);
    let entry = LedgerEntry {
        schema: 1,
        kind: kind.name().to_string(),
        producer: producer.to_string(),
        epoch: epoch.0.clone(),
        inputs_hash: subject_hash,
        created_at_unix: now_unix_secs(),
        body: text.clone(),
    };
    if let Some(parent) = slot.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&slot, serde_json::to_string_pretty(&entry)?)
        .with_context(|| format!("writing {}", slot.display()))?;
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

    /// (a) An old bare-prose slot (the pre-B-022 on-disk shape) at the
    /// path the new key resolves to is a MISS — recomputed into the
    /// new `LedgerEntry` shape, then a second call hits. Roundtrip.
    #[test]
    fn old_bare_prose_slot_is_a_miss_then_recomputes_to_structured() {
        let tmp = tempfile::tempdir().unwrap();
        seed_epoch_inputs(tmp.path());
        let map = mini_map();

        // Resolve the exact slot the new key computes, and pre-seed
        // it with an old bare-prose object.
        let kind = QueryKind::ExplainItem;
        let producer = kind.producer();
        let subgraph = crate::explain::explain_json(&map, "demo::thing").unwrap();
        let subject = serde_json::to_string(&subgraph).unwrap();
        let subject_hash = content_hash(&subject);
        let ep = epoch(tmp.path());
        let key = content_hash(&cache_key(kind, producer, &ep.0, &subject_hash));
        let key_hex = key.strip_prefix("sha256:").unwrap_or(&key);
        let slot = object_path(tmp.path(), key_hex);
        std::fs::create_dir_all(slot.parent().unwrap()).unwrap();
        std::fs::write(&slot, "this is old bare prose, not a JSON entry").unwrap();

        let render = prose_explain(tmp.path(), &map, "demo::thing").unwrap();
        assert!(
            !render.cached,
            "an old bare-prose slot must miss and recompute"
        );

        // The slot is now a structured entry.
        let on_disk: LedgerEntry =
            serde_json::from_str(&std::fs::read_to_string(&slot).unwrap()).unwrap();
        assert_eq!(on_disk.schema, 1);
        assert_eq!(on_disk.kind, "explain.item");
        assert_eq!(on_disk.producer, "explain.item/prose-template-1");
        assert_eq!(on_disk.epoch, ep.0);
        assert_eq!(on_disk.inputs_hash, subject_hash);
        assert_eq!(on_disk.body, render.text);
        assert!(on_disk.created_at_unix > 0);

        // A second call now hits the recomputed structured slot.
        let second = prose_explain(tmp.path(), &map, "demo::thing").unwrap();
        assert!(second.cached);
        assert_eq!(second.text, render.text);

        let t = load_telemetry(tmp.path());
        assert_eq!(
            (t.hits, t.misses),
            (1, 1),
            "old-format miss counted once, then a hit"
        );
    }

    /// (b) The canonical key material is stable for identical inputs
    /// and discriminates across producer / epoch / subject. The kind
    /// name is embedded in the `k=` field, so a future second variant
    /// would differ by construction (only one kind ships today).
    #[test]
    fn cache_key_is_stable_and_discriminating() {
        let kind = QueryKind::ExplainItem;
        let producer = kind.producer();
        let subject_hash = "sha256:aaaa";
        let epoch_a = "sha256:1111";
        let epoch_b = "sha256:2222";

        let k1 = cache_key(kind, producer, epoch_a, subject_hash);
        let k2 = cache_key(kind, producer, epoch_a, subject_hash);
        assert_eq!(k1, k2, "identical inputs must yield identical material");

        // The kind name is embedded — a second kind would differ here.
        assert!(k1.contains("k=explain.item\n"), "kind folds into the key");

        assert_ne!(
            cache_key(kind, producer, epoch_a, "sha256:bbbb"),
            k1,
            "different subject must differ"
        );
        assert_ne!(
            cache_key(kind, producer, epoch_b, subject_hash),
            k1,
            "different epoch must differ"
        );
        assert_ne!(
            cache_key(kind, "explain.item/other-producer", epoch_a, subject_hash),
            k1,
            "different producer must differ"
        );
    }

    /// (c) `LedgerEntry` roundtrips through serde losslessly.
    #[test]
    fn ledger_entry_roundtrips_through_serde() {
        let entry = LedgerEntry {
            schema: 1,
            kind: "explain.item".into(),
            producer: "explain.item/prose-template-1".into(),
            epoch: "sha256:deadbeef".into(),
            inputs_hash: "sha256:cafe".into(),
            created_at_unix: 1_700_000_000,
            body: "# demo::thing\n\n— provenance: ...".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: LedgerEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema, entry.schema);
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.producer, entry.producer);
        assert_eq!(back.epoch, entry.epoch);
        assert_eq!(back.inputs_hash, entry.inputs_hash);
        assert_eq!(back.created_at_unix, entry.created_at_unix);
        assert_eq!(back.body, entry.body);
    }

    /// (d) §8 FAILURE-CACHE-POISONING predicate: a bad producer's
    /// entries can now be selected by filtering the `producer` field
    /// the entry carries — impossible when the slot was opaque prose.
    /// This is the capability the structured entry exists to enable.
    #[test]
    fn entries_can_be_selected_by_producer_predicate() {
        let good = LedgerEntry {
            schema: 1,
            kind: "explain.item".into(),
            producer: "explain.item/prose-template-1".into(),
            epoch: "sha256:e1".into(),
            inputs_hash: "sha256:s1".into(),
            created_at_unix: 1,
            body: "good".into(),
        };
        let bad = LedgerEntry {
            schema: 1,
            kind: "explain.item".into(),
            producer: "explain.item/POISONED-llm-7".into(),
            epoch: "sha256:e1".into(),
            inputs_hash: "sha256:s1".into(),
            created_at_unix: 2,
            body: "bad".into(),
        };
        let entries = vec![good, bad];

        // Wholesale invalidation of the poisoned producer is now a
        // one-predicate filter over a field the entry carries.
        let poisoned: Vec<&LedgerEntry> = entries
            .iter()
            .filter(|e| e.producer == "explain.item/POISONED-llm-7")
            .collect();
        assert_eq!(poisoned.len(), 1);
        assert_eq!(poisoned[0].body, "bad");
    }
}
