//! Deterministic first API-call frontier for tag and release effects.
//! Its private driver binds one selected call to a sealed ReadyRun; no public
//! or CLI publication route invokes the driver.

use serde::{Deserialize, Serialize};
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::publication_effect_plan::{
    PublicationEffectPlan, PublicationObservedRelease, PublicationObservedReleaseAbsent,
    PublicationObservedTag, PublicationObservedTagPresent, PublicationPartition,
    PublicationPartitionReleaseAction, PublicationPartitionTagAction,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_approval::ApprovalHome;
use crate::publication_effect_plan::{
    PublicationEffectPlanExpectations, publication_effect_plan_digest,
    require_apply_ready_effect_plan,
};

use super::journal::{self, Intent, sha};
use super::session::ReadyRun;
use super::{Error, Result};

const DOMAIN: &[u8] = b"vibevm.publication.api-call/e1\0";
const PENDING_FORMAT: &str = "vibevm.publication.api-call-pending/e1";
const LANDED_FORMAT: &str = "vibevm.publication.api-call-landed/e1";

#[path = "calls/asset_history.rs"]
pub(super) mod asset_history;
mod assets;
mod driver;

#[cfg(test)]
#[path = "calls/asset_many_tests.rs"]
mod asset_many_tests;
#[cfg(test)]
#[path = "calls/assets_tests.rs"]
mod assets_tests;
#[cfg(test)]
#[path = "calls/e2_metadata_tests.rs"]
mod e2_metadata_tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub(super) enum CallAction {
    /// Read-only exact ref checkpoint. It has an ID and journal proof but
    /// never invokes a remote write method.
    RetainTag {
        tag: String,
        commit: String,
    },
    CreateTag {
        tag: String,
        commit: String,
    },
    DeletePriorRelease {
        release_id: String,
        inventory_sha256: String,
    },
    CreateDraft {
        tag: String,
        target_commit: String,
        name: String,
        body: String,
        prerelease: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PlannedCall {
    pub ordinal: usize,
    pub partition_ordinal: usize,
    pub call_id: String,
    pub semantic_plan_sha256: String,
    pub expected_tag: PublicationObservedTag,
    pub expected_release: PublicationObservedRelease,
    pub action: CallAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CallSchedule {
    pub calls: Vec<PlannedCall>,
    /// Assets, cleanup and finalize have no live API-call adapter yet.
    pub unavailable_after_prefix: &'static str,
}

fn call_id(
    plan_sha256: &str,
    ordinal: usize,
    partition_ordinal: usize,
    action: &CallAction,
    tag: &PublicationObservedTag,
    release: &PublicationObservedRelease,
) -> Result<String> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(DOMAIN);
    for frame in [
        plan_sha256.as_bytes().to_vec(),
        (ordinal as u64).to_be_bytes().to_vec(),
        (partition_ordinal as u64).to_be_bytes().to_vec(),
        serde_json::to_vec(action)?,
        serde_json::to_vec(tag)?,
        serde_json::to_vec(release)?,
    ] {
        bytes.extend_from_slice(&(frame.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&frame);
    }
    Ok(sha(&bytes))
}

/// Expand only the first two partitions. The remainder is explicitly
/// unavailable; a caller cannot mistake this prefix for full publication.
pub(super) fn decompose(
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<CallSchedule> {
    require_apply_ready_effect_plan(plan, subject, expected).map_err(Error::io)?;
    let digest = publication_effect_plan_digest(plan, subject, expected).map_err(Error::io)?;
    let PublicationPartition::Tag(tag) = &plan.partitions[0] else {
        unreachable!()
    };
    let PublicationPartition::Release(release) = &plan.partitions[1] else {
        unreachable!()
    };
    let tag_action = match tag.action {
        PublicationPartitionTagAction::Create => CallAction::CreateTag {
            tag: tag.tag.clone(),
            commit: tag.desired_commit.clone(),
        },
        PublicationPartitionTagAction::Retain => CallAction::RetainTag {
            tag: tag.tag.clone(),
            commit: tag.desired_commit.clone(),
        },
    };
    let mut calls = Vec::with_capacity(3);
    let mut add = |partition_ordinal: usize,
                   action: CallAction,
                   expected_tag: PublicationObservedTag,
                   expected_release: PublicationObservedRelease|
     -> Result<()> {
        let ordinal = calls.len();
        let call_id = call_id(
            &digest,
            ordinal,
            partition_ordinal,
            &action,
            &expected_tag,
            &expected_release,
        )?;
        calls.push(PlannedCall {
            ordinal,
            partition_ordinal,
            call_id,
            semantic_plan_sha256: digest.clone(),
            expected_tag,
            expected_release,
            action,
        });
        Ok(())
    };
    add(
        0,
        tag_action,
        plan.observed_tag.clone(),
        plan.observed_release.clone(),
    )?;
    let after_tag = PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
        reference: format!("refs/tags/{}", tag.tag),
        object_kind: "commit".into(),
        object_sha: tag.desired_commit.clone(),
    }));
    if release.action == PublicationPartitionReleaseAction::Replace {
        let PublicationObservedRelease::Present(old) = &plan.observed_release else {
            unreachable!()
        };
        let inventory_sha256 = sha(&serde_json::to_vec(&old.assets)?);
        add(
            1,
            CallAction::DeletePriorRelease {
                release_id: old.release_id.clone(),
                inventory_sha256,
            },
            after_tag.clone(),
            plan.observed_release.clone(),
        )?;
    }
    add(
        1,
        CallAction::CreateDraft {
            tag: release.desired_tag.clone(),
            target_commit: release.desired_target_commit.clone(),
            name: release.desired_name.clone(),
            body: release.desired_body.clone(),
            prerelease: release.desired_prerelease,
        },
        after_tag,
        PublicationObservedRelease::Absent(Box::new(PublicationObservedReleaseAbsent {})),
    )?;
    Ok(CallSchedule {
        calls,
        unavailable_after_prefix: "platform assets, cleanup, final assets and finalize have no per-call adapter",
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    format: String,
    run_id: String,
    call: PlannedCall,
    before: CallObservation,
    expected_observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CallObservation {
    generation_ref: String,
    tag: PublicationObservedTag,
    release: PublicationObservedRelease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Landed {
    format: String,
    run_id: String,
    pending_sha256: String,
    receipt_id: String,
    after_observation_sha256: String,
    after: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CallInspection {
    Next { ordinal: usize },
    Uncertain { ordinal: usize },
    Unavailable { landed: usize, reason: &'static str },
}

fn suffix(ordinal: usize, phase: &str) -> String {
    format!("api-{ordinal:03}-{phase}")
}

fn stray_records_after(lease: &LifecycleLease, run_id: &str, from: usize, through: usize) -> bool {
    (from..=through).any(|ordinal| {
        ["pending", "landed"].into_iter().any(|phase| {
            !matches!(
                journal::read_bytes(lease, run_id, &suffix(ordinal, phase)),
                Ok(None)
            )
        })
    })
}

fn verified_transition(
    call: &PlannedCall,
    before: &CallObservation,
    after: &CallObservation,
    receipt: &str,
) -> bool {
    if before.tag != call.expected_tag
        || before.release != call.expected_release
        || before.generation_ref.is_empty()
        || after.generation_ref.is_empty()
        || receipt.is_empty()
    {
        return false;
    }
    match &call.action {
        CallAction::RetainTag { tag, commit } => {
            matches!(&before.tag, PublicationObservedTag::Present(value)
                if value.reference == format!("refs/tags/{tag}") && value.object_sha == *commit)
                && after == before
        }
        CallAction::CreateTag { tag, commit } => {
            matches!(before.tag, PublicationObservedTag::Absent(_))
                && matches!(&after.tag, PublicationObservedTag::Present(value)
                    if value.reference == format!("refs/tags/{tag}")
                        && value.object_kind == "commit" && value.object_sha == *commit)
                && after.release == before.release
                && after.generation_ref != before.generation_ref
        }
        CallAction::DeletePriorRelease {
            release_id,
            inventory_sha256,
        } => {
            matches!(&before.release, PublicationObservedRelease::Present(old)
                if old.release_id == *release_id
                    && sha(&serde_json::to_vec(&old.assets).unwrap_or_default()) == *inventory_sha256)
                && matches!(after.release, PublicationObservedRelease::Absent(_))
                && after.tag == before.tag
                && after.generation_ref != before.generation_ref
        }
        CallAction::CreateDraft {
            tag,
            target_commit,
            name,
            body,
            prerelease,
        } => {
            matches!(before.release, PublicationObservedRelease::Absent(_))
                && matches!(&before.tag, PublicationObservedTag::Present(value)
                    if value.reference == format!("refs/tags/{tag}") && value.object_sha == *target_commit)
                && matches!(&after.release, PublicationObservedRelease::Present(value)
                    if value.release_id == receipt && value.tag == *tag
                        && value.draft && value.inventory_complete && value.assets.is_empty()
                        && value.target_commitish == *target_commit
                        && value.name.as_deref() == Some(name.as_str())
                        && value.body.as_deref() == Some(body.as_str())
                        && value.prerelease == *prerelease
                        && value.generation_ref == after.generation_ref)
                && after.tag == before.tag
                && after.generation_ref != before.generation_ref
        }
    }
}

fn inspect_ready(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    approval_home: Option<&ApprovalHome>,
) -> Result<()> {
    let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?
        .ok_or_else(|| Error::Refused("C.1 run absent".into()))?;
    if row.domain_run_id != run_id
        || row.retired
        || row.journal_admission != JournalAdmission::Ready
        || row.plan_sha256 != sha(&serde_json::to_vec(plan)?)
    {
        return Err(Error::Refused("C.1 run is not exact and Ready".into()));
    }
    let capsule = journal::read_input(lease, run_id)?
        .ok_or_else(|| Error::Refused("publication input capsule absent".into()))?;
    let intent = journal::read::<Intent>(lease, run_id, "intent")?
        .ok_or_else(|| Error::Refused("publication owner intent absent".into()))?;
    let production_lane = capsule.format == journal::INPUT_FORMAT
        && capsule.vault_manifest_sha256.is_some()
        && capsule.redemption_marker_sha256.is_some()
        && capsule.approval_record_sha256.is_some();
    #[cfg(test)]
    let fake_lane = capsule.format == journal::FAKE_INPUT_FORMAT
        && capsule.vault_manifest_sha256.is_none()
        && capsule.redemption_marker_sha256.is_none()
        && capsule.approval_record_sha256.is_none();
    #[cfg(not(test))]
    let fake_lane = false;
    let valid_lane = production_lane || fake_lane;
    if !valid_lane
        || capsule.run_id != run_id
        || capsule.subject != *subject
        || capsule.plan != *plan
        || capsule.root_correlation_token
            != lease
                .publication_journal_project()
                .root_correlation_token()
                .map_err(Error::io)?
        || capsule.request_json.as_bytes().is_empty()
        || row.request_sha256 != sha(capsule.request_json.as_bytes())
        || intent.run_id != run_id
        || intent.request_sha256 != row.request_sha256
        || intent.c1_plan_sha256 != row.plan_sha256
        || intent.semantic_plan_sha256
            != publication_effect_plan_digest(plan, subject, expected).map_err(Error::io)?
        || intent.upload_catalogue_sha256 != capsule.upload_catalogue_sha256
        || intent.root_correlation_token != capsule.root_correlation_token
        || intent.vault_manifest_sha256 != capsule.vault_manifest_sha256
        || intent.redemption_marker_sha256 != capsule.redemption_marker_sha256
        || intent.approval_record_sha256 != capsule.approval_record_sha256
    {
        return Err(Error::Refused(
            "C.1 capsule/intent differs from selected plan".into(),
        ));
    }
    if production_lane {
        let observed = match approval_home {
            Some(home) => super::cold::inspect_full_plan_cold_with_home(
                lease,
                namespace.id(),
                scope,
                key,
                home,
            ),
            None => super::cold::inspect_full_plan_cold(lease, namespace.id(), scope, key),
        };
        if !matches!(observed, Ok(super::Inspection::JournalReady { run_id: observed_run })
            if observed_run == run_id)
        {
            return Err(Error::Refused(
                "production vault or approval redemption is not cold Ready".into(),
            ));
        }
    }
    Ok(())
}

/// Read-only per-call history. A Pending without Landed is uncertain even if
/// the remote later has byte-identical objects; nothing here retries it.
pub(super) fn inspect_calls(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<CallInspection> {
    inspect_calls_inner(
        lease, namespace, scope, key, run_id, subject, plan, expected, None, None,
    )
}

pub(super) fn inspect_calls_proved(session: &ReadyRun<'_>) -> Result<CallInspection> {
    inspect_calls_inner(
        session.lease,
        session.namespace,
        session.scope,
        session.key,
        &session.run_id,
        session.subject,
        session.plan,
        session.expected,
        Some(session.approval_home),
        Some(session),
    )
}

#[cfg(test)]
fn inspect_calls_with_home(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    home: &ApprovalHome,
) -> Result<CallInspection> {
    inspect_calls_inner(
        lease,
        namespace,
        scope,
        key,
        run_id,
        subject,
        plan,
        expected,
        Some(home),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn inspect_calls_inner(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    approval_home: Option<&ApprovalHome>,
    ready_proof: Option<&ReadyRun<'_>>,
) -> Result<CallInspection> {
    if let Some(session) = ready_proof {
        session.recheck_small()?;
    } else {
        inspect_ready(
            lease,
            namespace,
            scope,
            key,
            run_id,
            subject,
            plan,
            expected,
            approval_home,
        )?;
    }
    let schedule = decompose(subject, plan, expected)?;
    let mut previous_after: Option<CallObservation> = None;
    for call in &schedule.calls {
        let pending_name = suffix(call.ordinal, "pending");
        let pending = match journal::read::<Pending>(lease, run_id, &pending_name) {
            Ok(value) => value,
            Err(_) => {
                return Ok(CallInspection::Uncertain {
                    ordinal: call.ordinal,
                });
            }
        };
        let Some(pending) = pending else {
            return Ok(
                if stray_records_after(lease, run_id, call.ordinal, schedule.calls.len()) {
                    CallInspection::Uncertain {
                        ordinal: call.ordinal,
                    }
                } else {
                    CallInspection::Next {
                        ordinal: call.ordinal,
                    }
                },
            );
        };
        if pending.format != PENDING_FORMAT
            || pending.run_id != run_id
            || pending.call != *call
            || pending.expected_observation_sha256 != sha(&serde_json::to_vec(&pending.before)?)
            || call.ordinal == 0 && pending.before.generation_ref != plan.remote_generation_ref
            || previous_after
                .as_ref()
                .is_some_and(|prior| prior != &pending.before)
        {
            return Ok(CallInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        let landed_name = suffix(call.ordinal, "landed");
        let landed = match journal::read::<Landed>(lease, run_id, &landed_name) {
            Ok(value) => value,
            Err(_) => {
                return Ok(CallInspection::Uncertain {
                    ordinal: call.ordinal,
                });
            }
        };
        let Some(landed) = landed else {
            return Ok(CallInspection::Uncertain {
                ordinal: call.ordinal,
            });
        };
        if landed.format != LANDED_FORMAT
            || landed.run_id != run_id
            || landed.pending_sha256 != sha(&serde_json::to_vec(&pending)?)
            || landed.after_observation_sha256 != sha(&serde_json::to_vec(&landed.after)?)
            || !verified_transition(call, &pending.before, &landed.after, &landed.receipt_id)
        {
            return Ok(CallInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        previous_after = Some(landed.after);
    }
    if stray_records_after(lease, run_id, schedule.calls.len(), schedule.calls.len()) {
        return Ok(CallInspection::Uncertain {
            ordinal: schedule.calls.len(),
        });
    }
    Ok(CallInspection::Unavailable {
        landed: schedule.calls.len(),
        reason: schedule.unavailable_after_prefix,
    })
}

#[cfg(test)]
#[path = "calls/tests.rs"]
mod tests;
