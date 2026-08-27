//! `vibe update` and `vibe reinstall` under a HOSTING agent.
//!
//! Both run on the install substrate, and both are their OWN commands: each
//! reports its own registered root, its own run identity and a resume line
//! that names a command able to service the continuation. The same
//! hit-counting loopback provider the paid slot e2e uses is configured and
//! reachable in every case, so a fall-through to the paid path would be caught
//! by the counter rather than masked by "no provider configured".

mod common;

use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, add_version, assert_ok, documents, lifecycle_state, project_at,
    publish_slot_agent, sole_document, sole_root, write_declared_output,
};
use common::{UserScratch, git_available};
use vibe_wire::generated::lifecycle_state::{ExecutionRecordScope, ExecutionRecordStatus};
use vibe_wire::generated::reinstall_report::ReinstallReport;
use vibe_wire::generated::update_report::{UpdateReport, UpdateReportScope};

/// Install `org.demo/tools` in CLI mode so the project has a locked, already
/// materialised world for update/reinstall to work against.
fn seed(user: &UserScratch, project: &Path) {
    let output = user
        .vibe()
        .args(["install", "org.demo/tools", "--assume-yes", "--json"])
        .args(["--agent-mode", "cli"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_ok(&output);
}

fn assert_parked_row(project: &Path) {
    let state = lifecycle_state(project);
    assert!(
        state.execution.values().any(|row| {
            row.status == ExecutionRecordStatus::Delegated
                && row.scope == Some(ExecutionRecordScope::Slot)
        }),
        "the park is tagged with its typed slot scope: {state:?}",
    );
    assert!(
        state.run.slot_continuation.is_some(),
        "and the run records exactly what it owes: {state:?}",
    );
}

fn assert_settled(project: &Path, run_id: &str, task: &str) {
    let state = lifecycle_state(project);
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(run_id),
        "the resume ran under the ORIGINAL run id: {state:?}",
    );
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no delegated record survives the resume: {state:?}",
    );
    assert!(
        state.run.slot_continuation.is_none(),
        "and the continuation is cleared: {state:?}",
    );
    assert!(!project.join(task).exists(), "the exact owned task is gone");
    assert!(
        !project.join(".vibe/agentic/outbox").join(run_id).exists(),
        "and its proven-empty run directory is pruned",
    );
}

