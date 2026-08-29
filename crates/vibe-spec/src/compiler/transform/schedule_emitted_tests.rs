//! T9 acceptance (ABI §6.5): manager-owned emitted reconstruction.
//!
//! The world is the shared five-document `artifact_tests` fixture, so every
//! test here drives the REAL built-in schedule end to end — the emitted
//! position runs once per artifact, after the selected backend emitted.
//!
//! Each test guards one property that would be lost if its half were removed:
//! changed bytes really rebuild the artifact through the one digest cell while
//! every other provenance member survives byte for byte; byte-equal output
//! really returns the ORIGINAL value; a chain records the changers in plan
//! order and only the changers; and the whole law holds with the optional
//! inter-pass verifier hook absent, because it is the manager's and not the
//! verifier's.
//!
//! Position, cardinality and typed-fault classification of this wrapper stay
//! in `schedule_execution_tests`; what lives here is the reconstruction law.

use std::sync::Arc;

use specmark::verifies;

use crate::compiler::artifact_tests::{Fixture, fixture};
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{
    ArtifactCompileError, compile_artifact_with_registries, without_verify_each,
};
use crate::compiler::emit::emitted_bytes_digest;
use crate::compiler::ir::{ArtifactPlan, EmittedArtifact, emitted_output_fingerprint};

use super::plan::{TransformImplementation, TransformProvider, TransformSeed, TransformStage};
use super::plan_test_support::{build_or_panic, default_dependency};
use super::registry::TransformRegistry;
use super::registry_test_support::identity_registry;
use super::schedule_execution_vehicles::{AppendEmitted, registry_with};

/// The one changing vehicle every reconstruction test installs, and the exact
/// byte it appends — spelled once so an expected tape is derived from the
/// baseline rather than hand-copied.
const APPEND: &str = "test-emit-append";
const APPENDED_BYTE: u8 = b'\n';

/// The identity vehicle from the shared T5 catalog: the byte-equal arm.
const IDENTITY: &str = "test-identity-emitted";

/// One emitted-stage seed. The KEY is what the schedule pass name is built
/// from, so two entries may share one implementation and still be two distinct
/// recorded identities — which is exactly what the chain tests exploit.
fn emitted_seed(key: &str, implementation: &str) -> TransformSeed {
    TransformSeed::new(
        vibe_core::manifest::ExtensionKey::authored(key),
        TransformProvider::from(&default_dependency()),
        TransformStage::Emitted,
        TransformImplementation::builtin_candidate(implementation, 1),
        None,
        None,
    )
}

/// A plan of emitted entries in authored order, attached to the shared world.
fn emitted_plan(entries: &[(&str, &str)]) -> ArtifactPlan {
    fixture().plan.with_transforms(build_or_panic(
        entries
            .iter()
            .map(|(key, implementation)| emitted_seed(key, implementation))
            .collect(),
    ))
}

fn compile(
    plan: ArtifactPlan,
    world: &Fixture,
    registry: &TransformRegistry,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    compile_artifact_with_registries(plan, &world.source, &BackendRegistry::builtins(), registry)
}

/// The untransformed compile of the same world: the value every preservation
/// and identity assertion is stated against.
fn untransformed(world: &Fixture) -> EmittedArtifact {
    compile(fixture().plan.clone(), world, &identity_registry()).expect("the plain world compiles")
}

/// The registry carrying the shared identity catalog plus the appending
/// vehicle.
fn appending_registry() -> TransformRegistry {
    registry_with(&[Arc::new(AppendEmitted)])
}

/// The recorded chain as plain strings, in carried order.
fn chain_of(emitted: &EmittedArtifact) -> Vec<&str> {
    emitted
        .provenance()
        .emitted_transforms
        .iter()
        .map(|name| name.as_str())
        .collect()
}

