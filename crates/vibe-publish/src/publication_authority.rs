//! Pure decisions over one exact distribution publication plan.
//!
//! A decision is not a remote effect capability. The caller must establish
//! record provenance from an owner-controlled source, redeem a receipt at most
//! once in the publication owner's durable journal, and reobserve remote
//! preconditions before each effect. No record in this module contains a token.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PUBLICATION-JOURNAL-BOUNDARY");

use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_effect_plan::{
    PublicationEffectPlanExpectations, publication_effect_plan_digest,
    require_apply_ready_effect_plan,
};

/// Exact fields repeated in an authority record for inspection and checked
/// against the validated plan. `plan_sha256` still binds every partition and
/// observed remote object, including fields not repeated here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationAuthorityScope {
    pub plan_sha256: String,
    pub subject_sha256: String,
    pub host: String,
    pub owner: String,
    pub repository: String,
    pub principal: String,
    pub remote_generation_ref: String,
    pub smoke_policy_id: String,
    pub smoke_policy_epoch: u32,
    pub publish: bool,
    pub allow_replace_published: bool,
}

/// One owner-approved, exact batch. Persistence and one-use redemption are
/// responsibilities of the publication owner's M-16 pre-effect journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationApprovalReceipt {
    pub approval_id: String,
    pub scope: PublicationAuthorityScope,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
}

/// A bounded test policy. A valid instance does not enable Mode B by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationModeBPolicy {
    pub policy_id: String,
    pub epoch: u32,
    pub generation: String,
    pub scope: PublicationAuthorityScope,
    pub not_before_unix: u64,
    pub expires_at_unix: u64,
    pub revoked: bool,
}

/// The caller's provenance assertion must come from its owner-controlled
/// record loader, never from a parsed approval or policy document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationRecordProvenance {
    Unverified,
    OwnerControlled,
}

/// Mode A is the live default. Mode B requires an additional explicit test
/// switch even when a policy document exists.
#[derive(Debug, Default, Clone, Copy)]
pub enum PublicationAuthorityMode<'a> {
    #[default]
    ModeA,
    ModeBTest {
        policy: &'a PublicationModeBPolicy,
        explicitly_enabled: bool,
    },
}

