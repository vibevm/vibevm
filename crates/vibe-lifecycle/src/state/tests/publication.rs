//! Publication failures that never crossed the rename boundary: the
//! `BeforePublication` class, where the destination is provably unchanged.
//! Split from `tests/recovery.rs` when that file crossed the 600-line
//! budget — the seam is the transaction's own: everything here is invisible
//! on disk, everything there is the possibly-published doubt.

use std::fs;

use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::support::{KEY, OTHER, prior_store, vibe_names};
use super::{RUN_ID, record_for};
use crate::state::inject;
use crate::{LifecycleStateError, LifecycleStateStore};

/// A `BeforePublication` failure through the REAL staged publication: the
/// safefs injection writes and syncs the stage, then fails before the rename
/// and collects its own stage. The prior bytes and the prior state must be
/// exactly unchanged, nothing may survive beside the state file, and the
/// failure surfaces its TYPED stage.
#[test]
fn a_before_publication_failure_leaves_the_exact_prior_bytes_and_state() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, prior_bytes) = prior_store(dir.path());
    let before = store.state().clone();

    vibe_safefs::fail_before_publish(Some(LifecycleStateStore::FILE));
    let error = store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:next"),
        )
        .expect_err("an injected pre-publication failure is a failed checkpoint");
    vibe_safefs::fail_before_publish(None);

    let rendered = error.to_string();
    let LifecycleStateError::Publication {
        stage: vibe_safefs::PublishStage::BeforePublication,
        ..
    } = &error
    else {
        panic!("a pre-publication failure surfaces its typed stage: {error}");
    };
    assert!(rendered.contains("failed before publication"), "{rendered}");
    assert!(
        rendered.contains("injected pre-publication failure"),
        "the original failure is preserved verbatim: {rendered}",
    );
    assert_eq!(*store.state(), before, "the in-memory state did not move");
    assert_eq!(
        fs::read(store.path()).unwrap(),
        prior_bytes,
        "the durable bytes did not move",
    );
    assert_eq!(
        vibe_names(dir.path()),
        vec!["lifecycle.toml".to_string()],
        "the failed publication's own stage was collected",
    );
    assert!(store.poison_reason().is_none(), "nothing was poisoned");
}

/// No state-write failure surfaces outside the typed publication-stage
/// vocabulary: the injected durable-write fault — which stands for any
/// production I/O failure before the rename — is the `BeforePublication`
/// class with its TYPED stage and the original reason verbatim, and the
/// exact prior memory and bytes stay current. (The `Write` variant this
/// crate once carried is gone; a regression to it fails this match at
/// compile time.)
#[test]
fn an_injected_write_failure_surfaces_the_typed_before_publication_stage() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, prior_bytes) = prior_store(dir.path());
    let before = store.state().clone();

    inject::fail_state_writes(Some("injected typed-stage fault"));
    let error = store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
        )
        .expect_err("an injected durable-write fault is a failed checkpoint");
    inject::fail_state_writes(None);

    let LifecycleStateError::Publication {
        stage: vibe_safefs::PublishStage::BeforePublication,
        failure,
        ..
    } = &error
    else {
        panic!("a write failure surfaces the typed stage: {error}");
    };
    assert!(
        failure.contains("injected typed-stage fault"),
        "the original reason is preserved verbatim: {failure}",
    );
    assert_eq!(*store.state(), before, "the in-memory state did not move");
    assert_eq!(
        fs::read(store.path()).unwrap(),
        prior_bytes,
        "the durable bytes did not move",
    );
    assert!(
        store.prior(KEY).is_some(),
        "the prior row is still the last proven state",
    );
}
