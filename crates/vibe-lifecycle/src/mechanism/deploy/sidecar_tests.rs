//! §6.3.1.2 and §6.3.1.3's laws: the durable lock sidecar's two generations,
//! the order they are written in, and the exact slot each settlement moves.
//!
//! Its own cell because every test here is about a CRASH WINDOW — what is on
//! disk when a run stops between two durable writes — and the evidence is
//! always the raw record, read back from the state home rather than from the
//! value the engine held.
//!
//! Three of them stop the run at the pending write itself, with a plan whose
//! lock set breaks one of the sidecar's own laws. That is the only window
//! between "the settlement decided" and "this run staged its own pending
//! binding", and both sides of it are laws the packet states: the settlement
//! must move the right slot, and a lock set that is not usable must never
//! become a durable binding.

use specmark::verifies;

use super::sidecar::{LOCK_RESOURCES_FILE, LockBinding, LockResources};
use super::state::DeploymentHome;
use super::support::{Faults, Fixture, FixtureProvider, selected, selection, target};
use super::transaction::plan_hash;
use super::{DeployError, apply_selection};

/// One deployment's own directory inside a fixture's state home.
fn home(fixture: &Fixture, target_id: &str) -> DeploymentHome {
    DeploymentHome::new(&fixture.state_home(), "org.example/demo", None, target_id)
}

/// The durable lock record on disk, or `None` when there is none.
fn sidecar(fixture: &Fixture, target_id: &str) -> Option<LockResources> {
    let bytes = std::fs::read(
        home(fixture, target_id)
            .directory()
            .join(LOCK_RESOURCES_FILE),
    )
    .ok()?;
    Some(serde_json::from_slice(&bytes).expect("the sidecar is this engine's own shape"))
}

/// Overwrite one deployment's durable lock record.
fn write_sidecar(fixture: &Fixture, target_id: &str, record: &LockResources) {
    std::fs::write(
        home(fixture, target_id)
            .directory()
            .join(LOCK_RESOURCES_FILE),
        serde_json::to_vec_pretty(record).expect("the fixture record encodes"),
    )
    .expect("the fixture record writes");
}

/// Whether one deployment still has an unretired intent journal.
fn has_intent(fixture: &Fixture, target_id: &str) -> bool {
    home(fixture, target_id)
        .directory()
        .join("intent.json")
        .exists()
}

/// A reference-owning provider whose plan declares ONE physical destination
/// under two spellings — a defect the sidecar refuses to make durable.
///
/// It is how the tests below stop a run exactly at the pending write:
/// reference ownership exempts the provider from the equal-sets law, so the
/// plan reaches the transaction, and the binding's own identity law then
/// refuses it. Every assertion after such a refusal is about state the
/// SETTLEMENT left, because this run wrote nothing of its own.
fn defective(fixture: &Fixture) -> FixtureProvider {
    FixtureProvider::new(fixture.destination.path(), &["shared.json#alpha"])
        .referencing(&["shared.json", "Shared.json"])
}

