//! §6.3.1.1 and §6.3.1.5's laws: prior ownership is ENGINE evidence that
//! reaches every provider plan, and reading it creates nothing.
//!
//! Its own cell because the two are one law read from both ends. The engine
//! may not hand a provider a destination decision it did not make itself
//! (§6.3.1.1), and it may not pay for that read with a directory tree under
//! the operator's settings home (§6.3.1.5) — so the evidence here is always
//! a pair: the value ARRIVED, and the state home is exactly as it was.

use std::rc::Rc;

use specmark::verifies;

use super::sidecar::LOCK_RESOURCES_FILE;
use super::state::DeploymentHome;
use super::support::{Faults, Fixture, FixtureProvider, Witness, selected, selection, target};
use super::view::DeployStateView;
use super::{DeployError, apply_prepared, apply_selection, plan::plan_resolved, preplan};

/// §6.3.1.5 at the view itself: "If the root is absent it returns no
/// receipt/sidecar and creates no directory."
///
/// Both reads, because both are the view's own answer and the absence is a
/// VALUE rather than a fault — a machine that has never deployed anything is
/// the ordinary case, and a planner that refused on it would refuse on every
/// first run. A present root then answers from what is really there.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_no_create_view_answers_an_absent_state_home_without_creating_it() {
    let fixture = Fixture::new("helper-bytes");
    let state_home = fixture.state_home();
    let home = DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper");

    let absent = DeployStateView::open(&state_home).expect("an absent state home is a value");
    assert!(
        absent
            .read_receipt(&home)
            .expect("the receipt read")
            .is_none()
    );
    assert!(
        absent
            .read_lock_resources(&home)
            .expect("the sidecar read")
            .is_none()
    );
    assert!(!state_home.exists(), "and neither read created it");

    // The same two reads against a home a real deployment created.
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
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
    .expect("the deployment applies");

    let present = DeployStateView::open(&state_home).expect("a present state home pins");
    assert_eq!(
        present
            .read_receipt(&home)
            .expect("the receipt read")
            .expect("the finalised receipt")
            .generation,
        0,
    );
    assert_eq!(
        present
            .read_lock_resources(&home)
            .expect("the sidecar read")
            .and_then(|record| record.committed)
            .expect("the committed binding")
            .resources,
        ["bin/helper"],
    );
}

/// Acceptance 1's first half / §6.3.1.5: "`--plan` and a later-target
/// preplan refusal leave an initially absent `state_home` absent."
///
/// Both surfaces read prior receipts, and neither may create the home it
/// reads them from. The old planner opened `DeployState`, which creates —
/// so a `vibe deploy --plan` on a machine that had never deployed anything
/// left a `state/deployments/` tree behind. A read-only verb that writes a
/// directory is read-only about destinations only, and the difference is
/// observable, which is what makes it a law rather than a preference.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn neither_the_planner_nor_a_refused_preplan_creates_the_state_home() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["first", "second"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    assert!(
        !state_home.exists(),
        "the fixture starts with no state home"
    );

    let reports = plan_resolved(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/first"],
            )),
        )],
    )
    .expect("the read-only planner runs");
    assert_eq!(reports.len(), 1);
    assert!(
        !state_home.exists(),
        "a read-only planner creates no deployment state home",
    );

    // The same claim for the pre-apply epoch: target 1 refuses at `plan`,
    // and the reads target 0's plan needed left nothing behind either.
    let error = apply_selection(
        &execution,
        &[
            selected(
                &targets[0],
                Box::new(FixtureProvider::new(
                    fixture.destination.path(),
                    &["bin/first"],
                )),
            ),
            selected(
                &targets[1],
                Box::new(
                    FixtureProvider::new(fixture.destination.path(), &["bin/second"]).faulty(
                        Faults {
                            fail_plan: true,
                            ..Faults::default()
                        },
                    ),
                ),
            ),
        ],
    )
    .expect_err("the second target cannot be planned");
    assert!(matches!(error, DeployError::Provider(_)), "{error}");
    assert!(
        !state_home.exists(),
        "a refused pre-apply epoch creates no deployment state home either",
    );
}

