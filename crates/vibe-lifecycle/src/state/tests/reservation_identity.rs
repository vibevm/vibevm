//! The reservation selector keeps identity decisions without allocating scratch.

use std::fs;

use crate::state::current::ExecutionRecordStatus;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::{RUN_ID, lease, record_for};
use crate::{
    LifecycleStateStore, process::is_valid_run_id, select_run_identity,
    select_run_identity_for_reservation,
};

fn chain() -> Vec<String> {
    ["validate", "install", "create"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn fresh_reservation_identity_does_not_create_scratch_or_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let held = lease(root);
    // The lease itself owns `.vibe/lifecycle.lock`; the selector must add
    // nothing to that already established boundary.
    let before = fs::read_dir(root.join(".vibe"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    let identity = select_run_identity_for_reservation(
        &held,
        root,
        "create",
        &chain(),
        ".",
        RunAgentMode::Cli,
        false,
        true,
        "2026-09-30T00:00:00Z".into(),
    )
    .unwrap();
    assert!(is_valid_run_id(&identity.run_id));
    assert!(!identity.adopted);
    assert!(identity.compile_trace);
    assert_eq!(identity.started, "2026-09-30T00:00:00Z");
    assert_eq!(
        fs::read_dir(root.join(".vibe"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        before
    );
    assert!(!root.join(".vibe/lifecycle").exists());
    assert!(!root.join(LifecycleStateStore::FILE).exists());
}

#[test]
fn reservation_selector_adopts_exactly_the_default_parked_identity() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    {
        let mut store = LifecycleStateStore::begin(
            lease(root),
            "create".into(),
            chain(),
            "2026-09-29T00:00:00Z".into(),
            RUN_ID.into(),
            ".".into(),
            true,
        )
        .unwrap();
        store
            .checkpoint(
                "key".into(),
                record_for("key", RUN_ID, ExecutionRecordStatus::Delegated, "sha256:x"),
            )
            .unwrap();
    }
    let held = lease(root);
    let bytes_before = fs::read(root.join(LifecycleStateStore::FILE)).unwrap();
    let proposal = select_run_identity_for_reservation(
        &held,
        root,
        "create",
        &chain(),
        ".",
        RunAgentMode::Agent,
        false,
        false,
        "2026-09-30T00:00:00Z".into(),
    )
    .unwrap();
    let default = select_run_identity(
        &held,
        root,
        "create",
        &chain(),
        ".",
        RunAgentMode::Agent,
        false,
        false,
        "2026-09-30T00:00:00Z".into(),
    )
    .unwrap();
    assert_eq!(proposal, default);
    assert_eq!(proposal.run_id, RUN_ID);
    assert!(proposal.adopted && proposal.compile_trace);
    assert_eq!(proposal.started, "2026-09-29T00:00:00Z");
    assert_eq!(
        fs::read(root.join(LifecycleStateStore::FILE)).unwrap(),
        bytes_before
    );
    assert!(!root.join(".vibe/lifecycle").exists());
}
