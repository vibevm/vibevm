//! T6b execution tests (ABI §6.2–6.3): per-document versus per-artifact
//! invocation, causal position proofs, config delivery, the lane/emitted
//! interim refusals, typed error surfacing, and pass-name trace identity.
//!
//! The world is the shared `artifact_tests` fixture (five discovered
//! documents: alpha, shared, the simple local entry, omega, the embedded
//! piece — `parse_invocations() == 5`). Local mutating/failing vehicles
//! extend a CLONE of the shared identity registry: they never enter
//! `builtins()` and never alter the T5 golden. Causality vehicles append a
//! FENCED code block — real parsed content, no anchor or fact-id pressure
//! across the shared/qualified lane.

use std::sync::{Arc, Mutex};

use specmark::verifies;

use crate::compiler::artifact_tests::{Fixture, fixture};
use crate::compiler::assemble::{assemble_invocations, reset_assemble_invocations};
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{
    compile_artifact_traced_with_registries, compile_artifact_with_registries, parse_invocations,
    reset_parse_invocations,
};
use crate::compiler::emit::{emit_invocations, reset_emit_invocations};
use crate::compiler::ir::emitted_output_fingerprint;
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};

use super::behavior::TransformBehaviorError;
use super::config::{ConfigTable, ConfigValue};
use super::plan::{TransformConfig, TransformSeed, TransformStage};
use super::plan_test_support::{build_or_panic, default_dependency, dependency_seed, empty_config};
use super::registry::TransformRegistry;
use super::registry_test_support::{identity_plan, identity_registry, identity_seed};
use super::schedule::{TransformCapabilityGap, TransformError};
use super::schedule_execution_vehicles::*;
use crate::compiler::builtin::ArtifactCompileError;

/// One local vehicle seed under a local catalog name.
fn vehicle_seed(key: &str, stage: TransformStage, name: &str) -> TransformSeed {
    TransformSeed::new(
        vibe_core::manifest::ExtensionKey::authored(key),
        super::plan::TransformProvider::from(&default_dependency()),
        stage,
        super::plan::TransformImplementation::builtin_candidate(name, 1),
        None,
        None,
    )
}

fn plan_of(seeds: Vec<TransformSeed>) -> crate::compiler::ir::ArtifactPlan {
    fixture().plan.with_transforms(build_or_panic(seeds))
}

