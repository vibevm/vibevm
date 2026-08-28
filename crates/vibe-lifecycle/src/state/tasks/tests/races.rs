use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use vibe_wire::generated::lifecycle_state::ExecutionRecordScope;
use vibe_wire::generated::lifecycle_tasks::LifecycleTasksStatus;

use super::*;

fn one_park(key: &str) -> LifecycleState {
    parked_state(
        ".",
        &CHAIN,
        [(
            key.into(),
            delegated_record(key, "create", ExecutionRecordScope::Phase),
        )],
    )
}

#[test]
fn a_stable_missing_task_is_an_honest_refusal() {
    let fixture = single_fixture();
    let key = "missing";
    write_state(&fixture.root, &one_park(key));
    let error = query(&fixture).unwrap_err();
    assert!(matches!(error, LifecycleTasksError::TaskMissing { .. }));
}

#[test]
fn a_missing_task_completed_before_the_second_read_retries_to_idle() {
    let fixture = single_fixture();
    let key = "completed";
    write_state(&fixture.root, &one_park(key));
    let root = fixture.root.clone();
    arm_before_second_state_read(Some(Box::new(move |attempt| {
        if attempt == 0 {
            write_state(&root, &idle_state(Some("."), "completed"));
        }
    })));
    let report = query(&fixture).unwrap();
    arm_before_second_state_read(None);
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert_eq!(report.run.unwrap().started, "completed");
}

#[test]
fn malformed_state_is_provisional_when_repaired_and_typed_when_stable() {
    let repaired = single_fixture();
    let path = state_path(&repaired.root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not = [valid").unwrap();
    let root = repaired.root.clone();
    arm_before_second_state_read(Some(Box::new(move |attempt| {
        if attempt == 0 {
            write_state(&root, &idle_state(Some("."), "repaired"));
        }
    })));
    let report = query(&repaired).unwrap();
    arm_before_second_state_read(None);
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert_eq!(report.run.unwrap().started, "repaired");

    let stable = single_fixture();
    let path = state_path(&stable.root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"not = [valid").unwrap();
    let error = query(&stable).unwrap_err();
    assert!(matches!(error, LifecycleTasksError::State(_)));
    assert!(error.to_string().contains("malformed"), "{error}");
}

#[test]
fn three_changes_then_a_stable_fourth_attempt_succeeds() {
    let fixture = single_fixture();
    write_state(&fixture.root, &idle_state(Some("."), "v0"));
    let root = fixture.root.clone();
    let calls = Rc::new(Cell::new(0usize));
    let observed = Rc::clone(&calls);
    arm_before_second_state_read(Some(Box::new(move |attempt| {
        observed.set(observed.get() + 1);
        if attempt < 3 {
            write_state(&root, &idle_state(Some("."), &format!("v{}", attempt + 1)));
        }
    })));
    let report = query(&fixture).unwrap();
    arm_before_second_state_read(None);
    assert_eq!(calls.get(), 4, "initial attempt plus exactly three retries");
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert_eq!(report.run.unwrap().started, "v3");
}

#[test]
fn a_change_on_all_four_attempts_is_typed_unstable() {
    let fixture = single_fixture();
    write_state(&fixture.root, &idle_state(Some("."), "v0"));
    let root = fixture.root.clone();
    let calls = Rc::new(Cell::new(0usize));
    let observed = Rc::clone(&calls);
    arm_before_second_state_read(Some(Box::new(move |attempt| {
        observed.set(observed.get() + 1);
        write_state(&root, &idle_state(Some("."), &format!("v{}", attempt + 1)));
    })));
    let error = query(&fixture).unwrap_err();
    arm_before_second_state_read(None);
    assert_eq!(calls.get(), 4);
    assert!(matches!(
        error,
        LifecycleTasksError::UnstableSnapshot { attempts: 4 }
    ));
}

#[test]
fn state_disappearance_retries_then_linearizes_as_absent() {
    let fixture = single_fixture();
    write_state(&fixture.root, &idle_state(Some("."), "present"));
    let path = state_path(&fixture.root);
    arm_before_second_state_read(Some(Box::new(move |attempt| {
        if attempt == 0 {
            fs::remove_file(&path).unwrap();
        }
    })));
    let report = query(&fixture).unwrap();
    arm_before_second_state_read(None);
    assert_eq!(report.status, LifecycleTasksStatus::Absent);
    assert!(report.run.is_none());
}

#[test]
fn an_unsafe_second_state_read_is_immediate_not_retried() {
    let fixture = single_fixture();
    write_state(&fixture.root, &idle_state(Some("."), "safe-first"));
    let path = state_path(&fixture.root);
    let second_name = fixture.root.join("state-second-name.toml");
    let calls = Rc::new(Cell::new(0usize));
    let observed = Rc::clone(&calls);
    arm_before_second_state_read(Some(Box::new(move |_| {
        observed.set(observed.get() + 1);
        fs::hard_link(&path, &second_name).unwrap();
    })));
    let error = query(&fixture).unwrap_err();
    arm_before_second_state_read(None);
    assert!(matches!(error, LifecycleTasksError::State(_)));
    assert_eq!(
        calls.get(),
        1,
        "a current state safety refusal is immediate"
    );
}

#[test]
fn repeated_calls_reload_state_without_a_server_cache() {
    let fixture = single_fixture();
    let key = "reload";
    let state = one_park(key);
    write_state(&fixture.root, &state);
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    write_task(&fixture.selected, &task, b"do it");
    assert_eq!(
        query(&fixture).unwrap().status,
        LifecycleTasksStatus::Parked
    );

    write_state(&fixture.root, &idle_state(Some("."), "done"));
    let report = query(&fixture).unwrap();
    assert_eq!(report.status, LifecycleTasksStatus::Idle);
    assert!(report.tasks.is_empty());
}
