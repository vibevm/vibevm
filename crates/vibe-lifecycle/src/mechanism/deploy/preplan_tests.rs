//! §6.3.0.9 and §6.3.0.10's laws: the pre-apply epoch, owned-versus-
//! physical ownership, and the two places that judgement has to hold — the
//! apply path and the read-only planner.
//!
//! Its own cell because every one of these is a statement about what has
//! NOT happened yet: no destination byte, no state file, no receipt. A
//! suite that shared a file with the transaction's own laws would make
//! "nothing was applied" harder to read, not easier. The injected-authority
//! laws are the sibling cell [`authority_tests`](super::authority_tests).

use std::rc::Rc;

use specmark::verifies;

use super::support::{Faults, Fixture, FixtureProvider, Witness, selected, selection, target};
use super::{DeployError, apply_selection};

/// §6.3.0.10's first sentence: "Every selected plan is prepared before the
/// first apply."
///
/// Target 1 refuses at `plan`. Target 0 is a perfectly good deployment and
/// still must not have touched anything: no destination file, no receipt,
/// no intent, no deployment directory at all. The call log proves the
/// order rather than inferring it — both `plan`s ran, no `apply` did.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_later_plan_refusal_leaves_the_first_target_byte_absent() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["first", "second"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let good = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/first"],
    ));
    let refusing =
        FixtureProvider::new(fixture.destination.path(), &["bin/second"]).faulty(Faults {
            fail_plan: true,
            ..Faults::default()
        });

    let error = apply_selection(
        &execution,
        &[
            selected(&targets[0], Box::new(Witness(Rc::clone(&good)))),
            selected(&targets[1], Box::new(refusing)),
        ],
    )
    .expect_err("the second target cannot be planned");

    assert!(
        matches!(error, DeployError::Provider(_)),
        "the provider's own refusal, not a saga: {error}",
    );
    assert!(
        !error.to_string().contains("rolled back"),
        "nothing was applied, so nothing was rolled back: {error}",
    );
    assert_eq!(
        good.calls(),
        ["plan"],
        "the first target was planned and never applied",
    );
    assert!(
        !fixture.destination.path().join("bin/first").exists(),
        "the first target's destination is byte-absent",
    );
    assert!(
        !state_home.exists() || is_empty(&state_home),
        "no deployment state was written at all",
    );
}

/// Acceptance 5 / §6.3.0.10: "Duplicate owned identity always refuses",
/// through the shared Unicode-9 physical identity rather than by spelling.
///
/// `bin/Helper` and `bin/helper` are two spellings of one file on the hosts
/// this project supports. The refusal quotes BOTH exact spellings, because
/// an operator fixes this by editing one of them.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn two_targets_owning_one_physical_path_under_a_case_alias_refuse_before_apply() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["first", "second"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    let error = apply_selection(
        &execution,
        &[
            selected(
                &targets[0],
                Box::new(FixtureProvider::new(
                    fixture.destination.path(),
                    &["bin/Helper"],
                )),
            ),
            selected(
                &targets[1],
                Box::new(FixtureProvider::new(
                    fixture.destination.path(),
                    &["bin/helper"],
                )),
            ),
        ],
    )
    .expect_err("one physical file cannot have two owners");

    let DeployError::DuplicateOwnedResource {
        first,
        second,
        resource,
        alias,
    } = &error
    else {
        panic!("expected the owned-identity refusal, got: {error}");
    };
    assert_eq!(first, "first");
    assert_eq!(second, "second");
    assert_eq!(resource, "bin/Helper", "the exact spelling is retained");
    assert_eq!(alias, "bin/helper");
    assert!(!fixture.destination.path().join("bin/Helper").exists());
    assert!(!fixture.destination.path().join("bin/helper").exists());
}