/// Acceptance 1's second half / §6.3.1.1 and §6.3.1.5: "prior receipt
/// ownership reaches provider plan when state exists", and it is "the same
/// prior receipt value … in both `--plan` and preapply".
///
/// The recording is the whole proof. An engine that read its state home and
/// then dropped the answer is indistinguishable from one that never read it,
/// unless the value is observed where a provider would use it — so the
/// fixture keeps every prior receipt it was handed and the three plans below
/// are compared against what the state home really holds.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_prior_receipt_reaches_provider_plan_on_both_surfaces() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    // Plan 1 — nothing is deployed, so prior ownership is honestly absent.
    let first = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    apply_selection(
        &execution,
        &[selected(&targets[0], Box::new(Witness(Rc::clone(&first))))],
    )
    .expect("generation 0 deploys");
    assert_eq!(
        first.priors(),
        [None],
        "a first deployment plans against no prior receipt",
    );

    // Plan 2 — the read-only planner, against the receipt now on disk.
    let planner = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    plan_resolved(
        &execution,
        &[selected(
            &targets[0],
            Box::new(Witness(Rc::clone(&planner))),
        )],
    )
    .expect("the planner runs");
    let planned = planner.priors();
    let seen = planned[0]
        .as_ref()
        .expect("`--plan` hands the provider the receipt on disk");
    assert_eq!(seen.generation, 0);
    assert_eq!(seen.target, "local-helper");
    assert_eq!(
        seen.resources
            .iter()
            .map(|owned| owned.resource.as_str())
            .collect::<Vec<_>>(),
        ["bin/helper"],
        "and it owns exactly what the deployment recorded",
    );

    // Plan 3 — the pre-apply epoch, which must see the SAME value.
    fixture.rebuild("updated-bytes");
    let applier = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(Witness(Rc::clone(&applier))),
        )],
    )
    .expect("generation 1 deploys");
    assert_eq!(
        applier.priors(),
        planned,
        "preapply plans against the same prior receipt `--plan` reported",
    );
}

/// Acceptance 2 / §6.3.1.1: "Apply rechecks the same receipt under the
/// deployment-state lock before writing." A receipt changed after preplan
/// refuses BEFORE the intent and before any external apply.
///
/// The window is real and only visible from outside a single call: the
/// pre-apply epoch reads prior ownership, and an unbounded amount of time
/// later — an operator reading a `--plan`, a slow artifact resolution, a
/// concurrent `vibe deploy` — the transaction begins. This drives exactly
/// that: preplan against generation 0, let a second run finish generation 1,
/// then apply what was prepared.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn apply_refuses_a_receipt_that_changed_after_the_pre_apply_epoch() {
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

    let stale = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    let resolved = [selected(&targets[0], Box::new(Witness(Rc::clone(&stale))))];
    let prepared = preplan(&execution, &resolved).expect("the epoch plans against generation 0");
    assert_eq!(
        prepared[0]
            .prior_receipt
            .as_ref()
            .expect("generation 0's receipt")
            .generation,
        0,
    );

    // A concurrent run of the same deployment finishes generation 1.
    fixture.rebuild("updated-bytes");
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
    .expect("a concurrent run lands generation 1");

    let error = apply_prepared(&execution, &resolved, &prepared)
        .expect_err("the prepared plan is against ownership that changed");

    let DeployError::PriorReceiptChanged {
        target,
        planned,
        found,
        ..
    } = &error
    else {
        panic!("expected the prior-ownership refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert!(planned.contains("generation 0"), "{planned}");
    assert!(found.contains("generation 1"), "{found}");
    assert_eq!(
        stale.calls(),
        ["plan"],
        "the stale provider was never asked to apply: {:?}",
        stale.calls(),
    );
}

/// The receipt recheck is not merely before provider apply: it is before
/// every durable write of the new attempt, including the ordinary legacy
/// pending-binding repair. A stale prepared plan must leave an interrupted
/// journal and its absent sidecar byte-for-byte as it found them.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn changed_prior_ownership_refuses_before_legacy_sidecar_repair() {
    let fixture = Fixture::new("generation-zero");
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
    let stale = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    let resolved = [selected(&targets[0], Box::new(Witness(Rc::clone(&stale))))];
    let prepared = preplan(&execution, &resolved).expect("the stale plan sees generation 0");

    fixture.rebuild("generation-one");
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
    .expect("generation 1 deploys");
    fixture.rebuild("generation-two");
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
    .expect_err("generation 2 leaves an interrupted intent");

    let home = DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper");
    let sidecar = home.directory().join(LOCK_RESOURCES_FILE);
    let intent = home.directory().join("intent.json");
    std::fs::remove_file(&sidecar).expect("simulate an ordinary pre-sidecar journal");
    let intent_before = std::fs::read(&intent).expect("the interrupted intent exists");

    let error = apply_prepared(&execution, &resolved, &prepared)
        .expect_err("the prepared plan is against older ownership");
    assert!(
        matches!(error, DeployError::PriorReceiptChanged { .. }),
        "{error}"
    );
    assert!(
        !sidecar.exists(),
        "the recheck preceded legacy sidecar repair"
    );
    assert_eq!(
        std::fs::read(&intent).expect("the intent remains"),
        intent_before,
        "the refusal did not rewrite or retire the interrupted journal",
    );
    assert_eq!(stale.calls(), ["plan"]);
}
