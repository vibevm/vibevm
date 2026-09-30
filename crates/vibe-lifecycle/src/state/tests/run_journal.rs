//! A reserved run ID is inspected without creating or completing a run.

use std::fs;
use std::sync::Arc;

use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::{OTHER_RUN, RUN_ID, lease, record_for};
use crate::state::{
    LifecycleRunJournalError, LifecycleRunJournalObservation, inspect_reserved_lifecycle_run,
};
use crate::{LifecycleLease, LifecycleStateStore};

fn begin(lease: Arc<LifecycleLease>, run_id: &str) -> LifecycleStateStore {
    LifecycleStateStore::begin(
        lease,
        "build".into(),
        vec![
            "validate".into(),
            "install".into(),
            "generate".into(),
            "build".into(),
        ],
        "2026-09-30T00:00:00Z".into(),
        run_id.into(),
        ".".into(),
        false,
    )
    .unwrap()
}

#[test]
fn absent_state_is_read_only_and_not_a_terminal_result() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let state_file = root.path().join(LifecycleStateStore::FILE);
    assert!(!state_file.exists());
    assert!(matches!(
        inspect_reserved_lifecycle_run(&lease, RUN_ID).unwrap(),
        LifecycleRunJournalObservation::Absent
    ));
    assert!(!state_file.exists());
}

#[test]
fn matching_header_returns_exact_state_but_requires_recovery() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let store = begin(Arc::clone(&lease), RUN_ID);
    let before = fs::read(store.path()).unwrap();
    let LifecycleRunJournalObservation::RecoveryRequired { state } =
        inspect_reserved_lifecycle_run(&lease, RUN_ID).unwrap()
    else {
        panic!("a matching header has no terminal outcome proof");
    };
    assert_eq!(state.run.run_id.as_deref(), Some(RUN_ID));
    assert_eq!(state.run.requested, "build");
    assert_eq!(state.run.selected.as_deref(), Some("."));
    assert_eq!(
        state.run.chain,
        ["validate", "install", "generate", "build"]
    );
    assert_eq!(fs::read(store.path()).unwrap(), before);
}

#[test]
fn foreign_current_run_refuses_without_rewriting_it() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let store = begin(Arc::clone(&lease), RUN_ID);
    let before = fs::read(store.path()).unwrap();
    assert!(matches!(
        inspect_reserved_lifecycle_run(&lease, OTHER_RUN),
        Err(LifecycleRunJournalError::ForeignCurrent { reserved, observed })
            if reserved == OTHER_RUN && observed == RUN_ID
    ));
    assert_eq!(fs::read(store.path()).unwrap(), before);
}

#[test]
fn delegated_and_success_rows_do_not_synthesize_terminal_completion() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let mut store = begin(Arc::clone(&lease), RUN_ID);
    store
        .checkpoint(
            "parked".into(),
            record_for(
                "parked",
                RUN_ID,
                ExecutionRecordStatus::Delegated,
                "sha256:parked",
            ),
        )
        .unwrap();
    assert!(matches!(
        inspect_reserved_lifecycle_run(&lease, RUN_ID).unwrap(),
        LifecycleRunJournalObservation::RecoveryRequired { state }
            if state.execution["parked"].status == ExecutionRecordStatus::Delegated
    ));
    store
        .checkpoint(
            "parked".into(),
            record_for("parked", RUN_ID, ExecutionRecordStatus::Ok, "sha256:ok"),
        )
        .unwrap();
    assert!(matches!(
        inspect_reserved_lifecycle_run(&lease, RUN_ID).unwrap(),
        LifecycleRunJournalObservation::RecoveryRequired { state }
            if state.execution["parked"].status == ExecutionRecordStatus::Ok
    ));
}

#[test]
fn unattributed_state_is_uncertain_not_absent() {
    let root = tempfile::tempdir().unwrap();
    let lease = lease(root.path());
    let store = begin(Arc::clone(&lease), "");
    assert!(matches!(
        inspect_reserved_lifecycle_run(&lease, RUN_ID).unwrap(),
        LifecycleRunJournalObservation::Uncertain { state }
            if state.run.run_id.is_none()
    ));
    assert!(store.path().exists());
}
