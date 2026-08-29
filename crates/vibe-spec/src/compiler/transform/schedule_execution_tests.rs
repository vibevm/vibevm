//! T6b execution tests (ABI §6.2–6.3): per-document versus per-artifact
//! invocation, causal position proofs, config delivery, the lane/emitted
//! positions' cardinality, typed error surfacing, and pass-name trace
//! identity.
//!
//! The world is the shared `artifact_tests` fixture (five discovered
//! documents: alpha, shared, the simple local entry, omega, the embedded
//! piece — `parse_invocations() == 5`). Local mutating/failing vehicles
//! extend a CLONE of the shared identity registry: they never enter
//! `builtins()` and never alter the T5 golden. Causality vehicles append a
//! FENCED code block — real parsed content, no anchor or fact-id pressure
//! across the shared/qualified lane.
//!
//! The lane position's own admission law is T6c's, and its tests live in
//! `schedule_lane_tests`; the emitted position's reconstruction law is T9's,
//! and its tests live in `schedule_emitted_tests`.

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
use crate::compiler::verify::DocumentIdentityField;

use super::behavior::TransformBehaviorError;
use super::config::{ConfigTable, ConfigValue};
use super::fault::TransformError;
use super::plan::{TransformConfig, TransformSeed, TransformStage};
use super::plan_test_support::{build_or_panic, default_dependency, dependency_seed, empty_config};
use super::registry::TransformRegistry;
use super::registry_test_support::{identity_plan, identity_registry, identity_seed};
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
fn the_identity_lane_crosses_once_and_returns_the_whole_baseline_value() {
    // Position and cardinality: the lane admission gate T6c installs is
    // exercised in `schedule_lane_tests`, and identity output remains the
    // commissioning vector here.
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
fn changed_emitted_bytes_are_reconstructed_after_the_backend_ran() {
    // T9 retired the interim refusal: a changed tape is LAWFUL, and the
    // manager rebuilds the artifact around it. What this cell owns is the
    // POSITION — the rewrite happens once, after the backend really emitted;
    // the reconstruction law itself lives in `schedule_emitted_tests`.
    reset_emit_invocations();
    reset_vehicle_counts();
    let world = fixture();
    let registry = registry_with(&[Arc::new(AppendEmitted)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#emit",
        TransformStage::Emitted,
        "test-emit-append",
    )]);
    let baseline = compile(fixture().plan.clone(), &world, &identity_registry()).unwrap();

    let carried = compile(plan, &world, &registry).unwrap();

    let mut expected = baseline.bytes().to_vec();
    expected.push(b'\n');
    assert_eq!(
        carried.bytes(),
        expected.as_slice(),
        "the behavior's tape is the tape that came back"
    );
    assert_eq!(
        carried.output_fingerprint(),
        emitted_output_fingerprint(carried.bytes()),
        "and the manager's recomputed digest describes exactly those bytes"
    );
    assert_eq!(EMITTED_COUNT.with(std::cell::Cell::get), 1);
    assert_eq!(
        emit_invocations("static-xml"),
        2,
        "both worlds emitted once; the rewrite is post-emit, never instead of it"
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
    assert!(
        carried.provenance().emitted_transforms.is_empty(),
        "a byte-equal behavior rewrote nothing, so it recorded nothing"
    );
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

/// The T7 carrier at the position that will evaluate selectors: every
/// discovered document arrives with its OWN subject, and the two ABSENCES are
/// two different answers in one compile.
///
/// The exact set is asserted, not the count. Two of the five rows are the
/// load-bearing ones for the path: the alpha and omega seeds carry the path
/// their contribution row DECLARED (`boot/alpha.md`, `boot/omega.md`), which
/// the addresses `…/boot/entry` do not spell — so the subject cannot have been
/// re-derived — while `shared` and `piece`, which no row declared, carry their
/// own document paths.
///
/// The provider column is the distinguishability property, and asserting the
/// exact set IS the proof: the literal below names three `undetermined` rows
/// (declared by a contribution row, so their owner exists and merely has no
/// typed spelling yet) beside two `unclaimed` ones (reached through
/// `#use`/`#embed`, so no row declared them and none ever will). A single
/// fused absence cannot satisfy a literal that spells both, so collapsing the
/// arms — in either direction — makes this red and names the documents that
/// answered wrongly.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn every_discovered_document_arrives_with_its_own_declared_subject() {
    OBSERVED_SUBJECTS.with(|rows| rows.borrow_mut().clear());
    let world = fixture();
    let registry = registry_with(&[Arc::new(RecordingSubjects)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#doc",
        TransformStage::Document,
        "test-record-subject",
    )]);

    compile(plan, &world, &registry).unwrap();

    let mut observed = OBSERVED_SUBJECTS.with(|rows| rows.borrow().clone());
    observed.sort();
    let mut expected: Vec<(String, String, String)> = [
        // Declared by a contribution row: the row's path, not the address',
        // and an owner that exists but is not yet typed.
        (
            "spec://org.demo/alpha/boot/entry#root",
            "boot/alpha.md",
            "undetermined",
        ),
        (
            "spec://org.demo/omega/boot/entry#root",
            "boot/omega.md",
            "undetermined",
        ),
        // The simple contribution's static entry: declared, path and all.
        (
            "static entry (origin \"host\", path \"boot/local.md\")",
            "boot/local.md",
            "undetermined",
        ),
        // Reached through `#use` and `#embed`: their own document paths, and
        // no owner to name — permanently, not pending.
        (
            "spec://org.demo/shared/boot/base#root",
            "boot/base",
            "unclaimed",
        ),
        (
            "spec://org.demo/piece/boot/piece#root",
            "boot/piece",
            "unclaimed",
        ),
    ]
    .into_iter()
    .map(|(address, path, provider)| (address.to_string(), path.to_string(), provider.to_string()))
    .collect();
    expected.sort();
    assert_eq!(observed, expected);
}

