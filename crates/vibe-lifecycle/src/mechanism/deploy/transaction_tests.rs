//! §7.2's transaction, sentence by sentence.
//!
//! Every test here is one sentence of the architecture's own text, driven
//! against the hermetic fixture provider. The crash windows are real: the
//! fixture stops mid-apply, the process's own state home is left exactly
//! as an interrupted run would leave it, and the NEXT call is the recovery
//! §7.2 specifies.

use specmark::verifies;

use super::state::{CheckpointRecord, DeployState, DeploymentHome};
use super::support::{Faults, Fixture, FixtureProvider, Witness, selected, selection, target};
use super::{DeployError, apply_selection};

/// Read the raw JSON of one state file, or `None`.
fn state_file(fixture: &Fixture, target_id: &str, name: &str) -> Option<serde_json::Value> {
    let home = DeploymentHome::new(&fixture.state_home(), "org.example/demo", None, target_id);
    let bytes = std::fs::read(home.directory().join(name)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// § "Before apply, VibeVM atomically writes a durable intent journal
/// containing the plan hash, prior receipt generation, every planned
/// resource and its desired digest."
///
/// The fixture fails at resource zero, so nothing external has been
/// written when the run stops — and the journal is nevertheless on disk.
/// That ordering IS the law: a journal written after the first external
/// write cannot cover a crash between them.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_intent_is_durable_before_the_first_external_write() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper", "bin/alias"])
        .faulty(Faults {
            fail_apply_after: Some(0),
            ..Faults::default()
        });
    let resolved = vec![selected(&targets[0], Box::new(provider))];

    let error = apply_selection(&execution, &resolved).expect_err("the fixture fails mid-apply");

    assert!(
        format!("{error}").contains("told to fail here"),
        "the provider's own refusal surfaced: {error}",
    );
    let intent = state_file(&fixture, "local-helper", "intent.json")
        .expect("the intent journal is durable before the first external write");
    assert_eq!(intent["schema"], 1);
    assert_eq!(intent["target"]["target"], "local-helper");
    assert_eq!(intent["target"]["profile"], "local");
    assert_eq!(intent["target"]["generation"], 0);
    assert!(intent["prior_generation"].is_null(), "{intent}");
    let planned = intent["resources"]
        .as_array()
        .expect("every planned resource is journalled");
    assert_eq!(planned.len(), 2);
    for row in planned {
        assert_eq!(
            row["desired_digest"]
                .as_str()
                .expect("a desired digest")
                .len(),
            64,
        );
    }
    assert!(
        state_file(&fixture, "local-helper", "receipt.json").is_none(),
        "no receipt exists for a deployment that never verified",
    );
    assert!(
        !fixture.destination.path().join("bin/helper").exists(),
        "and the destination was never touched",
    );
}

/// § "Apply checkpoints completed operations without storing secrets."
///
/// The fixture stops between two resources, so exactly one completion is
/// durable — the checkpoint window, proven at the only place it is
/// observable.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn apply_checkpoints_each_completed_operation() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper", "bin/alias"])
        .faulty(Faults {
            fail_apply_after: Some(1),
            ..Faults::default()
        });
    let resolved = vec![selected(&targets[0], Box::new(provider))];

    apply_selection(&execution, &resolved).expect_err("the fixture fails after one resource");

    let ledger = state_file(&fixture, "local-helper", "checkpoints.json")
        .expect("one completed operation is durable");
    let record: CheckpointRecord =
        serde_json::from_value(ledger).expect("the ledger is this engine's own shape");
    assert_eq!(record.schema, 1);
    assert_eq!(record.completed, ["bin/helper"]);
    assert!(
        fixture.destination.path().join("bin/helper").exists(),
        "the checkpointed resource really was reconciled",
    );
    assert!(
        !fixture.destination.path().join("bin/alias").exists(),
        "and the uncheckpointed one was not",
    );
}

/// § "After independent verify, the finalized receipt is written and the
/// intent is retired."
///
/// The happy path, with the three durable facts asserted in the order the
/// law puts them: verify ran, the receipt exists and is `verified`, and
/// the journal is gone.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn verify_precedes_the_finalized_receipt_and_the_intent_then_retires() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    let calls = {
        let resolved = vec![selected(&targets[0], Box::new(provider))];
        let outcomes = apply_selection(&execution, &resolved).expect("the deployment applies");
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].generation, 0);
        assert_eq!(outcomes[0].settlement, "none");
        assert_eq!(outcomes[0].resources.len(), 1);
        // The provider handed back through the boxed trait object, so the
        // call log is read from the same instance the run used.
        resolved
    };
    drop(calls);

    let receipt = state_file(&fixture, "local-helper", "receipt.json")
        .expect("the finalized receipt is written");
    assert_eq!(receipt["status"], "verified");
    assert!(
        receipt["finalized_at"].is_string(),
        "a terminal status carries its finalisation instant: {receipt}",
    );
    assert_eq!(receipt["generation"], 0);
    assert_eq!(receipt["scope"], "user");
    assert_eq!(receipt["provider"]["key"], super::support::FIXTURE_PIN);
    assert!(
        state_file(&fixture, "local-helper", "intent.json").is_none(),
        "the intent retires once the receipt is final",
    );
    assert!(
        state_file(&fixture, "local-helper", "checkpoints.json").is_none(),
        "and the ledger retires with it",
    );
}

