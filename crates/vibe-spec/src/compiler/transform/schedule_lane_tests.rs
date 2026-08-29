//! T6c acceptance (ABI §6.2 item 3): the manager-side lane witness.
//!
//! The world is the shared five-document `artifact_tests` fixture, so every
//! test here drives the REAL built-in schedule end to end — the lane position
//! runs once per artifact, after assemble and before the backend.
//!
//! Each test guards one property that would be lost if its check were removed:
//! the intrinsic contract really runs, the transition really runs (once per
//! immutable provenance field), a lawful contribution rewrite is ACCEPTED and
//! moves the bytes, the three checks run exactly once per artifact, and the
//! whole guarantee survives with the optional inter-pass verifier hook absent.

use std::sync::Arc;

use specmark::verifies;

use crate::compiler::artifact_tests::{Fixture, fixture};
use crate::compiler::assemble::{
    LaneValidationError, assemble_invocations, reset_assemble_invocations,
};
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{
    ArtifactCompileError, BuiltinSchedule, compile_artifact_with_registries, without_verify_each,
};
use crate::compiler::emit::{emit_invocations, reset_emit_invocations};
use crate::compiler::ir::{ArtifactPlan, EmittedArtifact};
use crate::compiler::verify::{LaneProvenanceField, TransitionError};

use super::behavior::TransformBehavior;
use super::lane_admission::{lane_admission_counts, reset_lane_admission_counts};
use super::plan::{TransformImplementation, TransformProvider, TransformSeed, TransformStage};
use super::plan_test_support::{build_or_panic, default_dependency};
use super::registry_test_support::{
    identity_invocations, identity_plan, identity_registry, reset_identity_invocations,
};
use super::schedule::TransformError;
use super::schedule_execution_vehicles::registry_with;
use super::schedule_lane_vehicles::*;

/// The one lane entry every test installs: dense order 0, no config, no
/// selector — the lane stage carries none by grammar.
fn lane_plan(name: &str) -> ArtifactPlan {
    let seed = TransformSeed::new(
        vibe_core::manifest::ExtensionKey::authored("org.demo/tools#lane"),
        TransformProvider::from(&default_dependency()),
        TransformStage::Lane,
        TransformImplementation::builtin_candidate(name, 1),
        None,
        None,
    );
    fixture().plan.with_transforms(build_or_panic(vec![seed]))
}

fn compile_with(
    vehicle: Arc<dyn TransformBehavior>,
    name: &str,
    world: &Fixture,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let registry = registry_with(&[vehicle]);
    compile_artifact_with_registries(
        lane_plan(name),
        &world.source,
        &BackendRegistry::builtins(),
        &registry,
    )
}

