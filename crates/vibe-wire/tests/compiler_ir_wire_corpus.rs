//! Authored valid documents for the epoch-1 compiler IR wire
//! (`schemas/compiler_ir/e1/ir.jtd.json` — the six manager carriers of
//! PROP-054 `##WHOLE-IR-WIRE`, frozen per SPEC-DEBT §8.8's cardinality
//! ruling). The same document is the R3.4 trace snapshot: no second
//! trace-only IR shape exists.
//!
//! Every `valid/` document is DERIVED, not shaped by hand: the corpus author
//! ports the landed parser and re-runs the landed R3 invariants before
//! writing. These tests are the second, independent gate.
//!
//! Two other kinds of check sit beside them, and they are not the same kind.
//! CONVERSION GATES (`compiler_ir_conversion_gates.rs`,
//! `compiler_ir_domain_invariants.rs`, and FOREST / EMIT IDENTITY in
//! `compiler_ir_emit_and_forest.rs`) are what the R6.3 decoder owes on EVERY
//! carrier it is handed, including one a plugin transformed and returned.
//! CORPUS PRODUCER ORACLES (`compiler_ir_producer_laws.rs`,
//! `compiler_ir_qualify_oracle.rs`, OPAQUE TAPE) characterize what THIS
//! corpus's builtin passes emitted — a plugin may legally return a
//! verifier-valid carrier they would reject, and the decoder must accept it.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::compiler_ir::e1::ir::{
    AbsorptionState, ArtifactContext, ArtifactFrame, ArtifactTarget, Authority,
    CardinalityArtifact, CardinalityDocument, ClosureContribution, ClosureDocument, ClosureEdge,
    ClosureEdgeKind, CompileMode, ContributionAbsorption, DirectiveKind, DocumentAddress,
    DocumentObservation, ExpansionObservation, FenceSnapshot, Ir, LaneChunk, LaneContribution,
    LaneFrame, LaneIr, LaneNode, LevelClosure, LevelDocument, LevelEmitted, LevelLane, LevelSource,
    LinkContributionWitness, LinkOccurrence, LinkState, QualificationState,
};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn read_valid<T: DeserializeOwned + Serialize>(name: &str) -> T {
    let path = corpus().join("valid").join(name);
    let bytes = std::fs::read(&path).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let value: T = serde_json::from_value(authored.clone()).unwrap();
    let round_trip = serde_json::to_value(&value).unwrap();
    assert_eq!(
        round_trip,
        authored,
        "{} loses data on generated round-trip",
        path.display()
    );
    value
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The variant each valid corpus document carries, with its redundant
/// closed level/cardinality and schema epoch checked on the way through.
/// No wildcard arm: a seventh variant is a compile error here, which is the
/// acceptance form of "exactly six variants, no ambiguous blob".
fn variant_of(value: &Ir) -> &'static str {
    match value {
        Ir::SourceDocument(arm) => {
            assert_eq!(arm.ir_schema, 1);
            assert!(matches!(arm.level, LevelSource::Source));
            assert!(matches!(arm.cardinality, CardinalityDocument::Document));
            "source-document"
        }
        Ir::DocumentDocument(arm) => {
            assert_eq!(arm.ir_schema, 1);
            assert!(matches!(arm.level, LevelDocument::Document));
            assert!(matches!(arm.cardinality, CardinalityDocument::Document));
            "document-document"
        }
        Ir::DocumentsArtifact(arm) => {
            assert_eq!(arm.ir_schema, 1);
            assert!(matches!(arm.level, LevelDocument::Document));
            assert!(matches!(arm.cardinality, CardinalityArtifact::Artifact));
            "documents-artifact"
        }
        Ir::ClosureArtifact(arm) => {
            assert_eq!(arm.ir_schema, 1);
            assert!(matches!(arm.level, LevelClosure::Closure));
            assert!(matches!(arm.cardinality, CardinalityArtifact::Artifact));
            "closure-artifact"
        }
        Ir::LaneArtifact(arm) => {
            assert_eq!(arm.ir_schema, 1);
            assert!(matches!(arm.level, LevelLane::Lane));
            assert!(matches!(arm.cardinality, CardinalityArtifact::Artifact));
            "lane-artifact"
        }
        Ir::EmittedArtifact(arm) => {
            assert_eq!(arm.ir_schema, 1);
            assert!(matches!(arm.level, LevelEmitted::Emitted));
            assert!(matches!(arm.cardinality, CardinalityArtifact::Artifact));
            "emitted-artifact"
        }
    }
}

