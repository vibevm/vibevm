//! §6.3.1.3 and §6.3.1.4's laws at the INVERSE: what `undeploy` and a saga
//! rollback lock, and what they refuse rather than guess.
//!
//! Its own cell because an inverse is the operation the durable sidecar
//! exists for, and because its evidence is a different genre from every
//! other suite's: not "what is on disk afterwards" but "what was HELD while
//! the provider ran". A lock file outlives its guard, so only a probe from
//! inside `remove` can tell a held lock from a released one — and only a
//! probe can tell the recorded PHYSICAL destination from the receipt's
//! logical member, which is exactly the substitution §6.3.1.4 forbids.

use specmark::verifies;
use vibe_safefs::Project;

use super::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource, RemoveReport,
};
use super::sidecar::{LOCK_RESOURCES_FILE, LockResources};
use super::state::{CheckpointLedger, DeploymentHome};
use super::support::{Faults, Fixture, FixtureProvider, selected, selection, target};
use super::{DeployError, apply_selection, undeploy_resolved};
use crate::mechanism::{DeployProvider, DeployTargetRequest, MechanismError};

/// The engine's own lock-file name for one resource spelling.
fn destination_lock(resource: &str) -> String {
    let mut digest = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(
        &mut digest,
        vibe_safefs::path_identity_key(resource).as_bytes(),
    );
    format!("{:x}.lock", sha2::Digest::finalize(digest))
}

/// The engine's own lock-file name for one deployment's state lock.
fn deployment_lock(state_home: &std::path::Path, target_id: &str) -> String {
    let home = DeploymentHome::new(state_home, "org.example/demo", None, target_id);
    format!("deployment-{}.lock", home.id())
}

/// Whether a second, independent capability can take one lock right now.
///
/// `false` means the engine is holding it. The guard is dropped
/// immediately, so a probe never changes what the run under it may do.
fn free(state_home: &std::path::Path, name: &str) -> bool {
    Project::open(state_home)
        .expect("the probe opens its own capability")
        .try_lock(name)
        .expect("the probe's try_lock itself works")
        .is_some()
}

/// One deployment's durable lock record on disk.
fn sidecar(state_home: &std::path::Path, target_id: &str) -> Option<LockResources> {
    let home = DeploymentHome::new(state_home, "org.example/demo", None, target_id);
    let bytes = std::fs::read(home.directory().join(LOCK_RESOURCES_FILE)).ok()?;
    Some(serde_json::from_slice(&bytes).expect("the sidecar is this engine's own shape"))
}

/// Replace one deployment's durable lock record.
fn write_sidecar(state_home: &std::path::Path, target_id: &str, body: &[u8]) {
    let home = DeploymentHome::new(state_home, "org.example/demo", None, target_id);
    std::fs::write(home.directory().join(LOCK_RESOURCES_FILE), body)
        .expect("the fixture record writes");
}

/// Delete one deployment's durable lock record — a receipt from before the
/// sidecar existed, in the only shape a test can produce it.
fn drop_sidecar(state_home: &std::path::Path, target_id: &str) {
    let home = DeploymentHome::new(state_home, "org.example/demo", None, target_id);
    std::fs::remove_file(home.directory().join(LOCK_RESOURCES_FILE))
        .expect("the fixture record is there to remove");
}

/// A provider that runs one assertion at the exact moment `remove` is
/// called, then behaves as the hermetic fixture does.
struct RemoveProbe {
    inner: FixtureProvider,
    probe: Box<dyn Fn()>,
}

impl DeployProvider for RemoveProbe {
    fn descriptor(&self) -> DeployDescriptor {
        self.inner.descriptor()
    }
    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        self.inner.plan(request)
    }
    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        self.inner.fingerprint(request, plan)
    }
    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.inner.apply(request, plan, checkpoint)
    }
    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        self.inner.verify(request, resources)
    }
    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        (self.probe)();
        self.inner.remove(request, resources, prior_state_handle)
    }
    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.inner.recover(request, plan, observed, checkpoint)
    }
}