/// RED 4 / §6.3.1.3: "The deploy plan hash binds `lock_resources` as well as
/// owned resources. A lock-only change is a different plan."
///
/// Two plans that own the same resource while locking different physical
/// documents are different plans, because the pending binding and the
/// checkpoint ledger both join on this hash — one hash would let a recovery
/// of the second run adopt the first's binding and hold the wrong document.
/// The two lists get distinct frames, so a resource MOVING between them is a
/// change too rather than cancelling out.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_plan_hash_changes_when_only_the_lock_resources_change() {
    use super::protocol::{DeployPlan, PlannedDeployResource};
    use crate::mechanism::DeployTargetRequest;

    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let request = DeployTargetRequest {
        target: &targets[0],
        profile: "local",
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
        clients: execution.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: None,
        staging: None,
    };
    let base = DeployPlan {
        resources: vec![PlannedDeployResource {
            resource: "shared.json#alpha".to_owned(),
            desired_digest: "a".repeat(64),
        }],
        lock_resources: vec!["shared.json".to_owned()],
        config_digest: "b".repeat(64),
        reversible: true,
        summary: "one member of one document".to_owned(),
    };
    let elsewhere = DeployPlan {
        lock_resources: vec!["other.json".to_owned()],
        ..base.clone()
    };
    let unlocked = DeployPlan {
        lock_resources: Vec::new(),
        ..base.clone()
    };
    // The owned set is the lock set — the ordinary provider's shape — so a
    // resource that MOVED between the two lists must not hash alike either.
    let moved = DeployPlan {
        resources: Vec::new(),
        lock_resources: vec!["shared.json#alpha".to_owned()],
        ..base.clone()
    };
    let owned_only = DeployPlan {
        lock_resources: Vec::new(),
        ..base.clone()
    };

    let pin = super::support::FIXTURE_PIN;
    assert_eq!(
        plan_hash(&request, pin, &base),
        plan_hash(&request, pin, &base.clone()),
        "the hash is a function of the plan and nothing else",
    );
    assert_ne!(
        plan_hash(&request, pin, &base),
        plan_hash(&request, pin, &elsewhere),
        "locking a different document is a different plan",
    );
    assert_ne!(
        plan_hash(&request, pin, &base),
        plan_hash(&request, pin, &unlocked),
        "locking nothing is a different plan",
    );
    assert_ne!(
        plan_hash(&request, pin, &moved),
        plan_hash(&request, pin, &owned_only),
        "owning a resource and locking it are different claims",
    );
}

/// RED 5 / acceptance 3 / §6.3.1.2: the pending binding is "durable before
/// its matching intent and therefore before the first external write".
///
/// The order is proven by breaking the first write: a plan whose lock set
/// names one physical destination twice cannot become a durable binding, and
/// the run stops there. If the intent were written first it would be on
/// disk when this refusal lands — so its ABSENCE is the ordering.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_pending_binding_is_written_before_the_intent() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("codex-entry", "helper.exe", &[])];
    let chosen = selection("local", &["codex-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    let error = apply_selection(
        &execution,
        &[selected(&targets[0], Box::new(defective(&fixture)))],
    )
    .expect_err("an unusable lock set never becomes a durable binding");

    let DeployError::RecordInvalid { record, reason } = &error else {
        panic!("expected the sidecar record refusal, got: {error}");
    };
    assert_eq!(*record, LOCK_RESOURCES_FILE);
    assert!(
        reason.contains("one physical destination twice"),
        "the refusal names the law that was broken: {reason}",
    );
    assert!(
        !has_intent(&fixture, "codex-entry"),
        "the intent is written AFTER the pending binding, so it is not there",
    );
    assert!(
        sidecar(&fixture, "codex-entry").is_none(),
        "and nothing invalid was left on disk",
    );
    assert!(
        !fixture
            .destination
            .path()
            .join("shared.json#alpha")
            .exists(),
        "and no destination byte was written",
    );
}

/// RED 6 / acceptance 6's second half / §6.3.1.2: "finalisation promotes it
/// to committed only after the receipt is durable", and EVERY new deploy
/// writes a sidecar — not only a reference owner's.
///
/// One state path, one algorithm: a per-provider sidecar would make the
/// ordinary case the untested one, and the ordinary case is every case that
/// exists today.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_finalised_deployment_holds_a_committed_binding_and_no_pending() {
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
    .expect("an ordinary deployment applies");

    let record = sidecar(&fixture, "local-helper").expect("every new deploy writes a sidecar");
    assert_eq!(record.schema, 1);
    assert!(
        record.pending.is_none(),
        "promotion retains no pending binding: {record:?}",
    );
    let committed = record.committed.expect("the promoted binding");
    assert_eq!(committed.generation, 0);
    assert_eq!(committed.resources, ["bin/helper"]);
    assert_eq!(committed.plan_hash.len(), 64);
}