const SIX: [&str; 6] = [
    "source-document",
    "document-document",
    "documents-artifact",
    "closure-artifact",
    "lane-artifact",
    "emitted-artifact",
];

/// Every document in the valid corpus, discovered rather than listed: a
/// fixture nobody reads proves nothing. More documents than carriers is
/// expected — closure needs two to hold both ends of its typestate.
#[test]
fn every_valid_document_round_trips_and_the_six_carriers_are_covered() {
    let mut seen: Vec<&str> = valid_names()
        .iter()
        .map(|name| variant_of(&read_valid::<Ir>(name)))
        .collect();
    seen.sort_unstable();
    seen.dedup();
    let mut expected = SIX;
    expected.sort_unstable();
    assert_eq!(seen, expected, "the corpus must cover each carrier");
}

#[test]
fn documents_artifact_is_the_document_batch_carrier() {
    let ir = read_valid::<Ir>("documents_artifact.json");
    let Ir::DocumentsArtifact(arm) = &ir else {
        panic!("documents_artifact.json must be the documents-artifact shape");
    };
    assert_eq!(arm.documents.len(), 2);
    assert!(
        matches!(arm.documents[0].source.address, DocumentAddress::Spec(_)),
        "batch order is the deterministic worklist order"
    );
    let scan = &arm.documents[0].tree.directives;
    assert!(matches!(scan.directives[0].kind, DirectiveKind::Use));
    assert!(scan.aliases.contains_key("Part"), "`#use … as` binds here");
    assert_eq!(scan.in_place_uses.len(), 1, "the `@spec://` line is read");
    assert_eq!(
        scan.errors[0].message, "`as` is a `#use` clause, not valid on #embed",
        "only diagnostics `Directives::parse` can emit live in `errors`"
    );
}

/// The terminal closure: absorb has run, so no pending snapshot survives, the
/// plan is aligned witness-for-witness, and every normal emission order is
/// exactly the non-absorbed projection (`absorb.rs` §validate_applied).
#[test]
fn terminal_closure_is_applied_with_no_pending_snapshot() {
    let ir = read_valid::<Ir>("closure_artifact.json");
    let Ir::ClosureArtifact(arm) = &ir else {
        panic!("closure_artifact.json must be the closure-artifact shape");
    };
    let closure = &arm.closure;
    assert!(matches!(
        closure.qualification,
        QualificationState::Applied(ref state) if matches!(state.mode, CompileMode::QualifyPerNode)
    ));
    assert!(
        closure.pending_sources.is_none() && closure.pending_embeds.is_none(),
        "an applied absorption forbids a pending source/embed snapshot"
    );
    assert!(matches!(closure.context.target, ArtifactTarget::StaticMd));
    assert!(matches!(closure.edges[0].kind, ClosureEdgeKind::Use));

    let AbsorptionState::Applied(state) = &closure.absorption else {
        panic!("the terminal closure has applied absorption");
    };
    let planned = &state.plan.contributions;
    assert_eq!(
        planned.len(),
        closure.contributions.len(),
        "one plan witness per contribution"
    );
    let kinds: Vec<&str> = closure
        .contributions
        .iter()
        .map(contribution_kind)
        .collect();
    assert_eq!(kinds, ["normal", "simple", "elided", "hoisted"]);
    for (want, got) in planned.iter().zip(&closure.contributions) {
        assert_eq!(absorption_kind(want), contribution_kind(got), "kinds align");
        let (ContributionAbsorption::Normal(plan), ClosureContribution::Normal(live)) = (want, got)
        else {
            continue;
        };
        assert_eq!(plan.seed, live.seed);
        assert_eq!(plan.seed_address, live.seed_address);
        assert!(
            plan.occurrences.iter().any(|entry| entry.absorbed),
            "READ-ONCE absorbed at least one repeat"
        );
        let expected: Vec<_> = plan
            .occurrences
            .iter()
            .filter(|entry| !entry.absorbed)
            .map(|entry| (entry.node, entry.requested_address.clone()))
            .collect();
        let actual: Vec<_> = live
            .emission_order
            .iter()
            .map(|entry| (entry.node, entry.requested_address.clone()))
            .collect();
        assert_eq!(expected, actual, "the applied order is the live projection");
    }

    let LinkState::Linked(link) = &closure.link else {
        panic!("the terminal closure is linked");
    };
    assert_eq!(
        link.result.contributions.len(),
        closure.contributions.len(),
        "link witnesses every contribution, empty ones included"
    );
    for (witness, live) in link.result.contributions.iter().zip(&closure.contributions) {
        assert_eq!(witness_kind(witness), contribution_kind(live));
        if let (LinkContributionWitness::Normal(seen), ClosureContribution::Normal(live)) =
            (witness, live)
        {
            assert_eq!(seen.occurrence_count as usize, live.emission_order.len());
        }
    }
    let normal = link
        .result
        .occurrences
        .iter()
        .filter(|entry| matches!(entry, LinkOccurrence::Normal(_)))
        .count();
    assert_eq!(normal, 2, "both surviving occurrences reach link");
}

