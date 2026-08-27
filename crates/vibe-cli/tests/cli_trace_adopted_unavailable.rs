//! R3.4 — RED 13: a parked traced run whose trace DIRECTORY is gone when the
//! resume adopts it.
//!
//! The lifecycle state is the proof of identity — it names the run and records
//! the sticky trace request — but the state never promised the directory still
//! exists. An operator (or a retention sweep) may have removed it while the
//! park stood. The resume then owes two honest answers at once: the
//! continuation is still serviced (the work is done, the delegation cleared)
//! and the trace is reported `unavailable` with the exact honest reason (see
//! [`UNAVAILABLE_REASON`]) — never a fresh partial history published as if it
//! were the run's whole one, and never a silent success that reads like the
//! trace never happened.
//!
//! Both halves of the install-family park are driven here — the scoped
//! `vibe update` park and the forced `vibe reinstall` park — compactly: the
//! full park/resume matrices live in the RED 8/12 binaries; what this file
//! proves is the deleted-directory seam.

mod common;
mod trace_support;

use std::path::Path;

use common::agent_provider::{MockProvider, configure_provider};
use common::hosted_slot::{
    PAID_RESULT, add_version, assert_ok, declare_static_tools, documents, lifecycle_state,
    project_at, publish_slot_agent_with_boot, write_declared_output,
};
use common::{UserScratch, git_available};
use serde_json::Value;
use trace_support::{run_directories, trace_dir, trace_member};
use vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

/// The exact honest reason an adopted run whose directory was deleted owes
/// the operator. This fixture PROVES the directory existed and was removed,
/// so the current product's "never opened" is false here; the owner will
/// centralise this successor wording, and this RED pins it exactly.
const UNAVAILABLE_REASON: &str = concat!(
    "this invocation adopted a parked run whose trace could not be reopened ",
    "because no existing trace directory was found, so it compiles untraced ",
    "rather than starting a partial mid-run history",
);