/// §6.3.1.2's "Receipt failure after external apply still promotes pending
/// to committed": a failed receipt owns whatever independent verify
/// observed, so the inverse that removes it must lock what this generation
/// physically held.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_receipt_that_failed_verification_still_promotes_its_binding() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    let error = apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/helper"]).faulty(Faults {
                    corrupt: true,
                    ..Faults::default()
                }),
            ),
        )],
    )
    .expect_err("independent verify disagrees");
    assert!(
        matches!(error, DeployError::VerifyMismatch { .. }),
        "{error}"
    );

    let record = sidecar(&fixture, "local-helper").expect("a failed deployment still has a record");
    assert!(record.pending.is_none(), "{record:?}");
    assert_eq!(
        record.committed.expect("the promoted binding").resources,
        ["bin/helper"],
        "the destination it really touched is the one an inverse must lock",
    );
}

/// Acceptance 4 / §6.3.1.2: "The old committed binding is retained
/// throughout an update, so no crash window loses the inverse lock" — then
/// matching recovery transitions the exact slot.
///
/// The update reconciles a DIFFERENT destination from the generation still
/// deployed, which is what makes the retention observable: a sidecar that
/// replaced committed at the pending write would leave `bin/first` deployed
/// with no durable record of it at all.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_update_crash_keeps_the_old_committed_beside_the_new_pending() {
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
                &["bin/first"],
            )),
        )],
    )
    .expect("generation 0 deploys");

    // Generation 1 reconciles a different destination and is interrupted
    // after the intent but before the receipt.
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/second"]).faulty(Faults {
                    fail_apply_after: Some(0),
                    ..Faults::default()
                }),
            ),
        )],
    )
    .expect_err("generation 1 is interrupted");

    let crashed = sidecar(&fixture, "local-helper").expect("the crash window has a record");
    assert_eq!(
        crashed
            .committed
            .as_ref()
            .expect("the old committed binding survives")
            .resources,
        ["bin/first"],
        "the generation still deployed keeps its inverse lock",
    );
    let pending = crashed.pending.as_ref().expect("the new pending binding");
    assert_eq!(pending.generation, 1);
    assert_eq!(pending.resources, ["bin/second"]);
    assert!(has_intent(&fixture, "local-helper"));

    // The same plan, healthy: §7.2's recovery, under the pending binding's
    // own physical locks, and the promotion that ends it.
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/second"],
            )),
        )],
    )
    .expect("the interrupted deployment recovers");

    let settled = sidecar(&fixture, "local-helper").expect("the recovered record");
    assert!(settled.pending.is_none(), "{settled:?}");
    let committed = settled.committed.expect("the promoted binding");
    assert_eq!(committed.generation, 1);
    assert_eq!(committed.resources, ["bin/second"]);
    assert!(!has_intent(&fixture, "local-helper"));
}

/// RED 8 / §6.3.1.3: "stale retirement clears only that pending generation."
///
/// The interrupted generation 1 is retired by a run whose plan is a
/// different one, so its pending binding is not this run's to keep — while
/// the committed binding of generation 0, which is still deployed, is not
/// this decision's to touch. The follow-up plan then refuses at its own
/// pending write, which is what makes the settlement's own state the state
/// on disk.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_stale_intent_retirement_clears_only_its_own_pending_binding() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("codex-entry", "helper.exe", &[])];
    let chosen = selection("local", &["codex-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/first"],
            )),
        )],
    )
    .expect("generation 0 deploys");
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/second"]).faulty(Faults {
                    fail_apply_after: Some(0),
                    ..Faults::default()
                }),
            ),
        )],
    )
    .expect_err("generation 1 is interrupted");
    assert!(sidecar(&fixture, "codex-entry").is_some_and(|record| record.pending.is_some()));

    // A third run with a DIFFERENT plan: the journal is stale, and the plan
    // that retires it refuses at its own pending write.
    apply_selection(
        &execution,
        &[selected(&targets[0], Box::new(defective(&fixture)))],
    )
    .expect_err("the follow-up plan's own lock set is unusable");

    let record = sidecar(&fixture, "codex-entry").expect("the settled record");
    assert!(
        record.pending.is_none(),
        "the stale journal's pending binding went with it: {record:?}",
    );
    assert_eq!(
        record
            .committed
            .expect("the committed binding is not the stale decision's to touch")
            .resources,
        ["bin/first"],
    );
    assert!(
        !has_intent(&fixture, "codex-entry"),
        "and the stale journal itself retired",
    );
}

