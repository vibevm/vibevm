use super::*;
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use crate::publication_full_plan::{
    RemoteObservation, prepare,
    tests::{expected, prepared, scope},
};
use vibe_lifecycle::SubmissionReservations;
use vibe_safefs::Project;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedReleasePresent, PublicationObservedTagPresent,
};

fn replacement() -> (PublicationSubject, PublicationEffectPlan) {
    let (subject, _) = prepared();
    let plan = serde_json::from_str(include_str!(
        "../../../../../formats/corpora/publication_effect_plan/e1/valid/replace-known-draft.json"
    ))
    .unwrap();
    (subject, plan)
}

struct Ready {
    root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    run_id: String,
    subject: PublicationSubject,
    plan: PublicationEffectPlan,
}

impl Ready {
    fn new(subject: PublicationSubject, plan: PublicationEffectPlan) -> Self {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let selected =
            super::super::select(&lease, &scope, &subject, &plan, &expected(), None).unwrap();
        let before = RemoteObservation {
            generation_ref: plan.remote_generation_ref.clone(),
            tag: plan.observed_tag.clone(),
            release: plan.observed_release.clone(),
            latest_release_id: None,
            last: None,
        };
        let prepared = prepare::prepare_journal_fake(
            &lease,
            &namespace,
            &scope,
            "call-key",
            &subject,
            &plan,
            &expected(),
            selected,
            || Ok(before),
        )
        .unwrap();
        let prepare::PreparedJournal::New { run_id, .. } = prepared else {
            panic!("C.1 was not fresh Ready")
        };
        Self {
            root,
            lease,
            namespace,
            scope,
            run_id,
            subject,
            plan,
        }
    }
    fn inspect(&self) -> CallInspection {
        inspect_calls(
            &self.lease,
            &self.namespace,
            &self.scope,
            "call-key",
            &self.run_id,
            &self.subject,
            &self.plan,
            &expected(),
        )
        .unwrap()
    }
}

struct FakeHost {
    current: CallObservation,
    calls: Vec<CallAction>,
    lost_at: Option<usize>,
    unchanged_at: Option<usize>,
    mismatched_release_generation_at: Option<usize>,
    drift_generation_on_observation: Option<usize>,
    pause_before_ordinal: Option<usize>,
    observations: usize,
    forbidden: bool,
}

impl FakeHost {
    fn new(plan: &PublicationEffectPlan) -> Self {
        Self {
            current: CallObservation {
                generation_ref: plan.remote_generation_ref.clone(),
                tag: plan.observed_tag.clone(),
                release: plan.observed_release.clone(),
            },
            calls: Vec::new(),
            lost_at: None,
            unchanged_at: None,
            mismatched_release_generation_at: None,
            drift_generation_on_observation: None,
            pause_before_ordinal: None,
            observations: 0,
            forbidden: false,
        }
    }
    fn observe(&mut self) -> CallObservation {
        assert!(!self.forbidden, "duplicate touched fake remote");
        self.observations += 1;
        if self.drift_generation_on_observation == Some(self.observations) {
            self.current.generation_ref.push_str("-foreign");
        }
        self.current.clone()
    }
    fn apply(&mut self, action: &CallAction) -> std::result::Result<String, ()> {
        assert!(!self.forbidden, "duplicate called fake remote");
        let index = self.calls.len();
        self.calls.push(action.clone());
        let receipt = match action {
            CallAction::RetainTag { .. } => panic!("RetainTag must not invoke a remote write"),
            CallAction::CreateTag { tag, commit } => {
                if self.unchanged_at != Some(index) {
                    self.current.tag =
                        PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                            reference: format!("refs/tags/{tag}"),
                            object_kind: "commit".into(),
                            object_sha: commit.clone(),
                        }));
                    self.current.generation_ref = format!("fake-generation-{}", index + 1);
                }
                "tag-created".into()
            }
            CallAction::DeletePriorRelease { release_id, .. } => {
                if self.unchanged_at != Some(index) {
                    self.current.release = PublicationObservedRelease::Absent(Box::new(
                        PublicationObservedReleaseAbsent {},
                    ));
                    self.current.generation_ref = format!("fake-generation-{}", index + 1);
                }
                format!("deleted:{release_id}")
            }
            CallAction::CreateDraft { tag, .. } => {
                if self.unchanged_at != Some(index) {
                    self.current.release = PublicationObservedRelease::Present(Box::new(
                        PublicationObservedReleasePresent {
                            assets: Vec::new(),
                            draft: true,
                            generation_ref: if self.mismatched_release_generation_at == Some(index)
                            {
                                "foreign-release-generation".into()
                            } else {
                                format!("fake-generation-{}", index + 1)
                            },
                            inventory_complete: true,
                            release_id: "700".into(),
                            tag: tag.clone(),
                        },
                    ));
                    self.current.generation_ref = format!("fake-generation-{}", index + 1);
                }
                "700".into()
            }
        };
        if self.lost_at == Some(index) {
            Err(())
        } else {
            Ok(receipt)
        }
    }
}

