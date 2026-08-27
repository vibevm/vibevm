//! Durable run identity: the exact adoption matrix, the no-leaked-candidate
//! rule, and what a fresh header may and may not inherit from a parked run.

use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

use super::{OTHER_RUN, RUN_ID, record_for};
use crate::{LifecycleStateStore, select_run_identity};

const CHAIN: [&str; 3] = ["validate", "install", "create"];
const STARTED: &str = "2026-08-26T00:00:00Z";
const FRESH: &str = "2026-08-26T09:00:00Z";
const KEY: &str = "org.demo/tools#produce";

fn chain(phases: &[&str]) -> Vec<String> {
    phases.iter().map(|phase| (*phase).to_string()).collect()
}

/// Write a state file that parked `KEY` under `RUN_ID` for `vibe create`.
fn parked(root: &Path) {
    let mut store = LifecycleStateStore::begin(
        root,
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
    )
    .unwrap();
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Delegated, "sha256:x"),
        )
        .unwrap();
}

fn select(
    root: &Path,
    requested: &str,
    phases: &[&str],
    mode: RunAgentMode,
    force: bool,
) -> crate::RunIdentity {
    select_run_identity(
        root,
        root,
        requested,
        &chain(phases),
        mode,
        force,
        FRESH.into(),
    )
    .unwrap()
}

fn candidate_dirs(root: &Path) -> Vec<String> {
    let base = root.join(".vibe/lifecycle");
    if !base.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(base)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The one positive case: agent mode, no `--force`, same requested phase, the
/// same complete chain, a delegated row, a valid persisted id. The original
/// `started` survives — the park and its resume are one run, so the run's
/// clock is not restarted.
#[test]
fn the_resume_of_a_parked_run_adopts_its_identity_and_original_start() {
    let dir = tempfile::tempdir().unwrap();
    parked(dir.path());
    let identity = select(dir.path(), "create", &CHAIN, RunAgentMode::Agent, false);
    assert!(identity.adopted);
    assert_eq!(identity.run_id, RUN_ID);
    assert_eq!(identity.started, STARTED);
    assert!(
        candidate_dirs(dir.path()).is_empty(),
        "selection precedes allocation: adoption leaks no candidate scratch run",
    );
}

/// Everything else allocates fresh. Each row is one reason on its own; the
/// baseline above proves none of them is passing vacuously.
#[test]
fn a_different_command_chain_mode_force_or_missing_park_never_inherits() {
    for (label, requested, phases, mode, force) in [
        (
            "a different requested phase",
            "build",
            CHAIN.as_slice(),
            RunAgentMode::Agent,
            false,
        ),
        (
            "a shorter chain",
            "create",
            ["install", "create"].as_slice(),
            RunAgentMode::Agent,
            false,
        ),
        (
            "a clean-composed chain",
            "create",
            ["clean", "validate", "install", "create"].as_slice(),
            RunAgentMode::Agent,
            false,
        ),
        (
            "cli mode",
            "create",
            CHAIN.as_slice(),
            RunAgentMode::Cli,
            false,
        ),
        (
            "--force",
            "create",
            CHAIN.as_slice(),
            RunAgentMode::Agent,
            true,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        parked(dir.path());
        let identity = select(dir.path(), requested, phases, mode, force);
        assert!(!identity.adopted, "{label} must not inherit the parked run");
        assert_ne!(identity.run_id, RUN_ID, "{label}");
        assert_eq!(identity.started, FRESH, "{label} restarts the run clock");
        assert_eq!(
            candidate_dirs(dir.path()),
            std::slice::from_ref(&identity.run_id),
            "{label} allocates exactly the id it returns",
        );
    }
}

/// A prior run with no delegated row is complete, not parked: there is
/// nothing to resume, so its identity is not adopted.
#[test]
fn a_prior_run_without_a_parked_row_is_not_resumable() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
    )
    .unwrap();
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
        )
        .unwrap();
    let identity = select(dir.path(), "create", &CHAIN, RunAgentMode::Agent, false);
    assert!(!identity.adopted);
}

/// A pre-R7.3 file — no `run_id`, no `tasks`, no delegated row — is valid
/// input: it reads, it does not adopt, and its rows survive the new header.
#[test]
fn a_pre_r73_state_file_still_opens_and_keeps_its_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "schema = 1\n\
         [run]\nrequested = 'create'\nchain = ['validate', 'install', 'create']\n\
         started = '2026-08-20T09:00:00Z'\n\
         [execution.'org.demo/tools#produce']\n\
         phase = 'create'\nfingerprint = 'sha256:old'\nstatus = 'ok'\nduration_ms = 4\n\
         artifacts = []\n",
    )
    .unwrap();
    let identity = select(dir.path(), "create", &CHAIN, RunAgentMode::Agent, false);
    assert!(!identity.adopted, "an identity-less run cannot be resumed");
    let store = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        chain(&CHAIN),
        FRESH.into(),
        identity.run_id.clone(),
    )
    .unwrap();
    assert!(store.prior(KEY).is_some(), "old rows are preserved");
}

