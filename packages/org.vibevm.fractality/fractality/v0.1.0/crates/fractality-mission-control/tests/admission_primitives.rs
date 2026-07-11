//! Admission/kill primitives on `AppState` (Phase 4): FIFO candidacy,
//! slot accounting, subtree walks, and pend/take kill delivery — pinned
//! against a real journal-backed state over a scratch home.

use fractality_core::ids::{PodId, RunId};
use fractality_core::journal::Event;
use fractality_core::packet::{BudgetSpec, WorkspaceMode};
use fractality_core::routing::{CapabilityClass, RoutingPolicy};
use fractality_core::run::{KillReason, RunRecord, RunState, UsageTotals};
use fractality_mission_control::Config;
use fractality_mission_control::admission::check_spawn_depth;
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

/// The depth guard (D-C3-3) charges a spawn against the parent class's
/// policy cap. Medium (default cap 1): a first-level child is admitted, a
/// second level is refused with a message naming the depth and the cap.
#[test]
fn depth_guard_medium_admits_one_level_and_refuses_two() {
    let policy = RoutingPolicy::default();
    assert!(
        check_spawn_depth(CapabilityClass::Medium, 0, &policy, 1).is_ok(),
        "a first-level child (depth 1) fits the medium cap"
    );
    let err = check_spawn_depth(CapabilityClass::Medium, 0, &policy, 2)
        .expect_err("depth 2 is past the medium cap");
    assert!(err.contains("depth 2"), "names the offending depth: {err}");
    assert!(err.contains("cap 1"), "names the cap: {err}");
}

/// Weak class (policy cap 0) is never a spawning root: any child is
/// refused, routing semantics — `max_depth = 0` means "no spawning", not
/// "unlimited" (the overload the wiring must keep straight).
#[test]
fn depth_guard_weak_never_spawns() {
    let policy = RoutingPolicy::default();
    let err = check_spawn_depth(CapabilityClass::Weak, 0, &policy, 1)
        .expect_err("weak may not spawn at all");
    assert!(err.contains("may not spawn"), "{err}");
}

/// Strong matches medium at the default cap (1); depth 2 needs the
/// experimental flag (a later slice) and is refused here.
#[test]
fn depth_guard_strong_default_cap_is_one() {
    let policy = RoutingPolicy::default();
    assert!(check_spawn_depth(CapabilityClass::Strong, 0, &policy, 1).is_ok());
    assert!(check_spawn_depth(CapabilityClass::Strong, 0, &policy, 2).is_err());
}

/// The parent's own `budget.max_depth` tightens the class ceiling but
/// never loosens it: an authored strong cap of 2 is clamped to 1 by a
/// budget of 1, and the message names the tightening bound.
#[test]
fn depth_guard_budget_tightens_but_never_loosens() {
    let strong_two = RoutingPolicy::from_toml_str(
        "schema = 1\n[class.strong]\nmax_depth = 2\n\
         allow_experimental_depth2 = true\nadvisor_enabled = true\n",
    )
    .expect("policy parses");
    // Without a budget, the authored table lets strong reach depth 2.
    assert!(check_spawn_depth(CapabilityClass::Strong, 0, &strong_two, 2).is_ok());
    // budget.max_depth = 1 clamps it back to one level.
    let err = check_spawn_depth(CapabilityClass::Strong, 1, &strong_two, 2)
        .expect_err("budget 1 tightens the cap to 1");
    assert!(err.contains("budget.max_depth = 1"), "{err}");
    // A budget looser than the class ceiling cannot raise it: default
    // strong cap is 1, so depth 2 is still refused with budget 5.
    let default_policy = RoutingPolicy::default();
    assert!(check_spawn_depth(CapabilityClass::Strong, 5, &default_policy, 2).is_err());
}
