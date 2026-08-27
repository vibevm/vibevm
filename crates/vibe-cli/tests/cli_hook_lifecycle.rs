//! `[hooks]` compatibility sugar at the production install boundary.
//!
//! The sugar half of the family: `pre-install` / `post-install` timing, a
//! pre-install failure rolling the slot back, and a non-zero post-install
//! flagged after the install is already durable. The explicit-slot half —
//! contributions declared without sugar — is `cli_slot_contributions.rs`;
//! both drive the same engine through the shared fixture.

mod common;

use std::fs;

use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use common::hook_slot::{
    Fixture, documents, install, lifecycle_state, setup, slot_outcomes, state_key,
};

#[test]
fn hook_sugar_runs_once_at_pre_and_post_install_timing() {
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let output = install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let slot = project
        .path()
        .join(common::slot_dir("org.example.hooked", "0.1.0"));
    assert_eq!(
        fs::read_to_string(slot.join("hook-order.txt"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["pre", "post"],
    );
    let generated_lane = project.path().join(common::index_rel());
    assert!(
        fs::read_to_string(generated_lane)
            .unwrap()
            .contains("generated.md"),
        "pre hook must create its declared boot source before boot regeneration",
    );
    let docs = documents(&output.stdout);
    let plan_index = docs
        .iter()
        .position(|doc| {
            doc["command"] == "lifecycle:plan"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["point"] == "slot:pre-install"))
        })
        .expect("slot ritual must be surfaced before execution");
    // The slot outcomes now live on the OUTERMOST command's sole root report,
    // which is the last document: the per-row echo that used to sit between
    // the plan and the report was removed so parking can emit one document.
    let outcome_index = docs
        .iter()
        .position(|doc| {
            doc["command"] == "install"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["point"] == "slot:pre-install"))
        })
        .expect("slot outcomes ride the generated install report");
    let planned = docs[plan_index]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["point"] == "slot:pre-install")
        .unwrap();
    assert_eq!(
        planned["reference"],
        "org.example/hooked#@vibe/hooks/pre-install"
    );
    assert_eq!(planned["slot_target"]["name"], "hooked");
    assert!(
        plan_index < outcome_index,
        "the plan is surfaced before the outcomes it previews",
    );
    assert_eq!(outcome_index, docs.len() - 1, "the root report is last");
    assert_eq!(docs.last().unwrap()["command"], "install");
    assert!(docs.last().unwrap().get("lifecycle_hooks").is_none());
    let hooks = slot_outcomes(&docs);
    assert_eq!(
        hooks.len(),
        2,
        "pre/post sugar must each execute exactly once"
    );
    assert_eq!(hooks[0]["point"], "slot:pre-install");
    assert_eq!(hooks[1]["point"], "slot:post-install");
    let state = lifecycle_state(project.path());
    assert_eq!(
        state.execution[&state_key("@vibe/hooks/pre-install")].status,
        ExecutionRecordStatus::Ok,
    );
    assert_eq!(
        state.execution[&state_key("@vibe/hooks/post-install")].status,
        ExecutionRecordStatus::Ok,
    );

    let second = install(&user, project.path(), &registry);
    assert!(second.status.success());
    assert_eq!(
        fs::read_to_string(slot.join("hook-order.txt"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["pre", "post"],
    );
}

#[test]
fn pre_failure_aborts_and_rolls_back_the_slot() {
    let (_outer, user, project, registry) = setup(Fixture::PreFail);
    let lock_before = fs::read(project.path().join("vibe.lock")).unwrap();
    let output = install(&user, project.path(), &registry);
    assert!(!output.status.success());
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists(),
        "failed pre hook must roll the materialised slot back",
    );
    assert_eq!(
        fs::read(project.path().join("vibe.lock")).unwrap(),
        lock_before,
        "failed pre hook must not register the package in the lockfile",
    );
    assert_eq!(
        lifecycle_state(project.path()).execution[&state_key("@vibe/hooks/pre-install")].status,
        ExecutionRecordStatus::Fail,
    );
    let docs = documents(&output.stdout);
    assert_eq!(docs[0]["command"], "install:plan");
    let slot_plan = docs
        .iter()
        .position(|doc| doc["command"] == "lifecycle:plan")
        .unwrap();
    // A failure is an outcome, and the outermost command reports it in its
    // OWN root: one `cli-install-report` with `ok: false` carrying the failed
    // row, rather than a per-row `lifecycle` echo.
    let failure = docs
        .iter()
        .position(|doc| doc["command"] == "install")
        .expect("the failed install still emits its one root document");
    assert!(slot_plan < failure);
    assert_eq!(docs[failure]["ok"], false);
    assert_eq!(docs[failure]["contributions"][0]["status"], "fail");
    assert_eq!(
        docs[failure]["contributions"][0]["point"],
        "slot:pre-install",
    );
    assert!(
        !output.stderr.is_empty(),
        "terminal error follows failure outcome"
    );
}

#[test]
fn post_nonzero_is_flagged_after_the_install_is_durable() {
    let (_outer, user, project, registry) = setup(Fixture::PostFail);
    let output = install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(project.path().join("vibe.lock").is_file());
    assert!(
        project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .is_dir(),
    );
    let docs = documents(&output.stdout);
    let hooks = slot_outcomes(&docs);
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["point"], "slot:post-install");
    assert_eq!(hooks[0]["status"], "fail");
    assert_eq!(hooks[0]["flagged"], true);
    assert!(hooks[0]["stdout"].as_str().unwrap().contains("SOFT-STDOUT"));
    assert!(hooks[0]["stderr"].as_str().unwrap().contains("SOFT-STDERR"));
    assert_eq!(
        lifecycle_state(project.path()).execution[&state_key("@vibe/hooks/post-install")].status,
        ExecutionRecordStatus::Fail,
    );
}