/// The untraced seed: a DECLARED, CLI-mode install, so the project holds a
/// locked, materialised world whose one agent row was legitimately paid for.
fn seed(user: &UserScratch, project: &Path) {
    let output = user
        .vibe()
        .args(["install", "--json", "--assume-yes"])
        .args(["--agent-mode", "cli"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert_ok(&output);
}

/// The identity facts the unavailable resume must be held to.
struct Parked {
    run_id: String,
    state_started: String,
    task: String,
}

/// Drive the traced park COMPACTLY: the full matrices are the RED 8/12
/// binaries' job. Here only what makes the deletion meaningful is proved —
/// the park really suspended one identified run with a live continuation.
fn assert_traced_park(project: &Path, output: &std::process::Output, command: &str) -> Parked {
    assert_ok(output);
    let docs = documents(&output.stdout);
    assert_eq!(
        docs.len(),
        1,
        "a park emits exactly one document — its registered root: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let report = &docs[0];
    assert_eq!(report["command"].as_str(), Some(command));
    let handoff = &report["delegation"];
    assert!(handoff.is_object(), "the slot row parked: {report}");
    let trace = trace_member(report).expect("a parked traced run reports its trace");
    assert_eq!(trace["status"], "running");
    assert_eq!(trace["finalised"], false);
    let run_id = trace["run_id"]
        .as_str()
        .expect("the member names its run")
        .to_string();
    assert_eq!(
        handoff["run_id"].as_str(),
        Some(run_id.as_str()),
        "the trace belongs to the lifecycle run that parked",
    );
    let state = lifecycle_state(project);
    assert_eq!(state.run.run_id.as_deref(), Some(run_id.as_str()));
    assert!(state.run.compile_trace, "the sticky bit is set");
    assert!(
        state.run.slot_continuation.is_some(),
        "the run still owes its continuation: {state:?}",
    );
    assert_eq!(
        run_directories(project),
        vec![run_id.clone()],
        "exactly one run directory exists to delete",
    );
    let tasks = handoff["tasks"].as_array().expect("the owed task list");
    assert_eq!(tasks.len(), 1, "exactly ONE task is owed: {report}");
    let task = tasks[0].as_str().expect("the task path").to_string();
    assert!(
        project.join(&task).is_file(),
        "the exact owed task exists before the trace directory is removed",
    );
    assert!(
        state
            .execution
            .values()
            .any(|row| row.status == ExecutionRecordStatus::Delegated),
        "the park has durable delegated debt before resume: {state:?}",
    );
    Parked {
        run_id,
        state_started: state.run.started,
        task,
    }
}

/// Delete ONLY the parked run's trace directory, and prove the two facts that
/// make the resume's answer honest: nothing 32-hex remains, while the sticky
/// lifecycle state and the continuation it owes both do.
fn delete_parked_trace(project: &Path, parked: &Parked) {
    let gone = trace_dir(project).join(&parked.run_id);
    assert!(
        gone.is_dir(),
        "the parked run's directory is there to delete"
    );
    std::fs::remove_dir_all(&gone).unwrap();
    assert!(
        run_directories(project).is_empty(),
        "zero 32-hex run directories remain after the deletion",
    );
    let state = lifecycle_state(project);
    assert!(
        state.run.compile_trace,
        "the sticky trace request survives the deletion: {state:?}",
    );
    assert!(
        state.run.slot_continuation.is_some(),
        "and so does the continuation the resume must still service",
    );
}

/// The ONE registered terminal root of a completed (or unavailable) stream,
/// with every other document in that stream accounted for.
///
/// A terminal stream may legitimately carry known supplementary documents —
/// `<command>:plan`, `<command>:closure-diff` — and only those. Anything else
/// beside the expected root is a SECOND report and fails here. The root must
/// also be the stream's LAST document: nothing reports after the terminal
/// report.
fn sole_terminal_root(bytes: &[u8], command: &str) -> Value {
    let docs = documents(bytes);
    let mut roots: Vec<(usize, &Value)> = Vec::new();
    let supplements: &[&str] = match command {
        "update" => &[
            "install:plan",
            "lifecycle:plan",
            "install:closure-diff",
            "update:closure-diff",
        ],
        "reinstall" => &["lifecycle:plan"],
        other => panic!("no supplementary-document contract for `{other}`"),
    };
    for (index, doc) in docs.iter().enumerate() {
        assert!(
            doc.get("run_id").is_none(),
            "a bare run id is not a report: {docs:?}",
        );
        let name = doc["command"].as_str().unwrap_or("");
        if !supplements.contains(&name) {
            roots.push((index, doc));
        }
    }
    let [(index, root)] = roots.as_slice() else {
        panic!("exactly one non-supplementary document — the registered root: {docs:?}");
    };
    assert_eq!(
        root["command"].as_str(),
        Some(command),
        "the terminal root is the expected command: {docs:?}",
    );
    assert_eq!(
        *index,
        docs.len() - 1,
        "the registered root is the LAST document in the stream: {docs:?}",
    );
    (*root).clone()
}

/// The adopted-unavailable matrix: the continuation is serviced, the member is
/// `unavailable` about the SAME run, and no fresh partial trace appears.
fn assert_unavailable_resume(
    project: &Path,
    output: &std::process::Output,
    command: &str,
    parked: &Parked,
    provider: &MockProvider,
    baseline: usize,
) {
    assert_ok(output);
    let root = sole_terminal_root(&output.stdout, command);
    assert!(
        root.get("delegation").is_none(),
        "the resume still serviced the continuation: {root}",
    );
    let trace = trace_member(&root).expect("the sticky bit still names a member");
    assert_eq!(
        trace["run_id"].as_str(),
        Some(parked.run_id.as_str()),
        "the member names the SAME adopted run",
    );
    assert_eq!(
        trace["status"], "unavailable",
        "an adopted run with no directory is unavailable — never `ok` over a history \
         that does not exist: {trace}",
    );
    assert_eq!(trace["finalised"], false);
    assert!(
        trace.get("run_path").is_none_or(|path| path.is_null()),
        "no path is named for a trace that cannot be opened: {trace}",
    );
    assert_eq!(trace["events"], "0", "counts are decimal zeroes");
    assert_eq!(trace["snapshots"], "0");
    assert_eq!(trace["snapshot_bytes"], "0");
    assert_eq!(
        trace["timings"].as_array().map(Vec::is_empty),
        Some(true),
        "no timings for a recorder that never opened",
    );
    assert_eq!(trace["budget_exhausted"], false);
    let warnings = trace["warnings"].as_array().expect("a warning list");
    assert_eq!(
        warnings.len(),
        1,
        "exactly ONE warning is owed by the unavailable owner: {trace}",
    );
    let exact = warnings[0].as_str().expect("the warning is text");
    assert_eq!(
        exact, UNAVAILABLE_REASON,
        "the honest reason is exact — no prefix, suffix, or paraphrase",
    );
    assert!(
        exact.len() <= DIAGNOSTIC_CAP_BYTES,
        "the reason is bounded by the shared diagnostic cap: {trace}",
    );

    let state = lifecycle_state(project);
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(parked.run_id.as_str()),
        "the state keeps the original run id",
    );
    assert_eq!(
        state.run.started, parked.state_started,
        "and its original start",
    );
    assert!(
        state.run.compile_trace,
        "the sticky bit survives an unavailable resume",
    );
    assert!(
        state.run.slot_continuation.is_none(),
        "the serviced continuation is cleared: {state:?}",
    );
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no delegated record survives the resume — clearing the continuation is not enough: \
         {state:?}",
    );
    assert!(
        !project.join(&parked.task).exists(),
        "the exact owed task file is gone",
    );
    assert!(
        !project
            .join(".vibe/agentic/outbox")
            .join(&parked.run_id)
            .exists(),
        "and its proven-empty run directory is pruned",
    );

    assert!(
        !trace_dir(project).join(&parked.run_id).exists(),
        "the deleted directory is NOT recreated",
    );
    assert!(
        run_directories(project).is_empty(),
        "and no fresh partial run was started: {:?}",
        run_directories(project),
    );
    assert_eq!(provider.hits(), baseline, "the resume pays nothing");
}

/// `vibe update org.demo/tools …` — the scoped grammar.
fn update_scoped(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    user.vibe()
        .args(["update", "org.demo/tools", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(extra)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

/// `vibe reinstall … <project>` — the path is POSITIONAL, reinstall's grammar.
fn reinstall(user: &UserScratch, project: &Path, extra: &[&str]) -> std::process::Output {
    user.vibe()
        .args(["reinstall", "--json", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .args(extra)
        .arg(project)
        .output()
        .unwrap()
}

/// The scoped traced update parks; its run directory vanishes; the flagless
/// resume services the continuation and reports the run `unavailable`.
#[test]
fn an_update_resume_over_a_deleted_trace_reports_the_run_unavailable() {
    if !git_available() {
        eprintln!("skipping hosted adopted-unavailable e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent_with_boot(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_static_tools(project.path());
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();
    add_version(&published, "slot:post-install", "0.1.1");

    let parked = assert_traced_park(
        project.path(),
        &update_scoped(&user, project.path(), &["--trace-compile"]),
        "update",
    );
    delete_parked_trace(project.path(), &parked);

    write_declared_output(project.path());
    assert_unavailable_resume(
        project.path(),
        &update_scoped(&user, project.path(), &[]),
        "update",
        &parked,
        &provider,
        baseline,
    );
}

/// The forced traced reinstall parks; same deletion; the PLAIN base verb
/// resumes and the run is reported `unavailable`, not re-recorded.
#[test]
fn a_reinstall_resume_over_a_deleted_trace_reports_the_run_unavailable() {
    if !git_available() {
        eprintln!("skipping hosted adopted-unavailable e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(PAID_RESULT);
    let outer = tempfile::tempdir().unwrap();
    let published = publish_slot_agent_with_boot(outer.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    declare_static_tools(project.path());
    configure_provider(&user, &provider.endpoint());
    seed(&user, project.path());
    let baseline = provider.hits();
    // The payload must differ from source for `--force` to re-materialise —
    // the corruption the hosted reinstall family relies on.
    std::fs::write(
        project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools")
            .join("0.1.0")
            .join("payload.txt"),
        "corrupted\n",
    )
    .unwrap();

    let parked = assert_traced_park(
        project.path(),
        &reinstall(&user, project.path(), &["--force", "--trace-compile"]),
        "reinstall",
    );
    delete_parked_trace(project.path(), &parked);

    write_declared_output(project.path());
    assert_unavailable_resume(
        project.path(),
        &reinstall(&user, project.path(), &[]),
        "reinstall",
        &parked,
        &provider,
        baseline,
    );
}