/// The complementary closure: the earliest, `plain`, compatibility-fragment
/// value a registered custom backend produces — a POST-CLOSE state and nothing
/// more. `close` is the only pass that has run, so both snapshots are still
/// pending, its one node pair came from the `#use` topology, and its one edge
/// is a `Use` edge (`close.rs:184` hardcodes the kind). An `Embed`/`Source`
/// edge here would mean embed/merge had run — and each clears its own snapshot
/// in the same run; those two arms live in the residuals instead.
#[test]
fn compat_closure_is_the_early_state_that_still_carries_its_snapshots() {
    let ir = read_valid::<Ir>("closure_artifact_compat.json");
    let Ir::ClosureArtifact(arm) = &ir else {
        panic!("closure_artifact_compat.json must be the closure-artifact shape");
    };
    let closure = &arm.closure;
    assert!(
        matches!(closure.context.target, ArtifactTarget::Unknown(ref id) if id == "demo-backend"),
        "a registered custom backend arrives as the typed open value, never a catch-all"
    );
    assert!(matches!(
        closure.context.frame,
        ArtifactFrame::CompatibilityFragment(_)
    ));
    assert!(matches!(closure.context.mode, CompileMode::Plain));
    assert!(matches!(
        closure.qualification,
        QualificationState::Pending(_)
    ));
    assert!(matches!(closure.absorption, AbsorptionState::Unplanned(_)));
    assert!(matches!(closure.link, LinkState::Unlinked(_)));

    // The dependency is minted first, so the seed is the SECOND node.
    let notes = &closure.nodes[1];
    let DocumentAddress::Spec(spec) = &notes.address else {
        panic!("the seed node is addressed by a spec:// address");
    };
    assert!(
        matches!(spec.address.authority, Authority::Package(ref pkg)
            if pkg.version.as_deref() == Some("1.2.3")),
        "a versioned package coordinate survives the wire verbatim"
    );
    assert_eq!(notes.tree.duplicate_anchors, ["DUP"]);
    assert_eq!(notes.tree.nodes[1].trailing, ":replace");
    let DocumentAddress::Spec(base) = &closure.nodes[0].address else {
        panic!("the dependency node is addressed by a spec:// address");
    };
    assert!(matches!(base.address.authority, Authority::Host(_)));
    assert_eq!(base.address.anchor, ["base"]);
    assert_eq!(closure.edges.len(), 1, "close mints only `use` edges");
    assert!(matches!(closure.edges[0].kind, ClosureEdgeKind::Use));
    // All three directive kinds are scanned; only `#use` becomes a close edge.
    let scan = &notes.tree.directives.directives;
    assert!(matches!(scan[0].kind, DirectiveKind::Use));
    assert!(matches!(scan[3].kind, DirectiveKind::Source));
    assert_eq!(scan.len(), 4);
    let embed = &scan[2];
    assert!(matches!(embed.kind, DirectiveKind::Embed));
    assert_eq!(embed.address.anchor, ["base", "v1"], "a tree-path anchor");

    let sources = closure
        .pending_sources
        .as_ref()
        .expect("the source snapshot rides the value until the fold consumes it");
    assert!(matches!(
        sources.documents["spec://demo/manual/base.md#base"],
        DocumentObservation::Resolved(_)
    ));
    assert!(matches!(
        sources.documents["spec://demo/manual/missing.md"],
        DocumentObservation::Failed(_)
    ));
    assert!(matches!(
        sources.expansions["spec://demo/manual/base.md#base"],
        ExpansionObservation::Resolved(_)
    ));
    assert!(matches!(
        sources.expansions["spec://demo/manual/base.md#base.v1"],
        ExpansionObservation::Failed(_)
    ));
    assert!(closure.pending_embeds.is_some());

    // The other side of the same optional: a closure whose source fold has
    // already consumed its snapshot carries neither member.
    let mut bare = serde_json::to_value(&ir).unwrap();
    let stripped = bare["closure"].as_object_mut().unwrap();
    stripped.remove("pending_sources");
    stripped.remove("pending_embeds");
    round_trip::<Ir>(bare);
}

