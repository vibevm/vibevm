use vibe_wire::generated::lifecycle_state::ExecutionRecordScope;
use vibe_wire::generated::lifecycle_tasks::{LifecycleTasksStatus, PendingTaskScope};

use super::*;

#[test]
fn absent_returns_immediately_and_creates_nothing() {
    let fixture = single_fixture();
    arm_before_second_state_read(Some(Box::new(|_| {
        panic!("absence owes no second state read")
    })));
    let report = query(&fixture).unwrap();
    arm_before_second_state_read(None);
    assert_eq!(report.status, LifecycleTasksStatus::Absent);
    assert!(report.run.is_none());
    assert!(report.tasks.is_empty());
    assert!(!fixture.root.join(".vibe").exists());
}

#[test]
fn current_legacy_and_foreign_idle_states_all_report_idle_exactly() {
    let current = single_fixture();
    write_state(&current.root, &idle_state(Some("."), "current"));
    let report = query(&current).unwrap();
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert_eq!(report.run.as_ref().unwrap().selected.as_deref(), Some("."));
    assert!(report.tasks.is_empty());

    let legacy = single_fixture();
    let mut state = idle_state(None, "legacy");
    state.run.run_id = None;
    write_state(&legacy.root, &state);
    let report = query(&legacy).unwrap();
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert_eq!(report.run.as_ref().unwrap().selected, None);
    assert_eq!(report.run.as_ref().unwrap().run_id, None);

    // No delegated row means there is no node-relative task to misread: every
    // sibling may observe idle while the exact stored header remains visible.
    let foreign = member_fixture("members/b");
    write_state(&foreign.root, &idle_state(Some("members/a"), "from-a"));
    let report = query(&foreign).unwrap();
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert_eq!(
        report.run.as_ref().unwrap().selected.as_deref(),
        Some("members/a")
    );
}

#[test]
fn parked_phase_and_slot_tasks_follow_chain_order_not_key_order() {
    let fixture = member_fixture("members/a");
    let create_key = "a-create";
    let install_key = "z-install";
    let state = parked_state(
        "members/a",
        &CHAIN,
        [
            (
                create_key.into(),
                delegated_record(create_key, "create", ExecutionRecordScope::Phase),
            ),
            (
                install_key.into(),
                delegated_record(install_key, "install", ExecutionRecordScope::Slot),
            ),
        ],
    );
    write_state(&fixture.root, &state);
    for (key, body) in [
        (create_key, b"create".as_slice()),
        (install_key, b"install"),
    ] {
        write_task(
            &fixture.selected,
            &crate::outbox_task_path(RUN_ID, key).unwrap(),
            body,
        );
    }

    let report = query(&fixture).unwrap();
    assert_eq!(report.status, LifecycleTasksStatus::Parked);
    assert_eq!(report.tasks.len(), 2);
    assert_eq!(report.tasks[0].execution, install_key);
    assert_eq!(report.tasks[0].scope, PendingTaskScope::Slot);
    assert_eq!(report.tasks[0].document, "install");
    assert_eq!(report.tasks[1].execution, create_key);
    assert_eq!(report.tasks[1].scope, PendingTaskScope::Phase);
    assert_eq!(report.tasks[1].document, "create");
    let run = report.run.expect("parked carries run");
    assert_eq!(run.run_id.as_deref(), Some(RUN_ID));
    assert_eq!(run.selected.as_deref(), Some("members/a"));
}

#[test]
fn unknown_and_duplicate_phases_have_a_total_deterministic_order() {
    let fixture = single_fixture();
    let known = "z-known";
    let unknown = "a-unknown";
    let state = parked_state(
        ".",
        &["create", "create"],
        [
            (
                unknown.into(),
                delegated_record(unknown, "future", ExecutionRecordScope::Phase),
            ),
            (
                known.into(),
                delegated_record(known, "create", ExecutionRecordScope::Phase),
            ),
        ],
    );
    write_state(&fixture.root, &state);
    for key in [known, unknown] {
        write_task(
            &fixture.selected,
            &crate::outbox_task_path(RUN_ID, key).unwrap(),
            key.as_bytes(),
        );
    }
    let report = query(&fixture).unwrap();
    assert_eq!(report.tasks[0].execution, known, "first chain index wins");
    assert_eq!(report.tasks[1].execution, unknown, "unknown sorts last");
}

#[test]
fn orphan_files_and_foreign_run_directories_are_invisible() {
    let fixture = single_fixture();
    let key = "owned";
    let state = parked_state(
        ".",
        &CHAIN,
        [(
            key.into(),
            delegated_record(key, "create", ExecutionRecordScope::Phase),
        )],
    );
    write_state(&fixture.root, &state);
    let owned = crate::outbox_task_path(RUN_ID, key).unwrap();
    write_task(&fixture.selected, &owned, b"owned");
    write_task(
        &fixture.selected,
        ".vibe/agentic/outbox/ffffffffffffffffffffffffffffffff/task-orphan.md",
        b"orphan",
    );
    let report = query(&fixture).unwrap();
    assert_eq!(report.tasks.len(), 1);
    assert_eq!(report.tasks[0].path, owned);
}

#[test]
fn foreign_live_park_wins_before_task_open_and_bad_owned_path_is_state_error() {
    let fixture = member_fixture("members/b");
    let key = "owned-by-a";
    let state = parked_state(
        "members/a",
        &CHAIN,
        [(
            key.into(),
            delegated_record(key, "create", ExecutionRecordScope::Phase),
        )],
    );
    write_state(&fixture.root, &state);
    // Deliberately do NOT write the task: reading it would yield TaskMissing.
    let error = query(&fixture).unwrap_err();
    assert!(matches!(
        error,
        LifecycleTasksError::ForeignPark {
            ref stored,
            ref selected,
            ..
        } if stored == "members/a" && selected == "members/b"
    ));
    assert!(!fixture.selected.join(".vibe").exists());

    let local = single_fixture();
    let mut state = parked_state(
        ".",
        &CHAIN,
        [(
            key.into(),
            delegated_record(key, "create", ExecutionRecordScope::Phase),
        )],
    );
    state.execution.get_mut(key).unwrap().tasks[0] = "wrong/path.md".into();
    write_state(&local.root, &state);
    let error = query(&local).unwrap_err();
    assert!(matches!(error, LifecycleTasksError::State(_)));
    assert!(
        error.to_string().contains("owns"),
        "the central task-path invariant fired: {error}"
    );
    assert!(!local.root.join("wrong").exists());
}