/// The baseline tape plus one appended byte per changing entry.
fn appended(baseline: &EmittedArtifact, times: usize) -> Vec<u8> {
    let mut expected = baseline.bytes().to_vec();
    expected.extend(std::iter::repeat_n(APPENDED_BYTE, times));
    expected
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn changed_bytes_rebuild_the_artifact_around_the_new_tape() {
    // The digest half of the law: the tape the behavior returned is the tape
    // that came back, its digest is the ONE digest cell's digest of exactly
    // those bytes, and the fingerprint the trace records agrees with an
    // independent observation of the same bytes.
    let world = fixture();
    let baseline = untransformed(&world);
    let carried = compile(
        emitted_plan(&[("org.demo/tools#emit", APPEND)]),
        &world,
        &appending_registry(),
    )
    .unwrap();

    assert_eq!(carried.bytes(), appended(&baseline, 1).as_slice());
    assert_eq!(
        carried.provenance().bytes_digest,
        emitted_bytes_digest(carried.bytes()),
        "the digest is recomputed from the bytes actually carried"
    );
    assert_ne!(
        carried.provenance().bytes_digest,
        baseline.provenance().bytes_digest,
        "and it really moved — otherwise the assertion above is vacuous"
    );
    assert_eq!(
        carried.output_fingerprint(),
        emitted_output_fingerprint(carried.bytes()),
        "one fingerprint spelling, post-transform"
    );
    assert_eq!(
        chain_of(&carried),
        ["transform:emitted:org.demo/tools#emit"]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn every_other_provenance_member_survives_the_reconstruction() {
    // Member by member on purpose: a whole-value `assert_eq!` on provenance
    // would move with the digest and the chain, so it could never state
    // "everything ELSE was copied". Each member is checked against the
    // untransformed compile of the same world.
    let world = fixture();
    let baseline = untransformed(&world);
    let carried = compile(
        emitted_plan(&[("org.demo/tools#emit", APPEND)]),
        &world,
        &appending_registry(),
    )
    .unwrap();

    let before = baseline.provenance();
    let after = carried.provenance();
    assert_eq!(after.context, before.context);
    assert_eq!(after.backend, before.backend);
    assert_eq!(
        after.producer, before.producer,
        "`producer` names the BACKEND that made the bytes; a transform never becomes the producer"
    );
    assert_eq!(after.source_lane_digest, before.source_lane_digest);
    assert_eq!(after.renames, before.renames);
    assert_eq!(after.contributions, before.contributions);
    // The preservation assertions must have something to preserve: an empty
    // list would stay equal to an empty list however the cell mangled it.
    assert!(
        !before.contributions.is_empty(),
        "the shared world contributes emission witnesses"
    );
    assert!(
        !before.renames.is_empty(),
        "the shared world qualifies per node, so it carries origin renames"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn byte_equal_output_returns_the_original_artifact_untouched() {
    // WHOLE-VALUE equality is the observable of "the ORIGINAL came back":
    // selected-field comparisons would stay green through a rebuilt
    // provenance whose members happened to be copied correctly.
    let world = fixture();
    let baseline = untransformed(&world);
    let carried = compile(
        emitted_plan(&[("org.demo/tools#emit", IDENTITY)]),
        &world,
        &identity_registry(),
    )
    .unwrap();

    assert_eq!(carried, baseline);
    assert!(
        carried.provenance().emitted_transforms.is_empty(),
        "a behavior that changed nothing rewrote nothing, so it records nothing"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_chain_records_both_changers_in_plan_order_and_digests_the_final_tape() {
    // Two entries, one implementation, two keys: the recorded identity is the
    // SCHEDULE's (built from the key), not the behavior's name — and the
    // authored order is not the sorted one, so a sort would be visible.
    let world = fixture();
    let baseline = untransformed(&world);
    let carried = compile(
        emitted_plan(&[
            ("org.demo/tools#emit-second", APPEND),
            ("org.demo/tools#emit-first", APPEND),
        ]),
        &world,
        &appending_registry(),
    )
    .unwrap();

    assert_eq!(
        chain_of(&carried),
        [
            "transform:emitted:org.demo/tools#emit-second",
            "transform:emitted:org.demo/tools#emit-first",
        ],
        "plan order, never sorted or catalog order"
    );
    assert_eq!(
        carried.bytes(),
        appended(&baseline, 2).as_slice(),
        "both entries really ran"
    );
    assert_eq!(
        carried.provenance().bytes_digest,
        emitted_bytes_digest(carried.bytes()),
        "the digest is of the FINAL tape, not of an intermediate one"
    );
    assert_ne!(
        carried.provenance().bytes_digest,
        emitted_bytes_digest(&appended(&baseline, 1)),
        "and the intermediate tape's digest is a different value"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_mixed_chain_records_only_the_entries_that_changed_the_bytes() {
    // Both orders, because the two are different mistakes: appending on the
    // byte-equal entry would show up in the first, and losing the changer's
    // own record behind a following identity in the second.
    for (label, entries) in [
        (
            "identity first",
            [
                ("org.demo/tools#emit-idle", IDENTITY),
                ("org.demo/tools#emit-busy", APPEND),
            ],
        ),
        (
            "changer first",
            [
                ("org.demo/tools#emit-busy", APPEND),
                ("org.demo/tools#emit-idle", IDENTITY),
            ],
        ),
    ] {
        let world = fixture();
        let baseline = untransformed(&world);
        let carried = compile(emitted_plan(&entries), &world, &appending_registry()).unwrap();

        assert_eq!(
            chain_of(&carried),
            ["transform:emitted:org.demo/tools#emit-busy"],
            "{label}: exactly the entry that moved the bytes"
        );
        assert_eq!(
            carried.bytes(),
            appended(&baseline, 1).as_slice(),
            "{label}: one append, whichever position it sat in"
        );
        assert_eq!(
            carried.provenance().bytes_digest,
            emitted_bytes_digest(carried.bytes()),
            "{label}: and the digest follows the tape"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn reconstruction_holds_with_the_inter_pass_verifier_absent() {
    // The guarantee must not ride on `enable_verify_each_for_tests`: that seam
    // is `#[cfg(test)]`, so a law routed through it would leave production
    // unguarded. Here the schedule is built exactly as production builds it,
    // and both arms still hold.
    without_verify_each(|| {
        let world = fixture();
        let baseline = untransformed(&world);

        let changed = compile(
            emitted_plan(&[("org.demo/tools#emit", APPEND)]),
            &world,
            &appending_registry(),
        )
        .unwrap();
        assert_eq!(changed.bytes(), appended(&baseline, 1).as_slice());
        assert_eq!(
            changed.provenance().bytes_digest,
            emitted_bytes_digest(changed.bytes())
        );
        assert_eq!(
            chain_of(&changed),
            ["transform:emitted:org.demo/tools#emit"]
        );

        let identical = compile(
            emitted_plan(&[("org.demo/tools#emit", IDENTITY)]),
            &world,
            &identity_registry(),
        )
        .unwrap();
        assert_eq!(identical, baseline);
    });
}