/// One reference-owning provider over a distinct logical member of one
/// shared physical document — the §6.3.0.9 shape.
fn sharer(fixture: &Fixture, member: &str) -> FixtureProvider {
    FixtureProvider::new(
        fixture.destination.path(),
        &[&format!("shared.json#{member}")],
    )
    .referencing(&["shared.json"])
}

/// RED 9 / acceptance 5 / §6.3.1.3: "Reference-owned undeploy and saga
/// remove run under the recorded physical lock."
///
/// Three probes, and the second is the one that matters: the DOCUMENT is
/// locked, the logical member is NOT, and the deployment's own state lock is
/// held. An implementation that fell back to the receipt's owned strings
/// would hold the member and leave the document free — a lock no sibling
/// entry contends on — and one that skipped locking would leave all three
/// free. Both are the same red.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_reference_owned_undeploy_runs_under_the_recorded_physical_lock() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("codex-entry", "helper.exe", &[])];
    let chosen = selection("local", &["codex-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    apply_selection(
        &execution,
        &[selected(&targets[0], Box::new(sharer(&fixture, "alpha")))],
    )
    .expect("the reference owner deploys");
    assert_eq!(
        sidecar(&state_home, "codex-entry")
            .and_then(|record| record.committed)
            .expect("the committed binding")
            .resources,
        ["shared.json"],
        "the receipt owns a member; the sidecar records the document",
    );

    let probed = state_home.clone();
    let probe = RemoveProbe {
        inner: sharer(&fixture, "alpha"),
        probe: Box::new(move || {
            assert!(
                !free(&probed, &destination_lock("shared.json")),
                "the recorded PHYSICAL destination must be held while `remove` runs",
            );
            assert!(
                free(&probed, &destination_lock("shared.json#alpha")),
                "and the logical member is not what an inverse locks — a fallback to the \
                 receipt's owned strings would take this one instead",
            );
            assert!(
                !free(&probed, &deployment_lock(&probed, "codex-entry")),
                "the deployment's own state lock is held too",
            );
        }),
    };

    let removals = undeploy_resolved(&execution, &[selected(&targets[0], Box::new(probe))])
        .expect("a sidecar-backed reference owner reaches real removal");

    assert_eq!(removals[0].removed, ["shared.json#alpha"]);
    assert!(
        !fixture
            .destination
            .path()
            .join("shared.json#alpha")
            .exists(),
        "the owned member is gone",
    );
    let record = sidecar(&state_home, "codex-entry").expect("the record survives the reversal");
    assert!(
        record.committed.is_none(),
        "a successful inverse clears committed ownership: {record:?}",
    );
}

