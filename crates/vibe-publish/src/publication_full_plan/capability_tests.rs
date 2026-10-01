use super::prepare::{prepare_local_full_plan, trusted};
use super::tests::{FakeRemote, expected, prepared, scope};
use super::*;
use crate::publication_effect_plan::{CapturedPublicationRemote, plan_from_captured_remote};
use crate::publication_payload::vault::FinalPayloadBytes;
use crate::publication_payload::vault::stage_payload_vault_until;
use sha2::{Digest, Sha256};
use vibe_lifecycle::ReservationDecision;
use vibe_lifecycle::process::propose_run_id;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedReleasePresent, PublicationObservedTagPresent,
};
use vibe_wire::generated::publication_subject::{PublicationAsset, PublicationSmokeProbeStatus};

pub(super) struct TinyPayloadWorld {
    pub subject: PublicationSubject,
    pub plan: PublicationEffectPlan,
    pub output: tempfile::TempDir,
    pub settings: tempfile::TempDir,
    finals: [Vec<u8>; 4],
}

impl TinyPayloadWorld {
    pub fn new() -> Self {
        let (mut subject, old_plan) = prepared();
        let output = tempfile::tempdir().unwrap();
        let settings = tempfile::tempdir().unwrap();
        for target in &mut subject.targets {
            for asset in [
                &mut target.bundle,
                &mut target.bootstrap,
                &mut target.fragment,
            ] {
                let bytes = format!("fixture payload: {}\n", asset.name).into_bytes();
                set_asset(asset, &bytes);
                std::fs::write(output.path().join(&asset.name), bytes).unwrap();
            }
            target.components[0].size_bytes = target.bootstrap.size_bytes.clone();
            target.components[0].sha256 = target.bootstrap.sha256.clone();
        }
        for (probe, target) in subject.smoke.probes.iter_mut().zip(&subject.targets) {
            if matches!(
                probe.status,
                PublicationSmokeProbeStatus::Passed | PublicationSmokeProbeStatus::Failed
            ) {
                probe.tested_sha256 = Some(target.bundle.sha256.clone());
            }
        }
        let finals = [
            format!("fixture payload: {}\n", subject.aggregate.name).into_bytes(),
            format!("fixture payload: {}\n", subject.installers[0].asset.name).into_bytes(),
            format!("fixture payload: {}\n", subject.installers[1].asset.name).into_bytes(),
            format!("fixture payload: {}\n", subject.installers[2].asset.name).into_bytes(),
        ];
        set_asset(&mut subject.aggregate, &finals[0]);
        for (installer, bytes) in subject.installers.iter_mut().zip(&finals[1..]) {
            set_asset(&mut installer.asset, bytes);
        }
        let remote = CapturedPublicationRemote {
            generation_ref: old_plan.remote_generation_ref,
            tag: old_plan.observed_tag,
            release: old_plan.observed_release,
        };
        let plan = plan_from_captured_remote(&subject, &expected(), &remote)
            .unwrap()
            .plan;
        Self {
            subject,
            plan,
            output,
            settings,
            finals,
        }
    }

    pub fn finals(&self) -> FinalPayloadBytes<'_> {
        FinalPayloadBytes {
            aggregate: &self.finals[0],
            bash: &self.finals[1],
            powershell: &self.finals[2],
            cmd: &self.finals[3],
        }
    }

    pub fn seed_approval(&self, lease: &LifecycleLease, scope: &SubmissionScope) {
        let home =
            crate::publication_approval::ApprovalHome::open_for_test(self.settings.path()).unwrap();
        home.seed_mode_a_for_test(
            lease,
            scope,
            &self.subject,
            &self.plan,
            &expected(),
            "fixture-approval",
        )
        .unwrap();
    }

    pub fn approval_home(&self) -> crate::publication_approval::ApprovalHome {
        crate::publication_approval::ApprovalHome::open_for_test(self.settings.path()).unwrap()
    }
}

fn set_asset(asset: &mut PublicationAsset, bytes: &[u8]) {
    asset.size_bytes = bytes.len().to_string();
    asset.sha256 = format!("sha256:{:x}", Sha256::digest(bytes));
}

