//! §7.2's saga and its inverse — the multi-target laws.
//!
//! > "A failed multi-target deploy is a recorded saga: already-applied
//! > reversible targets are rolled back in reverse order; irreversible
//! > results remain visible as partial, never reported as success."
//!
//! > "`undeploy` removes only receipt-owned state and refuses to erase a
//! > path changed after deployment without an explicit force/recovery
//! > decision."

use specmark::verifies;

use super::state::DeploymentHome;
use super::support::{Faults, Fixture, FixtureProvider, selected, selection, target};
use super::{DeployError, apply_selection, list_deployments};

/// Read one deployment's receipt as JSON, or `None`.
fn receipt(fixture: &Fixture, target_id: &str) -> Option<serde_json::Value> {
    let home = DeploymentHome::new(&fixture.state_home(), "org.example/demo", None, target_id);
    let bytes = std::fs::read(home.directory().join("receipt.json")).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// A reversible prefix is rolled back in REVERSE order when a later
/// target fails, and the run reports the saga rather than a success.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_failed_multi_target_deploy_rolls_the_reversible_prefix_back_in_reverse() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &["first"]),
        target("third", "helper.exe", &["second"]),
    ];
    let selection = selection("local", &["first", "second", "third"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let resolved = vec![
        selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/first"],
            )),
        ),
        selected(
            &targets[1],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/second"],
            )),
        ),
        selected(
            &targets[2],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/third"]).faulty(Faults {
                    fail_apply_after: Some(0),
                    ..Faults::default()
                }),
            ),
        ),
    ];

    let error = apply_selection(&execution, &resolved).expect_err("the third target fails");

    let DeployError::Saga {
        target,
        rolled_back,
        retained,
        ..
    } = &error
    else {
        panic!("expected the saga refusal, got: {error}");
    };
    assert_eq!(target, "third");
    assert_eq!(
        rolled_back, "second, first",
        "reverse order, not declaration order",
    );
    assert_eq!(retained, "none");
    assert!(
        !fixture.destination.path().join("bin/first").exists(),
        "the first target's destination was reversed",
    );
    assert!(
        !fixture.destination.path().join("bin/second").exists(),
        "and so was the second's",
    );
    for reversed in ["first", "second"] {
        let receipt = receipt(&fixture, reversed).expect("a reversed receipt survives");
        assert_eq!(receipt["status"], "rolled-back");
        assert_eq!(
            receipt["resources"].as_array().expect("an owned set").len(),
            0,
            "a rolled-back deployment owns nothing",
        );
    }
}

/// An IRREVERSIBLE applied target stays visible as partial and the run is
/// still a failure — §7.2's "never reported as success".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_irreversible_applied_target_remains_visible_as_partial() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("permanent", "helper.exe", &[]),
        target("later", "helper.exe", &["permanent"]),
    ];
    let selection = selection("local", &["permanent", "later"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let resolved = vec![
        selected(
            &targets[0],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/permanent"]).irreversible(),
            ),
        ),
        selected(
            &targets[1],
            Box::new(
                FixtureProvider::new(fixture.destination.path(), &["bin/later"]).faulty(Faults {
                    fail_apply_after: Some(0),
                    ..Faults::default()
                }),
            ),
        ),
    ];

    let error = apply_selection(&execution, &resolved).expect_err("the later target fails");

    let DeployError::Saga {
        rolled_back,
        retained,
        ..
    } = &error
    else {
        panic!("expected the saga refusal, got: {error}");
    };
    assert_eq!(rolled_back, "none");
    assert_eq!(retained, "permanent");
    assert!(
        fixture.destination.path().join("bin/permanent").is_file(),
        "an irreversible deployment is not undone behind the operator's back",
    );
    let receipt = receipt(&fixture, "permanent").expect("the partial deployment stays recorded");
    assert_eq!(receipt["status"], "verified");
    assert_eq!(receipt["reversible"], false);
    // And it is visible: `vibe deployments` shows the partial state.
    let rows = list_deployments(&state_home).expect("the state home lists");
    assert!(
        rows.iter()
            .any(|row| row.target == "permanent" && row.status.as_str() == "verified"),
        "{rows:?}",
    );
}