/// Acceptance 5's second half / §6.3.1.4: "missing/malformed/mismatched
/// sidecar refuses or retains without parsing owned strings."
///
/// Three shapes of the same refusal, and in every one the provider's
/// `remove` is never reached and the deployed member is still there. There
/// is no arm in which a reference owner's physical destination is
/// reconstructed from `…#alpha`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_reference_owner_refuses_to_reverse_without_its_recorded_binding() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("codex-entry", "helper.exe", &[])];
    let chosen = selection("local", &["codex-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let deployed = fixture.destination.path().join("shared.json#alpha");
    apply_selection(
        &execution,
        &[selected(&targets[0], Box::new(sharer(&fixture, "alpha")))],
    )
    .expect("the reference owner deploys");
    let intact = sidecar(&state_home, "codex-entry").expect("the record");

    // 1 — malformed: a record this engine does not speak.
    write_sidecar(&state_home, "codex-entry", br#"{"schema": 2}"#);
    let error = undeploy_resolved(
        &execution,
        &[selected(&targets[0], Box::new(sharer(&fixture, "alpha")))],
    )
    .expect_err("a record from another epoch is never partly believed");
    let DeployError::RecordInvalid { record, .. } = &error else {
        panic!("expected the sidecar record refusal, got: {error}");
    };
    assert_eq!(*record, LOCK_RESOURCES_FILE);

    // 2 — mismatched: a binding for a generation this receipt is not.
    let mut moved = intact.clone();
    moved
        .committed
        .as_mut()
        .expect("the committed binding")
        .generation = 7;
    write_sidecar(
        &state_home,
        "codex-entry",
        &serde_json::to_vec_pretty(&moved).expect("the fixture record encodes"),
    );
    let error = undeploy_resolved(
        &execution,
        &[selected(&targets[0], Box::new(sharer(&fixture, "alpha")))],
    )
    .expect_err("a binding for another generation names another generation's destinations");
    let DeployError::LockSidecarMismatch {
        target: named,
        recorded,
        wanted,
        ..
    } = &error
    else {
        panic!("expected the sidecar mismatch refusal, got: {error}");
    };
    assert_eq!(named, "codex-entry");
    assert_eq!((*recorded, *wanted), (7, 0));

    // 3 — missing: the fallback an ordinary provider has and this one
    //     never does.
    drop_sidecar(&state_home, "codex-entry");
    let error = undeploy_resolved(
        &execution,
        &[selected(&targets[0], Box::new(sharer(&fixture, "alpha")))],
    )
    .expect_err("a reference owner has no missing-sidecar fallback");
    let DeployError::LockSidecarMissing {
        target: named,
        sidecar: file,
        ..
    } = &error
    else {
        panic!("expected the missing-sidecar refusal, got: {error}");
    };
    assert_eq!(named, "codex-entry");
    assert_eq!(*file, LOCK_RESOURCES_FILE);
    assert!(
        error.to_string().contains(LOCK_RESOURCES_FILE),
        "the refusal names the record that has to be there: {error}",
    );
    assert!(
        deployed.is_file(),
        "nothing was removed under any of the three refusals",
    );
}

/// Acceptance 6 / §6.3.1.4: "Ordinary pre-sidecar receipt/intent remains
/// removable/recoverable from owned resources."
///
/// Both halves, because they use the fallback from opposite ends: an
/// undeploy derives the physical set from the receipt, and a recovery
/// materialises the pending binding from the interrupted journal. The
/// derivation is typed in both — a recorded owned-resource list, never a
/// parsed string — and it is only legal because `validate_lock_set` proved
/// this provider's lock set equals its owned set before it ever applied.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_ordinary_pre_sidecar_deployment_stays_removable_and_recoverable() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("removable", "helper.exe", &[]),
        target("recoverable", "helper.exe", &[]),
    ];
    let state_home = fixture.state_home();

    // Removable: a finalised receipt whose sidecar predates the record.
    let chosen = selection("local", &["removable"]);
    let execution = fixture.execution(&targets, &chosen, &state_home);
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/legacy"],
            )),
        )],
    )
    .expect("the ordinary deployment applies");
    drop_sidecar(&state_home, "removable");
    let probed = state_home.clone();
    let probe = RemoveProbe {
        inner: FixtureProvider::new(fixture.destination.path(), &["bin/legacy"]),
        probe: Box::new(move || {
            assert!(
                !free(&probed, &destination_lock("bin/legacy")),
                "the derived physical destination is locked exactly as a recorded one is",
            );
        }),
    };
    let removals = undeploy_resolved(&execution, &[selected(&targets[0], Box::new(probe))])
        .expect("a pre-sidecar ordinary receipt is still removable");
    assert_eq!(removals[0].removed, ["bin/legacy"]);

    // Recoverable: an interrupted journal whose sidecar predates the record.
    let chosen = selection("local", &["recoverable"]);
    let execution = fixture.execution(&targets, &chosen, &state_home);
    apply_selection(
        &execution,
        &[selected(
            &targets[1],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/one", "bin/two"]).faulty(
                    Faults {
                        fail_apply_after: Some(1),
                        ..Faults::default()
                    },
                ),
            ),
        )],
    )
    .expect_err("the deployment is interrupted");
    drop_sidecar(&state_home, "recoverable");

    let outcomes = apply_selection(
        &execution,
        &[selected(
            &targets[1],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/one", "bin/two"],
            )),
        )],
    )
    .expect("a pre-sidecar interrupted deployment still recovers");
    assert_eq!(outcomes[0].settlement, "recovered");
    let record = sidecar(&state_home, "recoverable").expect("recovery leaves a record behind");
    assert!(record.pending.is_none(), "{record:?}");
    assert_eq!(
        record.committed.expect("the promoted binding").resources,
        ["bin/one", "bin/two"],
        "the materialised fallback became this deployment's committed lock set",
    );
}

