//! The hosted engine branch: park, satisfied resume, honest repark.
//!
//! Every case drives the REAL `LifecycleRun::execute_one` in resolved agent
//! mode and asserts the paid backend's counter — "parks and calls the
//! provider zero times" is a measurement, not a claim.

use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle::e1::context::{Project, RunAgentMode, World};
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

use std::sync::Arc;

use super::LifecycleRun;
use crate::ExecutionReuse;
use crate::LifecycleLease;
use crate::agent::tests::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, row};
use crate::execution::HandlerExecution;
use crate::handlers::{HandlerRuntime, NoPackageBindingBackend};
use crate::process::{StreamMode, SystemProcessRunner};
use crate::{RunMetadata, select_run_identity};
use vibe_workspace::hooks::SystemProbe;

/// The sticky-trace half of the hosted branch — the one seam that needs
/// an explicit metadata value rather than this file's `(run_id, force)`
/// fixture, kept beside it rather than inside it (600-line cell budget).
#[cfg(test)]
#[path = "tests/trace_sticky.rs"]
mod trace_sticky;

/// The selected-ownership proof across the real engine — same cell-budget
/// split, same reason.
#[cfg(test)]
#[path = "tests/selected_owner.rs"]
mod selected_owner;

/// The input-measurement carriage reds (R7.5 P2/A4b) — same cell-budget
/// split, same reason.
#[cfg(test)]
#[path = "tests/measurement.rs"]
mod measurement;

/// The artifact-witness carriage and hosted-convergence reds (R7.5 P2/A4c1).
#[cfg(test)]
#[path = "tests/artifacts.rs"]
mod artifacts;

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const KEY: &str = "org.demo/tools#produce";

