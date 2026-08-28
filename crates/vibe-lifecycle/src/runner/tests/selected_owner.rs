//! The selected-ownership seam across the REAL engine: a park authored by
//! one workspace member puts its task under that member's root while the
//! workspace-root state names the member — and a sibling's identical
//! command is the typed foreign-park refusal (PROP-054
//! `##REF-AGENT-RESUME`, R7.4 A6). Kept beside the hosted-branch fixtures
//! rather than inside them (600-line cell budget).

use std::fs;

use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::{
    KEY, LifecycleRun, RUN_ID, lease, metadata_for_node, project_fixture, runtime, state,
    world_fixture,
};
use crate::ExecutionReuse;
use crate::agent::tests::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, row};
use crate::execution::HandlerExecution;
use crate::select_run_identity;

/// Two members of one workspace can present the SAME requested/chain tuple,
/// so the lower seam of selected ownership is pinned here: a park authored
/// by member A puts the task under A's own node root while the
/// workspace-root state names `selected = "members/a"`, the sibling's
/// identical command is the typed [`crate::LifecycleStateError::ForeignPark`]
/// — before any allocation, with zero provider calls and B's tree untouched —
/// and the owning member's own resume still adopts the exact identity.
#[test]
fn a_park_belongs_to_its_node_and_a_sibling_never_adopts_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let member_a = root.join("members/a");
    let member_b = root.join("members/b");
    // A real workspace root manifest declaring both members, so the fixture
    // is what its narrative says it is. The lower seam still tests the
    // MANUALLY supplied selected input — no discovery runs here.
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname='ws'\nversion='0.1.0'\n\n[workspace]\nmembers = ['members/a', \
         'members/b']\n",
    )
    .unwrap();
    for member in [&member_a, &member_b] {
        fs::create_dir_all(member).unwrap();
        fs::write(
            member.join("vibe.toml"),
            "[project]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
    }

    // Park from member A: the lease (and so the state) is at the WORKSPACE
    // root; the task is published under A's own node root.
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let lease = lease(&root);
    let mut run = LifecycleRun::begin(
        lease.clone(),
        project_fixture(&member_a),
        world_fixture(&member_a),
        metadata_for_node(RUN_ID, false, "members/a"),
        vec!["validate".into(), "install".into(), "create".into()],
    )
    .unwrap();
    let handler = HandlerExecution::from_row(&row(TWO_OUTPUTS, PROMPT));
    let rt = runtime(&backend);
    let parked = run
        .execute_one(&handler, "create", ExecutionReuse::FreshnessAware, &rt)
        .unwrap();
    assert_eq!(parked.status, ExecutionRecordStatus::Delegated);
    drop(run);

    let parked_state = state(&root);
    assert_eq!(
        parked_state.run.selected.as_deref(),
        Some("members/a"),
        "the workspace-root state names the node that authored the park",
    );
    let task = parked_state.execution[KEY].tasks[0].clone();
    assert!(
        member_a.join(&task).is_file(),
        "the parked task lives under the owning member",
    );
    assert!(
        !member_b.join(".vibe").exists(),
        "the sibling's tree is untouched"
    );

    // The sibling's IDENTICAL command: a typed ownership refusal naming both
    // nodes and the exact parked run — before allocation, spending nothing.
    let chain = vec!["validate".into(), "install".into(), "create".into()];
    let error = select_run_identity(
        &lease,
        &member_b,
        "create",
        &chain,
        "members/b",
        RunAgentMode::Agent,
        false,
        false,
        "2026-08-26T02:00:00Z".into(),
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
        "the refusal names both nodes and the exact parked run: {error}",
    );
    assert!(
        !member_b.join(".vibe").exists(),
        "a foreign refusal allocates nothing under the sibling",
    );
    assert_eq!(backend.calls(), 0);
    // The parked state is exactly as found — the refusal superseded nothing.
    let after = state(&root);
    assert_eq!(after.run.run_id.as_deref(), Some(RUN_ID));
    assert_eq!(after.run.selected.as_deref(), Some("members/a"));
    assert_eq!(after.execution[KEY].tasks, [task]);

    // The owning member's own resume adopts its exact identity.
    let identity = select_run_identity(
        &lease,
        &member_a,
        "create",
        &chain,
        "members/a",
        RunAgentMode::Agent,
        false,
        false,
        "2026-08-26T03:00:00Z".into(),
    )
    .unwrap();
    assert!(identity.adopted);
    assert_eq!(identity.run_id, RUN_ID);
}