// ── The lane bracket law ─────────────────────────────────────────────────────
// `assemble/project.rs` emits, PER OCCURRENCE, open → node → [forced-newline]
// → close, and `assemble/validate.rs` re-walks exactly that. The walk below is
// the same law; the mutation cases prove it bites when a bracket is lost,
// reordered, or nested inside another occurrence's.

fn walk_normal(chunks: &[LaneChunk], contribution: u32, nodes: u32) -> Result<u32, String> {
    let mut cursor = 0usize;
    let mut occurrence = 0u32;
    let mut fence: Option<FenceSnapshot> = None;
    while cursor < chunks.len() {
        let LaneChunk::NormalOpen(open) = &chunks[cursor] else {
            return Err(format!(
                "chunk {cursor} is not the occurrence's normal-open"
            ));
        };
        if open.contribution != contribution || open.occurrence != occurrence {
            return Err(format!("open {cursor} is out of order"));
        }
        cursor += 1;
        let Some(LaneChunk::Node(carrier)) = chunks.get(cursor) else {
            return Err(format!("chunk {cursor} is not this occurrence's node"));
        };
        let LaneNode::Normal(node) = &carrier.node else {
            return Err(format!("chunk {cursor} is a simple node inside a bracket"));
        };
        if node.contribution != contribution || node.occurrence != occurrence {
            return Err(format!("node {cursor} is out of order"));
        }
        if node.node >= nodes {
            return Err(format!("node {cursor} indexes outside the arena"));
        }
        if open.marker != node.marker {
            return Err(format!("open {cursor} carries a foreign marker"));
        }
        // One continuous fence history: each occurrence resumes where the
        // previous one left off, and the first begins closed.
        let resumes = match &fence {
            None => matches!(node.fence_before, FenceSnapshot::Closed(_)),
            Some(previous) => &node.fence_before == previous,
        };
        if !resumes {
            return Err(format!("node {cursor} breaks the fence history"));
        }
        fence = Some(node.fence_after.clone());
        cursor += 1;
        if !node.body.ends_with('\n') {
            match chunks.get(cursor) {
                Some(LaneChunk::ForcedNewline(newline))
                    if newline.contribution == contribution && newline.occurrence == occurrence => {
                }
                _ => return Err(format!("chunk {cursor} is not the forced newline")),
            }
            cursor += 1;
        }
        let Some(LaneChunk::NormalClose(close)) = chunks.get(cursor) else {
            return Err(format!(
                "chunk {cursor} is not the occurrence's normal-close"
            ));
        };
        if close.contribution != contribution
            || close.occurrence != occurrence
            || close.marker != node.marker
        {
            return Err(format!("close {cursor} is out of order"));
        }
        cursor += 1;
        occurrence += 1;
    }
    Ok(occurrence)
}