/// Acceptance 6 / §6.3.0.10's exception, in both directions.
///
/// Two providers that BOTH declare reference ownership and own distinct
/// logical members of one document share its physical lock and deploy. The
/// same pair with one flag turned off refuses, and the refusal names the
/// participant that did not declare it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_shared_physical_lock_needs_reference_ownership_from_every_participant() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("codex-entry", "helper.exe", &[]),
        target("opencode-entry", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["codex-entry", "opencode-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    // Both declare it: one document, two distinct logical members.
    let outcomes = apply_selection(
        &execution,
        &[
            selected(&targets[0], Box::new(sharer(&fixture, "alpha"))),
            selected(&targets[1], Box::new(sharer(&fixture, "beta"))),
        ],
    )
    .expect("two reference owners share one document");
    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].resources[0].resource, "shared.json#alpha");
    assert_eq!(outcomes[1].resources[0].resource, "shared.json#beta");

    // One does not. The honest shape of that case is a provider that owns
    // the whole document outright — §6.3.0's own rejected alternative,
    // "treating a shared JSON file as one deployment's owned file … it
    // prevents unrelated plugins from coexisting". Its owned set and its
    // lock set are both `shared.json`, so it passes its own lock-set law
    // and is refused by the GROUP law, which names it.
    let outright = FixtureProvider::new(fixture.destination.path(), &["shared.json"]);
    let error = apply_selection(
        &execution,
        &[
            selected(&targets[0], Box::new(sharer(&fixture, "alpha"))),
            selected(&targets[1], Box::new(outright)),
        ],
    )
    .expect_err("one unreferenced participant refuses the group");

    let DeployError::SharedLockNotReferenced {
        first,
        second,
        resource,
        unreferenced,
        ..
    } = &error
    else {
        panic!("expected the shared-lock refusal, got: {error}");
    };
    assert_eq!(first, "codex-entry");
    assert_eq!(second, "opencode-entry");
    assert_eq!(resource, "shared.json");
    assert_eq!(
        unreferenced, "opencode-entry",
        "the refusal names the participant that did not declare it",
    );
}

/// The all-participants rule remains closed after an already-admitted pair.
/// A map implementation that forgets the shared identity after two reference
/// owners would incorrectly admit the third non-reference owner.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_third_shared_lock_participant_must_also_declare_reference_ownership() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("alpha", "helper.exe", &[]),
        target("beta", "helper.exe", &[]),
        target("gamma", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["alpha", "beta", "gamma"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    let error = apply_selection(
        &execution,
        &[
            selected(&targets[0], Box::new(sharer(&fixture, "alpha"))),
            selected(&targets[1], Box::new(sharer(&fixture, "beta"))),
            selected(
                &targets[2],
                Box::new(FixtureProvider::new(
                    fixture.destination.path(),
                    &["shared.json"],
                )),
            ),
        ],
    )
    .expect_err("every participant of one shared lock declares the capability");

    let DeployError::SharedLockNotReferenced {
        second,
        unreferenced,
        ..
    } = error
    else {
        panic!("expected the shared-lock group refusal, got: {error}");
    };
    assert_eq!(second, "gamma");
    assert_eq!(unreferenced, "gamma");
    assert!(
        !fixture.destination.path().join("shared.json").exists(),
        "the third-participant refusal happens before apply zero",
    );
}

/// §6.3.0.9: "A normal provider's lock resources equal its owned
/// resources." A provider that has not declared reference ownership and
/// hands back any other lock set is a defect, and it stops before apply.
///
/// Both directions are wrong for the same reason: a wider lock set
/// silently serialises unrelated deployments, and a narrower one reconciles
/// a destination nobody holds.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_provider_without_the_capability_cannot_shift_its_lock_set() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let shifted = FixtureProvider::new(fixture.destination.path(), &["shared.json#gamma"])
        .mislocking(&["shared.json"]);

    let error = apply_selection(&execution, &[selected(&targets[0], Box::new(shifted))])
        .expect_err("an undeclared provider locks exactly what it owns");

    let DeployError::LockSetNotDeclared {
        target,
        pin,
        owned,
        locked,
    } = &error
    else {
        panic!("expected the lock-set refusal, got: {error}");
    };
    assert_eq!(target, "local-helper");
    assert_eq!(pin, super::support::FIXTURE_PIN);
    assert_eq!(owned, "shared.json#gamma");
    assert_eq!(locked, "shared.json");
    assert!(
        !fixture.destination.path().join("shared.json").exists(),
        "the defect stopped before any destination byte",
    );
}