fn drive(world: &Ready, host: &mut FakeHost) -> Result<CallInspection> {
    let start = world.inspect();
    let CallInspection::Next { ordinal } = start else {
        return Ok(start);
    };
    let schedule = decompose(&world.subject, &world.plan, &expected())?;
    let mut prior_after = if ordinal == 0 {
        None
    } else {
        let landed = journal::read::<Landed>(
            &world.lease,
            &world.run_id,
            &suffix(ordinal - 1, "landed"),
        )?
        .ok_or_else(|| Error::Refused("resumed call has no prior Landed observation".into()))?;
        Some(landed.after)
    };
    for call in schedule.calls.iter().skip(ordinal) {
        if host.pause_before_ordinal == Some(call.ordinal) {
            return Ok(CallInspection::Next {
                ordinal: call.ordinal,
            });
        }
        let before = host.observe();
        if before.tag != call.expected_tag
            || before.release != call.expected_release
            || call.ordinal == 0 && before.generation_ref != world.plan.remote_generation_ref
            || prior_after.as_ref().is_some_and(|prior| prior != &before)
        {
            return Err(Error::Refused(format!(
                "call {} remote precondition drifted before effect",
                call.ordinal
            )));
        }
        let pending = Pending {
            format: PENDING_FORMAT.into(),
            run_id: world.run_id.clone(),
            call: call.clone(),
            expected_observation_sha256: sha(&serde_json::to_vec(&before)?),
            before: before.clone(),
        };
        if journal::publish(
            &world.lease,
            &world.run_id,
            &suffix(call.ordinal, "pending"),
            &pending,
        )
        .is_err()
        {
            return Ok(CallInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        let receipt = if matches!(call.action, CallAction::RetainTag { .. }) {
            "retained-read-only".to_owned()
        } else {
            match host.apply(&call.action) {
                Ok(receipt) => receipt,
                Err(()) => {
                    return Ok(CallInspection::Uncertain {
                        ordinal: call.ordinal,
                    });
                }
            }
        };
        let after = host.observe();
        if !verified_transition(call, &before, &after, &receipt) {
            return Ok(CallInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        let landed = Landed {
            format: LANDED_FORMAT.into(),
            run_id: world.run_id.clone(),
            pending_sha256: sha(&serde_json::to_vec(&pending)?),
            receipt_id: receipt,
            after_observation_sha256: sha(&serde_json::to_vec(&after)?),
            after: after.clone(),
        };
        if journal::publish(
            &world.lease,
            &world.run_id,
            &suffix(call.ordinal, "landed"),
            &landed,
        )
        .is_err()
        {
            return Ok(CallInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        prior_after = Some(after);
    }
    Ok(world.inspect())
}

#[test]
fn tag_and_release_calls_have_stable_ids_and_unavailable_asset_frontier() {
    let (subject, plan) = prepared();
    let schedule = decompose(&subject, &plan, &expected()).unwrap();
    assert_eq!(schedule.calls.len(), 2);
    assert!(matches!(
        schedule.calls[0].action,
        CallAction::CreateTag { .. }
    ));
    assert!(matches!(
        schedule.calls[1].action,
        CallAction::CreateDraft { .. }
    ));
    assert_eq!(schedule, decompose(&subject, &plan, &expected()).unwrap());
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    assert!(matches!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Unavailable { landed: 2, .. }
    ));
    assert_eq!(host.calls.len(), 2);
    assert!(world.root.path().is_dir());

    let (subject, plan) = replacement();
    let replace = decompose(&subject, &plan, &expected()).unwrap();
    assert_eq!(replace.calls.len(), 3);
    assert!(matches!(
        replace.calls[0].action,
        CallAction::RetainTag { .. }
    ));
    assert!(matches!(
        replace.calls[1].action,
        CallAction::DeletePriorRelease { .. }
    ));
    assert!(matches!(
        replace.calls[2].action,
        CallAction::CreateDraft { .. }
    ));
    assert_ne!(schedule.calls[0].call_id, replace.calls[0].call_id);
}

#[test]
fn out_of_order_call_record_never_opens_an_earlier_effect_frontier() {
    let (subject, plan) = prepared();
    let world = Ready::new(subject, plan);
    journal::publish(
        &world.lease,
        &world.run_id,
        &suffix(1, "pending"),
        &serde_json::json!({"foreign": "future call"}),
    )
    .unwrap();
    assert_eq!(world.inspect(), CallInspection::Uncertain { ordinal: 0 });
    let mut host = FakeHost::new(&world.plan);
    host.forbidden = true;
    assert_eq!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Uncertain { ordinal: 0 }
    );
    assert!(host.calls.is_empty());
}

#[test]
fn lost_delete_response_stops_before_create_and_duplicate_never_reinvokes() {
    let (subject, plan) = replacement();
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    host.lost_at = Some(0);
    assert_eq!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Uncertain { ordinal: 1 }
    );
    assert_eq!(host.calls.len(), 1);
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(1, "pending"))
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read::<Landed>(&world.lease, &world.run_id, &suffix(1, "landed"))
            .unwrap()
            .is_none()
    );
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(2, "pending"))
            .unwrap()
            .is_none()
    );
    host.forbidden = true;
    assert_eq!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Uncertain { ordinal: 1 }
    );
    assert_eq!(host.calls.len(), 1);
}

