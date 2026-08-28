use std::fs;
use std::path::Path;

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
fn linked_directory_non_utf8_and_over_cap_tasks_refuse_stably() {
    // Hard link — mandatory on the same temp filesystem.
    let linked = single_fixture();
    let key = "linked";
    write_state(&linked.root, &one_park(key));
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    let task_path = write_task(&linked.selected, &task, b"shared");
    fs::hard_link(&task_path, linked.root.join("second-name.md")).unwrap();
    let error = query(&linked).unwrap_err();
    assert!(matches!(error, LifecycleTasksError::TaskRead { .. }));
    assert!(error.to_string().contains("link") || error.to_string().contains("names"));

    // Symlink where the host grants creation. The mandatory hard-link arm
    // above is the non-skipping link proof on every supported test host.
    let symlinked = single_fixture();
    let key = "symlinked";
    write_state(&symlinked.root, &one_park(key));
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    let task_path = symlinked.selected.join(&task);
    fs::create_dir_all(task_path.parent().unwrap()).unwrap();
    let outside = symlinked.root.join("outside-task.md");
    fs::write(&outside, b"outside").unwrap();
    if link_file(&outside, &task_path) {
        assert!(matches!(
            query(&symlinked).unwrap_err(),
            LifecycleTasksError::TaskRead { .. }
        ));
    }

    // Directory occupant at the exact state-owned task name.
    let directory = single_fixture();
    let key = "directory";
    write_state(&directory.root, &one_park(key));
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    fs::create_dir_all(directory.selected.join(&task)).unwrap();
    assert!(matches!(
        query(&directory).unwrap_err(),
        LifecycleTasksError::TaskRead { .. }
    ));

    let binary = single_fixture();
    let key = "binary";
    write_state(&binary.root, &one_park(key));
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    write_task(&binary.selected, &task, &[0xff, 0xfe]);
    assert!(matches!(
        query(&binary).unwrap_err(),
        LifecycleTasksError::TaskNotUtf8 { .. }
    ));

    let huge = single_fixture();
    let key = "huge";
    write_state(&huge.root, &one_park(key));
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    write_task(&huge.selected, &task, &vec![b'x'; TASK_CAP + 1]);
    let error = query(&huge).unwrap_err();
    assert!(matches!(error, LifecycleTasksError::TaskRead { .. }));
    assert!(error.to_string().contains(&TASK_CAP.to_string()));
}

#[test]
fn exact_file_and_aggregate_caps_are_accepted_and_one_byte_over_refuses() {
    let exact_file = single_fixture();
    let key = "exact-file";
    write_state(&exact_file.root, &one_park(key));
    let task = crate::outbox_task_path(RUN_ID, key).unwrap();
    write_task(&exact_file.selected, &task, &vec![b'x'; TASK_CAP]);
    let report = query(&exact_file).unwrap();
    assert_eq!(report.tasks[0].document.len(), TASK_CAP);

    let aggregate = single_fixture();
    let first = "a-first";
    let second = "b-second";
    let state = parked_state(
        ".",
        &CHAIN,
        [
            (
                first.into(),
                delegated_record(first, "create", ExecutionRecordScope::Phase),
            ),
            (
                second.into(),
                delegated_record(second, "create", ExecutionRecordScope::Phase),
            ),
        ],
    );
    let state_bytes = write_state(&aggregate.root, &state);
    let second_len = AGGREGATE_CAP - state_bytes.len() - TASK_CAP;
    assert!(second_len <= TASK_CAP);
    let first_task = crate::outbox_task_path(RUN_ID, first).unwrap();
    let second_task = crate::outbox_task_path(RUN_ID, second).unwrap();
    write_task(&aggregate.selected, &first_task, &vec![b'a'; TASK_CAP]);
    let second_path = write_task(&aggregate.selected, &second_task, &vec![b'b'; second_len]);
    let report = query(&aggregate).unwrap();
    assert_eq!(
        state_bytes.len()
            + report
                .tasks
                .iter()
                .map(|task| task.document.len())
                .sum::<usize>(),
        AGGREGATE_CAP,
    );

    fs::write(&second_path, vec![b'b'; second_len + 1]).unwrap();
    let error = query(&aggregate).unwrap_err();
    match &error {
        LifecycleTasksError::TaskRead { budget, task, .. } => {
            assert_eq!(*budget, second_len);
            assert_eq!(task, &second_task);
        }
        other => panic!("aggregate overflow is a bounded task read: {other}"),
    }
    assert!(error.to_string().contains("aggregate budget"), "{error}");
}

#[test]
fn sixty_four_rows_are_total_and_sixty_five_refuse_before_task_reads() {
    let accepted = single_fixture();
    let rows = (0..MAX_DELEGATED_ROWS).map(|index| {
        let key = format!("row-{index:02}");
        let record = delegated_record(&key, "create", ExecutionRecordScope::Phase);
        (key, record)
    });
    let state = parked_state(".", &CHAIN, rows);
    write_state(&accepted.root, &state);
    for key in state.execution.keys() {
        let task = crate::outbox_task_path(RUN_ID, key).unwrap();
        write_task(&accepted.selected, &task, b"x");
    }
    let report = query(&accepted).unwrap();
    assert_eq!(report.status, LifecycleTasksStatus::Parked);
    assert_eq!(report.tasks.len(), MAX_DELEGATED_ROWS);

    let refused = single_fixture();
    let rows = (0..=MAX_DELEGATED_ROWS).map(|index| {
        let key = format!("row-{index:02}");
        let record = delegated_record(&key, "create", ExecutionRecordScope::Phase);
        (key, record)
    });
    write_state(&refused.root, &parked_state(".", &CHAIN, rows));
    // No task files exist: TaskMissing would prove the ceiling ran too late.
    let error = query(&refused).unwrap_err();
    assert!(matches!(
        error,
        LifecycleTasksError::TooManyRows {
            count: 65,
            cap: 64,
            ..
        }
    ));
}

#[test]
fn unsafe_state_never_masquerades_as_absent() {
    let hardlinked = single_fixture();
    write_state(&hardlinked.root, &idle_state(Some("."), "linked"));
    let path = state_path(&hardlinked.root);
    fs::hard_link(&path, hardlinked.root.join("state-second-name.toml")).unwrap();
    assert!(matches!(
        query(&hardlinked).unwrap_err(),
        LifecycleTasksError::State(_)
    ));

    let linked_ancestor = single_fixture();
    let outside = tempfile::tempdir().unwrap();
    write_state(outside.path(), &idle_state(Some("."), "outside"));
    if link_dir(
        &outside.path().join(".vibe"),
        &linked_ancestor.root.join(".vibe"),
    ) {
        assert!(matches!(
            query(&linked_ancestor).unwrap_err(),
            LifecycleTasksError::State(_)
        ));
    }
}

#[cfg(unix)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}