/// The address is not the subject: the alpha row declares `boot/alpha.md`
/// while its seed address' `doc_path` is `boot/entry`, and the carried value
/// is the declared one.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_declared_path_that_differs_from_the_address_survives_to_the_position() {
    OBSERVED_SUBJECTS.with(|rows| rows.borrow_mut().clear());
    let world = fixture();
    let registry = registry_with(&[Arc::new(RecordingSubjects)]);
    let plan = plan_of(vec![vehicle_seed(
        "org.demo/tools#doc",
        TransformStage::Document,
        "test-record-subject",
    )]);

    compile(plan, &world, &registry).unwrap();

    let observed = OBSERVED_SUBJECTS.with(|rows| rows.borrow().clone());
    let alpha = observed
        .iter()
        .find(|(address, ..)| address == "spec://org.demo/alpha/boot/entry#root")
        .expect("the alpha seed reached the document position");
    assert_eq!(alpha.1, "boot/alpha.md");
    assert_ne!(alpha.1, "boot/entry", "the address path is not the subject");
}

/// A source-position transform that returns a different subject is refused,
/// and the fault names the moved member.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_source_transform_that_rewrites_the_subject_is_refused() {
    assert_subject_forgery_refused(
        Arc::new(RetargetSubjectSource),
        "org.demo/tools#src",
        TransformStage::Source,
        "test-source-retarget-subject",
    );
}

/// The same law one position later: a document transform owns the tree, never
/// the subject its source carries.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_document_transform_that_rewrites_the_subject_is_refused() {
    assert_subject_forgery_refused(
        Arc::new(RetargetSubjectDocument),
        "org.demo/tools#doc",
        TransformStage::Document,
        "test-document-retarget-subject",
    );
}

/// One subject forgery, refused with the transform-attributed verifier fault
/// that names `subject.declared_path` and both spellings.
fn assert_subject_forgery_refused(
    vehicle: Arc<dyn super::behavior::TransformBehavior>,
    key: &str,
    stage: TransformStage,
    name: &str,
) {
    let world = fixture();
    let registry = registry_with(&[vehicle]);
    let plan = plan_of(vec![vehicle_seed(key, stage, name)]);

    let error = compile(plan, &world, &registry).unwrap_err();
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("a subject forgery stays the typed transform family: {error:?}")
    };
    let TransformError::Verification { source, .. } = public.inner() else {
        panic!("the verifier owns the refusal: {public}")
    };
    let crate::compiler::verify::VerificationError::Transition(
        crate::compiler::verify::TransitionError::DocumentIdentity {
            field,
            expected,
            actual,
        },
    ) = source.as_ref()
    else {
        panic!("the moved member is typed, never a rendered string: {source:?}")
    };
    assert_eq!(*field, DocumentIdentityField::SubjectDeclaredPath);
    assert_eq!(actual, FORGED_PATH);
    assert_ne!(expected, actual);
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