/// A SCOPED `vibe update` parks, and the same command resumes it. The report
/// is an `update` report throughout — it never impersonates install.
#[test]
fn a_scoped_update_parks_and_the_same_command_resumes_it() {
    if !git_available() {
        eprintln!("skipping hosted update e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    // The seed runs in CLI mode and legitimately pays for its own row. Every
    // assertion below is about ADDITIONAL spend: a hosted invocation must add
    // nothing to this baseline.
    let baseline = provider.hits();
    add_version(&published, "slot:post-install", "0.1.1");

    let update = |user: &UserScratch| {
        user.vibe()
            .args(["update", "org.demo/tools", "--json", "--assume-yes"])
            .args(["--agent-mode", "agent"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    };

    let parked = update(&user);
    assert_ok(&parked);
    let report: UpdateReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    assert_eq!(report.command, "update", "update reports UPDATE");
    assert_eq!(report.scope, UpdateReportScope::Scoped);
    assert_eq!(report.packages, ["org.demo/tools"]);
    let handoff = report.delegation.expect("the slot row parked");
    assert_eq!(
        handoff.resume, "vibe update",
        "the resume line names the command the operator actually ran",
    );
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();
    assert!(project.path().join(&task).is_file());
    assert_eq!(provider.hits(), baseline, "parking adds no spend");
    assert_parked_row(project.path());
    assert!(
        !documents(&parked.stdout)[0]
            .to_string()
            .contains("SENTINEL-AFTER-SLOT-AGENT"),
        "the post-barrier sentinel did not run",
    );

    write_declared_output(project.path());
    let resumed = update(&user);
    assert_ok(&resumed);
    assert_eq!(provider.hits(), baseline, "and neither does the resume");
    let resumed_report: UpdateReport =
        serde_json::from_value(sole_root(&resumed.stdout, "update")).unwrap();
    assert!(
        resumed_report.delegation.is_none(),
        "the resume satisfied the park: {resumed_report:?}",
    );
    assert_settled(project.path(), &run_id, &task);
}

/// A WHOLE `vibe update` runs the install substrate but keeps its own
/// identity, report and resume line.
#[test]
fn a_whole_update_parks_as_update_not_install_and_resumes() {
    if !git_available() {
        eprintln!("skipping hosted update e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    // The seed runs in CLI mode and legitimately pays for its own row. Every
    // assertion below is about ADDITIONAL spend: a hosted invocation must add
    // nothing to this baseline.
    let baseline = provider.hits();
    add_version(&published, "slot:post-install", "0.1.1");

    // A whole update delegates to install-from-manifest, which is freshness
    // based: with the lock still satisfying `[requires]`, nothing re-resolves
    // and no payload event fires. Removing the slot is what makes this run a
    // real materialisation, which is what reaches the hosted row.
    std::fs::remove_dir_all(
        project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools"),
    )
    .unwrap();

    let update = |user: &UserScratch| {
        user.vibe()
            .args(["update", "--all", "--json", "--assume-yes"])
            .args(["--agent-mode", "agent"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    };

    let parked = update(&user);
    assert_ok(&parked);
    let report: UpdateReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    assert_eq!(report.command, "update");
    assert_eq!(report.scope, UpdateReportScope::All);
    let handoff = report.delegation.expect("the slot row parked");
    assert_eq!(
        handoff.resume, "vibe update",
        "a whole update never impersonates install in its handoff",
    );
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();
    assert_eq!(provider.hits(), baseline);
    assert_parked_row(project.path());

    write_declared_output(project.path());
    let resumed = update(&user);
    assert_ok(&resumed);
    assert_eq!(provider.hits(), baseline);
    let resumed_report: UpdateReport =
        serde_json::from_value(sole_root(&resumed.stdout, "update")).unwrap();
    assert!(resumed_report.delegation.is_none());
    assert_settled(project.path(), &run_id, &task);
}

/// `vibe reinstall --force` is the mode that reaches changed slot callbacks,
/// so it is the mode that parks. Its handoff names the base verb, and the base
/// verb is what services the continuation — materialisation force and hosted
/// repark are separate concepts.
#[test]
fn a_forced_reinstall_parks_and_the_base_verb_services_the_continuation() {
    if !git_available() {
        eprintln!("skipping hosted reinstall e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();
    // A slot whose payload did not change raises no post-install event, so
    // nothing would reach the hosted row. Corrupt it, exactly as the paid
    // slot-agent e2e does, so `--force` really re-materialises.
    std::fs::write(
        project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools")
            .join("0.1.0")
            .join("payload.txt"),
        "corrupted
",
    )
    .unwrap();

    let parked = user
        .vibe()
        .args(["reinstall", "--force", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&parked);
    let report: ReinstallReport = serde_json::from_value(sole_document(&parked.stdout)).unwrap();
    assert_eq!(report.command, "reinstall");
    assert!(report.forced, "the forced mode is what parked");
    let handoff = report.delegation.expect("the slot row parked");
    let run_id = handoff.run_id.clone();
    let task = handoff.tasks[0].clone();
    assert!(project.path().join(&task).is_file());
    assert_eq!(provider.hits(), baseline, "parking adds no spend");
    assert_parked_row(project.path());

    // The base verb — the one the handoff can actually be resumed by.
    write_declared_output(project.path());
    let resumed = user
        .vibe()
        .args(["reinstall", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&resumed);
    assert_eq!(
        provider.hits(),
        baseline,
        "and the resume pays nothing either"
    );
    let resumed_report: ReinstallReport =
        serde_json::from_value(sole_root(&resumed.stdout, "reinstall")).unwrap();
    assert!(
        resumed_report.delegation.is_none(),
        "the base verb SERVICED the continuation: {resumed_report:?}",
    );
    assert!(
        !resumed_report.forced,
        "and it did so without re-fetching from source",
    );
    assert_settled(project.path(), &run_id, &task);
}