/// A sidecar that exists is not legacy. If its committed generation does
/// not match the receipt, even an ordinary provider refuses instead of
/// silently falling back to receipt-owned strings.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_ordinary_mismatched_sidecar_never_uses_the_legacy_fallback() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/helper"],
            )),
        )],
    )
    .expect("the ordinary deployment applies");
    let mut record = sidecar(&state_home, "local-helper").expect("the sidecar");
    record.committed.as_mut().expect("committed").generation = 7;
    write_sidecar(
        &state_home,
        "local-helper",
        &serde_json::to_vec_pretty(&record).expect("the record encodes"),
    );

    let error = undeploy_resolved(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/helper"],
            )),
        )],
    )
    .expect_err("a present mismatched sidecar is not a legacy receipt");
    assert!(matches!(
        error,
        DeployError::LockSidecarMismatch {
            recorded: 7,
            wanted: 0,
            ..
        }
    ));
    assert!(fixture.destination.path().join("bin/helper").is_file());
}

/// During an interrupted update, inverse takes the union of the current
/// committed destination and the in-flight pending destination. The latter
/// may already have been touched and cannot be left open to a sibling edit.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn undeploy_holds_pending_as_well_as_committed_destination_locks() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/committed"],
            )),
        )],
    )
    .expect("generation 0 deploys");
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/pending"]).faulty(Faults {
                    fail_apply_after: Some(0),
                    ..Faults::default()
                }),
            ),
        )],
    )
    .expect_err("generation 1 leaves pending state");

    let probed = state_home.clone();
    let probe = RemoveProbe {
        inner: FixtureProvider::new(fixture.destination.path(), &["bin/committed"]),
        probe: Box::new(move || {
            assert!(!free(&probed, &destination_lock("bin/committed")));
            assert!(
                !free(&probed, &destination_lock("bin/pending")),
                "the pending physical destination participates in the inverse lock union",
            );
        }),
    };
    undeploy_resolved(&execution, &[selected(&targets[0], Box::new(probe))])
        .expect("the inverse runs under both generations' destinations");
}

/// RED 10 / §6.3.1.3: an ordinary saga rollback takes the deployment lock
/// and the committed destination locks too.
///
/// The incumbent unwind reversed a destination while holding nothing: the
/// failing run's own guards drop when its apply returns, so a rollback that
/// locked nothing raced any concurrent deployment of the same destination —
/// the exact race the per-destination lock exists for. Proven from inside
/// `remove`, because a lock file outlives its guard.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_ordinary_saga_rollback_holds_the_deployment_and_destination_locks() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &["first"]),
    ];
    let chosen = selection("local", &["first", "second"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let probed = state_home.clone();
    let probe = RemoveProbe {
        inner: FixtureProvider::new(fixture.destination.path(), &["bin/first"]),
        probe: Box::new(move || {
            assert!(
                !free(&probed, &destination_lock("bin/first")),
                "a saga rollback holds the destination it is reversing",
            );
            assert!(
                !free(&probed, &deployment_lock(&probed, "first")),
                "and the deployment-state lock of the receipt it is rewriting",
            );
        }),
    };

    let error = apply_selection(
        &execution,
        &[
            selected(&targets[0], Box::new(probe)),
            selected(
                &targets[1],
                Box::new(
                    FixtureProvider::new(fixture.destination.path(), &["bin/second"]).faulty(
                        Faults {
                            fail_apply_after: Some(0),
                            ..Faults::default()
                        },
                    ),
                ),
            ),
        ],
    )
    .expect_err("the second target fails");

    let DeployError::Saga {
        rolled_back,
        retained,
        ..
    } = &error
    else {
        panic!("expected the saga refusal, got: {error}");
    };
    assert_eq!(rolled_back, "first");
    assert_eq!(retained, "none");
    assert!(!fixture.destination.path().join("bin/first").exists());
    let record = sidecar(&state_home, "first").expect("the reversed deployment keeps a record");
    assert!(
        record.committed.is_none(),
        "a successful rollback clears committed ownership: {record:?}",
    );
}