#[test]
fn generation_only_drift_after_delete_refuses_before_create_draft() {
    let (subject, plan) = replacement();
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    // RetainTag observes twice; Delete observes before/after; the fifth read
    // is CreateDraft's preflight. Only the generation changes there.
    host.drift_generation_on_observation = Some(5);
    assert!(matches!(drive(&world, &mut host), Err(Error::Refused(_))));
    assert_eq!(host.calls.len(), 1, "only DELETE may have been invoked");
    assert!(matches!(
        host.calls[0],
        CallAction::DeletePriorRelease { .. }
    ));
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(2, "pending"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn resumed_next_call_uses_retained_landed_generation() {
    let (subject, plan) = prepared();
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    host.pause_before_ordinal = Some(1);
    assert_eq!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Next { ordinal: 1 }
    );
    assert_eq!(host.calls.len(), 1);
    host.pause_before_ordinal = None;
    host.current.generation_ref.push_str("-foreign");
    assert!(matches!(drive(&world, &mut host), Err(Error::Refused(_))));
    assert_eq!(host.calls.len(), 1);
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(1, "pending"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn foreign_old_id_or_tag_refuses_before_mutating_call() {
    let (subject, plan) = replacement();
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    let PublicationObservedRelease::Present(old) = &mut host.current.release else {
        unreachable!()
    };
    old.release_id = "999".into();
    assert!(matches!(drive(&world, &mut host), Err(Error::Refused(_))));
    assert!(host.calls.is_empty());
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(0, "pending"))
            .unwrap()
            .is_none()
    );
    let mut host = FakeHost::new(&world.plan);
    let PublicationObservedTag::Present(tag) = &mut host.current.tag else {
        unreachable!()
    };
    tag.object_sha = "f".repeat(tag.object_sha.len());
    assert!(matches!(drive(&world, &mut host), Err(Error::Refused(_))));
    assert!(host.calls.is_empty());
}

#[test]
fn identical_objects_after_callback_do_not_launder_call_as_landed() {
    let (subject, plan) = prepared();
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    host.unchanged_at = Some(0);
    assert_eq!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Uncertain { ordinal: 0 }
    );
    assert!(
        journal::read::<Landed>(&world.lease, &world.run_id, &suffix(0, "landed"))
            .unwrap()
            .is_none()
    );
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(1, "pending"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn draft_release_generation_mismatch_stays_uncertain_without_landed_record() {
    let (subject, plan) = prepared();
    let world = Ready::new(subject, plan);
    let mut host = FakeHost::new(&world.plan);
    host.mismatched_release_generation_at = Some(1);
    assert_eq!(
        drive(&world, &mut host).unwrap(),
        CallInspection::Uncertain { ordinal: 1 }
    );
    assert_eq!(host.calls.len(), 2);
    assert!(
        journal::read::<Pending>(&world.lease, &world.run_id, &suffix(1, "pending"))
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read::<Landed>(&world.lease, &world.run_id, &suffix(1, "landed"))
            .unwrap()
            .is_none()
    );
    assert_eq!(world.inspect(), CallInspection::Uncertain { ordinal: 1 });
}

#[test]
fn production_call_frontier_refuses_vault_or_approval_drift_before_next() {
    for drift in ["vault", "approval"] {
        let tiny = TinyPayloadWorld::new();
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let selected_scope = scope();
        tiny.seed_approval(&lease, &selected_scope);
        let namespace =
            SubmissionReservations::issue_namespace(&lease, &selected_scope, 60_000).unwrap();
        let output = Project::open(tiny.output.path()).unwrap();
        let outcome = prepare::prepare_local_full_plan(
            &lease,
            &namespace,
            &selected_scope,
            "protected-calls",
            &tiny.subject,
            &tiny.plan,
            &expected(),
            prepare::trusted(
                tiny.settings.path(),
                &lease,
                &selected_scope,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals(),
            ),
        )
        .unwrap();
        let super::super::Inspection::JournalReady { run_id } = outcome else {
            panic!("production fixture not Ready")
        };
        let home = tiny.approval_home();
        assert_eq!(
            inspect_calls_with_home(
                &lease,
                &namespace,
                &selected_scope,
                "protected-calls",
                &run_id,
                &tiny.subject,
                &tiny.plan,
                &expected(),
                &home,
            )
            .unwrap(),
            CallInspection::Next { ordinal: 0 }
        );
        if drift == "vault" {
            std::fs::write(
                root.path().join(format!(
                    ".vibe/publication-operations/{run_id}.payloads/{}",
                    tiny.subject.targets[0].bundle.name,
                )),
                b"changed after Ready",
            )
            .unwrap();
        } else {
            let redeemed = tiny.settings.path().join("publication-approvals/redeemed");
            let marker = std::fs::read_dir(redeemed)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            std::fs::write(marker, b"changed redemption").unwrap();
        }
        assert!(
            matches!(
                inspect_calls_with_home(
                    &lease,
                    &namespace,
                    &selected_scope,
                    "protected-calls",
                    &run_id,
                    &tiny.subject,
                    &tiny.plan,
                    &expected(),
                    &home,
                ),
                Err(Error::Refused(_))
            ),
            "{drift} drift must not yield Next"
        );
    }
}
