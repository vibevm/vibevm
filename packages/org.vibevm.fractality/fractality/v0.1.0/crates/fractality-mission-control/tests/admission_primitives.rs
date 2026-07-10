//! Admission/kill primitives on `AppState` (Phase 4): FIFO candidacy,
//! slot accounting, subtree walks, and pend/take kill delivery — pinned
//! against a real journal-backed state over a scratch home.

use fractality_core::ids::{PodId, RunId};
use fractality_core::journal::Event;
use fractality_core::packet::{BudgetSpec, WorkspaceMode};
use fractality_core::run::{KillReason, RunRecord, RunState, UsageTotals};
use fractality_mission_control::Config;
use fractality_mission_control::state::{AppState, PodRuntime};

fn record(id: &str, profile: &str, spawn: bool, parent: Option<RunId>) -> RunRecord {
    RunRecord {
        run_id: id.parse().expect("fixed ulid"),
        title: "t".into(),
        state: RunState::Queued,
        profile: profile.into(),
        model: "big".into(),
        workspace_mode: WorkspaceMode::Dir,
        parent,
        origin_session: None,
        depth: 0,
        spawn_requested: spawn,
        budget: BudgetSpec::default(),
        node_id: "n".into(),
        run_dir: "runs/x".into(),
        created_ts_ms: 1,
        updated_ts_ms: 1,
        started_ts_ms: None,
        pod: None,
        worker_pid: None,
        exit_code: None,
        failure: None,
        kill_reason: None,
        usage: UsageTotals::default(),
        collected: None,
        question: None,
        answer: None,
    }
}

fn scratch_state(tag: &str) -> (AppState, camino::Utf8PathBuf) {
    let dir = std::env::temp_dir().join(format!("fractality-state-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&dir).expect("mkdir scratch home");
    let home = camino::Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir");
    let state = AppState::open(Config::new(home.clone())).expect("state opens");
    (state, home)
}

/// queued_candidates returns only spawn-requested runs in FIFO (ULID) order.
#[test]
fn queued_candidates_are_fifo_and_spawn_only() {
    let (state, home) = scratch_state("fifo");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA1", "glm", true, None)),
        })
        .expect("register r1");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA2", "glm", false, None)),
        })
        .expect("register r2");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA3", "glm", true, None)),
        })
        .expect("register r3");

    let candidates = state.queued_candidates();
    assert_eq!(candidates.len(), 2, "only spawn=true runs qualify");
    assert_eq!(
        candidates[0].run_id,
        "01ARZ3NDEKTSV4RRFFQ69G5FA1".parse::<RunId>().expect("id1"),
        "first spawn run comes first"
    );
    assert_eq!(
        candidates[1].run_id,
        "01ARZ3NDEKTSV4RRFFQ69G5FA3".parse::<RunId>().expect("id3"),
        "third spawn run comes second"
    );

    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// active_count counts only launched, non-terminal runs of the given profile.
#[test]
fn active_count_counts_launched_nonterminal_only() {
    let (state, home) = scratch_state("active");
    let r2: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA2".parse().expect("id2");
    let r3: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA3".parse().expect("id3");

    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA1", "glm", true, None)),
        })
        .expect("register r1");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA2", "glm", true, None)),
        })
        .expect("register r2");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA3", "glm", true, None)),
        })
        .expect("register r3");

    state
        .record(Event::State {
            run_id: r2,
            state: RunState::Starting,
            detail: None,
        })
        .expect("r2 starting");
    state
        .record(Event::State {
            run_id: r3,
            state: RunState::Starting,
            detail: None,
        })
        .expect("r3 starting");
    state
        .record(Event::Completed {
            run_id: r3,
            exit_code: Some(0),
        })
        .expect("r3 completed");

    assert_eq!(state.active_count("glm"), 1, "only r2 holds a slot");
    assert_eq!(state.active_count("other"), 0, "no runs on other profile");

    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// subtree_ids walks the call tree root-first; an unregistered root yields empty.
