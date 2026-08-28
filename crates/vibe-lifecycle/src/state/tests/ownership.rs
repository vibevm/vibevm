//! Selected-NODE ownership over the adoption machinery (PROP-054
//! `##REF-AGENT-RESUME`, R7.4 A6): a live park belongs to the node that
//! authored it — a sibling's identical command is the typed `ForeignPark`
//! refusal even under `--force` — while an idle prior is nobody's park and
//! crosses nodes freely. Split from `adoption.rs` when the ownership laws
//! outgrew the 600-line cell budget; the fixtures it needs stay beside the
//! adoption matrix and are reached through `pub(super)`.

use std::fs;

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

use super::adoption::{CHAIN, FRESH, KEY, STARTED, candidate_dirs, chain, parked_at, select_as};
use super::{RUN_ID, lease, record_for};
use crate::LifecycleStateStore;

/// Two members of one workspace can present the IDENTICAL requested/chain
/// tuple. A live park owned by another node is the typed ownership refusal —
/// fired before adoption, displacement and allocation, with or without
/// `--force` (force and supersession are SAME-node rulings) — and the
/// owner's state is left exactly as found: identity, selected spelling and
/// sticky trace bit all untouched, nothing allocated under the sibling, and
/// deliberately no `SupersededTrace` side effect (this node has no right to
/// terminalise a sibling's running trace).
#[test]
fn a_live_park_owned_by_another_node_refuses_even_under_force() {
    for force in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        parked_at(dir.path(), "members/a", true);
        let error = select_as(
            dir.path(),
            "create",
            &CHAIN,
            RunAgentMode::Agent,
            force,
            "members/b",
        )
        .unwrap_err();
        assert!(
            matches!(
                &error,
                crate::LifecycleStateError::ForeignPark {
                    stored,
                    selected,
                    run_id,
                    ..
                } if stored == "members/a" && selected == "members/b" && run_id == RUN_ID
            ),
            "force={force}: the refusal names both nodes and the exact parked run: {error}",
        );
        assert!(
            error.to_string().contains("never force through it"),
            "force={force}: the remedy states the force law: {error}",
        );
        assert!(
            candidate_dirs(dir.path()).is_empty(),
            "force={force}: a foreign refusal allocates nothing",
        );
        let state = LifecycleStateStore::peek(dir.path())
            .expect("the owner's state still reads")
            .expect("state is present");
        assert_eq!(state.run.run_id.as_deref(), Some(RUN_ID));
        assert_eq!(state.run.selected.as_deref(), Some("members/a"));
        assert!(
            state.run.compile_trace,
            "force={force}: the foreign park's running trace is NOT superseded or rewritten",
        );
    }
}

/// A nondelegated prior is not a park, so cross-node it is nobody's to
/// refuse: member B starts fresh, rewrites the header to its own selected
/// identity, and keeps the eligible rows exactly as a same-node run would.
#[test]
fn an_idle_prior_from_another_node_begins_fresh_and_renames_the_header() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = LifecycleStateStore::begin(
        lease(dir.path()),
        "create".into(),
        chain(&CHAIN),
        STARTED.into(),
        RUN_ID.into(),
        "members/a".into(),
        false,
    )
    .unwrap();
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
        )
        .unwrap();
    drop(store);

    let identity = select_as(
        dir.path(),
        "build",
        &["validate", "install", "build"],
        RunAgentMode::Agent,
        false,
        "members/b",
    )
    .unwrap();
    assert!(!identity.adopted, "an idle prior is nobody's park");
    assert_ne!(identity.run_id, RUN_ID);

    let store = LifecycleStateStore::begin(
        lease(dir.path()),
        "build".into(),
        chain(&["validate", "install", "build"]),
        FRESH.into(),
        identity.run_id.clone(),
        "members/b".into(),
        false,
    )
    .unwrap();
    assert!(
        store.prior(KEY).is_some(),
        "eligible nondelegated rows survive the cross-node begin",
    );
    let written: LifecycleState =
        toml::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(written.run.selected.as_deref(), Some("members/b"));
    assert_eq!(
        written.run.run_id.as_deref(),
        Some(identity.run_id.as_str())
    );
}
