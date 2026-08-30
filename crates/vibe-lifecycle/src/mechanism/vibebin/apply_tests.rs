//! The vibe-bin provider's RECONCILING laws — the four verbs that touch a
//! destination: the write-once payload store, independent observation,
//! restoration-or-removal, and the idempotent roll-forward.
//!
//! Its pure half is [`super::tests`], and the world both share is
//! [`super::support`].

use super::launcher::LauncherFlavour;
use super::support::{World, apply, count, launcher_name, plan_of, refusal, request, target};
use super::{VibeBinProvider, launcher, store};
use crate::mechanism::deploy::protocol::ObservedResource;
use crate::mechanism::deploy::state::{CheckpointLedger, DeployState, DeploymentHome};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::{BUILTIN_VIBE_BIN_PIN, DeployProvider, ProviderOperation};
use specmark::verifies;
use vibe_core::manifest::ArtifactKind;

/// §7.1.0 ruling 4: "A CAS payload is write-once, idempotent to re-write
/// (which is what makes apply §7.2-recoverable for free)."
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_payload_store_is_write_once_and_a_re_apply_is_a_no_op() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let first = apply(&world, &row, &artifact);
    assert!(first.evidence.contains("payload written"), "{first:?}");
    let payload = world.at(&store::payload_relative(
        LauncherFlavour::NATIVE,
        &artifact.digest,
    ));
    let stamp = std::fs::metadata(&payload)
        .and_then(|meta| meta.modified())
        .expect("the payload has a timestamp");

    let second = apply(&world, &row, &artifact);

    assert!(
        second.evidence.contains("payload already present"),
        "a second apply must not rewrite a content-addressed payload: {second:?}",
    );
    assert_eq!(
        std::fs::metadata(&payload)
            .and_then(|meta| meta.modified())
            .expect("the payload still has a timestamp"),
        stamp,
        "and the bytes on disk were never replaced",
    );
    assert_eq!(count(&world.at("store")), 1, "one payload, two applies");
}

/// A store entry holding bytes its own address does not name is damage, and
/// repairing it silently would erase what a prior generation still resolves
/// through.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_corrupted_store_entry_refuses_rather_than_being_overwritten() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let payload = world.at(&store::payload_relative(
        LauncherFlavour::NATIVE,
        &artifact.digest,
    ));
    std::fs::create_dir_all(payload.parent().expect("store has a parent")).expect("store creates");
    std::fs::write(&payload, "SOMETHING ELSE").expect("the damaged entry writes");

    let request = request(&world, &row, Some(&artifact), true);
    let plan = plan_of(&world, &row, &artifact);
    let state = DeployState::open(world.state.path()).expect("the state home opens");
    let home = DeploymentHome::new(world.state.path(), "org.example/demo", None, &row.id);
    let mut ledger = CheckpointLedger::open(&state, &home, "plan-hash").expect("the ledger opens");
    let error = VibeBinProvider
        .apply(&request, &plan, &mut ledger)
        .expect_err("a damaged store entry is never overwritten");

    assert!(
        matches!(refusal(&error), DeployProviderError::PayloadCorrupt { .. }),
        "expected the corrupt-payload refusal, got: {error}",
    );
    assert_eq!(
        std::fs::read_to_string(&payload).expect("the damaged entry survives"),
        "SOMETHING ELSE",
    );
}

/// `verify` observes the two owned resources and NEVER the payload;
/// absence is a `None` digest rather than a fault.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn verify_observes_the_owned_resources_and_reports_absence_as_a_value() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let names = [
        launcher_name("vibe-helper"),
        "bin/vibe-helper.current".to_owned(),
    ];

    let before = VibeBinProvider
        .verify(&request(&world, &row, Some(&artifact), false), &names)
        .expect("an empty destination observes");
    assert!(before.iter().all(|seen| seen.digest.is_none()));

    apply(&world, &row, &artifact);
    let after = VibeBinProvider
        .verify(&request(&world, &row, Some(&artifact), false), &names)
        .expect("the installed destination observes");
    let plan = plan_of(&world, &row, &artifact);
    for (seen, planned) in after.iter().zip(&plan.resources) {
        assert_eq!(
            seen.digest.as_deref(),
            Some(planned.desired_digest.as_str()),
            "independent verify must find what the plan promised",
        );
    }
}

/// §7.1.0 ruling 6: "rollback is the landed saga/remove path restoring the
/// prior pointer through that handle" — and the launcher, whose bytes are
/// the prior generation's too, stays so the restored command still runs.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn remove_with_a_prior_handle_restores_the_pointer_and_keeps_the_launcher() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let first = world.artifact("helper.exe", "ONE", ArtifactKind::Executable);
    apply(&world, &row, &first);
    let second = world.artifact("helper.exe", "TWO", ArtifactKind::Executable);
    let updated = apply(&world, &row, &second);
    let names = [
        launcher_name("vibe-helper"),
        "bin/vibe-helper.current".to_owned(),
    ];

    let report = VibeBinProvider
        .remove(
            &request(&world, &row, None, false),
            &names,
            updated.prior_state_handle.as_deref(),
        )
        .expect("the rollback restores");

    assert!(report.removed.is_empty(), "a rollback removes nothing");
    assert!(report.evidence.contains("restored"), "{report:?}");
    assert!(
        world.at(&launcher_name("vibe-helper")).is_file(),
        "the version-free launcher survives the rollback and still runs",
    );
    let pointer =
        std::fs::read(world.at("bin/vibe-helper.current")).expect("the pointer reads back");
    assert_eq!(launcher::pointer_digest(&pointer), Some(first.digest));
    assert_eq!(count(&world.at("store")), 2, "and no payload was touched");
}

