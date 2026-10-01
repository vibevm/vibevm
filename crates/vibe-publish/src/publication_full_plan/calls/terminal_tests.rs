use super::*;
use crate::publication_full_plan::calls::terminal::{self, TerminalInspection};
use crate::publication_full_plan::{Inspection, cold};

fn complete_checkpoints() -> World {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!("sealed ReadyRun absent")
    };
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Next { step: 1 }
    );
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    drop(session);
    world
}

fn terminal_reader(world: &World) -> Reader {
    Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    }
}

#[test]
fn complete_call_chain_is_distinct_from_fake_partition_result_and_cold_attaches() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Ready { calls: 42 }
    );
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainReady { calls: 42, .. }
    ));
    let mut reader = terminal_reader(&world);
    assert_eq!(
        terminal::seal(&session, &mut reader).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    drop(session);
    // Dropping the caller's success response does not require another remote call.
    assert!(matches!(world.open(), SessionOpen::Completed { .. }));
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainCompleted { calls: 42, .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "result")
            .unwrap()
            .is_none()
    );
    let mut foreign_plan = world.tiny.plan.clone();
    foreign_plan.remote_generation_ref.push_str("-foreign");
    assert!(matches!(
        ReadyRun::open(
            &world.lease,
            &world.namespace,
            &world.scope,
            "noop-key",
            &world.tiny.subject,
            &foreign_plan,
            &world.expected,
            &world.home
        )
        .unwrap(),
        SessionOpen::RecoveryRequired { .. }
    ));
}

#[test]
fn missing_or_tampered_terminal_marker_is_uncertain_after_cold_attach() {
    for mode in 0..3 {
        let world = complete_checkpoints();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        assert_eq!(
            terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
            TerminalInspection::Completed { calls: 42 }
        );
        drop(session);
        let phase = if mode == 2 { "pending" } else { "result" };
        let path = world._root.path().join(format!(
            ".vibe/publication-operations/{}.full-plan-call-terminal-{phase}.json",
            world.run_id
        ));
        if mode == 1 {
            std::fs::write(&path, b"{\"tampered\":true}").unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
        assert!(matches!(
            cold::inspect_full_plan_cold_with_home(
                &world.lease,
                world.namespace.id(),
                &world.scope,
                "noop-key",
                &world.home
            )
            .unwrap(),
            Inspection::CallChainUncertain { .. }
        ));
    }
}

#[test]
fn eof_or_latest_drift_after_terminal_pending_never_seals_or_replays() {
    for latest_drift in [false, true] {
        let world = complete_checkpoints();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = terminal_reader(&world);
        if latest_drift {
            reader.latest_drift_from = Some(3);
        } else {
            reader.fail_at = Some(3);
        }
        assert_eq!(
            terminal::seal(&session, &mut reader).unwrap(),
            TerminalInspection::Uncertain
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "call-terminal-pending")
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "call-terminal-result")
                .unwrap()
                .is_none()
        );
        let reads = reader.reads;
        assert_eq!(
            terminal::seal(&session, &mut reader).unwrap(),
            TerminalInspection::Uncertain
        );
        assert_eq!(reader.reads, reads);
        drop(session);
        assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
    }
}

#[test]
fn changed_call_record_is_uncertain_instead_of_escaping_as_decode_error() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let assets = assets::decompose_asset_calls(
        &world.tiny.subject,
        &world.tiny.plan,
        &world.expected,
        &world.run_id,
    )
    .unwrap();
    let path = world._root.path().join(format!(
        ".vibe/publication-operations/{}.full-plan-{}.json",
        world.run_id,
        assets::asset_suffix(assets[0].ordinal, "landed")
    ));
    std::fs::write(path, b"{\"tampered\":true}").unwrap();
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Uncertain
    );
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn replaced_vault_root_cannot_cold_attach_a_terminal_result() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    drop(session);
    let vault = world._root.path().join(format!(
        ".vibe/publication-operations/{}.payloads",
        world.run_id
    ));
    let old = world._root.path().join("vault-retired");
    std::fs::rename(&vault, &old).unwrap();
    std::fs::create_dir(&vault).unwrap();
    assert!(matches!(world.open(), SessionOpen::RecoveryRequired { .. }));
    assert!(!matches!(
        cold::inspect_full_plan_cold_with_home(
            &world.lease,
            world.namespace.id(),
            &world.scope,
            "noop-key",
            &world.home
        )
        .unwrap(),
        Inspection::CallChainCompleted { .. }
    ));
}

#[test]
fn terminal_result_survives_api_exit_and_new_lease() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        terminal::seal(&session, &mut terminal_reader(&world)).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    drop(session);
    let World {
        tiny,
        _root,
        lease,
        namespace,
        scope,
        home,
        expected,
        snapshot: _,
        run_id: _,
    } = world;
    drop(home);
    drop(lease);
    let reopened = LifecycleLease::acquire(_root.path()).unwrap();
    let home = tiny.approval_home();
    assert!(matches!(
        cold::inspect_full_plan_cold_with_home(
            &reopened,
            namespace.id(),
            &scope,
            "noop-key",
            &home
        )
        .unwrap(),
        Inspection::CallChainCompleted { calls: 42, .. }
    ));
    assert!(matches!(
        ReadyRun::open(
            &reopened,
            &namespace,
            &scope,
            "noop-key",
            &tiny.subject,
            &tiny.plan,
            &expected,
            &home
        )
        .unwrap(),
        SessionOpen::Completed { .. }
    ));
}

#[test]
fn latest_drift_before_terminal_pending_writes_no_marker() {
    let world = complete_checkpoints();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = terminal_reader(&world);
    reader.latest_drift_from = Some(1);
    assert_eq!(
        terminal::seal(&session, &mut reader).unwrap(),
        TerminalInspection::Uncertain
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "call-terminal-pending")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Ready { calls: 42 }
    );
}