/// `undeploy` removes exactly the receipt-owned state and leaves every
/// unrecorded neighbour alone.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn undeploy_removes_only_receipt_owned_state() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    std::fs::create_dir_all(fixture.destination.path().join("bin"))
        .expect("the fixture destination exists");
    std::fs::write(
        fixture.destination.path().join("bin/neighbour"),
        "not ours\n",
    )
    .expect("the unowned neighbour writes");

    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("the deployment applies");
    assert!(fixture.destination.path().join("bin/helper").is_file());

    let inverse = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    let resolved = vec![selected(&targets[0], Box::new(inverse))];
    let removals =
        super::undeploy_resolved(&execution, &resolved).expect("the inverse deployment runs");

    assert_eq!(removals.len(), 1);
    assert_eq!(removals[0].removed, ["bin/helper"]);
    assert!(
        !fixture.destination.path().join("bin/helper").exists(),
        "the owned resource is gone",
    );
    assert!(
        fixture.destination.path().join("bin/neighbour").is_file(),
        "and the unrecorded neighbour is untouched",
    );
    let receipt = receipt(&fixture, "local-helper").expect("the receipt records the reversal");
    assert_eq!(receipt["status"], "rolled-back");
}

/// § "refuses to erase a path changed after deployment without an explicit
/// force/recovery decision".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn undeploy_refuses_a_path_changed_after_deployment() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("the deployment applies");
    std::fs::write(
        fixture.destination.path().join("bin/helper"),
        "edited after deployment",
    )
    .expect("the drift writes");

    let inverse = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]);
    let error = super::undeploy_resolved(&execution, &[selected(&targets[0], Box::new(inverse))])
        .expect_err("a changed path is never erased silently");

    let DeployError::UndeployDrift { target, resources } = &error else {
        panic!("expected the drift refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert_eq!(resources, "bin/helper");
    assert!(
        fixture.destination.path().join("bin/helper").is_file(),
        "and the changed path still exists — nothing was erased",
    );
}

/// § "A collision with state owned by another deployment is an error."
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_second_deployment_may_not_seize_a_resource_another_already_owns() {
    let fixture = Fixture::new("helper-bytes");
    let first = [target("first", "helper.exe", &[])];
    let second = [target("second", "helper.exe", &[])];
    let state_home = fixture.state_home();
    let first_selection = selection("local", &["first"]);
    let second_selection = selection("local", &["second"]);
    apply_selection(
        &fixture.execution(&first, &first_selection, &state_home),
        &[selected(
            &first[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/shared"],
            )),
        )],
    )
    .expect("the first deployment owns the resource");

    let error = apply_selection(
        &fixture.execution(&second, &second_selection, &state_home),
        &[selected(
            &second[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/shared"],
            )),
        )],
    )
    .expect_err("a second deployment may not seize it");

    let DeployError::OwnershipCollision {
        target,
        owner,
        resources,
    } = &error
    else {
        panic!("expected the ownership refusal, got: {error}");
    };
    assert_eq!(target, "second");
    assert!(owner.starts_with("first ("), "{owner}");
    assert_eq!(resources, "bin/shared");
    assert!(
        std::fs::read_to_string(fixture.destination.path().join("bin/shared"))
            .expect("the first deployment's bytes survive")
            .contains("bin/shared"),
    );
}

/// A deployment that applied and then FAILED independent verification
/// still owns what it touched: its receipt records the resources and §7.2
/// grants no exception for a failed status. Only a rolled-back receipt —
/// one whose resources were removed and emptied — stops owning, so a
/// second deployment may not seize a failed one's destination either.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_failed_deployment_still_owns_the_resources_it_touched() {
    let fixture = Fixture::new("helper-bytes");
    let first = [target("first", "helper.exe", &[])];
    let second = [target("second", "helper.exe", &[])];
    let state_home = fixture.state_home();
    let corrupt =
        FixtureProvider::new(fixture.destination.path(), &["bin/shared"]).faulty(Faults {
            corrupt: true,
            ..Faults::default()
        });
    apply_selection(
        &fixture.execution(&first, &selection("local", &["first"]), &state_home),
        &[selected(&first[0], Box::new(corrupt))],
    )
    .expect_err("the first deployment applies and fails verification");
    assert_eq!(
        receipt(&fixture, "first").expect("the failed receipt exists")["status"],
        "failed"
    );

    let error = apply_selection(
        &fixture.execution(&second, &selection("local", &["second"]), &state_home),
        &[selected(
            &second[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/shared"],
            )),
        )],
    )
    .expect_err("the destination a failed deployment mutated is still owned");
    let DeployError::OwnershipCollision { owner, .. } = &error else {
        panic!("expected the ownership refusal, got: {error}");
    };
    assert!(owner.starts_with("first ("), "{owner}");
}