#[test]
fn e2_release_transition_requires_every_approved_successor_field() {
    let tiny = TinyPayloadWorld::new();
    let PublicationPartition::Release(step) = &tiny.plan.partitions[1] else {
        panic!()
    };
    let before = RemoteObservation {
        generation_ref: "before-release".into(),
        tag: PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
            reference: format!("refs/tags/{}", step.desired_tag),
            object_kind: "commit".into(),
            object_sha: step.desired_target_commit.clone(),
        })),
        release: PublicationObservedRelease::Absent(Box::new(
            vibe_wire::generated::publication_effect_plan::PublicationObservedReleaseAbsent {},
        )),
        latest_release_id: None,
        last: None,
    };
    let mut after = before.clone();
    after.generation_ref = "after-release".into();
    after.release =
        PublicationObservedRelease::Present(Box::new(PublicationObservedReleasePresent {
            assets: Vec::new(),
            draft: step.desired_draft,
            generation_ref: after.generation_ref.clone(),
            inventory_complete: true,
            release_id: "700".into(),
            tag: step.desired_tag.clone(),
            target_commitish: step.desired_target_commit.clone(),
            name: Some(step.desired_name.clone()),
            body: Some(step.desired_body.clone()),
            prerelease: step.desired_prerelease,
        }));
    assert!(transition::verifies(
        &before,
        &after,
        &tiny.plan.partitions[1]
    ));
    for field in ["target", "name", "body", "prerelease"] {
        let mut changed = after.clone();
        let PublicationObservedRelease::Present(row) = &mut changed.release else {
            panic!()
        };
        match field {
            "target" => row.target_commitish = "b".repeat(40),
            "name" => row.name = Some("foreign".into()),
            "body" => row.body = Some("foreign".into()),
            "prerelease" => row.prerelease = !row.prerelease,
            _ => unreachable!(),
        }
        assert!(
            !transition::verifies(&before, &changed, &tiny.plan.partitions[1]),
            "{field}"
        );
    }
}

#[test]
fn bad_vault_keeps_c1_pending_and_duplicate_never_restages() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected = scope();
    tiny.seed_approval(&lease, &selected);
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let missing_name = &tiny.subject.targets[0].bundle.name;
    let original = std::fs::read(tiny.output.path().join(missing_name)).unwrap();
    std::fs::remove_file(tiny.output.path().join(missing_name)).unwrap();
    let output = vibe_safefs::Project::open(tiny.output.path()).unwrap();
    let first = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "vault-bad",
        &tiny.subject,
        &tiny.plan,
        &expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    let Inspection::RecoveryRequired { run_id } = first else {
        panic!("missing upload bytes cannot reach Ready")
    };
    assert_eq!(
        SubmissionReservations::journal_admission(
            &lease,
            &namespace,
            &selected,
            "vault-bad",
            &run_id
        )
        .unwrap(),
        JournalAdmission::Pending,
    );
    assert!(journal::read_input(&lease, &run_id).unwrap().is_none());
    assert!(
        journal::read::<Intent>(&lease, &run_id, "intent")
            .unwrap()
            .is_none()
    );
    std::fs::write(tiny.output.path().join(missing_name), original).unwrap();
    let second = prepare_local_full_plan(
        &lease,
        &namespace,
        &selected,
        "vault-bad",
        &tiny.subject,
        &tiny.plan,
        &expected(),
        trusted(
            tiny.settings.path(),
            &lease,
            &selected,
            &tiny.subject,
            &tiny.plan,
            &output,
            tiny.finals(),
        ),
    )
    .unwrap();
    assert_eq!(
        second,
        Inspection::RecoveryRequired {
            run_id: run_id.clone()
        }
    );
    assert!(
        !root
            .path()
            .join(format!(".vibe/publication-operations/{run_id}.payloads"))
            .exists()
    );
}

#[test]
fn post_ready_vault_drift_downgrades_cold_inspection() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let selected = scope();
    let (token, run_id) = {
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        tiny.seed_approval(&lease, &selected);
        let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
        let output = vibe_safefs::Project::open(tiny.output.path()).unwrap();
        let first = prepare_local_full_plan(
            &lease,
            &namespace,
            &selected,
            "vault-drift",
            &tiny.subject,
            &tiny.plan,
            &expected(),
            trusted(
                tiny.settings.path(),
                &lease,
                &selected,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals(),
            ),
        )
        .unwrap();
        let Inspection::JournalReady { run_id } = first else {
            panic!("complete vault must reach Ready")
        };
        (namespace.id().to_owned(), run_id)
    };
    let staged = root.path().join(format!(
        ".vibe/publication-operations/{run_id}.payloads/{}",
        tiny.subject.targets[0].bundle.name,
    ));
    std::fs::write(staged, b"changed after Ready").unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(
        cold::inspect_full_plan_cold_with_home(
            &lease,
            &token,
            &selected,
            "vault-drift",
            &tiny.approval_home()
        )
        .unwrap(),
        Inspection::RecoveryRequired { run_id },
    );
}