/// RED 7 / §6.3.1.3: "Receipt finalisation and benign-intent retirement
/// promote the matching pending binding."
///
/// The residue is exactly what a crash between the receipt write and the
/// promotion leaves — a durable receipt, its own still-unretired intent, and
/// the pending binding that receipt's generation earned — so the fixture
/// re-plants that, byte for byte, from what the real deployment produced.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_benign_intent_window_promotes_its_matching_pending_binding() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("codex-entry", "helper.exe", &[])];
    let chosen = selection("local", &["codex-entry"]);
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
    .expect("generation 0 deploys");
    let earned: LockBinding = sidecar(&fixture, "codex-entry")
        .and_then(|record| record.committed)
        .expect("the binding generation 0 earned");

    // Rewind to the crash window: the promotion had not happened yet.
    write_sidecar(
        &fixture,
        "codex-entry",
        &LockResources {
            schema: 1,
            committed: None,
            pending: Some(earned.clone()),
        },
    );
    std::fs::write(
        home(&fixture, "codex-entry")
            .directory()
            .join("intent.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "plan_hash": earned.plan_hash,
            "resources": [],
            "started_at": "2026-08-30T12:00:00Z",
            "target": {
                "generation": 0,
                "profile": "local",
                "project": "org.example/demo",
                "target": "codex-entry",
            },
        }))
        .expect("the fixture journal encodes"),
    )
    .expect("the fixture journal writes");

    // The next run settles it. Its own plan then refuses at the pending
    // write, so what is left on disk is the SETTLEMENT's work alone.
    apply_selection(
        &execution,
        &[selected(&targets[0], Box::new(defective(&fixture)))],
    )
    .expect_err("the follow-up plan's own lock set is unusable");

    let record = sidecar(&fixture, "codex-entry").expect("the settled record");
    assert_eq!(
        record.committed.as_ref(),
        Some(&earned),
        "the benign window promoted the binding its receipt earned: {record:?}",
    );
    assert!(record.pending.is_none(), "{record:?}");
    assert!(
        !has_intent(&fixture, "codex-entry"),
        "and the benign journal retired",
    );
}

/// A pre-sidecar ordinary benign window is repaired from its typed intent,
/// then promoted like every new deployment. This preserves legacy cleanup
/// without weakening the invariant that a finalised receipt has a matching
/// committed physical lock binding.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_ordinary_legacy_benign_window_materializes_then_promotes() {
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
    .expect("generation 0 deploys");
    let earned = sidecar(&fixture, "local-helper")
        .and_then(|record| record.committed)
        .expect("the generation earned a binding");
    std::fs::remove_file(
        home(&fixture, "local-helper")
            .directory()
            .join(LOCK_RESOURCES_FILE),
    )
    .expect("simulate a pre-sidecar receipt");
    std::fs::write(
        home(&fixture, "local-helper")
            .directory()
            .join("intent.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "plan_hash": earned.plan_hash,
            "resources": [{
                "resource": "bin/helper",
                "desired_digest": "a".repeat(64),
            }],
            "started_at": "2026-08-30T12:00:00Z",
            "target": {
                "generation": 0,
                "profile": "local",
                "project": "org.example/demo",
                "target": "local-helper",
            },
        }))
        .expect("the legacy intent encodes"),
    )
    .expect("the legacy intent writes");

    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/helper"]).faulty(Faults {
                    fail_apply_after: Some(0),
                    ..Faults::default()
                }),
            ),
        )],
    )
    .expect_err("the follow-up plan stops after benign settlement and its new intent");
    let repaired = sidecar(&fixture, "local-helper").expect("the sidecar was repaired");
    assert_eq!(
        repaired
            .committed
            .expect("the repaired committed binding")
            .resources,
        ["bin/helper"],
    );
    assert_eq!(
        repaired
            .pending
            .expect("the follow-up run staged its own generation")
            .generation,
        1,
    );
    assert!(has_intent(&fixture, "local-helper"));
}
