use super::*;
use crate::publication_full_plan::calls::asset_driver::AssetCallTransport;
use crate::publication_full_plan::calls::driver::CallTransport;
use crate::publication_full_plan::calls::owner::{self, CallRef, OwnerStep, Phase};
use crate::publication_payload::vault::VaultAssetStream;

struct NoWrites;
impl CallTransport for NoWrites {
    fn create_tag(
        &self,
        _: &str,
    ) -> std::result::Result<crate::github_release::TagResponseEvidence, String> {
        panic!("prefix write selected unexpectedly")
    }
    fn delete_release(
        &self,
        _: u64,
    ) -> std::result::Result<crate::github_release::DeleteReleaseResponseEvidence, String> {
        panic!("release deletion selected unexpectedly")
    }
    fn create_draft(
        &self,
        _: &crate::github_release::DraftCallFields,
    ) -> std::result::Result<crate::github_release::DraftResponseEvidence, String> {
        panic!("draft creation selected unexpectedly")
    }
}
impl AssetCallTransport for NoWrites {
    fn upload(
        &self,
        _: u64,
        _: &assets::PlannedAssetCall,
        _: VaultAssetStream,
    ) -> std::result::Result<u64, String> {
        panic!("asset upload selected unexpectedly")
    }
    fn rename(&self, _: u64, _: &assets::PlannedAssetCall) -> std::result::Result<u64, String> {
        panic!("asset rename selected unexpectedly")
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

#[test]
fn dispatcher_selects_one_checkpoint_then_terminal_per_invocation() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let prefix_len = crate::publication_full_plan::calls::decompose(
        &world.tiny.subject,
        &world.tiny.plan,
        &world.expected,
    )
    .unwrap()
    .calls
    .len();
    let first = prefix_len + 38;
    assert_eq!(
        owner::inspect_one(&session).unwrap(),
        OwnerStep::Ready {
            run_id: world.run_id.clone(),
            next: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            }
        }
    );
    let mut reader = reader(&world);
    let no_writes = NoWrites;
    assert_eq!(
        owner::drive_one_fake(&session, &mut reader, &no_writes, &no_writes).unwrap(),
        OwnerStep::Advanced {
            run_id: world.run_id.clone(),
            attempted: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            },
            next: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first + 1
            }
        }
    );
    assert!(world.record(first, "landed").is_some());
    assert!(world.record(first + 1, "pending").is_none());
    assert_eq!(
        owner::drive_one_fake(&session, &mut reader, &no_writes, &no_writes).unwrap(),
        OwnerStep::Advanced {
            run_id: world.run_id.clone(),
            attempted: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first + 1
            },
            next: CallRef {
                phase: Phase::Terminal,
                ordinal: first + 2
            }
        }
    );
    assert_eq!(
        owner::drive_one_fake(&session, &mut reader, &no_writes, &no_writes).unwrap(),
        OwnerStep::Completed {
            run_id: world.run_id.clone(),
            calls: first + 2,
            sealed_now: true
        }
    );
    let reads = reader.reads;
    assert_eq!(
        owner::drive_one_fake(&session, &mut reader, &no_writes, &no_writes).unwrap(),
        OwnerStep::Completed {
            run_id: world.run_id.clone(),
            calls: first + 2,
            sealed_now: false
        }
    );
    assert_eq!(reader.reads, reads, "cold completion retried a remote call");
}

#[test]
fn dispatcher_stops_on_uncertain_checkpoint_without_next_effect() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let first = 2 + 38;
    let mut reader = reader(&world);
    reader.fail_at = Some(3); // first post-Pending epoch
    let no_writes = NoWrites;
    assert_eq!(
        owner::drive_one_fake(&session, &mut reader, &no_writes, &no_writes).unwrap(),
        OwnerStep::Uncertain {
            run_id: world.run_id.clone(),
            at: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            },
            attempted: Some(CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            })
        }
    );
    let reads = reader.reads;
    assert_eq!(
        owner::drive_one_fake(&session, &mut reader, &no_writes, &no_writes).unwrap(),
        OwnerStep::Uncertain {
            run_id: world.run_id.clone(),
            at: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            },
            attempted: None
        }
    );
    assert_eq!(reader.reads, reads);
    assert!(world.record(first + 1, "pending").is_none());
}

#[test]
fn dispatcher_selects_prefix_on_fresh_sealed_run() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    tiny.seed_approval(&lease, &scope);
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let expected = expected();
    let output = Project::open(tiny.output.path()).unwrap();
    assert!(matches!(
        prepare::prepare_local_full_plan(
            &lease,
            &namespace,
            &scope,
            "owner-fresh",
            &tiny.subject,
            &tiny.plan,
            &expected,
            prepare::trusted(
                tiny.settings.path(),
                &lease,
                &scope,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals()
            )
        )
        .unwrap(),
        crate::publication_full_plan::Inspection::JournalReady { .. }
    ));
    let home = tiny.approval_home();
    let SessionOpen::Ready(session) = ReadyRun::open(
        &lease,
        &namespace,
        &scope,
        "owner-fresh",
        &tiny.subject,
        &tiny.plan,
        &expected,
        &home,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(
        owner::inspect_one(&session).unwrap(),
        OwnerStep::Ready {
            run_id: session.run_id.clone(),
            next: CallRef {
                phase: Phase::TagRelease,
                ordinal: 0
            }
        }
    );
}

