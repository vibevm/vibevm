//! BUILTIN producer oracle — CLOSE ORDER (`x-corpus-producer-oracles`) over
//! the real corpus closures. Deliberately NOT a conversion gate.
//!
//! The distinction is load-bearing. A conversion gate is what the R6.3 decoder
//! owes on every carrier it is handed, including one a plugin transformed and
//! returned; those live in `compiler_ir_conversion_gates.rs`,
//! `compiler_ir_domain_invariants.rs` and (FOREST, EMIT IDENTITY)
//! `compiler_ir_emit_and_forest.rs`. An oracle says only what THIS corpus's
//! builtin passes emitted — a plugin may legally return a verifier-valid
//! closure the oracle rejects, and the decoder must accept it. The QUALIFY
//! SPELLING oracle sits in `compiler_ir_qualify_oracle.rs`, the OPAQUE TAPE
//! oracle beside the emit gate.
//!
//! Because it is characterization and not a gate, it is EXACT and it owns BOTH
//! witnesses: the `use` edge vector is first RECONSTRUCTED from each reached
//! node's own carried `#use` directives the way `close.rs:146-197` mints it,
//! and only the adjacency so derived is fed into the `topology::order_by`
//! replay. Trusting `closure.edges` for the adjacency would prove no more than
//! that the order agrees with whatever edges the carrier claims. The shared
//! machinery is `compiler_ir_close_oracle/mod.rs`; the synthetic replay cases
//! are `compiler_ir_close_replay.rs`.

mod compiler_ir_close_oracle;

use std::path::PathBuf;

use compiler_ir_close_oracle::{close_violations, pre_absorb_orders, view};
use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::compiler_ir::e1::ir::{
    AbsorptionState, ClosureContribution, ClosureIr, ContributionAbsorption, Ir,
};

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

fn mutated(document: &serde_json::Value) -> ClosureIr {
    let Ir::ClosureArtifact(arm) = typed::<Ir>(document) else {
        panic!("the mutated document is still a closure");
    };
    arm.closure
}

#[test]
fn the_corpus_closures_are_builtin_close_output() {
    for (name, closure) in closures() {
        assert_eq!(close_violations(&closure), Vec::<String>::new(), "{name}");
        let orders = pre_absorb_orders(&closure);
        assert_eq!(orders.len(), 1, "{name}: one normal contribution");
        let (seed, order) = &orders[0];
        assert_eq!(order.last(), Some(seed), "{name}: the seed is minted last");
        // The edge witness is not vacuous: every carried `use` edge was
        // reconstructed from a directive some node really declares.
        let (nodes, edges, _) = view(&closure);
        assert!(!edges.is_empty(), "{name}: the graph carries `use` edges");
        assert_eq!(
            edges.len(),
            nodes.iter().map(|node| node.uses.len()).sum::<usize>(),
            "{name}: one edge per declared `#use`, deduplication aside"
        );
    }
}

#[test]
fn a_retargeted_edge_is_red() {
    // FREEZE EXHIBIT 1. Only `edges[0].requested_target` moves — to the OTHER
    // dependency the seed really declares — while `from`, `to` and every
    // directive stay untouched. The graph still has the right shape, the right
    // arity and the right endpoints; the edge simply is not the one the
    // seed's line 4 `#use` asked for. Only reconstructing the vector from the
    // carried directives can see that.
    let mut document = raw("closure_artifact.json");
    let other = document["closure"]["edges"][1]["requested_target"].clone();
    let (from, to) = (
        document["closure"]["edges"][0]["from"].clone(),
        document["closure"]["edges"][0]["to"].clone(),
    );
    document["closure"]["edges"][0]["requested_target"] = other;
    assert_eq!(document["closure"]["edges"][0]["from"], from);
    assert_eq!(document["closure"]["edges"][0]["to"], to);

    let violations = close_violations(&mutated(&document));
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("the carried directives mint")),
        "a retargeted edge must be red, got {violations:?}"
    );
}

