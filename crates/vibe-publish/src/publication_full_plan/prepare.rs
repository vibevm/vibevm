//! Local-only C.1 reservation and owner journal preparation. No remote write
//! port is reachable from this module or from production callers.

use vibe_lifecycle::process::propose_run_id;
use vibe_lifecycle::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;
use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_approval::{ApprovalHome, VerifiedApproval};
use crate::publication_effect_plan::PublicationEffectPlanExpectations;
use crate::publication_payload::vault::{FinalPayloadBytes, VaultInspection, stage_payload_vault};

use super::journal::{self, InputCapsule, Intent};
use super::{Error, Inspection, RemoteObservation, Result, Selected, initial_matches, select, sha};

pub(super) enum PreparedJournal {
    Existing(Inspection),
    New {
        run_id: String,
        intent: Intent,
        reservation_ref: String,
        before: RemoteObservation,
    },
}

/// The production constructor retains a host capture and selects the fixed
/// settings home lazily after C.1 lookup. No live remote/public route calls it.
pub(crate) struct TrustedLocalPublication<'a> {
    approval_source: ApprovalSource,
    before: RemoteObservation,
    output: &'a Project,
    finals: FinalPayloadBytes<'a>,
}

enum ApprovalSource {
    HostSettings,
    #[cfg(test)]
    TestPath(std::path::PathBuf),
}

impl ApprovalSource {
    fn open(
        &self,
    ) -> std::result::Result<ApprovalHome, crate::publication_approval::ApprovalError> {
        match self {
            Self::HostSettings => ApprovalHome::open_from_settings(),
            #[cfg(test)]
            Self::TestPath(path) => ApprovalHome::open_for_test(path),
        }
    }
}

impl<'a> TrustedLocalPublication<'a> {
    /// Host composition supplies captured bytes and observation; the approval
    /// home is selected only through the fixed user-settings resolver.
    pub(crate) fn from_host_capture(
        before: RemoteObservation,
        output: &'a Project,
        finals: FinalPayloadBytes<'a>,
    ) -> Self {
        Self {
            approval_source: ApprovalSource::HostSettings,
            before,
            output,
            finals,
        }
    }
}

/// Production-compiled, crate-private local journal seam. It never calls a
/// remote method and a Ready result does not authorize a later effect.
pub(crate) fn prepare_local_full_plan(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    trusted: TrustedLocalPublication<'_>,
) -> Result<Inspection> {
    if let Some(existing) =
        SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?
    {
        if existing.retired {
            return Ok(Inspection::Retired {
                run_id: Some(existing.domain_run_id),
            });
        }
        return match trusted.approval_source.open() {
            Ok(home) => super::cold::inspect_full_plan_cold_with_home(
                lease,
                namespace.id(),
                scope,
                key,
                &home,
            ),
            Err(_) => Ok(Inspection::RecoveryRequired {
                run_id: existing.domain_run_id,
            }),
        };
    }
    let approval_home = trusted.approval_source.open().map_err(Error::io)?;
    let verified = approval_home
        .load_mode_a(lease, scope, subject, plan, expected)
        .map_err(Error::io)?;
    let selected = select(
        lease,
        scope,
        subject,
        plan,
        expected,
        Some(("A", verified.authority_id())),
    )?;
    if verified.plan_sha256() != selected.intent.semantic_plan_sha256 {
        return Err(Error::Refused(
            "approved plan digest differs from selected plan".into(),
        ));
    }
    match prepare_journal_inner(
        lease,
        namespace,
        scope,
        key,
        subject,
        plan,
        expected,
        selected,
        || Ok(trusted.before),
        ReadyLane::Vault {
            output: trusted.output,
            finals: trusted.finals,
            approval_home: &approval_home,
            verified: &verified,
        },
    )? {
        PreparedJournal::Existing(inspection) => Ok(inspection),
        PreparedJournal::New { run_id, .. } => Ok(Inspection::JournalReady { run_id }),
    }
}

/// The common pre-effect fence for local preparation and the test fake.
/// Existing keys are inspected before `observe` is called. Fresh runs publish
/// the input capsule and intent create-new, read both back, then mark C.1
/// Ready. No effect callback exists here.
enum ReadyLane<'a> {
    Vault {
        output: &'a Project,
        finals: FinalPayloadBytes<'a>,
        approval_home: &'a ApprovalHome,
        verified: &'a VerifiedApproval,
    },
    #[cfg(test)]
    FakeNoVault,
}

#[cfg(test)]
pub(super) fn prepare_journal_fake(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    selected: Selected,
    observe: impl FnOnce() -> Result<RemoteObservation>,
) -> Result<PreparedJournal> {
    prepare_journal_inner(
        lease,
        namespace,
        scope,
        key,
        subject,
        plan,
        expected,
        selected,
        observe,
        ReadyLane::FakeNoVault,
    )
}