/// A fresh (non-adopted) run id may not retain the previous run's parked
/// work: those task paths live under the OTHER run's outbox directory. The
/// success/freshness rows still survive, and the orphaned task file is left
/// on disk to be named honestly rather than silently claimed or deleted.
#[test]
fn a_fresh_run_prunes_prior_delegated_rows_but_keeps_reusable_ones() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
    )
    .unwrap();
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Delegated, "sha256:x"),
        )
        .unwrap();
    store
        .checkpoint(
            "org.demo/tools#other".into(),
            record_for(
                "org.demo/tools#other",
                RUN_ID,
                ExecutionRecordStatus::Ok,
                "sha256:y",
            ),
        )
        .unwrap();

    // An UNRELATED run — different command, its own fresh identity.
    let store = LifecycleStateStore::begin(
        dir.path(),
        "build".into(),
        chain(&["validate", "install", "build"]),
        FRESH.into(),
        OTHER_RUN.into(),
    )
    .unwrap();
    assert!(
        store.prior(KEY).is_none(),
        "a delegated row is never rehomed under a different run id",
    );
    assert!(
        store.prior("org.demo/tools#other").is_some(),
        "ordinary success rows are preserved exactly as before",
    );
    let written: LifecycleState =
        toml::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(written.run.run_id.as_deref(), Some(OTHER_RUN));
    assert!(
        written
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
    );
}

/// Adopting the SAME id keeps the park: that is the whole point of adoption.
#[test]
fn adopting_the_same_id_retains_the_delegated_row() {
    let dir = tempfile::tempdir().unwrap();
    parked(dir.path());
    let identity = select(dir.path(), "create", &CHAIN, RunAgentMode::Agent, false);
    let store = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        chain(&CHAIN),
        identity.started,
        identity.run_id,
    )
    .unwrap();
    let prior = store.prior(KEY).expect("the park survives its own resume");
    assert_eq!(prior.status, ExecutionRecordStatus::Delegated);
    assert_eq!(prior.tasks, [crate::outbox_task_path(RUN_ID, KEY).unwrap()]);
}

/// Corrupt delegated state refuses with the erasable-cache remediation; it
/// does not silently mint a new identity around the damage.
#[test]
fn corrupt_delegated_state_refuses_rather_than_minting_a_new_identity() {
    let base = "schema = 1\n\
                [run]\nrequested = 'create'\nchain = ['validate', 'install', 'create']\n\
                started = '2026-08-26T00:00:00Z'\n";
    let row = |extra: &str| {
        format!(
            "{base}{extra}[execution.'org.demo/tools#produce']\n\
             phase = 'create'\nfingerprint = 'sha256:x'\nstatus = 'delegated'\nduration_ms = 4\n\
             scope = 'phase'\n\
             artifacts = [{{ id = 'a', kind = 'file', path = 'C:/out' }}]\n"
        )
    };
    let owned = crate::outbox_task_path(RUN_ID, KEY).unwrap();
    for (label, body) in [
        (
            "no run id at all",
            format!("{}tasks = ['{owned}']\n", row("")),
        ),
        (
            "an invalid run id",
            format!(
                "{}tasks = ['{owned}']\n",
                row("run_id = 'not-32-lowercase-hex'\n")
            ),
        ),
        (
            "no task at all",
            row(&format!("run_id = '{RUN_ID}'\n")).to_string(),
        ),
        (
            "another run's task",
            format!(
                "{}tasks = ['{}']\n",
                row(&format!("run_id = '{RUN_ID}'\n")),
                crate::outbox_task_path(OTHER_RUN, KEY).unwrap(),
            ),
        ),
        (
            "another execution's task",
            format!(
                "{}tasks = ['{}']\n",
                row(&format!("run_id = '{RUN_ID}'\n")),
                crate::outbox_task_path(RUN_ID, "org.demo/tools#elsewhere").unwrap(),
            ),
        ),
        (
            "two tasks under one row",
            format!(
                "{}tasks = ['{owned}', '{owned}']\n",
                row(&format!("run_id = '{RUN_ID}'\n")),
            ),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LifecycleStateStore::FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &body).unwrap();
        let error = select_run_identity(
            dir.path(),
            dir.path(),
            "create",
            &chain(&CHAIN),
            RunAgentMode::Agent,
            false,
            FRESH.into(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("remove this erasable cache"),
            "{label}: {error}"
        );
        assert!(
            candidate_dirs(dir.path()).is_empty(),
            "{label}: a refusal allocates nothing",
        );
    }
}

/// A non-delegated row may not carry task files, and any run id a header
/// carries must be a real identity — both judged centrally, on write as well
/// as on read.
#[test]
fn a_non_delegated_row_with_tasks_and_a_bad_header_id_are_both_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
    )
    .unwrap();
    let mut smuggled = record_for(KEY, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x");
    smuggled.tasks = vec![crate::outbox_task_path(RUN_ID, KEY).unwrap()];
    let error = store
        .checkpoint(KEY.into(), smuggled)
        .unwrap_err()
        .to_string();
    assert!(error.contains("may not carry outbox task files"), "{error}");

    let dir = tempfile::tempdir().unwrap();
    let error = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        "NOT-HEX".into(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("not a valid 32-hex run id"), "{error}");
}