#[test]
fn subtree_ids_walks_the_tree_root_first() {
    let (state, home) = scratch_state("subtree");
    let root: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA1".parse().expect("root");
    let child1: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA2".parse().expect("child1");
    let child2: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA3".parse().expect("child2");
    let grand: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA4".parse().expect("grand");

    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA1", "glm", true, None)),
        })
        .expect("register root");
    state
        .record(Event::Registered {
            run: Box::new(record(
                "01ARZ3NDEKTSV4RRFFQ69G5FA2",
                "glm",
                true,
                Some(root),
            )),
        })
        .expect("register child1");
    state
        .record(Event::Registered {
            run: Box::new(record(
                "01ARZ3NDEKTSV4RRFFQ69G5FA3",
                "glm",
                true,
                Some(root),
            )),
        })
        .expect("register child2");
    state
        .record(Event::Registered {
            run: Box::new(record(
                "01ARZ3NDEKTSV4RRFFQ69G5FA4",
                "glm",
                true,
                Some(child1),
            )),
        })
        .expect("register grandchild");

    let subtree = state.subtree_ids(root);
    assert_eq!(subtree.len(), 4, "all four ids in the subtree");
    assert_eq!(subtree[0], root, "root is first");
    assert!(subtree.contains(&root), "contains root");
    assert!(subtree.contains(&child1), "contains child1");
    assert!(subtree.contains(&child2), "contains child2");
    assert!(subtree.contains(&grand), "contains grandchild");

    let unknown: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA9".parse().expect("unknown");
    assert!(
        state.subtree_ids(unknown).is_empty(),
        "unknown root is empty"
    );

    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// pend_kill arms a kill on the pod's runtime; take_pending_kill drains it once.
#[test]
fn pend_kill_round_trips_through_the_pod() {
    let (state, home) = scratch_state("pendkill");
    let run: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA1".parse().expect("run");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA1", "glm", true, None)),
        })
        .expect("register run");

    assert!(
        !state.pend_kill(run, KillReason::Manual),
        "no pod means no kill armed"
    );

    let pod: PodId = "01ARZ3NDEKTSV4RRFFQ69G5FB1".parse().expect("pod");
    state.upsert_pod(
        pod,
        PodRuntime {
            run_id: run,
            pod_pid: 4242,
            last_heartbeat_ms: 1,
            pending_kill: None,
        },
    );

    assert!(
        state.pend_kill(run, KillReason::Manual),
        "pod present means kill armed"
    );
    assert_eq!(
        state.take_pending_kill(pod),
        Some(KillReason::Manual),
        "first take delivers the kill"
    );
    assert_eq!(state.take_pending_kill(pod), None, "second take is empty");

    std::fs::remove_dir_all(home.as_std_path()).ok();
}

/// The first pend_kill reason wins; a later pend_kill does not overwrite it.
#[test]
fn first_kill_reason_wins() {
    let (state, home) = scratch_state("firstwins");
    let run: RunId = "01ARZ3NDEKTSV4RRFFQ69G5FA1".parse().expect("run");
    state
        .record(Event::Registered {
            run: Box::new(record("01ARZ3NDEKTSV4RRFFQ69G5FA1", "glm", true, None)),
        })
        .expect("register run");

    let pod: PodId = "01ARZ3NDEKTSV4RRFFQ69G5FB1".parse().expect("pod");
    state.upsert_pod(
        pod,
        PodRuntime {
            run_id: run,
            pod_pid: 4242,
            last_heartbeat_ms: 1,
            pending_kill: None,
        },
    );

    assert!(
        state.pend_kill(run, KillReason::Manual),
        "first pend_kill arms"
    );
    assert!(
        state.pend_kill(run, KillReason::Budget),
        "second pend_kill still returns true"
    );
    assert_eq!(
        state.take_pending_kill(pod),
        Some(KillReason::Manual),
        "first reason wins"
    );

    std::fs::remove_dir_all(home.as_std_path()).ok();
}