/// The typed transform fault one refusal carries, or a named panic.
fn transform_fault(error: &ArtifactCompileError) -> &TransformError {
    let ArtifactCompileError::Transform(public) = error else {
        panic!("a lane refusal is the transform family: {error:?}")
    };
    public.inner()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_structurally_broken_lane_is_refused_by_the_intrinsic_contract() {
    // Reordering without renumbering leaves every carried `contribution`
    // index naming the position it used to occupy — the exact fault the
    // intrinsic walk reports, with its typed source intact.
    reset_assemble_invocations();
    reset_emit_invocations();
    reset_lane_vehicle_count();
    let world = fixture();

    let error = compile_with(Arc::new(ReorderLane), "test-lane-reorder", &world).unwrap_err();
    let fault = transform_fault(&error);
    let TransformError::LaneIntrinsic {
        order,
        stage,
        source,
        ..
    } = fault
    else {
        panic!("the intrinsic contract owns this refusal: {fault}")
    };
    assert_eq!(*order, 0, "the entry identity rides along");
    assert_eq!(*stage, TransformStage::Lane);
    assert!(
        matches!(
            source.as_ref(),
            LaneValidationError::ChunkMismatch {
                contribution: 0,
                field: "open contribution",
                ..
            }
        ),
        "the typed LaneValidationError survives: {source}"
    );
    assert_eq!(lane_vehicle_invocations(), 1, "the behavior ran once");
    assert_eq!(assemble_invocations(), 1, "the refusal is after assemble");
    assert_eq!(emit_invocations("static-xml"), 0, "and before the backend");
}

/// One provenance vehicle refused, naming exactly the field it moved.
fn refuse_provenance(
    vehicle: Arc<dyn TransformBehavior>,
    name: &str,
    expected_field: LaneProvenanceField,
) -> (String, String) {
    let world = fixture();
    let error = compile_with(vehicle, name, &world).unwrap_err();
    let fault = transform_fault(&error);
    let TransformError::LaneTransition {
        order,
        stage,
        source,
        ..
    } = fault
    else {
        panic!("a provenance rewrite is a transition refusal: {fault}")
    };
    assert_eq!(*order, 0, "the entry identity rides along");
    assert_eq!(*stage, TransformStage::Lane);
    let TransitionError::LaneProvenance {
        field,
        expected,
        actual,
    } = source.as_ref()
    else {
        panic!("the lane arm owns its own variant, never `Identity`: {source}")
    };
    assert_eq!(*field, expected_field, "the moved field is named: {source}");
    assert_ne!(expected, actual, "expected/actual are the two real values");
    (expected.clone(), actual.clone())
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn rewriting_the_artifact_context_is_refused() {
    let (expected, actual) = refuse_provenance(
        Arc::new(RewriteContextLane),
        "test-lane-context",
        LaneProvenanceField::Context,
    );
    assert!(
        expected.contains("QualifyPerNode") && actual.contains("Plain"),
        "the refusal names both compile modes: {expected} -> {actual}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn rewriting_the_source_node_count_is_refused() {
    let (expected, actual) = refuse_provenance(
        Arc::new(BumpNodeCountLane),
        "test-lane-node-count",
        LaneProvenanceField::SourceNodeCount,
    );
    let before: usize = expected.parse().expect("the count renders as a number");
    let after: usize = actual.parse().expect("the count renders as a number");
    assert_eq!(after, before + 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn rewriting_the_source_link_digest_is_refused() {
    let (expected, actual) = refuse_provenance(
        Arc::new(RewriteDigestLane),
        "test-lane-digest",
        LaneProvenanceField::SourceLinkDigest,
    );
    assert_eq!(actual, "5a".repeat(32), "the forged digest is named in hex");
    assert_eq!(expected.len(), 64, "and so is the real one");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn rewriting_the_frame_generated_path_is_refused() {
    let (_, actual) = refuse_provenance(
        Arc::new(RewriteGeneratedPathLane),
        "test-lane-generated-path",
        LaneProvenanceField::FrameGeneratedPath,
    );
    assert!(actual.contains(FORGED_GENERATED_PATH), "{actual}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn rewriting_the_frame_source_root_is_refused() {
    let (_, actual) = refuse_provenance(
        Arc::new(RewriteSourceRootLane),
        "test-lane-source-root",
        LaneProvenanceField::FrameSourceRoot,
    );
    assert!(actual.contains(FORGED_SOURCE_ROOT), "{actual}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn rewriting_the_frame_renames_is_refused() {
    // The sharpest of the six: `frame.renames` flows onward into
    // `EmissionProvenance.renames`, so an accepted rewrite here would forge a
    // record the manager alone authors.
    let (_, actual) = refuse_provenance(
        Arc::new(RewriteRenamesLane),
        "test-lane-renames",
        LaneProvenanceField::FrameRenames,
    );
    assert!(actual.contains("org.demo/forged"), "{actual}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_lawful_contribution_rewrite_is_accepted_and_moves_the_bytes() {
    // The proof that T6c OPENED the position instead of relocating the
    // refusal: the same reordering the intrinsic contract refuses above is
    // accepted once it renumbers, and the emitted tape really changes.
    reset_emit_invocations();
    reset_lane_vehicle_count();
    let plain: Fixture = fixture();
    let carried: Fixture = fixture();

    let baseline = compile_artifact_with_registries(
        plain.plan,
        &plain.source,
        &BackendRegistry::builtins(),
        &identity_registry(),
    )
    .unwrap();
    let changed = compile_with(
        Arc::new(RenumberedReorderLane),
        "test-lane-renumber",
        &carried,
    )
    .unwrap();

    assert_eq!(lane_vehicle_invocations(), 1);
    assert_eq!(emit_invocations("static-xml"), 2, "both worlds emitted");
    assert_ne!(
        changed.bytes(),
        baseline.bytes(),
        "a lawful lane change must be observable in the emitted bytes"
    );
    // Observable in the exact way the transform asked for: the two normal
    // contributions swap places on the tape. The contribution PATHS are the
    // discriminator — an origin alone also appears in the renamed-anchor
    // tombstone, which precedes every contribution in both worlds.
    let baseline_text = String::from_utf8(baseline.bytes().to_vec()).unwrap();
    let changed_text = String::from_utf8(changed.bytes().to_vec()).unwrap();
    let order = |text: &str| {
        (
            text.find("boot/alpha.md").expect("alpha is emitted"),
            text.find("boot/omega.md").expect("omega is emitted"),
        )
    };
    let (baseline_alpha, baseline_omega) = order(&baseline_text);
    let (changed_alpha, changed_omega) = order(&changed_text);
    assert!(baseline_alpha < baseline_omega, "authored order");
    assert!(changed_omega < changed_alpha, "reversed order");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_lane_gate_runs_exactly_once_per_artifact() {
    // Instrumented on the three CHECKS themselves, not on the pass: a pass
    // counter proves a pass ran, never that the check inside it ran. Five
    // documents, one artifact, one lane entry: one of each.
    reset_lane_admission_counts();
    reset_identity_invocations();
    let world = fixture();
    let plan = fixture().plan.with_transforms(identity_plan(&[(
        "org.demo/tools#lane",
        TransformStage::Lane,
    )]));

    let emitted = compile_artifact_with_registries(
        plan,
        &world.source,
        &BackendRegistry::builtins(),
        &identity_registry(),
    )
    .unwrap();

    assert!(!emitted.bytes().is_empty());
    assert_eq!(
        lane_admission_counts(),
        (1, 1, 1),
        "witness, intrinsic and transition each ran once for the artifact"
    );
    assert_eq!(
        identity_invocations().2,
        1,
        "and the lane behavior itself ran once"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_lane_gate_still_refuses_with_the_inter_pass_verifier_absent() {
    // The guarantee must not ride on `enable_verify_each_for_tests`: that
    // seam is `#[cfg(test)]`, so a lane check routed through it would leave
    // production unguarded. Here the schedule is built exactly as production
    // builds it, and the two refusals still arrive typed.
    let world = fixture();
    let armed = BuiltinSchedule::emitted_for_test(&fixture().plan, &identity_registry()).unwrap();
    assert!(
        armed.pipeline_for_test().verify_each_enabled_for_test(),
        "the ordinary test schedule carries the verifier"
    );

    without_verify_each(|| {
        // The seam is genuinely disarmed — otherwise everything below would
        // be proving the verifier's law, not the manager's.
        let production =
            BuiltinSchedule::emitted_for_test(&fixture().plan, &identity_registry()).unwrap();
        assert!(
            !production
                .pipeline_for_test()
                .verify_each_enabled_for_test(),
            "the production construction carries no verifier"
        );

        let broken = compile_with(Arc::new(ReorderLane), "test-lane-reorder", &world).unwrap_err();
        assert!(
            matches!(
                transform_fault(&broken),
                TransformError::LaneIntrinsic { .. }
            ),
            "the intrinsic contract runs manager-side: {broken}"
        );

        let forged = compile_with(
            Arc::new(RewriteRenamesLane),
            "test-lane-renames",
            &fixture(),
        )
        .unwrap_err();
        assert!(
            matches!(
                transform_fault(&forged),
                TransformError::LaneTransition { .. }
            ),
            "the transition check runs manager-side: {forged}"
        );
    });
}