#[test]
fn a_dropped_declared_edge_is_red() {
    // FREEZE EXHIBIT 2. Drop the SECOND declared `use` edge and the plan
    // occurrence it reached — the absorbed twin — while the seed's tree still
    // declares both directives. The live `emission_order` is untouched, the
    // non-absorbed projection still matches it, every id is in bounds and the
    // arena is still a forest, so the projection and conversion gates stay
    // green. CLOSE alone catches it, on both witnesses.
    let mut document = raw("closure_artifact.json");
    let live = document["closure"]["contributions"][0]["emission_order"].clone();
    let declared = document["closure"]["nodes"][2]["tree"]["directives"]["directives"].clone();
    document["closure"]["edges"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let occurrences = document["closure"]["absorption"]["plan"]["contributions"][0]["occurrences"]
        .as_array_mut()
        .unwrap();
    occurrences.retain(|entry| entry["absorbed"] != serde_json::json!(true));
    assert_eq!(occurrences.len(), 2, "only the absorbed occurrence is gone");
    assert_eq!(
        document["closure"]["nodes"][2]["tree"]["directives"]["directives"], declared,
        "the seed still declares both `#use` directives"
    );
    assert_eq!(
        document["closure"]["contributions"][0]["emission_order"], live,
        "the live order is untouched"
    );

    let closure = mutated(&document);
    let AbsorptionState::Applied(state) = &closure.absorption else {
        panic!("the terminal closure has applied absorption");
    };
    let ContributionAbsorption::Normal(plan) = &state.plan.contributions[0] else {
        panic!("the first plan witness is normal");
    };
    let ClosureContribution::Normal(normal) = &closure.contributions[0] else {
        panic!("the first contribution is normal");
    };
    assert_eq!(
        plan.occurrences
            .iter()
            .filter(|entry| !entry.absorbed)
            .map(|entry| entry.node)
            .collect::<Vec<_>>(),
        normal
            .emission_order
            .iter()
            .map(|e| e.node)
            .collect::<Vec<_>>(),
        "the non-absorbed projection still matches, which is why CLOSE must catch it"
    );

    let violations = close_violations(&closure);
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("the carried directives mint")),
        "the dropped edge must be red, got {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("differs from the declaration-order DFS")),
        "and so must the shortened traversal, got {violations:?}"
    );
}

#[test]
fn swapping_the_first_two_plan_occurrences_is_red() {
    // Swap occurrences 0 and 1 of the real applied plan and leave the edges —
    // hence the declaration order — alone. The node SET is unchanged, the seed
    // is still last, the result is still a valid topological sort, and because
    // the swap moves the absorbed twin ahead of its survivor the non-absorbed
    // projection is byte-identical, so `validate_applied_absorption` stays
    // green. Only replaying `topology::order_by` sees it.
    let mut document = raw("closure_artifact.json");
    let live = document["closure"]["contributions"][0]["emission_order"].clone();
    let edges = document["closure"]["edges"].clone();
    document["closure"]["absorption"]["plan"]["contributions"][0]["occurrences"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert_eq!(
        document["closure"]["edges"], edges,
        "declaration order kept"
    );
    assert_eq!(
        document["closure"]["contributions"][0]["emission_order"], live,
        "the live order is untouched"
    );

    let violations = close_violations(&mutated(&document));
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("differs from the declaration-order DFS")),
        "a swapped plan order must be red, got {violations:?}"
    );
}

#[test]
fn a_duplicated_plan_occurrence_is_red() {
    // An extra `absorbed = true` occurrence inside `absorption.plan`, with the
    // live `emission_order` left alone — so the non-absorbed projection still
    // matches and every landed verifier passes.
    let mut document = raw("closure_artifact.json");
    let live = document["closure"]["contributions"][0]["emission_order"].clone();
    let occurrences = document["closure"]["absorption"]["plan"]["contributions"][0]["occurrences"]
        .as_array_mut()
        .unwrap();
    let mut extra = occurrences[0].clone();
    extra["absorbed"] = serde_json::json!(true);
    occurrences.push(extra);
    assert_eq!(
        document["closure"]["contributions"][0]["emission_order"], live,
        "the live order is untouched — that is what made this survive"
    );

    let violations = close_violations(&mutated(&document));
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("repeats a key")),
        "a duplicated plan occurrence must be red, got {violations:?}"
    );
}

#[test]
fn an_empty_normal_close_order_is_red() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["contributions"][0]["occurrences"] =
        serde_json::json!([]);
    document["closure"]["contributions"][0]["emission_order"] = serde_json::json!([]);
    let violations = close_violations(&mutated(&document));
    assert!(
        violations.iter().any(|entry| entry.contains("is empty")),
        "an empty normal close order must be red, got {violations:?}"
    );
}

#[test]
fn a_seed_first_arena_is_red() {
    // The inversion the repaired corpus replaced: the seed minted before the
    // dependencies it `#use`s.
    let mut document = raw("closure_artifact.json");
    document["closure"]["edges"][0]["from"] = serde_json::json!(0);
    document["closure"]["edges"][0]["to"] = serde_json::json!(2);
    let violations = close_violations(&mutated(&document));
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("the carried directives mint")),
        "a seed-first arena must be red, got {violations:?}"
    );
}

#[test]
fn an_unresolvable_directive_is_a_fault_not_a_panic() {
    // A `#use` naming a document the arena does not carry: the oracle answers
    // with a typed fault rather than indexing.
    let mut document = raw("closure_artifact.json");
    document["closure"]["nodes"][2]["tree"]["directives"]["directives"][0]["address"] = serde_json::json!({
        "raw": "spec://org.demo/lib/manual/absent.md",
        "authority": {"kind": "package", "group": "org.demo", "name": "lib"},
        "doc_path": "manual/absent.md",
        "anchor": [],
    });
    let violations = close_violations(&mutated(&document));
    assert!(
        violations
            .iter()
            .any(|entry| entry.contains("resolves to no carried node")),
        "an unresolvable directive must be a fault, got {violations:?}"
    );
}
