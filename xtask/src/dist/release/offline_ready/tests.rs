use super::*;

use std::path::PathBuf;

use vibe_publish::publication_effect_plan::{
    CapturedPublicationRemote, PublicationEffectPlanExpectations,
};
use vibe_publish::publication_subject::PublicationSubjectExpectations;
use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::dist::capture::{
    PreparedPublicationComposition, PreparedSubjectCapture, PublicationVisibilityChoice,
    PublishedReplacementChoice, plan_captured_subject,
};
use crate::dist::snapshot::GitIdentity;

fn corpus(family: &str, category: &str, name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../formats/corpora")
            .join(family)
            .join("e1")
            .join(category)
            .join(format!("{name}.json")),
    )
    .unwrap()
}

fn expected() -> PublicationEffectPlanExpectations<'static> {
    PublicationEffectPlanExpectations {
        subject: PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
        host: "github.com",
        owner: "vibevm",
        repository: "vibevm",
        publish: false,
        allow_replace_published: false,
    }
}

fn composition_from_corpus(subject: PublicationSubject) -> PreparedPublicationComposition {
    let plan: PublicationEffectPlan = serde_json::from_str(&corpus(
        "publication_effect_plan",
        "valid",
        "new-draft-complete",
    ))
    .unwrap();
    let remote = CapturedPublicationRemote {
        generation_ref: plan.remote_generation_ref,
        tag: plan.observed_tag,
        release: plan.observed_release,
    };
    let subject_digest = publication_subject_digest(&subject, &expected().subject).unwrap();
    // Corpus-only fixture: the capture type is the production carrier for
    // independently witnessed bytes; this first seam tests semantic gates.
    let capture = PreparedSubjectCapture {
        identity: GitIdentity {
            commit: subject.source.commit.clone(),
            tree: subject.source.tree.clone(),
            source_date_epoch: "1".into(),
        },
        subject,
        subject_digest,
        aggregate_bytes: Vec::new(),
    };
    plan_captured_subject(
        capture,
        &remote,
        PublicationVisibilityChoice::RetainDraft,
        PublishedReplacementChoice::Refuse,
    )
    .unwrap()
}

fn ready_composition() -> PreparedPublicationComposition {
    let subject: PublicationSubject = serde_json::from_str(&corpus(
        "publication_subject",
        "valid",
        "complete-five-targets",
    ))
    .unwrap();
    composition_from_corpus(subject)
}

#[derive(Default)]
struct FakeHost {
    calls: Vec<String>,
}
impl OfflineReleaseHost for FakeHost {
    fn tag(&mut self, value: &PublicationPartitionTag) -> Result<()> {
        self.calls.push(format!("tag:{}", value.tag));
        Ok(())
    }
    fn release(&mut self, value: &PublicationPartitionRelease) -> Result<()> {
        self.calls.push(format!("release:{}", value.desired_tag));
        Ok(())
    }
    fn platform_assets(&mut self, value: &PublicationPartitionPlatformAssets) -> Result<()> {
        self.calls.push(format!("platform:{}", value.target));
        Ok(())
    }
    fn cleanup(&mut self, _value: &PublicationPartitionCleanup) -> Result<()> {
        self.calls.push("cleanup".into());
        Ok(())
    }
    fn final_assets(&mut self, _value: &PublicationPartitionFinalAssets) -> Result<()> {
        self.calls.push("final-assets".into());
        Ok(())
    }
    fn finalize(&mut self, _value: &PublicationPartitionFinalize) -> Result<()> {
        self.calls.push("finalize".into());
        Ok(())
    }
}

#[test]
fn complete_captured_subject_and_apply_ready_plan_precede_every_fake_effect() {
    let composition = ready_composition();
    let mut host = FakeHost::default();
    drive_offline_ready(&composition, &expected(), &mut host).unwrap();
    let mut ordered = vec![
        format!("tag:v{}", composition.capture.subject.version),
        format!("release:v{}", composition.capture.subject.version),
    ];
    ordered.extend(
        composition
            .capture
            .subject
            .targets
            .iter()
            .map(|target| format!("platform:{}", target.target)),
    );
    ordered.extend(["cleanup".into(), "final-assets".into(), "finalize".into()]);
    assert_eq!(host.calls, ordered);
    assert_eq!(host.calls.len(), composition.plan.partitions.len());
}

#[test]
fn missing_target_and_stale_plan_digest_refuse_with_zero_fake_calls() {
    let mut composition = ready_composition();
    composition.capture.subject.targets.pop();
    let mut host = FakeHost::default();
    assert!(drive_offline_ready(&composition, &expected(), &mut host).is_err());
    assert!(host.calls.is_empty());

    let mut composition = ready_composition();
    composition.plan_digest = format!("sha256:{}", "0".repeat(64));
    assert!(drive_offline_ready(&composition, &expected(), &mut host).is_err());
    assert!(host.calls.is_empty());
}

#[test]
fn skipped_smoke_is_inspectable_but_cannot_call_any_fake_remote_method() {
    let subject: PublicationSubject = serde_json::from_str(&corpus(
        "publication_subject",
        "readiness-invalid",
        "skipped-smoke",
    ))
    .unwrap();
    let composition = composition_from_corpus(subject);
    let mut host = FakeHost::default();
    assert!(drive_offline_ready(&composition, &expected(), &mut host).is_err());
    assert!(host.calls.is_empty());
}