fn lane_of(ir: &Ir) -> &LaneIr {
    let Ir::LaneArtifact(arm) = ir else {
        panic!("lane_artifact.json must be the lane-artifact shape");
    };
    &arm.lane
}

#[test]
fn lane_brackets_every_normal_occurrence_and_keeps_one_fence_history() {
    let ir = read_valid::<Ir>("lane_artifact.json");
    let lane = lane_of(&ir);
    let LaneContribution::Normal(normal) = &lane.contributions[0] else {
        panic!("the first lane contribution is normal");
    };
    assert_eq!(
        walk_normal(&normal.chunks, 0, lane.source_node_count),
        Ok(2),
        "two occurrences, each in its own bracket"
    );
    let opened = normal.chunks.iter().any(|chunk| match chunk {
        LaneChunk::Node(carrier) => matches!(
            &carrier.node,
            LaneNode::Normal(node) if matches!(node.fence_before, FenceSnapshot::Open(_))
        ),
        _ => false,
    });
    assert!(opened, "the second occurrence resumes inside an open fence");

    let LaneContribution::Simple(simple) = &lane.contributions[1] else {
        panic!("the second lane contribution is simple");
    };
    let LaneChunk::Node(carrier) = &simple.chunks[0] else {
        panic!("a simple contribution opens with its node");
    };
    let LaneNode::Simple(node) = &carrier.node else {
        panic!("a simple contribution carries a simple node");
    };
    assert!(matches!(node.fence_before, FenceSnapshot::Closed(_)));
    assert_eq!(
        simple.chunks.len(),
        if node.body.ends_with('\n') { 1 } else { 2 },
        "a simple contribution is node plus its forced newline, never a bracket"
    );
    assert_eq!(
        lane.frame.generated_path.as_deref(),
        Some("vibevm/vibespecs/boot/STATIC.md")
    );
    let kinds: Vec<&str> = lane.contributions.iter().map(lane_kind).collect();
    assert_eq!(kinds, ["normal", "simple", "elided", "hoisted"]);
}

#[test]
fn the_bracket_walk_rejects_a_lost_reordered_or_nested_occurrence() {
    let ir = read_valid::<Ir>("lane_artifact.json");
    let lane = lane_of(&ir);
    let LaneContribution::Normal(normal) = &lane.contributions[0] else {
        panic!("the first lane contribution is normal");
    };
    let nodes = lane.source_node_count;
    let base = &normal.chunks;

    for drop in 0..base.len() {
        let mut mutated = base.clone();
        mutated.remove(drop);
        assert!(
            walk_normal(&mutated, 0, nodes).is_err(),
            "dropping chunk {drop} must be red"
        );
    }
    let mut swapped = base.clone();
    swapped.swap(0, 4);
    assert!(
        walk_normal(&swapped, 0, nodes).is_err(),
        "swapping the two occurrences' openers must be red"
    );
    // The exact defect this corpus was repaired from: occurrence 1's node
    // nested inside occurrence 0's bracket, with one open/close around both.
    let nested = vec![
        base[0].clone(),
        base[1].clone(),
        base[2].clone(),
        base[5].clone(),
        base[3].clone(),
    ];
    assert!(
        walk_normal(&nested, 0, nodes).is_err(),
        "a nested occurrence must be red"
    );
}

fn round_trip<T: DeserializeOwned + Serialize>(authored: serde_json::Value) -> T {
    let value: T = serde_json::from_value(authored.clone())
        .unwrap_or_else(|error| panic!("{authored} does not parse: {error}"));
    assert_eq!(serde_json::to_value(&value).unwrap(), authored);
    value
}