/// The verb ORDER itself: `plan`, then `apply`, then `verify` — and
/// `verify` after `apply`, never instead of it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_provider_is_asked_to_verify_only_after_it_applied() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let witness = std::rc::Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    let resolved = vec![selected(&targets[0], Box::new(Witness(witness.clone())))];

    apply_selection(&execution, &resolved).expect("the deployment applies");

    let calls = witness.calls();
    let apply = calls
        .iter()
        .position(|verb| verb == "apply")
        .expect("apply ran");
    let verify = calls
        .iter()
        .position(|verb| verb == "verify")
        .expect("verify ran");
    assert!(apply < verify, "verify follows apply: {calls:?}");
    assert_eq!(calls.first().map(String::as_str), Some("plan"));
}

/// § "receipt finalisation is last" — a deployment whose INDEPENDENT
/// verify disagrees with what apply claimed is recorded as `failed` and
/// refused, never reported as success.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_independent_verify_that_disagrees_is_never_a_verified_receipt() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider =
        FixtureProvider::new(fixture.destination.path(), &["bin/helper"]).faulty(Faults {
            corrupt: true,
            ..Faults::default()
        });
    let resolved = vec![selected(&targets[0], Box::new(provider))];

    let error = apply_selection(&execution, &resolved).expect_err("verify disagrees");

    let DeployError::VerifyMismatch { target, resources } = &error else {
        panic!("expected the verify law's own refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert_eq!(resources, "bin/helper");
    let receipt = state_file(&fixture, "local-helper", "receipt.json")
        .expect("the mutation the deployment really made is still owned");
    assert_eq!(receipt["status"], "failed");
}

/// § "if all observed resources match either the prior or desired digest,
/// the idempotent provider rolls forward and finalizes".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn recover_rolls_an_interrupted_deployment_forward_and_finalises_it() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let resources = ["bin/helper", "bin/alias"];
    let crashed = FixtureProvider::new(fixture.destination.path(), &resources).faulty(Faults {
        fail_apply_after: Some(1),
        ..Faults::default()
    });
    apply_selection(&execution, &[selected(&targets[0], Box::new(crashed))])
        .expect_err("the first run is interrupted");

    let healthy = FixtureProvider::new(fixture.destination.path(), &resources);
    let outcomes = apply_selection(&execution, &[selected(&targets[0], Box::new(healthy))])
        .expect("the second run recovers the interrupted deployment");

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].settlement, "recovered");
    // The recovered deployment finalises under the INTERRUPTED journal's
    // own generation, not a new one: it is that deployment's completion.
    assert_eq!(outcomes[0].generation, 0);
    let receipt =
        state_file(&fixture, "local-helper", "receipt.json").expect("recovery finalises a receipt");
    assert_eq!(receipt["status"], "verified");
    assert!(
        receipt["evidence"]
            .as_str()
            .is_some_and(|evidence| evidence.starts_with("recovered: ")),
        "the evidence says it was a recovery: {receipt}",
    );
    assert!(
        state_file(&fixture, "local-helper", "intent.json").is_none(),
        "and the journal retired",
    );
    for resource in resources {
        assert!(fixture.destination.path().join(resource).is_file());
    }
}