/// Values selected independently of the receipt/policy being evaluated.
pub struct PublicationAuthoritySelection<'a> {
    pub principal: &'a str,
    pub now_unix: u64,
    pub record_provenance: PublicationRecordProvenance,
    pub mode: PublicationAuthorityMode<'a>,
    pub approval: Option<&'a PublicationApprovalReceipt>,
    pub policy_epoch: u32,
    pub policy_generation: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationAuthorityReason {
    InvalidPlan,
    SmokeNotReady,
    InvalidPrincipal,
    ApprovalMissing,
    UnverifiedRecord,
    InvalidRecord,
    PlanChanged,
    SubjectChanged,
    TargetChanged,
    PrincipalChanged,
    CaptureChanged,
    SmokePolicyChanged,
    PostureChanged,
    NotYetEffective,
    Expired,
    ModeBDisabled,
    PolicyRevoked,
    PolicyEpochChanged,
    PolicyGenerationChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationAuthorityDecision {
    Approved {
        mode: &'static str,
        authority_ref: String,
        plan_sha256: String,
    },
    NeedsApproval {
        reason: PublicationAuthorityReason,
    },
    Refused {
        reason: PublicationAuthorityReason,
    },
}

use PublicationAuthorityDecision::{Approved, NeedsApproval, Refused};
use PublicationAuthorityReason as Reason;

fn nonblank(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

fn check_scope(
    scope: &PublicationAuthorityScope,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    principal: &str,
    plan_sha256: &str,
) -> Result<(), Reason> {
    if scope.plan_sha256 != plan_sha256 {
        return Err(Reason::PlanChanged);
    }
    if scope.subject_sha256 != plan.subject_sha256 {
        return Err(Reason::SubjectChanged);
    }
    if scope.host != plan.target.host
        || scope.owner != plan.target.owner
        || scope.repository != plan.target.repository
    {
        return Err(Reason::TargetChanged);
    }
    if scope.principal != principal {
        return Err(Reason::PrincipalChanged);
    }
    if scope.remote_generation_ref != plan.remote_generation_ref {
        return Err(Reason::CaptureChanged);
    }
    if scope.smoke_policy_id != plan.smoke_policy_id
        || scope.smoke_policy_epoch != expected.subject.smoke_policy_epoch
    {
        return Err(Reason::SmokePolicyChanged);
    }
    if scope.publish != expected.publish
        || scope.allow_replace_published != expected.allow_replace_published
    {
        return Err(Reason::PostureChanged);
    }
    Ok(())
}

/// Evaluate one plan and one separately sourced authority record. `Approved`
/// is an inspectable decision only; it is not a one-use claim or permission to
/// bypass the publication owner's journal and remote precondition checks.
pub fn evaluate_publication_authority(
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    selected: &PublicationAuthoritySelection<'_>,
) -> PublicationAuthorityDecision {
    let plan_sha256 = match publication_effect_plan_digest(plan, subject, expected) {
        Ok(digest) => digest,
        Err(_) => {
            return Refused {
                reason: Reason::InvalidPlan,
            };
        }
    };
    if require_apply_ready_effect_plan(plan, subject, expected).is_err() {
        return Refused {
            reason: Reason::SmokeNotReady,
        };
    }
    if !nonblank(selected.principal) {
        return Refused {
            reason: Reason::InvalidPrincipal,
        };
    }
    match selected.mode {
        PublicationAuthorityMode::ModeA => {
            let Some(receipt) = selected.approval else {
                return NeedsApproval {
                    reason: Reason::ApprovalMissing,
                };
            };
            if selected.record_provenance != PublicationRecordProvenance::OwnerControlled {
                return Refused {
                    reason: Reason::UnverifiedRecord,
                };
            }
            if !nonblank(&receipt.approval_id) || receipt.issued_at_unix >= receipt.expires_at_unix
            {
                return Refused {
                    reason: Reason::InvalidRecord,
                };
            }
            if selected.now_unix < receipt.issued_at_unix {
                return Refused {
                    reason: Reason::NotYetEffective,
                };
            }
            if selected.now_unix >= receipt.expires_at_unix {
                return Refused {
                    reason: Reason::Expired,
                };
            }
            if let Err(reason) = check_scope(
                &receipt.scope,
                plan,
                expected,
                selected.principal,
                &plan_sha256,
            ) {
                return Refused { reason };
            }
            Approved {
                mode: "A",
                authority_ref: receipt.approval_id.clone(),
                plan_sha256,
            }
        }
        PublicationAuthorityMode::ModeBTest {
            policy,
            explicitly_enabled,
        } => {
            if !explicitly_enabled {
                return Refused {
                    reason: Reason::ModeBDisabled,
                };
            }
            if selected.record_provenance != PublicationRecordProvenance::OwnerControlled {
                return Refused {
                    reason: Reason::UnverifiedRecord,
                };
            }
            if !nonblank(&policy.policy_id)
                || !nonblank(&policy.generation)
                || !nonblank(selected.policy_generation)
                || policy.epoch == 0
                || policy.not_before_unix >= policy.expires_at_unix
            {
                return Refused {
                    reason: Reason::InvalidRecord,
                };
            }
            if policy.revoked {
                return Refused {
                    reason: Reason::PolicyRevoked,
                };
            }
            if policy.epoch != selected.policy_epoch {
                return Refused {
                    reason: Reason::PolicyEpochChanged,
                };
            }
            if policy.generation != selected.policy_generation {
                return Refused {
                    reason: Reason::PolicyGenerationChanged,
                };
            }
            if selected.now_unix < policy.not_before_unix {
                return Refused {
                    reason: Reason::NotYetEffective,
                };
            }
            if selected.now_unix >= policy.expires_at_unix {
                return Refused {
                    reason: Reason::Expired,
                };
            }
            if let Err(reason) = check_scope(
                &policy.scope,
                plan,
                expected,
                selected.principal,
                &plan_sha256,
            ) {
                return Refused { reason };
            }
            Approved {
                mode: "B-test",
                authority_ref: policy.policy_id.clone(),
                plan_sha256,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publication_effect_plan::{CapturedPublicationRemote, plan_from_captured_remote};
    use crate::publication_subject::PublicationSubjectExpectations;
    use crate::release_manifest::{
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, DISTRIBUTION_BASH_INSTALLER_FILENAME,
        DISTRIBUTION_CMD_INSTALLER_FILENAME, DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
        DISTRIBUTION_PRODUCT, DISTRIBUTION_REPOSITORY, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME,
        SUPPORTED_DISTRIBUTION_TARGETS,
    };
    use vibe_wire::generated::publication_effect_plan::{
        PublicationObservedRelease, PublicationObservedReleaseAbsent, PublicationObservedTag,
        PublicationObservedTagAbsent,
    };
    use vibe_wire::generated::publication_subject::{
        PublicationAsset, PublicationComponent, PublicationComponentName, PublicationInstaller,
        PublicationInstallerKind, PublicationSmoke, PublicationSmokeProbe,
        PublicationSmokeProbeStatus, PublicationSource, PublicationSourceArchive,
        PublicationTarget,
    };

    fn sha() -> String {
        format!("sha256:{}", "a".repeat(64))
    }
    fn asset(name: String, media_type: &str) -> PublicationAsset {
        PublicationAsset {
            name,
            media_type: media_type.into(),
            size_bytes: "1".into(),
            sha256: sha(),
        }
    }
    fn subject() -> PublicationSubject {
        let targets = SUPPORTED_DISTRIBUTION_TARGETS
            .iter()
            .map(|target| {
                let suffix = if *target == "x86_64-pc-windows-msvc" {
                    ".exe"
                } else {
                    ""
                };
                PublicationTarget {
                    target: (*target).into(),
                    bundle: asset(format!("vibevm-1.1.0-{target}.zip"), "application/zip"),
                    bootstrap: asset(
                        format!("vibe-bootstrap-{target}{suffix}"),
                        "application/octet-stream",
                    ),
                    fragment: asset(
                        format!("vibevm-1.1.0-{target}.fragment.json"),
                        "application/json",
                    ),
                    components: [
                        (PublicationComponentName::Vibe, format!("vibe{suffix}")),
                        (
                            PublicationComponentName::VibeIndex,
                            format!("vibe-index{suffix}"),
                        ),
                    ]
                    .into_iter()
                    .map(|(name, path)| PublicationComponent {
                        name,
                        path,
                        size_bytes: "1".into(),
                        sha256: sha(),
                    })
                    .collect(),
                    source_archive: PublicationSourceArchive {
                        path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.into(),
                        size_bytes: "1".into(),
                        sha256: sha(),
                        tree_oid: "b".repeat(40),
                    },
                }
            })
            .collect::<Vec<_>>();
        let probes = targets
            .iter()
            .map(|target| PublicationSmokeProbe {
                probe_id: "launch".into(),
                target: target.target.clone(),
                status: PublicationSmokeProbeStatus::Passed,
                tested_sha256: Some(target.bundle.sha256.clone()),
                runner_ref: Some("runner".into()),
                evidence_ref: Some("proof".into()),
            })
            .collect();
        PublicationSubject {
            schema: 1,
            product: DISTRIBUTION_PRODUCT.into(),
            repository: DISTRIBUTION_REPOSITORY.into(),
            version: "1.1.0".into(),
            mutable: true,
            source: PublicationSource {
                commit: "a".repeat(40),
                tree: "b".repeat(40),
                cargo_lock: asset("Cargo.lock".into(), "text/plain"),
            },
            targets,
            aggregate: asset(
                DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.into(),
                "application/json",
            ),
            installers: [
                (
                    PublicationInstallerKind::Bash,
                    DISTRIBUTION_BASH_INSTALLER_FILENAME,
                    "text/x-shellscript",
                ),
                (
                    PublicationInstallerKind::Powershell,
                    DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
                    "text/plain",
                ),
                (
                    PublicationInstallerKind::Cmd,
                    DISTRIBUTION_CMD_INSTALLER_FILENAME,
                    "text/plain",
                ),
            ]
            .into_iter()
            .map(|(kind, name, media)| PublicationInstaller {
                kind,
                asset: asset(name.into(), media),
            })
            .collect(),
            smoke: PublicationSmoke {
                policy_id: "distribution-smoke/e1".into(),
                policy_epoch: 1,
                probes,
            },
        }
    }
    fn expected() -> PublicationEffectPlanExpectations<'static> {
        PublicationEffectPlanExpectations {
            subject: PublicationSubjectExpectations {
                version: "1.1.0",
                repository: DISTRIBUTION_REPOSITORY,
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
    fn plan(subject: &PublicationSubject) -> PublicationEffectPlan {
        plan_from_captured_remote(
            subject,
            &expected(),
            &CapturedPublicationRemote {
                generation_ref: "remote-g1".into(),
                tag: PublicationObservedTag::Absent(Box::new(PublicationObservedTagAbsent {})),
                release: PublicationObservedRelease::Absent(Box::new(
                    PublicationObservedReleaseAbsent {},
                )),
            },
        )
        .expect("valid fixture plan")
        .plan
    }
    fn scope(
        subject: &PublicationSubject,
        plan: &PublicationEffectPlan,
    ) -> PublicationAuthorityScope {
        PublicationAuthorityScope {
            plan_sha256: publication_effect_plan_digest(plan, subject, &expected()).unwrap(),
            subject_sha256: plan.subject_sha256.clone(),
            host: plan.target.host.clone(),
            owner: plan.target.owner.clone(),
            repository: plan.target.repository.clone(),
            principal: "owner".into(),
            remote_generation_ref: plan.remote_generation_ref.clone(),
            smoke_policy_id: plan.smoke_policy_id.clone(),
            smoke_policy_epoch: 1,
            publish: false,
            allow_replace_published: false,
        }
    }
    fn selection<'a>(
        approval: Option<&'a PublicationApprovalReceipt>,
        mode: PublicationAuthorityMode<'a>,
    ) -> PublicationAuthoritySelection<'a> {
        PublicationAuthoritySelection {
            principal: "owner",
            now_unix: 10,
            record_provenance: PublicationRecordProvenance::OwnerControlled,
            mode,
            approval,
            policy_epoch: 1,
            policy_generation: "generation-1",
        }
    }

    #[test]
    fn exact_mode_a_and_explicit_b_admit_the_same_plan() {
        let subject = subject();
        let plan = plan(&subject);
        let scope = scope(&subject, &plan);
        let receipt = PublicationApprovalReceipt {
            approval_id: "approval-1".into(),
            scope: scope.clone(),
            issued_at_unix: 1,
            expires_at_unix: 20,
        };
        let policy = PublicationModeBPolicy {
            policy_id: "policy-1".into(),
            epoch: 1,
            generation: "generation-1".into(),
            scope,
            not_before_unix: 1,
            expires_at_unix: 20,
            revoked: false,
        };
        assert!(matches!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(Some(&receipt), PublicationAuthorityMode::default())
            ),
            Approved { mode: "A", .. }
        ));
        assert!(matches!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(
                    None,
                    PublicationAuthorityMode::ModeBTest {
                        policy: &policy,
                        explicitly_enabled: true
                    }
                )
            ),
            Approved { mode: "B-test", .. }
        ));
    }

    #[test]
    fn default_needs_approval_and_b_stays_disabled() {
        let subject = subject();
        let plan = plan(&subject);
        let scope = scope(&subject, &plan);
        let policy = PublicationModeBPolicy {
            policy_id: "policy-1".into(),
            epoch: 1,
            generation: "generation-1".into(),
            scope,
            not_before_unix: 1,
            expires_at_unix: 20,
            revoked: false,
        };
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(None, PublicationAuthorityMode::default())
            ),
            NeedsApproval {
                reason: Reason::ApprovalMissing
            }
        );
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(
                    None,
                    PublicationAuthorityMode::ModeBTest {
                        policy: &policy,
                        explicitly_enabled: false
                    }
                )
            ),
            Refused {
                reason: Reason::ModeBDisabled
            }
        );
    }

    #[test]
    fn changed_plan_or_untrusted_receipt_refuses() {
        let subject = subject();
        let plan = plan(&subject);
        let mut receipt = PublicationApprovalReceipt {
            approval_id: "approval-1".into(),
            scope: scope(&subject, &plan),
            issued_at_unix: 1,
            expires_at_unix: 20,
        };
        receipt.scope.plan_sha256 = format!("sha256:{}", "b".repeat(64));
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(Some(&receipt), PublicationAuthorityMode::ModeA)
            ),
            Refused {
                reason: Reason::PlanChanged
            }
        );
        receipt.scope.plan_sha256 = scope(&subject, &plan).plan_sha256;
        let mut selected = selection(Some(&receipt), PublicationAuthorityMode::ModeA);
        selected.record_provenance = PublicationRecordProvenance::Unverified;
        assert_eq!(
            evaluate_publication_authority(&subject, &plan, &expected(), &selected),
            Refused {
                reason: Reason::UnverifiedRecord
            }
        );
    }

    #[test]
    fn b_expiry_revocation_and_scope_refuse() {
        let subject = subject();
        let plan = plan(&subject);
        let scope = scope(&subject, &plan);
        let mut policy = PublicationModeBPolicy {
            policy_id: "policy-1".into(),
            epoch: 1,
            generation: "generation-1".into(),
            scope,
            not_before_unix: 1,
            expires_at_unix: 20,
            revoked: false,
        };
        policy.revoked = true;
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(
                    None,
                    PublicationAuthorityMode::ModeBTest {
                        policy: &policy,
                        explicitly_enabled: true
                    }
                )
            ),
            Refused {
                reason: Reason::PolicyRevoked
            }
        );
        policy.revoked = false;
        policy.expires_at_unix = 10;
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(
                    None,
                    PublicationAuthorityMode::ModeBTest {
                        policy: &policy,
                        explicitly_enabled: true
                    }
                )
            ),
            Refused {
                reason: Reason::Expired
            }
        );
        policy.expires_at_unix = 20;
        policy.scope.principal = "someone-else".into();
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(
                    None,
                    PublicationAuthorityMode::ModeBTest {
                        policy: &policy,
                        explicitly_enabled: true
                    }
                )
            ),
            Refused {
                reason: Reason::PrincipalChanged
            }
        );
    }

    #[test]
    fn approval_must_match_every_named_boundary() {
        let subject = subject();
        let plan = plan(&subject);
        let original = scope(&subject, &plan);
        let mut receipt = PublicationApprovalReceipt {
            approval_id: "approval-1".into(),
            scope: original.clone(),
            issued_at_unix: 1,
            expires_at_unix: 20,
        };
        let decision = |receipt: &PublicationApprovalReceipt| {
            evaluate_publication_authority(
                &subject,
                &plan,
                &expected(),
                &selection(Some(receipt), PublicationAuthorityMode::ModeA),
            )
        };
        receipt.scope.subject_sha256 = sha();
        assert_eq!(
            decision(&receipt),
            Refused {
                reason: Reason::SubjectChanged
            }
        );
        receipt.scope = original.clone();
        receipt.scope.owner = "elsewhere".into();
        assert_eq!(
            decision(&receipt),
            Refused {
                reason: Reason::TargetChanged
            }
        );
        receipt.scope = original.clone();
        receipt.scope.remote_generation_ref = "old".into();
        assert_eq!(
            decision(&receipt),
            Refused {
                reason: Reason::CaptureChanged
            }
        );
        receipt.scope = original.clone();
        receipt.scope.smoke_policy_epoch = 2;
        assert_eq!(
            decision(&receipt),
            Refused {
                reason: Reason::SmokePolicyChanged
            }
        );
        receipt.scope = original;
        receipt.scope.publish = true;
        assert_eq!(
            decision(&receipt),
            Refused {
                reason: Reason::PostureChanged
            }
        );
    }

    #[test]
    fn policy_epoch_generation_and_changed_plan_refuse() {
        let subject = subject();
        let mut plan = plan(&subject);
        let scope = scope(&subject, &plan);
        let mut policy = PublicationModeBPolicy {
            policy_id: "policy-1".into(),
            epoch: 2,
            generation: "generation-1".into(),
            scope,
            not_before_unix: 1,
            expires_at_unix: 20,
            revoked: false,
        };
        let decide = |plan: &PublicationEffectPlan, policy: &PublicationModeBPolicy| {
            evaluate_publication_authority(
                &subject,
                plan,
                &expected(),
                &selection(
                    None,
                    PublicationAuthorityMode::ModeBTest {
                        policy,
                        explicitly_enabled: true,
                    },
                ),
            )
        };
        assert_eq!(
            decide(&plan, &policy),
            Refused {
                reason: Reason::PolicyEpochChanged
            }
        );
        policy.epoch = 1;
        policy.generation = "older-generation".into();
        assert_eq!(
            decide(&plan, &policy),
            Refused {
                reason: Reason::PolicyGenerationChanged
            }
        );
        policy.generation = "generation-1".into();
        plan.remote_generation_ref = "remote-g2".into();
        assert_eq!(
            decide(&plan, &policy),
            Refused {
                reason: Reason::PlanChanged
            }
        );
    }

    #[test]
    fn incomplete_smoke_never_becomes_approval() {
        let mut subject = subject();
        let original_plan = plan(&subject);
        let receipt = PublicationApprovalReceipt {
            approval_id: "approval-1".into(),
            scope: scope(&subject, &original_plan),
            issued_at_unix: 1,
            expires_at_unix: 20,
        };
        subject.smoke.probes[0].status = PublicationSmokeProbeStatus::Skipped;
        subject.smoke.probes[0].tested_sha256 = None;
        subject.smoke.probes[0].runner_ref = None;
        subject.smoke.probes[0].evidence_ref = None;
        // The subject digest now differs, so the first refusal is plan
        // identity. Even if replanned, the apply-ready smoke gate refuses.
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &original_plan,
                &expected(),
                &selection(Some(&receipt), PublicationAuthorityMode::ModeA)
            ),
            Refused {
                reason: Reason::InvalidPlan
            }
        );
        let replanned = plan(&subject);
        assert_eq!(
            evaluate_publication_authority(
                &subject,
                &replanned,
                &expected(),
                &selection(None, PublicationAuthorityMode::ModeA)
            ),
            Refused {
                reason: Reason::SmokeNotReady
            }
        );
    }
}
