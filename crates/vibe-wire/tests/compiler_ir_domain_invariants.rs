//! Two of the MANDATORY conversion invariants of the epoch-1 compiler IR wire
//! — the ones about a carrier's internal coherence rather than its shape.
//!
//! These are `x-conversion-gates`, not corpus characterization: every decoded
//! carrier owes them, INCLUDING one a plugin transformed and handed back. The
//! builtin producer oracles — what `close`, `qualify` and the opaque backend
//! happened to emit for THIS corpus — live in `compiler_ir_producer_laws.rs`
//! and `compiler_ir_emit_and_forest.rs` and bind nothing a plugin returns.
//!
//! * PASS/SNAPSHOT  an edge kind and its own pending snapshot cannot coexist.
//! * SET PROJECTION — a domain `BTreeSet<String>` projected as a list carries
//!   its canonical sorted, duplicate-free spelling.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::compiler_ir::e1::ir::{ClosureEdgeKind, ClosureIr, Ir};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn typed<T: DeserializeOwned + Serialize>(value: &serde_json::Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

fn closures() -> Vec<(String, ClosureIr)> {
    valid_names()
        .iter()
        .filter_map(|name| match typed::<Ir>(&raw(name)) {
            Ir::ClosureArtifact(arm) => Some((name.clone(), arm.closure)),
            _ => None,
        })
        .collect()
}

// ── PASS/SNAPSHOT ────────────────────────────────────────────────────────────

/// `embed.rs:308` mints `Embed` and clears `pending_embeds` (`:311`);
/// `merge.rs:192` mints `Source` and takes `pending_sources` (`:105`). Each
/// pass does both in ONE run, so an edge kind and its own pending snapshot can
/// never coexist in a carrier — whoever produced it. Note what this does NOT
/// say: nothing about which nodes exist or in what order, because a plugin may
/// legally return a graph the builtin `close` would not have shaped.
fn snapshot_violations(closure: &ClosureIr) -> Vec<String> {
    let mut out = Vec::new();
    let has = |kind: &ClosureEdgeKind| {
        closure
            .edges
            .iter()
            .any(|edge| std::mem::discriminant(&edge.kind) == std::mem::discriminant(kind))
    };
    if closure.pending_embeds.is_some() && has(&ClosureEdgeKind::Embed) {
        out.push("an `embed` edge exists while its snapshot is pending".to_string());
    }
    if closure.pending_sources.is_some() && has(&ClosureEdgeKind::Source) {
        out.push("a `source` edge exists while its snapshot is pending".to_string());
    }
    out
}

#[test]
fn a_pending_snapshot_forbids_the_edge_kind_of_its_own_pass() {
    let mut early = 0;
    for (name, closure) in closures() {
        assert_eq!(
            snapshot_violations(&closure),
            Vec::<String>::new(),
            "{name}"
        );
        if closure.pending_sources.is_some() {
            early += 1;
            assert!(
                closure
                    .edges
                    .iter()
                    .all(|edge| matches!(edge.kind, ClosureEdgeKind::Use)),
                "{name}: this corpus's pre-merge/pre-embed closure carries only `use` edges"
            );
        }
    }
    assert_eq!(early, 1, "exactly one closure is still pre-merge/pre-embed");
}

#[test]
fn an_embed_or_source_edge_beside_a_pending_snapshot_is_red() {
    // The exact contradiction the repaired corpus replaced: the pristine
    // post-close value carrying edges only embed and merge can mint.
    for (kind, needle) in [("embed", "`embed` edge"), ("source", "`source` edge")] {
        let mut document = raw("closure_artifact_compat.json");
        document["closure"]["edges"][0]["kind"] = serde_json::json!(kind);
        let Ir::ClosureArtifact(arm) = typed::<Ir>(&document) else {
            panic!("the mutated document is still a closure");
        };
        let violations = snapshot_violations(&arm.closure);
        assert!(
            violations.iter().any(|entry| entry.contains(needle)),
            "a {kind} edge beside its pending snapshot must be red, got {violations:?}"
        );
    }
}

#[test]
fn a_consumed_snapshot_permits_its_edge_kind() {
    // The other direction, and the reason this is a gate rather than an
    // oracle: once the snapshot is gone the edge is not merely allowed, it is
    // expected — so the law must not read as "an embed edge is suspicious".
    let mut document = raw("closure_artifact.json");
    assert!(document["closure"].get("pending_embeds").is_none());
    document["closure"]["edges"][0]["kind"] = serde_json::json!("embed");
    let Ir::ClosureArtifact(arm) = typed::<Ir>(&document) else {
        panic!("the mutated document is still a closure");
    };
    assert_eq!(snapshot_violations(&arm.closure), Vec::<String>::new());
}

// ── SET PROJECTION ───────────────────────────────────────────────────────────

fn canonical(label: &str, values: &[String], out: &mut Vec<String>) {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.dedup();
    if sorted != values {
        out.push(format!("{label} is not sorted+unique: {values:?}"));
    }
}

/// A domain `BTreeSet<String>` projected as a wire list: only the canonical
/// sorted, duplicate-free spelling survives wire→domain→wire unchanged, so a
/// decoder that accepted any other spelling would silently renormalise.
fn set_violations(closure: &ClosureIr) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(snapshot) = &closure.pending_sources {
        canonical("pending_sources", &snapshot.explicit_use_keys, &mut out);
    }
    if let Some(snapshot) = &closure.pending_embeds {
        canonical("pending_embeds", &snapshot.explicit_use_keys, &mut out);
    }
    out
}

#[test]
fn set_projected_lists_carry_their_canonical_spelling() {
    let mut checked = 0;
    for (name, closure) in closures() {
        assert_eq!(set_violations(&closure), Vec::<String>::new(), "{name}");
        if let Some(snapshot) = &closure.pending_sources {
            checked += 1;
            assert!(
                snapshot.explicit_use_keys.len() > 1,
                "{name}: the gate needs more than one key to mean anything"
            );
        }
    }
    assert_eq!(checked, 1);
}

#[test]
fn a_duplicated_or_unordered_set_projection_is_red() {
    // A domain `BTreeSet<String>` would silently absorb both, so wire →
    // domain → wire would not round-trip.
    for mutation in [
        serde_json::json!(["spec://z/z/z", "spec://a/a/a"]),
        serde_json::json!(["spec://a/a/a", "spec://a/a/a"]),
    ] {
        let mut document = raw("closure_artifact_compat.json");
        document["closure"]["pending_sources"]["explicit_use_keys"] = mutation;
        let Ir::ClosureArtifact(arm) = typed::<Ir>(&document) else {
            panic!("the mutated document is still a closure");
        };
        assert!(
            !set_violations(&arm.closure).is_empty(),
            "an unordered or duplicated set projection must be red"
        );
    }
}