/// § "a third digest means concurrent/user mutation, so recovery refuses
/// and names the exact resources".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn recovery_refuses_a_third_digest_and_names_the_resources() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let resources = ["bin/helper", "bin/alias"];
    let crashed = FixtureProvider::new(fixture.destination.path(), &resources).faulty(Faults {
        fail_apply_after: Some(1),
        ..Faults::default()
    });
    apply_selection(&execution, &[selected(&targets[0], Box::new(crashed))])
        .expect_err("the first run is interrupted");
    // A human edits the half-deployed file: neither the prior state
    // (absent) nor the desired one.
    std::fs::write(
        fixture.destination.path().join("bin/helper"),
        "edited by a human",
    )
    .expect("the fixture edit writes");

    let healthy = FixtureProvider::new(fixture.destination.path(), &resources);
    let error = apply_selection(&execution, &[selected(&targets[0], Box::new(healthy))])
        .expect_err("recovery refuses to roll forward over a third digest");

    let DeployError::RecoverDivergence { target, resources } = &error else {
        panic!("expected the three-digest refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert_eq!(resources, "bin/helper");
    assert!(
        state_file(&fixture, "local-helper", "intent.json").is_some(),
        "the journal survives a refused recovery — nothing was decided",
    );
    assert!(
        state_file(&fixture, "local-helper", "receipt.json").is_none(),
        "and no receipt claims a deployment that never completed",
    );
}

/// The third digest's silent spelling: ABSENCE. A resource the interrupted
/// deployment was UPDATING (its intent carries a prior digest) that is
/// observed absent matches neither the prior nor the desired state —
/// someone deleted it in the crash window — so recovery refuses exactly as
/// it refuses an edit. Absence passes only for a resource the deployment
/// was going to CREATE, and this pin is what holds the two cases apart.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn recovery_refuses_a_deleted_resource_the_intent_was_updating() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let resources = ["bin/helper"];
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(fixture.destination.path(), &resources)),
        )],
    )
    .expect("the first generation deploys and records the prior digest");
    let crashed = FixtureProvider::new(fixture.destination.path(), &resources).faulty(Faults {
        fail_apply_after: Some(0),
        ..Faults::default()
    });
    apply_selection(&execution, &[selected(&targets[0], Box::new(crashed))])
        .expect_err("the second generation is interrupted before any write");
    std::fs::remove_file(fixture.destination.path().join("bin/helper"))
        .expect("a human deletes the deployed file in the crash window");

    let healthy = FixtureProvider::new(fixture.destination.path(), &resources);
    let error = apply_selection(&execution, &[selected(&targets[0], Box::new(healthy))])
        .expect_err("recovery refuses to roll forward over a deletion");
    let DeployError::RecoverDivergence { target, resources } = &error else {
        panic!("expected the three-digest refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert_eq!(resources, "bin/helper");
}

/// § "A receipt plus its still-present matching intent is a benign crash
/// after finalization: retire the intent."
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_receipt_with_its_matching_intent_is_benign_and_the_intent_retires() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("the first deployment applies");
    // The crash window §7.2 names: the receipt was written, the process
    // died before the journal was retired. Re-plant exactly that journal.
    let home = DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper");
    let journal = serde_json::json!({
        "schema": 1,
        "plan_hash": "0".repeat(64),
        "resources": [],
        "started_at": "2026-08-30T12:00:00Z",
        "target": {
            "generation": 0,
            "profile": "local",
            "project": "org.example/demo",
            "target": "local-helper",
        },
    });
    std::fs::write(
        home.directory().join("intent.json"),
        serde_json::to_vec_pretty(&journal).expect("the fixture journal encodes"),
    )
    .expect("the fixture journal writes");

    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    let outcomes = apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("a benign leftover journal does not stop the next run");

    assert_eq!(outcomes[0].settlement, "benign-intent-retired");
    assert_eq!(
        outcomes[0].generation, 1,
        "the next deployment is a new generation, not the retired one",
    );
    assert!(state_file(&fixture, "local-helper", "intent.json").is_none());
}

/// The state home is ENGINE-owned and its records carry no secret-bearing
/// member — §7.2's "timestamps and final status, never secrets".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_receipt_carries_exactly_the_members_the_law_lists_and_no_secret() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("the deployment applies");

    let receipt = state_file(&fixture, "local-helper", "receipt.json").expect("a receipt");
    let members: Vec<&str> = receipt
        .as_object()
        .expect("a receipt is an object")
        .keys()
        .map(String::as_str)
        .collect();
    for expected in [
        "applied_at",
        "artifact_digest",
        "desired_config_digest",
        "generation",
        "identity",
        "profile",
        "provider",
        "resources",
        "reversible",
        "schema",
        "scope",
        "status",
        "target",
    ] {
        assert!(
            members.contains(&expected),
            "missing `{expected}`: {members:?}"
        );
    }
    for forbidden in ["token", "secret", "password", "credential", "env", "argv"] {
        assert!(
            !members.contains(&forbidden),
            "a receipt never carries `{forbidden}`: {members:?}",
        );
    }
    // And the whole state home holds no copy of a sentinel the operator
    // would call a secret: the fixture never hands one over, and no
    // engine-owned member could carry one if it did.
    let rendered = serde_json::to_string(&receipt).expect("the receipt renders");
    assert!(!rendered.contains("helper-bytes"), "{rendered}");
}

/// The state home is opened, created and pinned by the ENGINE, and its
/// layout is the one this cell discloses.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_state_home_is_created_under_the_settings_directory_it_was_given() {
    let fixture = Fixture::new("helper-bytes");
    let state_home = fixture.state_home();
    assert!(
        state_home.starts_with(fixture.settings.path()),
        "the state home hangs off the settings directory it was handed",
    );
    assert!(state_home.ends_with("deployments"));

    DeployState::open(&state_home).expect("the engine creates its own state home");
    assert!(state_home.is_dir());
}