fn compile(
    plan: crate::compiler::ir::ArtifactPlan,
    world: &Fixture,
    registry: &TransformRegistry,
) -> Result<crate::compiler::ir::EmittedArtifact, ArtifactCompileError> {
    compile_artifact_with_registries(plan, &world.source, &BackendRegistry::builtins(), registry)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn all_four_identity_behaviors_run_5_5_1_1_over_the_five_document_world() {
    // The commissioning cardinality proof: the SHARED identity behaviors
    // themselves count their invocations — no assemble/backend counter
    // substitutes for "the behavior method ran".
    reset_parse_invocations();
    reset_assemble_invocations();
    reset_emit_invocations();
    super::registry_test_support::reset_identity_invocations();
    let world = fixture();
    let plan = plan_of(vec![
        identity_seed("org.demo/tools#src", TransformStage::Source),
        identity_seed("org.demo/tools#doc", TransformStage::Document),
        identity_seed("org.demo/tools#lane", TransformStage::Lane),
        identity_seed("org.demo/tools#emit", TransformStage::Emitted),
    ]);

    compile(plan, &world, &identity_registry()).unwrap();
    assert_eq!(
        super::registry_test_support::identity_invocations(),
        (5, 5, 1, 1),
        "source/document per discovered document, lane/emitted once — the behaviors themselves counted"
    );
    // Secondary cardinalities: the wrappers live inside the parse closure,
    // and the artifact positions bracket assemble/emit exactly.
    assert_eq!(parse_invocations(), 5);
    assert_eq!(assemble_invocations(), 1);
    assert_eq!(emit_invocations("static-xml"), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_source_transform_feeds_the_parser_and_a_document_transform_does_not() {
    let world = fixture();
    let baseline = compile(fixture().plan.clone(), &world, &identity_registry()).unwrap();
    assert!(baseline.bytes().iter().any(|byte| *byte != 0));

    // Source position: the appended block is real PARSED content — it can
    // only exist in the bytes if the wrapper ran before `parse`.
    let source_registry = registry_with(&[Arc::new(AppendBlockSource)]);
    let source_plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#src",
        TransformStage::Source,
        "test-source-append",
    )]);
    let source_emitted = compile(source_plan, &world, &source_registry).unwrap();
    let source_text = String::from_utf8(source_emitted.bytes().to_vec()).unwrap();
    assert!(
        source_text.contains("Appended-1"),
        "the block was parsed and emitted: {source_text}"
    );

    // Document position: the tree gains the block while every authored
    // document text stays byte-identical (the SectionSource is unchanged and
    // a document wrapper has no source mutator) — the change is post-parse.
    let document_registry = registry_with(&[Arc::new(BlockTreeDocument)]);
    let document_plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#doc",
        TransformStage::Document,
        "test-tree-section",
    )]);
    let document_emitted = compile(document_plan, &world, &document_registry).unwrap();
    let document_text = String::from_utf8(document_emitted.bytes().to_vec()).unwrap();
    assert!(
        document_text.contains("Appended-1"),
        "the tree-only block still reaches emission"
    );
    // The negative control for both positions: the baseline world emits no
    // Appended content, and a document wrapper has no source mutator
    // (`DocumentIr::source` lends read-only), so the block can only have
    // entered through the parsed tree.
    let baseline_text = String::from_utf8(baseline.bytes().to_vec()).unwrap();
    assert!(!baseline_text.contains("Appended"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn config_delivery_keeps_none_authored_empty_and_values_distinct() {
    reset_vehicle_counts();
    DELIVERED_CONFIGS.with(|records| records.borrow_mut().clear());
    let world = fixture();
    let plan = plan_of(vec![
        configured_seed("org.demo/tools#none", None),
        configured_seed("org.demo/tools#empty", Some(empty_config())),
        configured_seed("org.demo/tools#values", Some(values_config())),
    ]);
    let registry = registry_with(&[Arc::new(RecordingDocument)]);

    compile(plan, &world, &registry).unwrap();
    // Three wrappers, five documents, authored order within each document.
    assert_eq!(DOCUMENT_COUNT.with(std::cell::Cell::get), 15);
    let delivered = DELIVERED_CONFIGS.with(|records| records.borrow().clone());
    assert_eq!(
        delivered,
        vec![["none", "empty", "values"]; 5].concat(),
        "each document delivers all three envelopes in plan order"
    );
}

fn configured_seed(key: &str, config: Option<TransformConfig>) -> TransformSeed {
    let base = dependency_seed(key, TransformStage::Document);
    TransformSeed::new(
        base.key().clone(),
        base.provider().clone(),
        TransformStage::Document,
        super::plan::TransformImplementation::builtin_candidate("test-record-config", 1),
        config,
        None,
    )
}

fn values_config() -> TransformConfig {
    let mut table = ConfigTable::new();
    table.insert(
        "threshold".to_string(),
        ConfigValue::String("loud".to_string()),
    );
    TransformConfig::new(table)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_changed_lane_refuses_after_assemble_before_the_backend() {
    reset_assemble_invocations();
    reset_emit_invocations();
    reset_vehicle_counts();
    let world = fixture();
    let registry = registry_with(&[Arc::new(ReorderLane)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#lane",
        TransformStage::Lane,
        "test-lane-reorder",
    )]);

    let error = compile(plan, &world, &registry).unwrap_err();
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("the lane refusal is the transform family: {error:?}")
    };
    assert!(
        matches!(
            public.inner(),
            TransformError::Capability {
                gap: TransformCapabilityGap::LaneChange,
                ..
            }
        ),
        "full-equality detection names the T6c gap: {public}"
    );
    assert_eq!(
        LANE_COUNT.with(std::cell::Cell::get),
        1,
        "the behavior ran once"
    );
    assert_eq!(assemble_invocations(), 1, "the refusal is after assemble");
    assert_eq!(emit_invocations("static-xml"), 0, "and before the backend");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_identity_lane_crosses_once_with_no_witness_claim() {
    // Position and cardinality only: T6c owns equivalence, and this test
    // deliberately asserts nothing about witnesses or transitions.
    super::registry_test_support::reset_identity_invocations();
    let world = fixture();
    let plan = fixture().plan.with_transforms(identity_plan(&[(
        "org.demo/tools#lane",
        TransformStage::Lane,
    )]));
    let baseline = compile(fixture().plan.clone(), &world, &identity_registry()).unwrap();
    let carried = compile(plan, &world, &identity_registry()).unwrap();
    assert_eq!(
        carried, baseline,
        "identity lane output is the whole baseline value"
    );
    assert_eq!(
        super::registry_test_support::identity_invocations().2,
        1,
        "the identity lane BEHAVIOR ran exactly once"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn changed_emitted_bytes_refuse_after_the_backend_ran() {
    reset_emit_invocations();
    reset_vehicle_counts();
    let world = fixture();
    let registry = registry_with(&[Arc::new(AppendEmitted)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#emit",
        TransformStage::Emitted,
        "test-emit-append",
    )]);

    let error = compile(plan, &world, &registry).unwrap_err();
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("the emitted refusal is the transform family: {error:?}")
    };
    // The fault carries the bounded key preview, dense order and stage —
    // never a reconstructed pass name (ABI 6.3).
    assert!(
        matches!(
            public.inner(),
            TransformError::Capability {
                order: 0,
                stage: TransformStage::Emitted,
                ..
            }
        ),
        "the entry identity is typed: {public}"
    );
    assert!(
        matches!(
            public.inner(),
            TransformError::Capability {
                gap: TransformCapabilityGap::EmittedChange,
                ..
            }
        ),
        "the typed T9 gap: {public}"
    );
    assert_eq!(EMITTED_COUNT.with(std::cell::Cell::get), 1);
    assert_eq!(
        emit_invocations("static-xml"),
        1,
        "the backend emitted once; the refusal is post-emit"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn equal_emitted_bytes_return_the_original_artifact_untouched() {
    super::registry_test_support::reset_identity_invocations();
    let world = fixture();
    let plan = fixture().plan.with_transforms(identity_plan(&[(
        "org.demo/tools#emit",
        TransformStage::Emitted,
    )]));

    let baseline = compile(fixture().plan.clone(), &world, &identity_registry()).unwrap();
    let carried = compile(plan, &world, &identity_registry()).unwrap();
    // WHOLE-VALUE equality — bytes AND full provenance — is the observable of
    // "the ORIGINAL artifact came back"; selected-field comparisons would
    // stay green through a rebuilt provenance.
    assert_eq!(carried, baseline);
    // The identity emitted BEHAVIOR really ran once (the wrapper omitted
    // nothing), and the live digest law holds on the returned value.
    assert_eq!(
        super::registry_test_support::identity_invocations().3,
        1,
        "the identity emitted BEHAVIOR ran exactly once"
    );
    assert_eq!(
        carried.output_fingerprint(),
        emitted_output_fingerprint(carried.bytes())
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_behavior_failure_reaches_the_public_variant_with_its_typed_fault() {
    let world = fixture();
    let registry = registry_with(&[Arc::new(FailingSource)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#src",
        TransformStage::Source,
        "test-source-fails",
    )]);

    let error = compile(plan, &world, &registry).unwrap_err();
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("a behavior fault is never a generic pass string: {error:?}")
    };
    assert!(
        matches!(
            public.inner(),
            TransformError::Behavior {
                stage: TransformStage::Source,
                source: TransformBehaviorError::WrongStage { .. },
                ..
            }
        ),
        "the typed behavior source and the wrapper's own stage survive: {public}"
    );
    assert!(
        !matches!(error, ArtifactCompileError::Pass { .. }),
        "no string erasure"
    );
    // The public error keeps the STANDARD source chain alive: the unnamed
    // dyn source downcasts, in-crate, to the exact private fault.
    let chained = std::error::Error::source(public)
        .expect("the opaque error does not terminate the source chain");
    assert!(
        chained.downcast_ref::<TransformError>().is_some(),
        "the chain carries the exact internal fault, not a rendered string"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_verifier_rejection_of_a_transform_output_is_transform_attributed() {
    let world = fixture();
    let registry = registry_with(&[Arc::new(BlankOriginSource)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#src",
        TransformStage::Source,
        "test-source-blank-origin",
    )]);

    let error = compile(plan, &world, &registry).unwrap_err();
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("a transform-attributed verifier fault stays typed: {error:?}")
    };
    assert!(
        matches!(public.inner(), TransformError::Verification { .. }),
        "classified through the exact name set, not a string sniff: {public}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_emitted_path_behavior_failure_is_classified_through_the_same_shared_helper() {
    // The emitted path crosses the ARTIFACT segment, not discovery: its
    // PassFailed box must still classify through the one shared
    // transform-first helper and reach the public variant typed.
    let world = fixture();
    let registry = registry_with(&[Arc::new(FailingEmitted)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#emit",
        TransformStage::Emitted,
        "test-emit-fails",
    )]);

    let error = compile(plan, &world, &registry).unwrap_err();
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("the emitted-path fault is the transform family: {error:?}")
    };
    assert!(
        matches!(
            public.inner(),
            TransformError::Behavior {
                stage: TransformStage::Emitted,
                ..
            }
        ),
        "the emitted path keeps the typed fault through the shared classifier: {public}"
    );
}

#[derive(Default)]
struct NameRecorder(Mutex<Vec<String>>);

impl CompileTraceSink for NameRecorder {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.0.lock().unwrap().push(event.pass().to_string());
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_exact_pass_name_survives_schedule_and_trace_identity() {
    // `transform:document:org.demo/tools#doc` carries `:`, `/` and `#` — the
    // three metacharacters a filename codec would love to mangle. Within
    // vibe-spec both the declared schedule and the observed trace events
    // spell it identically, once per document.
    let world = fixture();
    let plan = fixture().plan.with_transforms(identity_plan(&[(
        "org.demo/tools#doc",
        TransformStage::Document,
    )]));
    let recorder = NameRecorder::default();

    compile_artifact_traced_with_registries(
        plan,
        &world.source,
        &BackendRegistry::builtins(),
        &identity_registry(),
        Some(&recorder),
    )
    .unwrap();

    let exact = "transform:document:org.demo/tools#doc";
    let observed = recorder.0.lock().unwrap();
    assert_eq!(
        observed
            .iter()
            .filter(|name| name.as_str() == exact)
            .count(),
        5,
        "the exact spelling crossed the trace boundary once per document: {observed:?}"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|name| name.as_str() == "parse")
            .count(),
        5,
        "sanity: five documents were parsed"
    );
}