#[test]
fn residual_union_arms_parse_and_round_trip() {
    let meta = serde_json::json!({"origin": "org.demo/lib", "path": "manual/part.md"});
    let whole = serde_json::json!({
        "raw": "spec://org.demo/lib/manual/part.md", "doc_path": "manual/part.md", "anchor": [],
        "authority": {"kind": "package", "group": "org.demo", "name": "lib"},
    });

    // Qualify has judged the occurrence view; absorb has not projected it yet.
    let planned: AbsorptionState = round_trip(serde_json::json!({
        "state": "planned",
        "plan": {
            "mode": "plain",
            "contributions": [{"kind": "hoisted", "meta": meta, "target": whole}],
        },
    }));
    assert!(matches!(planned, AbsorptionState::Planned(_)));

    // The second shipping STATIC target, as a valid `ArtifactContext` tuple.
    let xml: ArtifactContext = round_trip(serde_json::json!({
        "artifact": "static-xml",
        "target": "static-xml",
        "frame": {
            "kind": "static-lane",
            "generated_path": "vibevm/vibespecs/boot/STATIC.xml",
            "source_root": "vibevm/vibespecs",
        },
        "mode": "qualify-per-node",
    }));
    assert!(matches!(xml.target, ArtifactTarget::StaticXml));

    // The compatibility fragment's lane frame: no generated path, no root.
    let frame: LaneFrame = round_trip(serde_json::json!({"renames": []}));
    assert!(frame.generated_path.is_none() && frame.source_root.is_none());

    // A closure document still carrying its `#use … as` bindings — embed sets
    // them and qualify clears them, so no post-qualify corpus value holds one.
    let aliased: ClosureDocument = round_trip(serde_json::json!({
        "address": {"kind": "static-entry", "origin": "__host__/demo", "path": "boot/00-core.md"},
        "origin": "__host__/demo",
        "tree": {
            "nodes": [{
                "level": 0, "kind": "heading", "heading": "", "trailing": "",
                "heading_line": 0, "span": {"start": 0, "end": 0}, "children": [],
            }],
            "anchors": {}, "duplicate_anchors": [], "lines": [],
            "directives": {
                "directives": [], "in_place_uses": [], "errors": [],
                "aliases": {"Part": whole},
            },
        },
        "aliases": {"Part": whole},
    }));
    assert_eq!(aliased.aliases.len(), 1);

    // The two edge kinds no post-CLOSE closure may carry: `embed.rs:308` and
    // `merge.rs:192` mint them, and each clears its own pending snapshot in
    // the same run, so they belong to a later state than either corpus
    // closure holds. Typed round-trips rather than an impossible graph.
    let embed: ClosureEdge = round_trip(serde_json::json!({
        "from": 1, "to": 0, "kind": "embed", "requested_target": whole,
    }));
    assert!(matches!(embed.kind, ClosureEdgeKind::Embed));
    let source: ClosureEdge = round_trip(serde_json::json!({
        "from": 1, "to": 0, "kind": "source", "requested_target": whole,
    }));
    assert!(matches!(source.kind, ClosureEdgeKind::Source));

    // The worklist batch may legitimately be empty (`x-empty: emit`).
    let empty: Ir = round_trip(serde_json::json!({
        "shape": "documents-artifact",
        "ir_schema": 1,
        "level": "document",
        "cardinality": "artifact",
        "documents": [],
    }));
    assert_eq!(variant_of(&empty), "documents-artifact");
}

// ── Small kind projections, so the assertions above read as tables ──────────
// Four unions carry the same four-arm contribution vocabulary; one macro
// spells each projection once rather than four near-identical matches.

macro_rules! kind_of {
    ($name:ident, $union:ident) => {
        fn $name(value: &$union) -> &'static str {
            match value {
                $union::Normal(_) => "normal",
                $union::Simple(_) => "simple",
                $union::Elided(_) => "elided",
                $union::Hoisted(_) => "hoisted",
            }
        }
    };
}

kind_of!(contribution_kind, ClosureContribution);
kind_of!(absorption_kind, ContributionAbsorption);
kind_of!(witness_kind, LinkContributionWitness);
kind_of!(lane_kind, LaneContribution);