fn prepare_journal_inner(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    selected: Selected,
    observe: impl FnOnce() -> Result<RemoteObservation>,
    lane: ReadyLane<'_>,
) -> Result<PreparedJournal> {
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &selected.request,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(PreparedJournal::Existing(Inspection::Retired {
                run_id: None,
            }));
        }
        ExistingSubmissionInspection::Reserved { .. } => {
            return Ok(PreparedJournal::Existing(match lane {
                ReadyLane::Vault { approval_home, .. } => {
                    super::cold::inspect_full_plan_cold_with_home(
                        lease,
                        namespace.id(),
                        scope,
                        key,
                        approval_home,
                    )?
                }
                #[cfg(test)]
                ReadyLane::FakeNoVault => {
                    super::inspect_fake_full_plan_cold(lease, namespace.id(), scope, key)?
                }
            }));
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let before = observe()?;
    if !initial_matches(&before, plan) {
        return Err(Error::Refused(
            "captured remote precondition changed before reservation".into(),
        ));
    }
    let run_id = propose_run_id(lease.root())?;
    if !matches!(SubmissionReservations::reserve(
        lease, namespace, scope, key, &selected.request, &selected.plan_bytes, &run_id,
    )?, ReservationDecision::Created { domain_run_id } if domain_run_id == run_id)
    {
        return Err(Error::Refused(
            "fresh full-plan reservation was not Created".into(),
        ));
    }
    let (vault_manifest_sha256, redemption_marker_sha256, approval_record_sha256) = match lane {
        ReadyLane::Vault {
            output,
            finals,
            approval_home,
            verified,
        } => {
            match stage_payload_vault(
                lease,
                namespace,
                scope,
                key,
                &run_id,
                subject,
                &expected.subject,
                output,
                &finals,
            ) {
                Ok(VaultInspection::Complete {
                    run_id: observed,
                    catalogue_digest,
                    manifest_sha256,
                }) if observed == run_id
                    && catalogue_digest == selected.upload_catalogue_sha256 =>
                {
                    match approval_home.redeem(
                        lease,
                        namespace,
                        scope,
                        key,
                        &run_id,
                        &selected.request,
                        verified,
                    ) {
                        Ok(receipt)
                            if receipt.run_id == run_id
                                && receipt.plan_sha256 == selected.intent.semantic_plan_sha256 =>
                        {
                            (
                                Some(manifest_sha256),
                                Some(receipt.marker_sha256),
                                Some(verified.record_sha256().to_owned()),
                            )
                        }
                        _ => {
                            return Ok(PreparedJournal::Existing(Inspection::RecoveryRequired {
                                run_id,
                            }));
                        }
                    }
                }
                _ => {
                    return Ok(PreparedJournal::Existing(Inspection::RecoveryRequired {
                        run_id,
                    }));
                }
            }
        }
        #[cfg(test)]
        ReadyLane::FakeNoVault => (None, None, None),
    };
    let mut intent = selected.intent;
    intent.run_id = run_id.clone();
    intent.vault_manifest_sha256 = vault_manifest_sha256.clone();
    intent.redemption_marker_sha256 = redemption_marker_sha256.clone();
    intent.approval_record_sha256 = approval_record_sha256.clone();
    let request_json = String::from_utf8(selected.request).map_err(Error::io)?;
    let capsule = InputCapsule {
        format: if vault_manifest_sha256.is_some() {
            journal::INPUT_FORMAT.into()
        } else {
            #[cfg(test)]
            {
                journal::FAKE_INPUT_FORMAT.into()
            }
            #[cfg(not(test))]
            {
                unreachable!("production lane always requires a vault")
            }
        },
        run_id: run_id.clone(),
        context: scope.context.clone(),
        target: scope.target.clone(),
        principal: scope.principal.clone(),
        key_sha256: sha(key.as_bytes()),
        request_json,
        subject: subject.clone(),
        plan: plan.clone(),
        publish: selected.publish,
        allow_replace_published: selected.allow_replace_published,
        upload_catalogue_sha256: selected.upload_catalogue_sha256,
        root_correlation_token: selected.root_correlation_token,
        vault_manifest_sha256,
        redemption_marker_sha256,
        approval_record_sha256,
        authority_mode: selected.authority_mode,
        authority_ref: selected.authority_ref,
    };
    if journal::publish_input(lease, &run_id, &capsule).is_err()
        || !matches!(journal::read_input(lease, &run_id), Ok(Some(ref row)) if row == &capsule)
        || journal::publish(lease, &run_id, "intent", &intent).is_err()
        || !matches!(journal::read::<Intent>(lease, &run_id, "intent"), Ok(Some(ref row)) if row == &intent)
        || SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)
            .is_err()
        || !matches!(
            SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id),
            Ok(JournalAdmission::Ready)
        )
    {
        return Ok(PreparedJournal::Existing(Inspection::RecoveryRequired {
            run_id,
        }));
    }
    let reservation_ref =
        match SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id) {
            Ok(value) => value,
            Err(_) => {
                return Ok(PreparedJournal::Existing(Inspection::RecoveryRequired {
                    run_id,
                }));
            }
        };
    Ok(PreparedJournal::New {
        run_id,
        intent,
        reservation_ref,
        before,
    })
}

#[cfg(test)]
pub(super) fn trusted<'a>(
    settings: &std::path::Path,
    _lease: &LifecycleLease,
    _scope: &SubmissionScope,
    _subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    output: &'a Project,
    finals: FinalPayloadBytes<'a>,
) -> TrustedLocalPublication<'a> {
    TrustedLocalPublication {
        approval_source: ApprovalSource::TestPath(settings.to_path_buf()),
        before: RemoteObservation {
            generation_ref: plan.remote_generation_ref.clone(),
            tag: plan.observed_tag.clone(),
            release: plan.observed_release.clone(),
            latest_release_id: None,
            last: None,
        },
        output,
        finals,
    }
}
