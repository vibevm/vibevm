use super::super::noop_completion_fixture::{Reader, World};
use super::super::owner::{self, OwnerStep, Phase};
use super::super::terminal::{self, TerminalInspection};
use super::*;
use crate::github_release::*;
use crate::publication_full_plan::remote_observation::RemoteSnapshot;
use crate::publication_full_plan::session::{ReadyRun, SessionOpen};
use crate::publication_full_plan::{Inspection, cold};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use vibe_lifecycle::LifecycleLease;

struct NoWrites;
impl super::super::driver::CallTransport for NoWrites {
    fn create_tag(&self, _: &str) -> std::result::Result<TagResponseEvidence, String> {
        panic!("prefix")
    }
    fn delete_release(&self, _: u64) -> std::result::Result<DeleteReleaseResponseEvidence, String> {
        panic!("delete")
    }
    fn create_draft(
        &self,
        _: &DraftCallFields,
    ) -> std::result::Result<DraftResponseEvidence, String> {
        panic!("draft")
    }
}
impl super::super::asset_driver::AssetCallTransport for NoWrites {
    fn upload(
        &self,
        _: u64,
        _: &assets::PlannedAssetCall,
        _: crate::publication_payload::vault::VaultAssetStream,
    ) -> std::result::Result<u64, String> {
        panic!("upload")
    }
    fn rename(&self, _: u64, _: &assets::PlannedAssetCall) -> std::result::Result<u64, String> {
        panic!("rename")
    }
}
struct Fake {
    snapshot: Rc<RefCell<RemoteSnapshot>>,
    calls: Cell<usize>,
    change: &'static str,
}
impl FinalizePort for Fake {
    fn finalize(
        &self,
        id: &str,
        action: &PublicationPartitionFinalizeAction,
        latest: &PublicationPartitionFinalizeMakeLatest,
    ) -> std::result::Result<String, UncertainResponse> {
        assert_eq!(id, "700");
        assert_eq!(*action, PublicationPartitionFinalizeAction::Publish);
        assert_eq!(*latest, PublicationPartitionFinalizeMakeLatest::Legacy);
        self.calls.set(self.calls.get() + 1);
        let mut row = self.snapshot.borrow_mut();
        row.release.as_mut().unwrap().draft = false;
        match self.change {
            "lost" => return Err(UncertainResponse),
            "response" => return Ok("701".into()),
            "empty-response" => return Ok(String::new()),
            "body" => row.release.as_mut().unwrap().body = Some("foreign".into()),
            "asset" => row.assets.as_mut().unwrap()[0].id += 1000,
            "missing-assets" => row.assets = None,
            "latest-prior" => {
                row.latest = LatestReleaseObservation::Present {
                    release_id: 101,
                    tag: "v1.0.0".into(),
                }
            }
            "latest-new" => {
                row.latest = LatestReleaseObservation::Present {
                    release_id: 700,
                    tag: "v1.1.0".into(),
                }
            }
            "latest-foreign" => {
                row.latest = LatestReleaseObservation::Present {
                    release_id: 900,
                    tag: "v2.0.0".into(),
                }
            }
            "latest-absent" => row.latest = LatestReleaseObservation::Absent,
            _ => {}
        }
        Ok(id.into())
    }
}
fn reader(world: &World) -> Reader {
    Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    }
}
fn fake(world: &World, change: &'static str) -> Fake {
    Fake {
        snapshot: world.snapshot.clone(),
        calls: Cell::new(0),
        change,
    }
}
fn cleanup(session: &ReadyRun<'_>, reader: &mut Reader) {
    assert_eq!(
        noop_completion::drive_one_read_only(session, reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
}
fn raw(world: &World, ordinal: usize, phase: &str) -> Option<Vec<u8>> {
    journal::read_bytes(&world.lease, &world.run_id, &suffix(ordinal, phase)).unwrap()
}
#[test]
fn exact_publish_owner_seals_and_source_less_cold_attach_never_replays() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!("ready")
    };
    let mut reader = reader(&world);
    let fake = fake(&world, "");
    assert!(
        matches!(owner::drive_one_fake_finalize(&session, &mut reader, &NoWrites, &NoWrites, &fake).unwrap(),
        OwnerStep::Advanced { next, .. } if next.phase == Phase::Finalize)
    );
    assert_eq!(fake.calls.get(), 0);
    assert!(
        matches!(owner::drive_one_fake_finalize(&session, &mut reader, &NoWrites, &NoWrites, &fake).unwrap(),
        OwnerStep::Advanced { next, .. } if next.phase == Phase::Terminal)
    );
    assert_eq!(fake.calls.get(), 1);
    assert!(matches!(
        owner::drive_one_fake_finalize(&session, &mut reader, &NoWrites, &NoWrites, &fake).unwrap(),
        OwnerStep::Completed {
            calls: 42,
            sealed_now: true,
            ..
        }
    ));
    let reads = reader.reads;
    assert!(matches!(
        owner::drive_one_fake_finalize(&session, &mut reader, &NoWrites, &NoWrites, &fake).unwrap(),
        OwnerStep::Completed {
            sealed_now: false,
            ..
        }
    ));
    assert_eq!(reader.reads, reads);
    assert_eq!(fake.calls.get(), 1);
    drop(session);
    let World {
        tiny,
        _root,
        lease,
        namespace,
        scope,
        home,
        expected,
        ..
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
fn lost_response_despite_published_state_stays_uncertain_without_replay() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = reader(&world);
    cleanup(&session, &mut reader);
    let fake = fake(&world, "lost");
    let ordinal = ordinal(&session).unwrap();
    assert_eq!(
        drive_one(&session, &mut reader, &fake).unwrap(),
        FinalizeInspection::Uncertain
    );
    assert!(!world.snapshot.borrow().release.as_ref().unwrap().draft);
    assert!(raw(&world, ordinal, "pending").is_some());
    assert!(raw(&world, ordinal, "landed").is_none());
    assert_eq!(
        drive_one(&session, &mut reader, &fake).unwrap(),
        FinalizeInspection::Uncertain
    );
    assert_eq!(fake.calls.get(), 1);
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Uncertain
    );
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}
#[test]
fn changed_latest_never_lands_including_same_prior_id_new_successor_and_foreign() {
    for change in [
        "latest-prior",
        "latest-new",
        "latest-foreign",
        "latest-absent",
    ] {
        let world = World::publish();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = reader(&world);
        cleanup(&session, &mut reader);
        let fake = fake(&world, change);
        assert_eq!(
            drive_one(&session, &mut reader, &fake).unwrap(),
            FinalizeInspection::Uncertain,
            "{change}"
        );
        assert!(raw(&world, ordinal(&session).unwrap(), "landed").is_none());
        assert_eq!(fake.calls.get(), 1);
    }
}
#[test]
fn metadata_inventory_and_response_identity_are_necessary() {
    for change in [
        "body",
        "asset",
        "missing-assets",
        "response",
        "empty-response",
    ] {
        let world = World::publish();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = reader(&world);
        cleanup(&session, &mut reader);
        let fake = fake(&world, change);
        assert_eq!(
            drive_one(&session, &mut reader, &fake).unwrap(),
            FinalizeInspection::Uncertain,
            "{change}"
        );
        assert!(raw(&world, ordinal(&session).unwrap(), "landed").is_none());
    }
}
#[test]
fn unknown_after_read_is_pending_only_and_cold_uncertain() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = reader(&world);
    cleanup(&session, &mut reader);
    reader.fail_at = Some(reader.reads + 3);
    let fake = fake(&world, "");
    assert_eq!(
        drive_one(&session, &mut reader, &fake).unwrap(),
        FinalizeInspection::Uncertain
    );
    assert_eq!(fake.calls.get(), 1);
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}
#[test]
fn before_drift_refuses_without_callback_or_pending() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = reader(&world);
    cleanup(&session, &mut reader);
    world.snapshot.borrow_mut().release.as_mut().unwrap().body = Some("foreign".into());
    let fake = fake(&world, "");
    assert!(matches!(
        drive_one(&session, &mut reader, &fake),
        Err(Error::Refused(_))
    ));
    assert_eq!(fake.calls.get(), 0);
    assert!(raw(&world, ordinal(&session).unwrap(), "pending").is_none());
}
#[test]
fn absent_transport_refuses_before_pending_or_attempt_and_cannot_infer_completion() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = reader(&world);
    cleanup(&session, &mut reader);
    world.snapshot.borrow_mut().release.as_mut().unwrap().draft = false;
    let reads = reader.reads;
    assert!(matches!(
        owner::drive_one_fake(&session, &mut reader, &NoWrites, &NoWrites).unwrap(),
        OwnerStep::Unavailable {
            phase: Phase::Finalize,
            attempted: None,
            ..
        }
    ));
    assert_eq!(reader.reads, reads);
    assert!(raw(&world, ordinal(&session).unwrap(), "pending").is_none());
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::NotReady
    );
}
#[test]
fn missing_or_tampered_finalize_cannot_retain_a_terminal_completion() {
    for (phase, remove) in [
        ("pending", true),
        ("landed", true),
        ("pending", false),
        ("landed", false),
    ] {
        let world = World::publish();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = reader(&world);
        cleanup(&session, &mut reader);
        let fake = fake(&world, "");
        assert!(matches!(
            drive_one(&session, &mut reader, &fake).unwrap(),
            FinalizeInspection::Landed { .. }
        ));
        assert_eq!(
            terminal::seal_inner(&session, &mut reader).unwrap(),
            TerminalInspection::Completed { calls: 42 }
        );
        let path = world._root.path().join(format!(
            ".vibe/publication-operations/{}.full-plan-{}.json",
            world.run_id,
            suffix(ordinal(&session).unwrap(), phase)
        ));
        if remove {
            std::fs::remove_file(path).unwrap();
        } else {
            std::fs::write(path, b"{}").unwrap();
        }
        assert_eq!(
            terminal::inspect_proved(&session).unwrap(),
            TerminalInspection::Uncertain
        );
        assert!(matches!(
            owner::drive_one_fake(&session, &mut reader, &NoWrites, &NoWrites).unwrap(),
            OwnerStep::Uncertain {
                attempted: None,
                ..
            }
        ));
        assert_eq!(fake.calls.get(), 1);
        drop(session);
        assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
    }
}
#[test]
fn finalize_record_before_cleanup_is_out_of_order_uncertain() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    journal::publish(
        &world.lease,
        &world.run_id,
        &suffix(ordinal(&session).unwrap(), "pending"),
        &serde_json::json!({"foreign":true}),
    )
    .unwrap();
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Uncertain
    );
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn callback_replacing_pending_cannot_write_landed_or_replay() {
    let world = World::publish();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = reader(&world);
    cleanup(&session, &mut reader);
    let ordinal = ordinal(&session).unwrap();
    struct ReplacePending<'a> {
        fake: Fake,
        world: &'a World,
        ordinal: usize,
    }
    impl FinalizePort for ReplacePending<'_> {
        fn finalize(
            &self,
            id: &str,
            action: &PublicationPartitionFinalizeAction,
            latest: &PublicationPartitionFinalizeMakeLatest,
        ) -> std::result::Result<String, UncertainResponse> {
            let response = self.fake.finalize(id, action, latest)?;
            let path = self.world._root.path().join(format!(
                ".vibe/publication-operations/{}.full-plan-{}.json",
                self.world.run_id,
                suffix(self.ordinal, "pending")
            ));
            std::fs::write(path, b"{}").unwrap();
            Ok(response)
        }
    }
    let port = ReplacePending {
        fake: fake(&world, ""),
        world: &world,
        ordinal,
    };
    assert_eq!(
        drive_one(&session, &mut reader, &port).unwrap(),
        FinalizeInspection::Uncertain
    );
    assert!(
        raw(&world, ordinal, "landed").is_none(),
        "changed Pending acquired Landed"
    );
    assert_eq!(
        drive_one(&session, &mut reader, &port).unwrap(),
        FinalizeInspection::Uncertain
    );
    assert_eq!(port.fake.calls.get(), 1);
}

#[test]
fn retain_draft_terminal_rejects_unselected_finalize_history() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let mut reader = reader(&world);
    assert_eq!(
        noop_completion::drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Next { step: 1 }
    );
    assert_eq!(
        noop_completion::drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    assert_eq!(
        terminal::seal_inner(&session, &mut reader).unwrap(),
        TerminalInspection::Completed { calls: 42 }
    );
    let cleanup = noop_completion::schedule(&session).unwrap().unwrap()[0].ordinal;
    journal::publish(
        &world.lease,
        &world.run_id,
        &suffix(cleanup + 1, "pending"),
        &serde_json::json!({"foreign":true}),
    )
    .unwrap();
    assert_eq!(
        terminal::inspect_proved(&session).unwrap(),
        TerminalInspection::Uncertain
    );
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}