#[test]
fn post_step_local_inspection_failure_is_typed_uncertain_without_second_call() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let first = 2 + 38;
    let mut reader = reader(&world);
    let no_writes = NoWrites;
    let intent = world._root.path().join(format!(
        ".vibe/publication-operations/{}.full-plan-intent.json",
        world.run_id
    ));
    let status =
        owner::drive_one_fake_with_hook(&session, &mut reader, &no_writes, &no_writes, || {
            std::fs::write(&intent, b"{\"tampered\":true}").unwrap();
        })
        .unwrap();
    assert_eq!(
        status,
        OwnerStep::Uncertain {
            run_id: world.run_id.clone(),
            at: CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            },
            attempted: Some(CallRef {
                phase: Phase::Checkpoint,
                ordinal: first
            }),
        }
    );
    assert!(world.record(first, "landed").is_some());
    assert!(world.record(first + 1, "pending").is_none());
}

#[test]
fn dispatcher_selects_first_asset_after_two_landed_prefix_calls() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let scope = scope();
    tiny.seed_approval(&lease, &scope);
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    let expected = expected();
    let output = Project::open(tiny.output.path()).unwrap();
    prepare::prepare_local_full_plan(
        &lease,
        &namespace,
        &scope,
        "owner-asset",
        &tiny.subject,
        &tiny.plan,
        &expected,
        prepare::trusted(
            tiny.settings.path(),
            &lease,
            &scope,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let home = tiny.approval_home();
    let SessionOpen::Ready(session) = ReadyRun::open(
        &lease,
        &namespace,
        &scope,
        "owner-asset",
        &tiny.subject,
        &tiny.plan,
        &expected,
        &home,
    )
    .unwrap() else {
        panic!()
    };
    let run_id = session.run_id.clone();
    drop(session);
    let schedule =
        crate::publication_full_plan::calls::decompose(&tiny.subject, &tiny.plan, &expected)
            .unwrap();
    assert_eq!(schedule.calls.len(), 2);
    let mut before = crate::publication_full_plan::calls::CallObservation {
        generation_ref: tiny.plan.remote_generation_ref.clone(),
        tag: tiny.plan.observed_tag.clone(),
        release: tiny.plan.observed_release.clone(),
        latest_release: tiny.plan.observed_latest_release.clone(),
    };
    for call in &schedule.calls {
        let pending = crate::publication_full_plan::calls::Pending {
            format: crate::publication_full_plan::calls::PENDING_FORMAT.into(),
            run_id: run_id.clone(),
            call: call.clone(),
            expected_observation_sha256: sha(&serde_json::to_vec(&before).unwrap()),
            before: before.clone(),
        };
        journal::publish(
            &lease,
            &run_id,
            &crate::publication_full_plan::calls::suffix(call.ordinal, "pending"),
            &pending,
        )
        .unwrap();
        let mut after = before.clone();
        after.generation_ref = format!("prefix-{}", call.ordinal);
        let receipt = match &call.action {
            crate::publication_full_plan::calls::CallAction::CreateTag { tag, commit } => {
                after.tag =
                    PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                        reference: format!("refs/tags/{tag}"),
                        object_kind: "commit".into(),
                        object_sha: commit.clone(),
                    }));
                "tag-created".to_owned()
            }
            crate::publication_full_plan::calls::CallAction::CreateDraft {
                tag,
                target_commit,
                name,
                body,
                prerelease,
            } => {
                after.release = PublicationObservedRelease::Present(Box::new(
                    PublicationObservedReleasePresent {
                        assets: vec![],
                        draft: true,
                        generation_ref: after.generation_ref.clone(),
                        inventory_complete: true,
                        release_id: "700".into(),
                        tag: tag.clone(),
                        target_commitish: target_commit.clone(),
                        name: Some(name.clone()),
                        body: Some(body.clone()),
                        prerelease: *prerelease,
                    },
                ));
                "700".to_owned()
            }
            _ => panic!("fresh-draft prefix expected"),
        };
        assert!(crate::publication_full_plan::calls::verified_transition(
            call, &before, &after, &receipt
        ));
        let landed = crate::publication_full_plan::calls::Landed {
            format: crate::publication_full_plan::calls::LANDED_FORMAT.into(),
            run_id: run_id.clone(),
            pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
            receipt_id: receipt,
            after_observation_sha256: sha(&serde_json::to_vec(&after).unwrap()),
            after: after.clone(),
        };
        journal::publish(
            &lease,
            &run_id,
            &crate::publication_full_plan::calls::suffix(call.ordinal, "landed"),
            &landed,
        )
        .unwrap();
        before = after;
    }
    let SessionOpen::Ready(session) = ReadyRun::open(
        &lease,
        &namespace,
        &scope,
        "owner-asset",
        &tiny.subject,
        &tiny.plan,
        &expected,
        &home,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(
        owner::inspect_one(&session).unwrap(),
        OwnerStep::Ready {
            run_id,
            next: CallRef {
                phase: Phase::Asset,
                ordinal: 2
            }
        }
    );
}

#[path = "owner_loopback_tests.rs"]
mod owner_loopback_tests;