/// The scope and continuation laws, judged BEFORE adoption. Each body is a
/// state a run could otherwise have adopted and then silently mis-reconciled.
#[test]
fn scope_and_continuation_violations_refuse_before_adoption() {
    let owned = crate::outbox_task_path(RUN_ID, KEY).unwrap();
    let header = |extra: &str| {
        format!(
            "schema = 1\n[run]\nrequested = 'create'\n\
             chain = ['validate', 'install', 'create']\n\
             started = '2026-08-26T00:00:00Z'\nrun_id = '{RUN_ID}'\n{extra}"
        )
    };
    let delegated = |scope: &str| {
        format!(
            "[execution.'{KEY}']\nphase = 'create'\nfingerprint = 'sha256:x'\n\
             status = 'delegated'\nduration_ms = 4\n{scope}\
             artifacts = [{{ id = 'a', kind = 'file', path = 'C:/out' }}]\n\
             tasks = ['{owned}']\n"
        )
    };
    let continuation = "[run.slot_continuation]\ntargets = [{ group = 'org.demo', \
                        name = 'tools', version = '0.1.0' }]\n";
    let empty_continuation = "[run.slot_continuation]\ntargets = []\n";

    for (label, body, needle) in [
        (
            "a delegated row with no typed scope",
            format!("{}{}", header(""), delegated("")),
            "carries no typed scope",
        ),
        (
            "slot debt with no continuation",
            format!("{}{}", header(""), delegated("scope = 'slot'\n")),
            "records no continuation",
        ),
        (
            "a continuation with an empty target list",
            format!(
                "{}{}",
                header(empty_continuation),
                delegated("scope = 'slot'\n")
            ),
            "names no payload-event target",
        ),
        (
            "a continuation with no slot debt",
            format!("{}{}", header(continuation), delegated("scope = 'phase'\n")),
            "nothing would ever consume it",
        ),
        (
            "a non-delegated row carrying a scope",
            format!(
                "{}[execution.'{KEY}']\nphase = 'create'\nfingerprint = 'sha256:x'\n\
                 status = 'ok'\nduration_ms = 4\nscope = 'phase'\nartifacts = []\n",
                header(""),
            ),
            "may not carry a delegation scope",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LifecycleStateStore::FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &body).unwrap();
        let error = select_run_identity(
            dir.path(),
            dir.path(),
            "create",
            &chain(&CHAIN),
            RunAgentMode::Agent,
            false,
            FRESH.into(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(needle), "{label}: {error}");
        assert!(
            error.contains("remove this erasable cache"),
            "{label}: {error}",
        );
        assert!(
            candidate_dirs(dir.path()).is_empty(),
            "{label}: a refusal allocates nothing",
        );
    }
}

/// A pre-R7.3 file legitimately carries NEITHER a scope NOR a continuation: it
/// has no delegated row, so both laws hold vacuously and the file still reads.
#[test]
fn a_pre_r73_file_without_scope_or_continuation_is_still_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "schema = 1\n\
         [run]\nrequested = 'create'\nchain = ['validate', 'install', 'create']\n\
         started = '2026-08-20T09:00:00Z'\n\
         [execution.'org.demo/tools#produce']\n\
         phase = 'create'\nfingerprint = 'sha256:old'\nstatus = 'ok'\nduration_ms = 4\n\
         artifacts = []\n",
    )
    .unwrap();
    let state = LifecycleStateStore::peek(dir.path())
        .expect("an old file still reads")
        .expect("state is present");
    assert!(state.run.slot_continuation.is_none());
    assert!(state.execution.values().all(|row| row.scope.is_none()));
}
