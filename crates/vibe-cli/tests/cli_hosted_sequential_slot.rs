//! TWO ordered slot `agent` rows against ONE slot target, parked and
//! satisfied in sequence.
//!
//! The interesting instant is between them. Row A is satisfied and
//! checkpoints `ok`; at that moment no delegated slot row is live, so the
//! durable continuation is correctly dropped — a continuation nothing owes is
//! exactly what the state invariant refuses to read back. Row B then parks in
//! the SAME pass and has to name the same ordered payload-event target set
//! again. It can only do that if the invocation staged that set, which is why
//! every slot-plan construction stages unconditionally rather than declining
//! whenever a durable continuation already exists.
//!
//! The hit-counting loopback provider is configured and reachable throughout,
//! so a fall-through to the paid path shows up as a counter rather than as
//! "no provider configured".

mod common;

use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, assert_ok, lifecycle_state, project_at, publish_two_slot_agents, sole_document,
    sole_root, write_declared_output, write_second_declared_output,
};
use common::{UserScratch, git_available};
use vibe_wire::generated::install_report::InstallReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordScope, ExecutionRecordStatus};

/// The FIRST invocation, which names the package explicitly. An explicit
/// pkgref always runs the full pipeline, so this is the shape that resolves,
/// materialises and writes the lock.
fn install_pkgref(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .args(["install", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// The RESUME, which is exactly the command the handoff names: bare
/// `vibe install`, install-from-manifest, whose fresh lock takes the fast
/// path — so the parked slot run is rebuilt from the persisted continuation
/// rather than re-derived from a materialise pass that will not happen.
fn resume(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .args(["install", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// The live slot-scoped delegated key, and the continuation the run records.
fn owed(project: &Path) -> (Option<String>, Option<Vec<String>>) {
    let state = lifecycle_state(project);
    let key = state
        .execution
        .iter()
        .find(|(_, row)| {
            row.status == ExecutionRecordStatus::Delegated
                && row.scope == Some(ExecutionRecordScope::Slot)
        })
        .map(|(key, _)| key.clone());
    let continuation = state.run.slot_continuation.as_ref().map(|continuation| {
        continuation
            .targets
            .iter()
            .map(|target| format!("{}/{}@{}", target.group, target.name, target.version))
            .collect()
    });
    (key, continuation)
}

#[test]
fn two_ordered_slot_rows_park_and_resume_in_sequence_without_losing_the_target_set() {
    if !git_available() {
        eprintln!("skipping sequential hosted slot e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_two_slot_agents(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    configure_provider(&user, &provider.endpoint());
    let _ = &published;

    // ── 1. First invocation parks row A ────────────────────────────────────
    let first = install_pkgref(&user, project.path());
    assert_ok(&first);
    let report: InstallReport = serde_json::from_value(sole_document(&first.stdout)).unwrap();
    let handoff = report.delegation.as_ref().expect("row A parked");
    let run_id = handoff.run_id.clone();
    let task_a = handoff.tasks[0].clone();
    assert!(
        task_a.contains("slot-produce.md") || task_a.contains("slot-produce%40"),
        "the published task is row A's: {task_a}",
    );
    assert!(project.path().join(&task_a).is_file());
    assert_eq!(
        handoff.resume, "vibe install",
        "and the resume line names the bare command the steps below actually run",
    );
    let (key_a, continuation) = owed(project.path());
    let key_a = key_a.expect("a slot-scoped park is live");
    assert!(key_a.contains("#slot-produce@"), "{key_a}");
    let targets = continuation.expect("and the run records what it owes");
    assert_eq!(targets, vec!["org.demo/tools@0.1.0".to_string()]);
    assert_eq!(provider.hits(), 0, "parking never reaches the provider");

    // ── 2. Satisfy A. The SAME invocation must checkpoint A `ok` and then
    //       park row B — crossing the instant where nothing is owed. ────────
    write_declared_output(project.path());
    let second = resume(&user, project.path());
    assert_ok(&second);
    assert_eq!(provider.hits(), 0, "the resume never pays for A");
    let report: InstallReport = serde_json::from_value(sole_document(&second.stdout)).unwrap();
    let handoff = report.delegation.as_ref().expect("row B parked in turn");
    assert_eq!(
        handoff.run_id, run_id,
        "row B parks under the ORIGINAL run identity, not a fresh one",
    );
    let task_b = handoff.tasks[0].clone();
    assert_ne!(task_b, task_a, "and it is a different declared task");
    assert!(project.path().join(&task_b).is_file());

    let state = lifecycle_state(project.path());
    assert_eq!(
        state.execution[&key_a].status,
        ExecutionRecordStatus::Ok,
        "row A really was satisfied on the way through: {state:?}",
    );
    assert!(
        state.execution[&key_a].tasks.is_empty(),
        "a satisfied row carries no task",
    );
    let (key_b, continuation) = owed(project.path());
    let key_b = key_b.expect("row B is the live park now");
    assert!(key_b.contains("#slot-produce-second@"), "{key_b}");
    assert_eq!(
        continuation.expect("and the EXACT target set is durable again"),
        targets,
        "the zero-debt instant between A and B did not erase it",
    );
    assert!(
        !project.path().join(&task_a).exists(),
        "A's task was cleaned up when A was satisfied",
    );

    // ── 3. Satisfy B. Nothing remains owed and the run completes. ──────────
    write_second_declared_output(project.path());
    let third = resume(&user, project.path());
    assert_ok(&third);
    assert_eq!(provider.hits(), 0, "and B is never paid for either");
    let report: InstallReport =
        serde_json::from_value(sole_root(&third.stdout, "install")).unwrap();
    assert!(report.delegation.is_none(), "nothing is parked: {report:?}");
    assert!(report.complete, "so the command completes: {report:?}");

    let state = lifecycle_state(project.path());
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no delegated row survives: {state:?}",
    );
    assert!(
        state.run.slot_continuation.is_none(),
        "and the continuation is finally cleared: {state:?}",
    );
    assert!(!project.path().join(&task_b).exists(), "B's task is gone");
    assert!(
        !project
            .path()
            .join(".vibe/agentic/outbox")
            .join(&run_id)
            .exists(),
        "and the proven-empty run directory is pruned",
    );
    // Downstream proceeded: the builtin sentinel declared AFTER both agent
    // rows only runs once neither is parked.
    assert!(
        String::from_utf8_lossy(&third.stdout).contains("SENTINEL-AFTER-SLOT-AGENT"),
        "the row after the two handoffs finally ran: {}",
        String::from_utf8_lossy(&third.stdout),
    );
}
