//! The candidate-state transaction: nothing in memory moves until the bytes do.
//!
//! Every mutating store verb builds a CANDIDATE, reconciles the continuation
//! against that candidate's own slot debt, and only adopts it after a durable
//! write succeeds. These cases arm a write fault at each verb and assert the
//! two things a hand-rolled rollback keeps getting wrong: the generated state
//! STRUCT is unchanged, and the durable file BYTES are unchanged. A rollback
//! that restores the row it removed but forgets the continuation it
//! reconciled passes neither.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, ExecutionRecordStatus, LifecycleState, SlotContinuation,
    SlotTargetRecord, StateArtifact,
};

use super::{RUN_ID, lease};
use crate::LifecycleStateStore;
use crate::state::inject;

const KEY: &str = "org.demo/tools#produce@slot(org.demo/tools@0.1.0)";
const SYNTHETIC: &str = "synthetic/hooks#post-install";

fn chain() -> Vec<String> {
    vec!["validate".into(), "install".into(), "create".into()]
}

fn open(root: &Path) -> LifecycleStateStore {
    LifecycleStateStore::begin(
        lease(root),
        "create".into(),
        chain(),
        "2026-08-27T00:00:00Z".into(),
        RUN_ID.into(),
        false,
    )
    .unwrap()
}

/// A slot-scoped delegated row naming exactly the task `(RUN_ID, KEY)` owns.
fn parked_row(status: ExecutionRecordStatus) -> ExecutionRecord {
    let delegated = status == ExecutionRecordStatus::Delegated;
    ExecutionRecord {
        artifacts: vec![StateArtifact {
            id: "docs/slot.md".into(),
            kind: "file".into(),
            path: "C:/out/docs/slot.md".into(),
        }],
        duration_ms: 5,
        fingerprint: "sha256:aa".into(),
        phase: "install".into(),
        status,
        tasks: if delegated {
            vec![crate::outbox_task_path(RUN_ID, KEY).unwrap()]
        } else {
            Vec::new()
        },
        scope: delegated.then_some(ExecutionRecordScope::Slot),
    }
}

fn targets() -> SlotContinuation {
    SlotContinuation {
        targets: vec![SlotTargetRecord {
            group: "org.demo".into(),
            name: "tools".into(),
            version: "0.1.0".into(),
        }],
    }
}

/// Publish the task file the parked row names, so a case can assert it was
/// left alone. The engine publishes BEFORE it checkpoints, by design, so a
/// failed checkpoint legitimately leaves the file behind as a named orphan.
fn publish_task(root: &Path) -> std::path::PathBuf {
    let task = root.join(crate::outbox_task_path(RUN_ID, KEY).unwrap());
    fs::create_dir_all(task.parent().unwrap()).unwrap();
    fs::write(&task, "# task\n").unwrap();
    task
}

/// The two things that must not move: the decoded state and the file bytes.
fn snapshot(store: &LifecycleStateStore) -> (LifecycleState, Vec<u8>) {
    (
        store.state().clone(),
        fs::read(store.path()).expect("the durable file exists"),
    )
}

fn assert_untouched(label: &str, store: &LifecycleStateStore, before: &(LifecycleState, Vec<u8>)) {
    let after = snapshot(store);
    assert_eq!(
        after.0, before.0,
        "{label}: the in-memory generated state moved",
    );
    assert_eq!(after.1, before.1, "{label}: the durable bytes moved",);
}

/// A store that already carries the durable park: one slot-scoped delegated
/// row and the continuation it owes, both written by a real commit.
fn parked_store(root: &Path) -> LifecycleStateStore {
    let mut store = open(root);
    store.record_slot_continuation(targets()).unwrap();
    store
        .checkpoint(KEY.into(), parked_row(ExecutionRecordStatus::Delegated))
        .unwrap();
    assert!(
        store.slot_continuation().is_some(),
        "the park's own write is what makes the continuation durable",
    );
    store
}

/// The FIRST park. A failed checkpoint must leave no row and no continuation —
/// neither durable nor in memory. The task file stays as an honest orphan:
/// publication precedes the checkpoint precisely so state never points at a
/// file that was not written, and this is the gap that ordering allows.
#[test]
fn a_failed_first_park_checkpoint_records_neither_row_nor_continuation() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    store.record_slot_continuation(targets()).unwrap();
    let task = publish_task(dir.path());
    let before = snapshot(&store);

    inject::fail_state_writes(Some("injected park-checkpoint fault"));
    let error = store
        .checkpoint(KEY.into(), parked_row(ExecutionRecordStatus::Delegated))
        .expect_err("a failed durable write is a failed checkpoint");
    inject::fail_state_writes(None);
    assert!(error.to_string().contains("injected park-checkpoint fault"));

    assert_untouched("first park", &store, &before);
    assert!(store.prior(KEY).is_none(), "no row was recorded");
    assert!(
        store.slot_continuation().is_none(),
        "and the staged set never became durable either",
    );
    assert!(
        task.is_file(),
        "the published task remains as a named orphan — never state pointing at nothing",
    );
}

