//! Exact retained-image confirmation regressions, not a filesystem sweep.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use super::*;
use std::{cell::Cell, rc::Rc};

struct SyncHook;
impl Drop for SyncHook {
    fn drop(&mut self) {
        vibe_safefs::arm_before_retained_file_sync(None);
    }
}
fn hook(fail: bool) -> (SyncHook, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    vibe_safefs::arm_before_retained_file_sync(Some(Box::new(move || {
        observed.set(observed.get() + 1);
        fail
    })));
    (SyncHook, calls)
}
fn orphan(f: &Fixture) {
    let grant = f.grant();
    let store =
        vibe_safefs::ExternalStore::open_or_create_disjoint(&f.store, grant.project()).unwrap();
    let lock = store
        .open_and_lock_project(&grant.project().identity_token().unwrap())
        .unwrap();
    std::fs::create_dir_all(f.directory()).unwrap();
    std::fs::write(f.directory().join("before.bin"), BEFORE).unwrap();
    drop(lock);
}
fn snapshot(f: &Fixture) -> vibe_safefs::StableFileSnapshot {
    f.grant()
        .project()
        .read_file_snapshot_bounded("vibe.toml", 8 * 1024 * 1024)
        .unwrap()
        .unwrap()
}
fn no_target_effect(report: &OperationReport) {
    assert!(report.effects.iter().all(|effect| {
        !matches!(
            effect.kind,
            EffectKind::TargetPublication | EffectKind::TargetRestore
        ) || effect.state == EffectState::NotStarted
    }));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn matching_orphan_requires_actual_image_sync_and_failure_blocks_target() {
    let f = Fixture::new();
    orphan(&f);
    let before = snapshot(&f);
    let (guard, calls) = hook(true);
    let report = f.call(&f.apply());
    assert_eq!(
        calls.get(),
        1,
        "matching-image path did not reach the native held-file sync boundary"
    );
    assert_eq!(
        report.error.as_ref().unwrap().code,
        ApplicationErrorCode::SynchronizationFailed
    );
    no_target_effect(&report);
    assert_eq!(snapshot(&f), before);
    assert!(!f.directory().join("journal.json").exists());
    assert_eq!(
        std::fs::read(f.directory().join("before.bin")).unwrap(),
        BEFORE.as_bytes()
    );
    drop(guard);

    let (guard, calls) = hook(false);
    let report = f.call(&f.apply());
    assert!(
        calls.get() >= 1,
        "successful matching image was not actually synchronized"
    );
    assert_eq!(report.outcome, OperationReportOutcome::Completed);
    assert_eq!(
        report.guarantees.durability,
        GuaranteesDurability::Unconfirmed,
        "one image confirmation cannot authenticate every older namespace boundary"
    );
    drop(guard);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE",
    r = 3
)]
fn loaded_restore_requires_image_sync_but_inspect_and_replay_do_not() {
    let f = Fixture::new();
    assert_eq!(
        f.call(&f.apply()).outcome,
        OperationReportOutcome::Completed
    );
    let id = f.journal().unwrap().intent_id;
    let before = snapshot(&f);
    let stable = tree(f._temp.path());
    let (guard, calls) = hook(true);
    let inspection = f.call(&f.inspect());
    assert_eq!(inspection.outcome, OperationReportOutcome::Completed);
    assert_eq!(
        calls.get(),
        0,
        "inspection must not perform retained file synchronization"
    );
    assert_eq!(tree(f._temp.path()), stable);
    assert_eq!(f.call(&f.apply()).outcome, OperationReportOutcome::NoOp);
    assert_eq!(
        calls.get(),
        0,
        "exact terminal no-write replay needs no sync certification upgrade"
    );
    let report = f.call(&f.recover(&id, "restore"));
    assert_eq!(
        calls.get(),
        1,
        "loaded restore did not reach the native held-file sync boundary"
    );
    assert_eq!(
        report.error.as_ref().unwrap().code,
        ApplicationErrorCode::SynchronizationFailed
    );
    no_target_effect(&report);
    assert_eq!(snapshot(&f), before);
    assert_eq!(
        f.journal().unwrap().phase,
        ApplicationJournalPhase::Restoring
    );
    assert!(f.directory().join("before.bin").exists());
    drop(guard);

    let (guard, calls) = hook(false);
    assert_eq!(
        f.call(&f.recover(&id, "restore")).outcome,
        OperationReportOutcome::Completed
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(f.target(), BEFORE.as_bytes());
    drop(guard);
}
