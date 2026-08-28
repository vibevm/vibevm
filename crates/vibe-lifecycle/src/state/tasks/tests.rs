//! Focused REDs for the bounded optimistic hosted-task projection.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, ExecutionRecordStatus, LifecycleState, SlotContinuation,
    SlotTargetRecord, StateArtifact, StateRun,
};

use super::*;

#[path = "tests/basic.rs"]
mod basic;
#[path = "tests/limits.rs"]
mod limits;
#[path = "tests/races.rs"]
mod races;

pub(super) const RUN_ID: &str = "00112233445566778899aabbccddeeff";
pub(super) const CHAIN: [&str; 3] = ["validate", "install", "create"];

pub(super) struct Fixture {
    pub(super) _dir: TempDir,
    pub(super) root: PathBuf,
    pub(super) selected: PathBuf,
}

pub(super) fn single_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'demo'\nversion = '0.1.0'\n",
    )
    .unwrap();
    Fixture {
        root: dir.path().to_path_buf(),
        selected: dir.path().to_path_buf(),
        _dir: dir,
    }
}

pub(super) fn member_fixture(selected: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'mono'\nversion = '0.1.0'\n\n\
         [workspace]\nmembers = ['members/a', 'members/b']\n",
    )
    .unwrap();
    for member in ["a", "b"] {
        let root = dir.path().join("members").join(member);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("vibe.toml"),
            format!(
                "[package]\ngroup = 'org.demo'\nname = '{member}'\nkind = 'flow'\nversion = '0.1.0'\n"
            ),
        )
        .unwrap();
    }
    Fixture {
        root: dir.path().to_path_buf(),
        selected: dir.path().join(selected),
        _dir: dir,
    }
}

pub(super) fn idle_state(selected: Option<&str>, started: &str) -> LifecycleState {
    LifecycleState {
        schema: 1,
        run: StateRun {
            chain: CHAIN.iter().map(|phase| (*phase).to_string()).collect(),
            requested: "create".into(),
            started: started.into(),
            compile_trace: false,
            run_id: Some(RUN_ID.into()),
            selected: selected.map(str::to_string),
            slot_continuation: None,
        },
        execution: BTreeMap::new(),
    }
}

pub(super) fn delegated_record(
    key: &str,
    phase: &str,
    scope: ExecutionRecordScope,
) -> ExecutionRecord {
    ExecutionRecord {
        artifacts: vec![StateArtifact {
            id: format!("output:{key}"),
            kind: "file".into(),
            path: format!("out/{}.txt", key.replace(['/', '#'], "-")),
        }],
        duration_ms: 0,
        fingerprint: format!("sha256:{key}"),
        phase: phase.into(),
        status: ExecutionRecordStatus::Delegated,
        scope: Some(scope),
        tasks: vec![crate::outbox_task_path(RUN_ID, key).unwrap()],
    }
}

pub(super) fn parked_state(
    selected: &str,
    chain: &[&str],
    rows: impl IntoIterator<Item = (String, ExecutionRecord)>,
) -> LifecycleState {
    let execution: BTreeMap<_, _> = rows.into_iter().collect();
    let slot = execution
        .values()
        .any(|record| record.scope == Some(ExecutionRecordScope::Slot));
    LifecycleState {
        schema: 1,
        run: StateRun {
            chain: chain.iter().map(|phase| (*phase).to_string()).collect(),
            requested: "create".into(),
            started: "2026-08-28T00:00:00Z".into(),
            compile_trace: false,
            run_id: Some(RUN_ID.into()),
            selected: Some(selected.into()),
            slot_continuation: slot.then(|| SlotContinuation {
                targets: vec![SlotTargetRecord {
                    group: "org.demo".into(),
                    name: "target".into(),
                    version: "1.0.0".into(),
                }],
            }),
        },
        execution,
    }
}

pub(super) fn state_path(root: &Path) -> PathBuf {
    root.join(LifecycleStateStore::FILE)
}

pub(super) fn write_state(root: &Path, state: &LifecycleState) -> Vec<u8> {
    let path = state_path(root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = toml::to_string_pretty(state).unwrap().into_bytes();
    fs::write(path, &bytes).unwrap();
    bytes
}

pub(super) fn write_task(selected_root: &Path, task: &str, bytes: &[u8]) -> PathBuf {
    let path = selected_root.join(task);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

pub(super) fn query(fixture: &Fixture) -> Result<LifecycleTasks, LifecycleTasksError> {
    pending_hosted_tasks(&fixture.selected)
}
