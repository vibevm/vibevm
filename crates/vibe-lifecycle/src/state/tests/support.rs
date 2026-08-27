//! Shared fixtures for the state-transaction tests: one canonical store
//! shape, the third-writer TOML, and the `.vibe` inventory a refusal must
//! leave untouched. Split from `tests/recovery.rs` when that file crossed the
//! 600-line budget; the same helpers serve the publication-failure and
//! recovery-window cells.

use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle_state::{
    ExecutionRecordStatus, SlotContinuation, SlotTargetRecord,
};

use super::{RUN_ID, record_for};
use crate::LifecycleStateStore;

pub(super) const KEY: &str = "org.demo/tools#produce";
pub(super) const OTHER: &str = "org.demo/tools#consume";

pub(super) fn open(root: &Path) -> LifecycleStateStore {
    LifecycleStateStore::begin(
        root,
        "create".into(),
        vec!["validate".into(), "install".into(), "create".into()],
        "2026-08-28T00:00:00Z".into(),
        RUN_ID.into(),
        false,
    )
    .unwrap()
}

/// A store already carrying one durable success row, plus the exact bytes of
/// that disk state — the "prior" every transaction case measures against.
pub(super) fn prior_store(root: &Path) -> (LifecycleStateStore, Vec<u8>) {
    let mut store = open(root);
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Ok, "sha256:prior"),
        )
        .unwrap();
    let bytes = fs::read(store.path()).unwrap();
    (store, bytes)
}

/// A valid state no party in the test wrote: the third writer's bytes.
pub(super) fn third_state_toml() -> &'static str {
    "schema = 1\n\
     [run]\nrequested = 'other'\nchain = []\nstarted = '2020-01-01T00:00:00Z'\n\
     [execution]\n"
}

pub(super) fn targets() -> SlotContinuation {
    SlotContinuation {
        targets: vec![SlotTargetRecord {
            group: "org.demo".into(),
            name: "tools".into(),
            version: "0.1.0".into(),
        }],
    }
}

/// Everything the `.vibe` directory holds: a refused or poisoned store must
/// leave exactly the bytes that were already there, and no staging residue.
pub(super) fn vibe_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join(".vibe"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