/// The other half of acceptance 6: two reference owners are still two
/// owners. A shared DOCUMENT is admitted; a shared logical MEMBER of it is
/// the unconditional duplicate-owned refusal, capability or not.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn two_reference_owners_claiming_one_logical_entry_still_refuse() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("codex-entry", "helper.exe", &[]),
        target("opencode-entry", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["codex-entry", "opencode-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);

    let error = apply_selection(
        &execution,
        &[
            selected(&targets[0], Box::new(sharer(&fixture, "alpha"))),
            selected(&targets[1], Box::new(sharer(&fixture, "alpha"))),
        ],
    )
    .expect_err("one logical member never has two owners");

    let DeployError::DuplicateOwnedResource { resource, .. } = &error else {
        panic!("expected the owned-identity refusal, got: {error}");
    };
    assert_eq!(resource, "shared.json#alpha");
}

/// Acceptance 7: "Intent/receipt resources remain the owned logical set;
/// lock resources never enter the generated wire."
///
/// The two reference owners above each locked `shared.json`, and neither
/// record mentions it: the intent's planned set and the receipt's owned set
/// are the logical members and nothing else.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn lock_resources_never_reach_an_intent_or_a_receipt() {
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

    let rows = super::list_deployments(&state_home).expect("the state home lists");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].resources, 1, "the one logical member, not the file");

    let recorded = std::fs::read_to_string(receipt_path(&state_home))
        .expect("the finalised receipt is readable");
    assert!(
        recorded.contains("shared.json#alpha"),
        "the owned logical member is recorded: {recorded}",
    );
    assert!(
        !recorded.contains("\"shared.json\""),
        "the physical lock is engine-internal and never reaches the wire: {recorded}",
    );
}

/// §7.0.6's read-only planner runs §6.3.0.10's judgement too, so it can
/// never report as deployable a profile apply would refuse.
///
/// A planner that reported two case-aliased owners as "planned" would be
/// promising work the engine is required to refuse — and an operator would
/// find out only after `vibe deploy` had already resolved, locked and begun.
/// The same function decides both, so the two answers cannot drift.
///
/// The refusal costs NOTHING: no destination byte, no receipt, no intent,
/// no deployment directory. `--plan` stays the read-only verb §7.0.6 says
/// it is, including when it says no.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_planner_refuses_an_alias_collision_it_would_otherwise_promise() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &[]),
    ];
    let chosen = selection("local", &["first", "second"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let resolved = [
        selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/Helper"],
            )),
        ),
        selected(
            &targets[1],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/helper"],
            )),
        ),
    ];

    let error = super::plan::plan_resolved(&execution, &resolved)
        .expect_err("a read-only plan may not promise what apply refuses");

    let DeployError::DuplicateOwnedResource {
        first,
        second,
        resource,
        alias,
    } = &error
    else {
        panic!("expected the owned-identity refusal, got: {error}");
    };
    assert_eq!(first, "first");
    assert_eq!(second, "second");
    assert_eq!(resource, "bin/Helper");
    assert_eq!(alias, "bin/helper");
    // The planner refused without touching anything at all.
    assert!(!fixture.destination.path().join("bin/Helper").exists());
    assert!(!fixture.destination.path().join("bin/helper").exists());
    assert!(
        !state_home.exists() || is_empty(&state_home),
        "a read-only planner writes no deployment state, including when it refuses",
    );
}