struct Scratch {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

/// One real temp lease over the scratch root — the single-writer proof every
/// tracked run construction in these tests must now carry. Sequential
/// acquisitions (a run dropped, then another begun) release and re-acquire;
/// two LIVE runs on one root were never a legal shape and now refuse.
fn lease(root: &Path) -> Arc<LifecycleLease> {
    Arc::new(LifecycleLease::acquire(root).expect("a temp root is leasable"))
}

fn scratch() -> Scratch {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    Scratch {
        root: dir.path().to_path_buf(),
        _dir: dir,
    }
}

fn metadata(run_id: &str, force: bool) -> RunMetadata {
    metadata_for_node(run_id, force, ".")
}

/// The metadata builder that names WHICH workspace node runs — the seam the
/// selected-ownership RED needs (single-node fixtures keep `"."`).
fn metadata_for_node(run_id: &str, force: bool, selected: &str) -> RunMetadata {
    RunMetadata {
        requested: "create".into(),
        chain: vec!["validate".into(), "install".into(), "create".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Agent,
        force,
        trace_compile: false,
        run_id: run_id.into(),
        started: "2026-08-26T00:00:00Z".into(),
        selected: selected.into(),
    }
}

fn runtime(backend: &RecordingBackend) -> HandlerRuntime<'_> {
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    HandlerRuntime {
        process: &PROCESS,
        binary: &crate::handlers::NoBinaryBackend,
        native: &crate::handlers::NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: backend,
        probe: &SystemProbe,
        streams: StreamMode::Capture,
    }
}

fn execute(
    root: &Path,
    backend: &RecordingBackend,
    run_id: &str,
    force: bool,
) -> super::ExecutionTransition {
    execute_result(root, backend, run_id, force).expect("the hosted transition completes")
}

fn execute_result(
    root: &Path,
    backend: &RecordingBackend,
    run_id: &str,
    force: bool,
) -> Result<super::ExecutionTransition, crate::LifecycleRunError> {
    transition(root, backend, metadata(run_id, force))
}

/// The same hosted transition driven by an EXPLICIT metadata value — the
/// seam the trace-sticky RED needs, because what it pins is precisely
/// which bit the metadata carried into `LifecycleRun::begin`.
fn execute_with(
    root: &Path,
    backend: &RecordingBackend,
    metadata: RunMetadata,
) -> super::ExecutionTransition {
    transition(root, backend, metadata).expect("the hosted transition completes")
}

fn transition(
    root: &Path,
    backend: &RecordingBackend,
    metadata: RunMetadata,
) -> Result<super::ExecutionTransition, crate::LifecycleRunError> {
    let row = row(TWO_OUTPUTS, PROMPT);
    let mut run = LifecycleRun::begin(
        lease(root),
        project_fixture(root),
        world_fixture(root),
        metadata,
        vec!["validate".into(), "install".into(), "create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    let rt = runtime(backend);
    run.execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
}

fn project_fixture(root: &Path) -> Project {
    let text = root.to_string_lossy().replace('\\', "/");
    Project {
        kind: "project".into(),
        manifest: format!("{text}/vibe.toml"),
        name: "demo".into(),
        root: text,
        spec_roots: Vec::new(),
        version: "0.1.0".into(),
    }
}

fn world_fixture(root: &Path) -> World {
    let text = root.to_string_lossy().replace('\\', "/");
    World {
        deps_root: format!("{text}/vibedeps"),
        lockfile: format!("{text}/vibe.lock"),
        packages: Vec::new(),
    }
}

fn state(root: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(root.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

/// Put a hand-edited record back, so a test can stage a state shape only an
/// OLDER writer could have produced — a pre-R7.5 row with no witness, say.
fn write_state(root: &Path, state: &LifecycleState) {
    fs::write(
        root.join(".vibe/lifecycle.toml"),
        toml::to_string(state).unwrap(),
    )
    .unwrap();
}

fn write_declared_outputs(root: &Path) {
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/guide.md"), "hosted body\n").unwrap();
    fs::write(root.join("docs/reference.md"), "hosted reference\n").unwrap();
}

/// A first hosted invocation parks: task published, delegated row
/// checkpointed with the exact planned rows, and ZERO provider calls.
#[test]
fn a_first_hosted_invocation_parks_without_spend() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let transition = execute(scratch.root.as_path(), &backend, RUN_ID, false);

    assert_eq!(transition.status, ExecutionRecordStatus::Delegated);
    let handoff = transition.delegation.as_ref().expect("a typed handoff");
    assert_eq!(handoff.run_id, RUN_ID);
    assert_eq!(handoff.resume, "vibe create");
    assert_eq!(handoff.tasks.len(), 1);
    assert!(handoff.tasks[0].starts_with(".vibe/agentic/outbox/"));
    assert!(scratch.root.join(&handoff.tasks[0]).is_file());

    let state = state(scratch.root.as_path());
    assert_eq!(state.run.run_id.as_deref(), Some(RUN_ID));
    let row = &state.execution[KEY];
    assert_eq!(row.status, ExecutionRecordStatus::Delegated);
    assert_eq!(row.tasks.len(), 1);
    assert_eq!(row.tasks[0], handoff.tasks[0]);
    assert_eq!(
        row.artifacts.len(),
        2,
        "the exact planned rows are recorded"
    );
    assert_eq!(row.artifacts[0].id, "docs/guide.md");
    assert_eq!(
        backend.calls(),
        0,
        "parking is engine-owned and never reaches the provider"
    );
}

/// Coincidental pre-existing outputs on a FIRST invocation do not satisfy:
/// only a prior delegated record for this execution with this exact
/// fingerprint may be satisfied by existing outputs.
#[test]
fn coincidental_pre_existing_outputs_do_not_satisfy_a_first_invocation() {
    let scratch = scratch();
    write_declared_outputs(scratch.root.as_path());
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let transition = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    assert_eq!(
        transition.status,
        ExecutionRecordStatus::Delegated,
        "no prior delegated record exists, so the outputs are not evidence"
    );
    assert_eq!(backend.calls(), 0);
}

/// Park twice unsatisfied: the SAME run and task, atomically replaced.
#[test]
fn parking_twice_unatisfied_keeps_the_same_run_and_task() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let first = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    let second = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    assert_eq!(first.delegation, second.delegation);
    let state = state(scratch.root.as_path());
    assert_eq!(state.run.run_id.as_deref(), Some(RUN_ID));
    let run_dir = scratch.root.join(".vibe/agentic/outbox").join(RUN_ID);
    let entries: Vec<_> = fs::read_dir(&run_dir).unwrap().collect();
    assert_eq!(entries.len(), 1, "one task file, atomically replaced");
    assert_eq!(backend.calls(), 0);
}

/// The packet's headline flow: park, hosting agent performs the task, the
/// SAME phase resumes — `ok`, zero provider calls, the owned task removed,
/// the run directory pruned.
#[test]
fn a_satisfied_same_phase_resume_marks_ok_removes_the_task_and_spends_nothing() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let parked = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    assert_eq!(parked.status, ExecutionRecordStatus::Delegated);

    write_declared_outputs(scratch.root.as_path());
    let resumed = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    assert_eq!(resumed.status, ExecutionRecordStatus::Ok);
    assert!(resumed.delegation.is_none());
    assert_eq!(
        resumed.artifacts.len(),
        2,
        "exactly those artifacts hydrate"
    );
    let message = resumed.message.as_deref().unwrap_or_default();
    assert!(message.contains("no provider spend"), "{message}");
    assert_eq!(backend.calls(), 0, "the resume never pays");

    let task = parked.delegation.unwrap().tasks.pop().unwrap();
    assert!(
        !scratch.root.join(&task).exists(),
        "the owned task is removed"
    );
    assert!(
        !scratch
            .root
            .join(".vibe/agentic/outbox")
            .join(RUN_ID)
            .exists(),
        "the proven-empty run directory is pruned"
    );
    let state = state(scratch.root.as_path());
    assert_eq!(state.execution[KEY].status, ExecutionRecordStatus::Ok);
    assert!(
        state.execution[KEY].tasks.is_empty(),
        "a satisfied row carries no tasks"
    );
}

/// A changed prompt changes the fingerprint and reparks honestly: the old
/// task is replaced under the SAME run identity, never treated as satisfied.
#[test]
fn a_changed_prompt_reparks_under_the_same_run() {
    let scratch = scratch();
    let first_backend = RecordingBackend::answering_prompt("Write v1.", r#"{"outputs":[]}"#);
    let parked = execute(scratch.root.as_path(), &first_backend, RUN_ID, false);
    assert_eq!(parked.status, ExecutionRecordStatus::Delegated);

    write_declared_outputs(scratch.root.as_path());
    let changed_backend = RecordingBackend::answering_prompt("Write v2.", r#"{"outputs":[]}"#);
    let reparked = execute(scratch.root.as_path(), &changed_backend, RUN_ID, false);
    assert_eq!(
        reparked.status,
        ExecutionRecordStatus::Delegated,
        "a changed fingerprint may not accept the old outputs"
    );
    assert_eq!(reparked.delegation.as_ref().unwrap().run_id, RUN_ID);
    assert_eq!(
        reparked.delegation.as_ref().unwrap().tasks,
        parked.delegation.as_ref().unwrap().tasks,
        "the deterministic task path is rewritten, not multiplied"
    );
    assert_eq!(changed_backend.calls(), 0);
}

/// `--force` always reparks under a FRESH run without probing — satisfied
/// outputs on disk do not short-circuit it.
#[test]
fn force_reparks_under_a_fresh_run_without_probing() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let parked = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    write_declared_outputs(scratch.root.as_path());

    // The lease scopes to the selection: the next `execute` below begins its
    // own run, and one workspace admits one live writer.
    let fresh = {
        let lease = lease(scratch.root.as_path());
        select_run_identity(
            &lease,
            scratch.root.as_path(),
            "create",
            &["validate".into(), "install".into(), "create".into()],
            ".",
            RunAgentMode::Agent,
            true,
            false,
            "2026-08-26T01:00:00Z".into(),
        )
        .unwrap()
    };
    assert!(!fresh.adopted, "force never inherits the parked identity");
    assert_ne!(fresh.run_id, RUN_ID);

    let forced = execute(scratch.root.as_path(), &backend, &fresh.run_id, true);
    assert_eq!(forced.status, ExecutionRecordStatus::Delegated);
    assert_eq!(forced.delegation.as_ref().unwrap().run_id, fresh.run_id);
    assert_eq!(backend.calls(), 0);
    // The parked run's task remains: a different fresh run does not claim to
    // supersede an orphan it does not own.
    let parked_task = parked.delegation.unwrap().tasks.pop().unwrap();
    assert!(scratch.root.join(&parked_task).exists());
}

/// The crash seam. The task is published FIRST and the state checkpoint
/// second, so the only gap a fault can open is an ORPHANED task — never state
/// pointing at a file that was not durably published. An injected failure
/// after the rename proves the ordering: the bytes are on disk, the run
/// refuses, and no delegated row claims them.
#[test]
fn an_injected_post_publication_fault_orphans_the_task_and_never_the_state() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let expected = crate::outbox_task_path(RUN_ID, KEY).unwrap();
    let filename = crate::task_filename(KEY).unwrap();
    vibe_safefs::fail_after_publish(Some(&filename));
    let error = execute_result(scratch.root.as_path(), &backend, RUN_ID, false).unwrap_err();
    vibe_safefs::fail_after_publish(None);

    let rendered = error.to_string();
    assert!(
        rendered.contains("cannot be published"),
        "the park refuses through the delegation seam: {rendered}"
    );
    // The two facts a caller cannot re-derive must survive the conversion:
    // how far the rename got, and which directories this run created. A
    // `{error:#}` flattening of `PublishError` drops both.
    assert!(
        rendered.contains(&expected),
        "the refusal names the deterministic task path: {rendered}"
    );
    assert!(
        rendered.contains("MAY ALREADY EXIST"),
        "a post-rename fault says the task may already be on disk: {rendered}"
    );
    assert!(
        rendered.contains("failed after the rename was attempted"),
        "the safefs stage evidence is preserved, not flattened away: {rendered}"
    );
    // `into_report` appends "this run created …" for every directory safefs
    // itself made; the outbox run directory is ensured by the pinned `dir`
    // call before publication, so it is named as part of the task path above
    // rather than duplicated here. What must never happen is the whole
    // conversion collapsing to the bare source string.
    assert_ne!(
        rendered.find("failed after the rename"),
        None,
        "the stage clause is present, not flattened: {rendered}"
    );
    assert!(
        rendered.contains("no lifecycle state points at that path"),
        "the refusal states the invariant it just upheld: {rendered}"
    );
    assert_eq!(backend.calls(), 0, "a refused park still spends nothing");
    let state = state(scratch.root.as_path());
    assert_eq!(
        state.execution[KEY].status,
        ExecutionRecordStatus::Fail,
        "the refusal is checkpointed as a failure, never as a handoff",
    );
    assert!(
        state.execution[KEY].tasks.is_empty(),
        "no state row points at the task the fault interrupted",
    );
    assert!(
        scratch.root.join(&expected).is_file(),
        "the published bytes remain as an honest orphan for later bounded GC",
    );
}

/// CLI mode is the R7.2 behavior: the same row, the same backend, and the
/// paid call happens. The branch is the mode's, not the row's.
#[test]
fn cli_mode_keeps_paying_and_never_parks() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(
        r#"{"outputs":[{"path":"docs/guide.md","content":"cli\n"},{"path":"docs/reference.md","content":"cli\n"}]}"#,
    );
    let row = row(TWO_OUTPUTS, PROMPT);
    let mut run = LifecycleRun::begin(
        lease(scratch.root.as_path()),
        project_fixture(scratch.root.as_path()),
        world_fixture(scratch.root.as_path()),
        {
            let mut metadata = metadata(RUN_ID, false);
            metadata.agent_mode = RunAgentMode::Cli;
            metadata
        },
        vec!["create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row);
    let rt = runtime(&backend);
    let transition = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap();
    assert_eq!(transition.status, ExecutionRecordStatus::Ok);
    assert!(transition.delegation.is_none());
    assert_eq!(backend.calls(), 1, "CLI mode pays exactly as before");
    let state = state(scratch.root.as_path());
    assert_eq!(state.execution[KEY].status, ExecutionRecordStatus::Ok);
}

/// Cancellation is STATE-FIRST. The durable row is what keeps a run from
/// completing over stranded work, so it is removed before the file it names.
/// An injected state-write failure must therefore leave BOTH halves intact —
/// the delegated row still readable in the durable bytes on disk, and its
/// task still published — because the only tolerable gap is an orphaned task,
/// never a live row pointing at a task that is already gone.
#[test]
fn a_failed_cancellation_write_leaves_the_row_and_its_task_intact() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let mut parked = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    let task = parked.delegation.take().unwrap().tasks.pop().unwrap();

    let mut run = LifecycleRun::begin(
        lease(scratch.root.as_path()),
        project_fixture(scratch.root.as_path()),
        world_fixture(scratch.root.as_path()),
        metadata(RUN_ID, false),
        vec!["validate".into(), "install".into(), "create".into()],
    )
    .unwrap();
    // The bytes as they stand AFTER the run header is written: the comparison
    // below is exactly "the cancelling write never landed", not "some earlier
    // write differed".
    let before = fs::read_to_string(scratch.root.join(".vibe/lifecycle.toml")).unwrap();

    crate::state::inject::fail_state_writes(Some("injected cancellation-write fault"));
    let error = run
        .cancel_delegated(KEY, scratch.root.as_path())
        .expect_err("a failed durable write is a failed cancellation");
    crate::state::inject::fail_state_writes(None);
    assert!(
        error
            .to_string()
            .contains("injected cancellation-write fault"),
        "the caller sees the real write failure: {error}"
    );

    // The durable file is not deleted, emptied or rewritten — it still READS,
    // and it still carries the delegated row with its task.
    let after = fs::read_to_string(scratch.root.join(".vibe/lifecycle.toml")).unwrap();
    assert_eq!(after, before, "a failed write changes no durable byte");
    let state = state(scratch.root.as_path());
    assert_eq!(
        state.execution[KEY].status,
        ExecutionRecordStatus::Delegated,
        "the row a failed cancellation could not remove is still delegated",
    );
    assert_eq!(state.execution[KEY].tasks, vec![task.clone()]);
    assert!(
        scratch.root.join(&task).is_file(),
        "state-first means the task is untouched when the state write fails",
    );
    // The in-memory store matches the bytes: a rolled-back `forget` leaves the
    // row visible to the very next question the run asks about it.
    assert_eq!(
        run.delegated_rows()
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        vec![KEY],
        "the rollback restores the row in memory, not only on disk",
    );
}

/// The reverse gap, and the only one this ordering allows: the state write
/// SUCCEEDS and the task cleanup then fails. The row is durably gone — it is
/// never put back, because reinstating it would resurrect work no plan will
/// visit — and the leftover file is named honestly as an orphan rather than
/// silently claimed as removed.
#[test]
fn a_successful_cancellation_with_a_failed_cleanup_names_a_named_orphan() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let mut parked = execute(scratch.root.as_path(), &backend, RUN_ID, false);
    let task = parked.delegation.take().unwrap().tasks.pop().unwrap();

    // Make the removal itself fail while the state stays exactly as the
    // engine wrote it: the owned task path becomes a NON-EMPTY DIRECTORY, so
    // `remove_file` refuses on every platform and the thing at that path
    // survives to be named. Nothing about the durable row is touched.
    let owned = scratch.root.join(&task);
    fs::remove_file(&owned).unwrap();
    fs::create_dir(&owned).unwrap();
    fs::write(owned.join("occupant"), "not a task\n").unwrap();

    let mut run = LifecycleRun::begin(
        lease(scratch.root.as_path()),
        project_fixture(scratch.root.as_path()),
        world_fixture(scratch.root.as_path()),
        metadata(RUN_ID, false),
        vec!["validate".into(), "install".into(), "create".into()],
    )
    .unwrap();
    let notice = run
        .cancel_delegated(KEY, scratch.root.as_path())
        .expect("a cleanup failure is a notice, not a failed cancellation")
        .expect("a live delegated row was cancelled");
    assert!(notice.contains("durably gone"), "{notice}");
    assert!(notice.contains("named orphan"), "{notice}");
    assert!(notice.contains(&task), "the orphan is named: {notice}");

    let state = state(scratch.root.as_path());
    assert!(
        !state.execution.contains_key(KEY),
        "the row is durably gone and is never reinstated by a cleanup failure",
    );
    assert!(
        run.delegated_rows().is_empty(),
        "and it is gone in memory too",
    );
    assert!(
        owned.exists(),
        "a failed cleanup removes nothing: what sits at the task path survives",
    );
}