/// SATISFYING the last delegated slot row. A failed checkpoint must leave the
/// row still `delegated`, the continuation still recorded, and the task still
/// on disk — in memory and on disk alike. This is the case a hand-rolled
/// rollback fails: the reconcile would have dropped the continuation the
/// moment the candidate's debt reached zero.
#[test]
fn a_failed_satisfying_checkpoint_keeps_the_row_the_continuation_and_the_task() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(dir.path());
    let task = publish_task(dir.path());
    let before = snapshot(&store);

    inject::fail_state_writes(Some("injected satisfy fault"));
    let error = store
        .checkpoint(KEY.into(), parked_row(ExecutionRecordStatus::Ok))
        .expect_err("a failed durable write is a failed checkpoint");
    inject::fail_state_writes(None);
    assert!(error.to_string().contains("injected satisfy fault"));

    assert_untouched("satisfying park", &store, &before);
    assert_eq!(
        store.prior(KEY).map(|row| row.status.clone()),
        Some(ExecutionRecordStatus::Delegated),
        "the row is still parked",
    );
    assert_eq!(
        store.slot_continuation(),
        Some(&targets()),
        "and the run still owes exactly what it owed",
    );
    assert!(
        task.is_file(),
        "the task is still awaiting the hosting agent"
    );
    let durable: LifecycleState =
        toml::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(
        durable.execution[KEY].status,
        ExecutionRecordStatus::Delegated,
    );
    assert_eq!(durable.run.slot_continuation, Some(targets()));
}

/// CANCELLING the last delegated slot row rolls back identically: `forget`
/// removes the row from a candidate, the candidate's reconcile drops the
/// continuation, and a failed write must undo BOTH.
#[test]
fn a_failed_cancellation_keeps_the_row_and_the_continuation() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(dir.path());
    let task = publish_task(dir.path());
    let before = snapshot(&store);

    inject::fail_state_writes(Some("injected forget fault"));
    let error = store
        .forget(KEY)
        .expect_err("a failed durable write is a failed forget");
    inject::fail_state_writes(None);
    assert!(error.to_string().contains("injected forget fault"));

    assert_untouched("cancellation", &store, &before);
    assert_eq!(
        store.prior(KEY).map(|row| row.status.clone()),
        Some(ExecutionRecordStatus::Delegated),
    );
    assert_eq!(store.slot_continuation(), Some(&targets()));
    assert!(task.is_file());
}

/// The two remaining mutation paths. `retain_prefixed` prunes rows and
/// `clear_slot_continuation` drops the header field; a candidate failure in
/// either changes nothing.
#[test]
fn failed_retain_and_explicit_clear_change_no_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(dir.path());
    store
        .checkpoint(SYNTHETIC.into(), parked_row(ExecutionRecordStatus::Ok))
        .unwrap();
    let before = snapshot(&store);

    inject::fail_state_writes(Some("injected retain fault"));
    let error = store
        .retain_prefixed("synthetic/", &BTreeSet::new())
        .expect_err("a failed durable write is a failed retain");
    inject::fail_state_writes(None);
    assert!(error.to_string().contains("injected retain fault"));
    assert_untouched("retain", &store, &before);
    assert!(
        store.prior(SYNTHETIC).is_some(),
        "the row the retain would have pruned is still there",
    );

    inject::fail_state_writes(Some("injected clear fault"));
    let error = store
        .clear_slot_continuation()
        .expect_err("a failed durable write is a failed clear");
    inject::fail_state_writes(None);
    assert!(error.to_string().contains("injected clear fault"));
    assert_untouched("clear", &store, &before);

    // And the pair-law is a property of the candidate, not of caller
    // discipline: an explicit clear while a slot-scoped park is still live
    // cannot erase what that park needs, even when the write SUCCEEDS.
    store.clear_slot_continuation().unwrap();
    assert_eq!(
        store.slot_continuation(),
        Some(&targets()),
        "the reconcile put back what the live park still owes",
    );
    assert!(
        store
            .retain_prefixed("synthetic/", &BTreeSet::new())
            .is_ok(),
        "and the same verbs still work once the fault is gone",
    );
    assert!(store.prior(SYNTHETIC).is_none(), "the prune landed");
    assert!(store.prior(KEY).is_some(), "and touched nothing else");
}

/// The adopted-target-set law, at the store. An empty selection RETAINS the
/// adopted set — that is a resume's ordinary materialise pass reporting
/// nothing payload-changing, not a claim about the parked run. A genuinely
/// different non-empty set refuses, and refuses without touching anything.
#[test]
fn an_adopted_continuation_is_retained_by_an_empty_pass_and_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(dir.path());
    let before = snapshot(&store);

    store
        .record_slot_continuation(SlotContinuation { targets: vec![] })
        .expect("an empty selection is not a disagreement");
    assert_eq!(store.slot_continuation(), Some(&targets()));
    assert_untouched("empty selection", &store, &before);

    store
        .record_slot_continuation(targets())
        .expect("the same set is simply retained");
    assert_untouched("matching selection", &store, &before);

    let error = store
        .record_slot_continuation(SlotContinuation {
            targets: vec![
                SlotTargetRecord {
                    group: "org.demo".into(),
                    name: "tools".into(),
                    version: "0.1.0".into(),
                },
                SlotTargetRecord {
                    group: "org.demo".into(),
                    name: "other".into(),
                    version: "9.9.9".into(),
                },
            ],
        })
        .expect_err("a different non-empty set is an invariant refusal");
    let rendered = error.to_string();
    assert!(
        rendered.contains(
            "this pass selected 2 payload-event target(s) but the run it adopted parked against \
             1; the recorded set is what the resume must rebuild, so it is never overwritten"
        ),
        "{rendered}",
    );
    assert!(
        !rendered.contains("  "),
        "source indentation must never reach the operator: {rendered}",
    );
    assert_untouched("mismatching selection", &store, &before);
}
