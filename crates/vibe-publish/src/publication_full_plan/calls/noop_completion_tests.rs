use super::super::noop_completion_fixture::*;
use super::*;
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use crate::publication_full_plan::prepare;
use crate::publication_full_plan::session::ReadyRun;
use crate::publication_full_plan::session::SessionOpen;
use crate::publication_full_plan::tests::{expected, scope};
use vibe_lifecycle::{LifecycleLease, SubmissionReservations};
use vibe_safefs::Project;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedRelease, PublicationObservedReleasePresent, PublicationObservedTag,
    PublicationObservedTagPresent,
};

#[test]
fn two_read_only_checkpoints_land_and_survive_cold_attach_without_completion_claim() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        inspect_proved(&session).unwrap(),
        NoOpInspection::Next { step: 0 }
    );
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
    let calls = schedule(&session).unwrap().unwrap();
    assert!(world.record(calls[0].ordinal, "landed").is_some());
    assert!(world.record(calls[1].ordinal, "landed").is_some());
    drop(session);
    let SessionOpen::Ready(cold) = world.open() else {
        panic!("not a full FinalResult")
    };
    assert_eq!(
        inspect_proved(&cold).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    let before = reader.reads;
    assert_eq!(
        drive_one_read_only(&cold, &mut reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    assert_eq!(
        reader.reads, before,
        "cold attach replayed read-only checkpoint"
    );
}

#[test]
fn drift_before_cleanup_refuses_without_pending() {
    let world = World::new();
    world.snapshot.borrow_mut().release.as_mut().unwrap().body = Some("foreign".into());
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    };
    assert!(matches!(
        drive_one_read_only(&session, &mut reader),
        Err(Error::Refused(_))
    ));
    assert!(world.record(calls[0].ordinal, "pending").is_none());
}

#[test]
fn out_of_order_or_post_pending_remote_drift_cannot_advance() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    journal::publish(
        &world.lease,
        &world.run_id,
        &suffix(calls[1].ordinal, "pending"),
        &serde_json::json!({"foreign": "future finalize"}),
    )
    .unwrap();
    assert_eq!(
        inspect_proved(&session).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert!(world.record(calls[0].ordinal, "pending").is_none());

    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: Some(4),
        latest_drift_from: None,
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert!(world.record(calls[0].ordinal, "pending").is_some());
    assert!(world.record(calls[0].ordinal, "landed").is_none());
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn latest_drift_after_noop_pending_never_lands_or_replays() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: Some(3),
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert!(world.record(calls[0].ordinal, "pending").is_some());
    assert!(world.record(calls[0].ordinal, "landed").is_none());
    let reads = reader.reads;
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert_eq!(reader.reads, reads);
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn eof_or_lost_reply_after_pending_is_uncertain_and_cold_attach_never_replays() {
    for failed_step in 0..2 {
        let world = World::new();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = Reader {
            snapshot: world.snapshot.clone(),
            reads: 0,
            fail_at: None,
            drift_at: None,
            latest_drift_from: None,
        };
        if failed_step == 1 {
            assert_eq!(
                drive_one_read_only(&session, &mut reader).unwrap(),
                NoOpInspection::Next { step: 1 }
            );
        }
        let calls = schedule(&session).unwrap().unwrap();
        reader.fail_at = Some(reader.reads + 3); // after Pending, first of two post reads
        assert_eq!(
            drive_one_read_only(&session, &mut reader).unwrap(),
            NoOpInspection::Uncertain { step: failed_step }
        );
        assert!(
            world
                .record(calls[failed_step].ordinal, "pending")
                .is_some()
        );
        assert!(world.record(calls[failed_step].ordinal, "landed").is_none());
        drop(session);
        let SessionOpen::Uncertain { .. } = world.open() else {
            panic!("pending was not retained")
        };
        assert!(world.record(calls[1].ordinal, "landed").is_none());
    }
}

#[path = "terminal_tests.rs"]
mod terminal_tests;

#[path = "owner_tests.rs"]
mod owner_tests;