/// The ACROSS-RUNS half of the owned-identity law: a path a PRIOR
/// deployment's receipt already owns cannot be claimed by a later target
/// under an alias.
///
/// The first run owns `bin/Helper`. A second target claims `bin/helper` —
/// one physical file on the hosts this project supports — and §7.2's
/// collision refuses before that target's provider applies anything. The
/// evidence quotes BOTH exact spellings, because reconciling them is what
/// the operator has to do.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_prior_receipt_owns_a_path_under_every_alias_of_it() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [
        target("first", "helper.exe", &[]),
        target("second", "helper.exe", &[]),
    ];
    let state_home = fixture.state_home();

    // Run one: `first` deploys and its receipt records `bin/Helper`.
    let owned = selection("local", &["first"]);
    apply_selection(
        &fixture.execution(&targets, &owned, &state_home),
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/Helper"],
            )),
        )],
    )
    .expect("the first deployment applies");

    // Run two: a DIFFERENT target claims the same file under a case alias.
    let claimant = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));
    let later = selection("local", &["second"]);
    let error = apply_selection(
        &fixture.execution(&targets, &later, &state_home),
        &[selected(
            &targets[1],
            Box::new(Witness(Rc::clone(&claimant))),
        )],
    )
    .expect_err("a recorded deployment owns its path under every spelling of it");

    let DeployError::OwnershipCollision {
        target, resources, ..
    } = &error
    else {
        panic!("expected the foreign-ownership refusal, got: {error}");
    };
    assert_eq!(target, "second");
    assert!(
        resources.contains("bin/helper") && resources.contains("bin/Helper"),
        "both exact spellings survive into the evidence: {resources}",
    );
    assert_eq!(
        claimant.calls(),
        ["plan"],
        "the refusal landed before the second provider applied anything",
    );
}

/// §6.3.0.9's capability has no inverse yet, and `undeploy` says so rather
/// than taking the wrong lock.
///
/// A reference owner's receipt records its logical member; the physical
/// destination it held while editing lives only in the plan, which is gone
/// by undeploy time. Locking the logical member would take a lock no
/// sibling entry contends on, so two removals could edit one document at
/// once. The engine refuses, names the atom that must land the durable lock
/// ledger, and — provably — never reaches the provider's `remove`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_reference_owner_cannot_be_undeployed_until_the_lock_ledger_lands() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("codex-entry", "helper.exe", &[])];
    let chosen = selection("local", &["codex-entry"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let provider = Rc::new(sharer(&fixture, "alpha"));

    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(Witness(Rc::clone(&provider))),
        )],
    )
    .expect("the reference owner deploys");

    let error = super::undeploy_resolved(
        &execution,
        &[selected(
            &targets[0],
            Box::new(Witness(Rc::clone(&provider))),
        )],
    )
    .expect_err("a reference-owned deployment cannot be reversed yet");

    let DeployError::ReferenceOwnedRemovalNotLandable { target, pin } = &error else {
        panic!("expected the interim removal refusal, got: {error}");
    };
    assert_eq!(target, "codex-entry");
    assert_eq!(pin, super::support::FIXTURE_PIN);
    assert!(
        error.to_string().contains("R8-CLIENTS-DEPLOY"),
        "the refusal names the atom that must land the durable lock ledger: {error}",
    );
    assert!(
        !provider.calls().contains(&"remove".to_owned()),
        "the provider's `remove` was never reached: {:?}",
        provider.calls(),
    );
    assert!(
        fixture
            .destination
            .path()
            .join("shared.json#alpha")
            .exists(),
        "and the deployed member is still there",
    );
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

/// The one finalised receipt this state home holds.
fn receipt_path(state_home: &std::path::Path) -> std::path::PathBuf {
    let entry = std::fs::read_dir(state_home)
        .expect("the state home is readable")
        .flatten()
        .find(|entry| {
            entry.path().is_dir() && !entry.file_name().to_string_lossy().starts_with('.')
        })
        .expect("one deployment directory");
    entry.path().join("receipt.json")
}

/// Whether a directory holds no deployment — the lock directory the
/// safe-filesystem primitive creates is infrastructure, not state.
fn is_empty(root: &std::path::Path) -> bool {
    std::fs::read_dir(root).is_ok_and(|entries| {
        entries
            .flatten()
            .all(|entry| entry.file_name().to_string_lossy().starts_with('.'))
    })
}