/// With no prior state there is nothing to restore: both owned files go and
/// the payload stays as §7.1.0 ruling 4's disclosed store garbage.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn remove_without_a_handle_deletes_both_owned_files_and_no_payload() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let applied = apply(&world, &row, &artifact);
    assert_eq!(
        applied.prior_state_handle, None,
        "a first deployment keeps nothing"
    );
    let names = [
        launcher_name("vibe-helper"),
        "bin/vibe-helper.current".to_owned(),
    ];

    let report = VibeBinProvider
        .remove(&request(&world, &row, None, false), &names, None)
        .expect("the inverse deployment removes");

    assert_eq!(report.removed, names);
    assert_eq!(
        count(&world.at("bin")),
        0,
        "the destination holds nothing owned"
    );
    assert_eq!(
        count(&world.at("store")),
        1,
        "and the payload is left behind"
    );
}

/// `recover` completes an interrupted apply idempotently: what already
/// holds the desired digest is left alone, and the rest is re-derived.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn recover_completes_an_interrupted_apply_without_rewriting_what_landed() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let plan = plan_of(&world, &row, &artifact);
    // The crash window: the payload and the launcher landed, the pointer
    // never did.
    let request = request(&world, &row, Some(&artifact), true);
    store::place_payload(
        &row.id,
        world.settings.path(),
        Some(world.staging.path()),
        LauncherFlavour::NATIVE,
        &artifact.absolute,
        &artifact.digest,
    )
    .expect("the payload lands");
    store::place_resource(
        &row.id,
        world.settings.path(),
        Some(world.staging.path()),
        &plan.resources[0].resource,
        &launcher::render(LauncherFlavour::NATIVE, "vibe-helper"),
        LauncherFlavour::NATIVE.needs_executable_bit(),
    )
    .expect("the launcher lands");
    let observed = vec![
        ObservedResource {
            resource: plan.resources[0].resource.clone(),
            digest: Some(plan.resources[0].desired_digest.clone()),
        },
        ObservedResource {
            resource: plan.resources[1].resource.clone(),
            digest: None,
        },
    ];
    let state = DeployState::open(world.state.path()).expect("the state home opens");
    let home = DeploymentHome::new(world.state.path(), "org.example/demo", None, &row.id);
    let mut ledger = CheckpointLedger::open(&state, &home, "plan-hash").expect("the ledger opens");

    let report = VibeBinProvider
        .recover(&request, &plan, &observed, &mut ledger)
        .expect("the interrupted apply rolls forward");

    assert!(
        report.evidence.contains("payload already present"),
        "{report:?}"
    );
    let pointer =
        std::fs::read(world.at("bin/vibe-helper.current")).expect("the pointer reads back");
    assert_eq!(launcher::pointer_digest(&pointer), Some(artifact.digest));
    assert_eq!(
        report.prior_state_handle, None,
        "there was no prior pointer to keep"
    );
}

/// §4.1's provider portion: the launcher template's epoch and the platform
/// flavour, and deliberately nothing about the artifact.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_fingerprint_is_the_template_epoch_and_the_platform_flavour() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let one = world.artifact("helper.exe", "ONE", ArtifactKind::Executable);
    let two = world.artifact("helper.exe", "TWO", ArtifactKind::Executable);

    let first = VibeBinProvider
        .fingerprint(
            &request(&world, &row, Some(&one), false),
            &plan_of(&world, &row, &one),
        )
        .expect("the fingerprint answers");
    let second = VibeBinProvider
        .fingerprint(
            &request(&world, &row, Some(&two), false),
            &plan_of(&world, &row, &two),
        )
        .expect("the fingerprint answers");

    assert_eq!(
        first, second,
        "an artifact is the ENGINE's half, not this one's"
    );
    assert_eq!(first.digest.len(), 64);
    assert!(
        first.summary.contains(LauncherFlavour::NATIVE.as_str()),
        "{}",
        first.summary,
    );
    assert!(
        first.summary.contains("template epoch 1"),
        "{}",
        first.summary
    );
}

/// The descriptor declares all six §3.2 operations and the user scope §7.1
/// gives this destination.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_descriptor_declares_six_operations_and_a_user_scope_destination() {
    let descriptor = VibeBinProvider.descriptor();
    assert_eq!(descriptor.provider.key, BUILTIN_VIBE_BIN_PIN);
    for operation in [
        ProviderOperation::Plan,
        ProviderOperation::Fingerprint,
        ProviderOperation::Apply,
        ProviderOperation::Verify,
        ProviderOperation::Remove,
        ProviderOperation::Recover,
    ] {
        assert!(
            descriptor.implements(operation),
            "{operation:?} is declared"
        );
    }
    assert_eq!(descriptor.scope().as_str(), "user");
    assert!(
        descriptor.atomic_replacement,
        "every owned file is published by rename",
    );
    // The rendered kind list a refusal quotes is a constant, so this is
    // what keeps it from drifting away from the list it renders.
    assert_eq!(
        super::SUPPORTED_KINDS
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        super::SUPPORTED_KINDS_LIST,
    );
    assert_eq!(descriptor.provider.kinds, super::SUPPORTED_KINDS);
}