#[test]
fn production_cold_inspector_never_accepts_test_only_no_vault_completion() {
    let root = tempfile::tempdir().unwrap();
    let (subject, plan) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    let first = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "fake-only",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    )
    .unwrap();
    let Inspection::Completed { run_id, .. } = first else {
        panic!("test fake must complete")
    };
    assert_eq!(
        inspect_full_plan_cold(&lease, namespace.id(), &selected, "fake-only").unwrap(),
        Inspection::RecoveryRequired { run_id },
    );
}

#[test]
fn interrupted_vault_stays_pending_and_same_key_does_not_stage_again() {
    let tiny = TinyPayloadWorld::new();
    let root = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    let selected_scope = scope();
    let namespace =
        SubmissionReservations::issue_namespace(&lease, &selected_scope, 60_000).unwrap();
    let selected = select(
        &lease,
        &selected_scope,
        &tiny.subject,
        &tiny.plan,
        &expected(),
        Some(("A", "fixture-approval")),
    )
    .unwrap();
    let run_id = propose_run_id(lease.root()).unwrap();
    assert!(matches!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &selected_scope,
            "interrupted",
            &selected.request,
            &selected.plan_bytes,
            &run_id,
        )
        .unwrap(),
        ReservationDecision::Created { .. }
    ));
    let output = vibe_safefs::Project::open(tiny.output.path()).unwrap();
    assert!(
        stage_payload_vault_until(
            &lease,
            &namespace,
            &selected_scope,
            "interrupted",
            &run_id,
            &tiny.subject,
            &expected().subject,
            &output,
            &tiny.finals(),
            3,
        )
        .is_err()
    );
    let vault_path = root
        .path()
        .join(format!(".vibe/publication-operations/{run_id}.payloads"));
    assert_eq!(std::fs::read_dir(&vault_path).unwrap().count(), 3);
    assert_eq!(
        prepare_local_full_plan(
            &lease,
            &namespace,
            &selected_scope,
            "interrupted",
            &tiny.subject,
            &tiny.plan,
            &expected(),
            trusted(
                tiny.settings.path(),
                &lease,
                &selected_scope,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals()
            ),
        )
        .unwrap(),
        Inspection::RecoveryRequired {
            run_id: run_id.clone()
        }
    );
    assert_eq!(std::fs::read_dir(vault_path).unwrap().count(), 3);
    assert_eq!(
        SubmissionReservations::journal_admission(
            &lease,
            &namespace,
            &selected_scope,
            "interrupted",
            &run_id,
        )
        .unwrap(),
        JournalAdmission::Pending,
    );
}

#[test]
fn nonempty_cleanup_refuses_before_c1_or_any_fake_remote_method() {
    let project = tempfile::tempdir().unwrap();
    let (subject, mut plan) = prepared();
    let index = plan
        .partitions
        .iter()
        .position(|step| matches!(step, PublicationPartition::Cleanup(_)))
        .unwrap();
    let PublicationPartition::Cleanup(cleanup) = &mut plan.partitions[index] else {
        unreachable!()
    };
    cleanup.assets.push(PublicationObservedAsset {
        asset_id: "999".into(),
        name: "stale.tmp".into(),
        size_bytes: "1".into(),
        sha256: format!("sha256:{}", "a".repeat(64)),
    });
    assert!(!transition::supports(&plan.partitions[index]));
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    fake.forbidden = true;
    let result = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    );
    assert!(
        matches!(result, Err(Error::Refused(reason)) if reason.contains(&format!("partition {index}")))
    );
    assert!(fake.calls.is_empty());
    assert!(
        lease
            .publication_journal_project()
            .dir_if_present(&[".vibe", "publication-operations"])
            .unwrap()
            .is_none()
    );
}

#[test]
fn replacement_release_without_old_id_deletion_proof_is_not_fake_admitted() {
    let project = tempfile::tempdir().unwrap();
    let (subject, _) = prepared();
    let plan: PublicationEffectPlan = serde_json::from_str(include_str!(
        "../../../../formats/corpora/publication_effect_plan/e2/valid/replace-known-draft.json"
    ))
    .unwrap();
    let index = plan
        .partitions
        .iter()
        .position(|step| matches!(step, PublicationPartition::Release(_)))
        .unwrap();
    assert!(!transition::supports(&plan.partitions[index]));
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    fake.forbidden = true;
    let result = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    );
    assert!(
        matches!(result, Err(Error::Refused(reason)) if reason.contains(&format!("partition {index}")))
    );
    assert!(fake.calls.is_empty());
}
