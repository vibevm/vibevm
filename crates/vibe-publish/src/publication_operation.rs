//! Internal first-partition publication journal. No live remote adapter is
//! implemented here; the only port implementation is a disposable test fake.
//! Existing keys inspect retained owner evidence and never call the port.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::process::{is_valid_run_id, propose_run_id};
use vibe_lifecycle::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::publication_effect_plan::{
    PublicationEffectPlan, PublicationObservedRelease, PublicationObservedTag,
    PublicationPartition, PublicationPartitionTagAction, PublicationRemoteTarget,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_effect_plan::{
    CapturedPublicationRemote, PublicationEffectPlanExpectations, publication_effect_plan_digest,
    require_apply_ready_effect_plan,
};

const HOME: [&str; 2] = [".vibe", "publication-operations"];
const CAP: usize = 64 * 1024;
const C1_CAP: usize = 1024 * 1024;
const INTENT_FORMAT: &str = "vibevm.publication.tag-intent/1";
const PARTITION_FORMAT: &str = "vibevm.publication.tag-partition/1";
const RESULT_FORMAT: &str = "vibevm.publication.tag-result/1";

#[derive(Debug, thiserror::Error)]
pub(crate) enum PublicationOperationError {
    #[error("publication operation refused: {0}")]
    Refused(String),
    #[error(transparent)]
    Submission(#[from] vibe_lifecycle::SubmissionError),
    #[error(transparent)]
    Lease(#[from] vibe_lifecycle::LifecycleLeaseError),
    #[error(transparent)]
    Scratch(#[from] vibe_lifecycle::process::ScratchError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, PublicationOperationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PublicationTagInspection {
    Completed {
        run_id: String,
        tag_ref: String,
        commit: String,
        replayed: bool,
    },
    RefusedBeforeEffect {
        run_id: String,
    },
    RecoveryRequired {
        run_id: String,
    },
    Uncertain {
        run_id: String,
        pending: bool,
        landed: bool,
    },
    Retired {
        run_id: Option<String>,
    },
}

/// A read-only snapshot of the remote generation and both objects from the
/// same adapter. `create_tag` receives the exact expected snapshot; the fake
/// compares it atomically with its in-memory state. No live adapter or
/// cross-object server CAS is implemented by this foundation.
pub(crate) trait TagCreatePort {
    fn observe(&mut self) -> std::result::Result<CapturedPublicationRemote, String>;
    fn create_tag(
        &mut self,
        target: &PublicationRemoteTarget,
        expected: &CapturedPublicationRemote,
        tag: &str,
        commit: &str,
    ) -> std::result::Result<TagCreateReceipt, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagCreateReceipt {
    pub tag_ref: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    format: String,
    run_id: String,
    request_sha256: String,
    c1_plan_sha256: String,
    subject_sha256: String,
    semantic_plan_sha256: String,
    remote_generation_ref: String,
    target: String,
    tag_ref: String,
    desired_commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Partition {
    format: String,
    run_id: String,
    c1_plan_sha256: String,
    kind: String,
    remote_generation_ref: String,
    tag_ref: String,
    desired_commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultRecord {
    format: String,
    run_id: String,
    reservation_ref: String,
    c1_plan_sha256: String,
    outcome: String,
    landed_sha256: Option<String>,
}

fn refused(reason: impl std::fmt::Display) -> PublicationOperationError {
    PublicationOperationError::Refused(reason.to_string())
}
fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn member(run_id: &str, suffix: &str) -> Result<String> {
    if !is_valid_run_id(run_id) {
        return Err(refused("invalid reserved run ID"));
    }
    Ok(format!("{run_id}.tag-{suffix}.json"))
}
fn read_bytes(lease: &LifecycleLease, run_id: &str, suffix: &str) -> Result<Option<Vec<u8>>> {
    let project = lease.publication_journal_project();
    let Some(dir) = project.dir_if_present(&HOME).map_err(refused)? else {
        return Ok(None);
    };
    project
        .read_file_bounded_in(&dir, &member(run_id, suffix)?, CAP)
        .map_err(refused)
}
fn read<T: for<'de> Deserialize<'de>>(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
) -> Result<Option<T>> {
    read_bytes(lease, run_id, suffix)?
        .map(|bytes| serde_json::from_slice(&bytes).map_err(Into::into))
        .transpose()
}
fn publish<T: Serialize>(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
    value: &T,
) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > CAP {
        return Err(refused("publication owner record exceeds cap"));
    }
    let project = lease.publication_journal_project();
    let home = project.ensure_subproject(&HOME).map_err(refused)?;
    let dir = home.root_dir().map_err(refused)?;
    let name = member(run_id, suffix)?;
    home.publish_new_in(&dir, &name, &bytes).map_err(|error| {
        refused(format!(
            "owner record publication: {:#}",
            error.into_report()
        ))
    })?;
    let observed = home
        .read_file_bounded_in(&dir, &name, CAP)
        .map_err(refused)?;
    if observed.as_deref() != Some(bytes.as_slice()) {
        return Err(refused("owner record changed before readback"));
    }
    Ok(bytes)
}

struct FixedTag {
    request: Vec<u8>,
    plan_bytes: Vec<u8>,
    intent: Intent,
    tag: String,
    commit: String,
}

fn fixed_tag(
    scope: &SubmissionScope,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<FixedTag> {
    require_apply_ready_effect_plan(plan, subject, expected).map_err(refused)?;
    let semantic_plan_sha256 =
        publication_effect_plan_digest(plan, subject, expected).map_err(refused)?;
    if scope.target != "github.com/vibevm/vibevm" || scope.principal.is_empty() {
        return Err(refused(
            "host scope is not the fixed distribution target/principal",
        ));
    }
    let Some(PublicationPartition::Tag(step)) = plan.partitions.first() else {
        return Err(refused("first partition is not a tag"));
    };
    if step.action != PublicationPartitionTagAction::Create
        || !matches!(&plan.observed_tag, PublicationObservedTag::Absent(_))
        || !matches!(
            &plan.observed_release,
            PublicationObservedRelease::Absent(_)
        )
    {
        return Err(refused(
            "first fake route accepts only absent tag and release",
        ));
    }
    let tag_ref = format!("refs/tags/{}", step.tag);
    let request = serde_json::to_vec(&serde_json::json!({
        "format": "vibevm.publication.tag-request/1",
        "scope": {"context": scope.context, "target": scope.target, "principal": scope.principal},
        "subject_sha256": plan.subject_sha256,
        "semantic_plan_sha256": semantic_plan_sha256,
        "remote_generation_ref": plan.remote_generation_ref,
        "tag_ref": tag_ref,
        "desired_commit": step.desired_commit,
    }))?;
    let plan_bytes = serde_json::to_vec(plan)?;
    if request.len() > C1_CAP || plan_bytes.len() > C1_CAP {
        return Err(refused("C.1 payload cap"));
    }
    let intent = Intent {
        format: INTENT_FORMAT.into(),
        run_id: String::new(),
        request_sha256: sha(&request),
        c1_plan_sha256: sha(&plan_bytes),
        subject_sha256: plan.subject_sha256.clone(),
        semantic_plan_sha256,
        remote_generation_ref: plan.remote_generation_ref.clone(),
        target: scope.target.clone(),
        tag_ref,
        desired_commit: step.desired_commit.clone(),
    };
    Ok(FixedTag {
        request,
        plan_bytes,
        intent,
        tag: step.tag.clone(),
        commit: step.desired_commit.clone(),
    })
}

fn remote_matches_before(
    observed: &CapturedPublicationRemote,
    plan: &PublicationEffectPlan,
) -> bool {
    observed.generation_ref == plan.remote_generation_ref
        && observed.tag == plan.observed_tag
        && observed.release == plan.observed_release
}

fn inspect(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    selected: &FixedTag,
    run_id: String,
    ready: JournalAdmission,
    reservation_ref: Option<String>,
) -> Result<PublicationTagInspection> {
    let intent = match read::<Intent>(lease, &run_id, "intent") {
        Ok(Some(value)) => value,
        Ok(None) | Err(_) => return Ok(PublicationTagInspection::RecoveryRequired { run_id }),
    };
    let mut expected = selected.intent.clone();
    expected.run_id = run_id.clone();
    if ready != JournalAdmission::Ready
        || intent != expected
        || intent.c1_plan_sha256
            != SubmissionReservations::inspect_reserved_plan_sha256(
                lease, namespace, scope, key, &run_id,
            )?
    {
        return Ok(PublicationTagInspection::RecoveryRequired { run_id });
    }
    let pending = match read::<Partition>(lease, &run_id, "pending") {
        Ok(value) => value,
        Err(_) => {
            return Ok(PublicationTagInspection::Uncertain {
                run_id,
                pending: true,
                landed: false,
            });
        }
    };
    let Some(pending) = pending else {
        return Ok(PublicationTagInspection::RecoveryRequired { run_id });
    };
    if pending
        != (Partition {
            format: PARTITION_FORMAT.into(),
            run_id: run_id.clone(),
            c1_plan_sha256: intent.c1_plan_sha256.clone(),
            kind: "pending".into(),
            remote_generation_ref: intent.remote_generation_ref.clone(),
            tag_ref: intent.tag_ref.clone(),
            desired_commit: intent.desired_commit.clone(),
        })
    {
        return Ok(PublicationTagInspection::RecoveryRequired { run_id });
    }
    let landed_bytes = read_bytes(lease, &run_id, "landed").ok().flatten();
    let result = read::<ResultRecord>(lease, &run_id, "result")
        .ok()
        .flatten();
    let Some(result) = result else {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: landed_bytes.is_some(),
        });
    };
    if result.format != RESULT_FORMAT
        || result.run_id != run_id
        || result.reservation_ref != reservation_ref.unwrap_or_default()
        || result.c1_plan_sha256 != intent.c1_plan_sha256
    {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: landed_bytes.is_some(),
        });
    }
    if result.outcome == "refused-before-effect"
        && landed_bytes.is_none()
        && result.landed_sha256.is_none()
    {
        return Ok(PublicationTagInspection::RefusedBeforeEffect { run_id });
    }
    let Some(landed_bytes) = landed_bytes else {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: false,
        });
    };
    let landed: Partition = match serde_json::from_slice(&landed_bytes) {
        Ok(value) => value,
        Err(_) => {
            return Ok(PublicationTagInspection::Uncertain {
                run_id,
                pending: true,
                landed: true,
            });
        }
    };
    if result.outcome != "completed"
        || result.landed_sha256.as_deref() != Some(sha(&landed_bytes).as_str())
        || landed.format != PARTITION_FORMAT
        || landed.kind != "landed"
        || landed.run_id != run_id
        || landed.c1_plan_sha256 != intent.c1_plan_sha256
        || landed.tag_ref != intent.tag_ref
        || landed.desired_commit != intent.desired_commit
    {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: true,
        });
    }
    Ok(PublicationTagInspection::Completed {
        run_id,
        tag_ref: landed.tag_ref,
        commit: landed.desired_commit,
        replayed: true,
    })
}

/// Execute only the first fake-remote tag-create partition. The port has no
/// live implementation and this crate-private route grants no public write.
pub(crate) fn submit_fake_tag_create(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    port: &mut dyn TagCreatePort,
) -> Result<PublicationTagInspection> {
    let selected = fixed_tag(scope, subject, plan, expected)?;
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &selected.request,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(PublicationTagInspection::Retired { run_id: None });
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission,
            reservation_ref,
            ..
        } => {
            return inspect(
                lease,
                namespace,
                scope,
                key,
                &selected,
                domain_run_id,
                journal_admission,
                reservation_ref,
            );
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let before = port.observe().map_err(refused)?;
    if !remote_matches_before(&before, plan) {
        return Err(refused(
            "remote precondition changed before C.1 reservation",
        ));
    }
    let run_id = propose_run_id(lease.root())?;
    if !matches!(SubmissionReservations::reserve(lease, namespace, scope, key,
        &selected.request, &selected.plan_bytes, &run_id)?,
        ReservationDecision::Created { domain_run_id } if domain_run_id == run_id)
    {
        return Err(refused("fresh publication reservation was not Created"));
    }
    let mut intent = selected.intent;
    intent.run_id = run_id.clone();
    if publish(lease, &run_id, "intent", &intent).is_err()
        || !matches!(read::<Intent>(lease, &run_id, "intent"), Ok(Some(ref row)) if row == &intent)
        || SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)
            .is_err()
        || !matches!(
            SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id),
            Ok(JournalAdmission::Ready)
        )
    {
        return Ok(PublicationTagInspection::RecoveryRequired { run_id });
    }
    let pending = Partition {
        format: PARTITION_FORMAT.into(),
        run_id: run_id.clone(),
        c1_plan_sha256: intent.c1_plan_sha256.clone(),
        kind: "pending".into(),
        remote_generation_ref: intent.remote_generation_ref.clone(),
        tag_ref: intent.tag_ref.clone(),
        desired_commit: intent.desired_commit.clone(),
    };
    if publish(lease, &run_id, "pending", &pending).is_err()
        || !matches!(read::<Partition>(lease, &run_id, "pending"), Ok(Some(ref row)) if row == &pending)
    {
        return Ok(PublicationTagInspection::RecoveryRequired { run_id });
    }
    let reservation_ref =
        match SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id) {
            Ok(value) => value,
            Err(_) => return Ok(PublicationTagInspection::RecoveryRequired { run_id }),
        };
    let still_before = match port.observe() {
        Ok(value) => value,
        Err(_) => {
            return Ok(PublicationTagInspection::Uncertain {
                run_id,
                pending: true,
                landed: false,
            });
        }
    };
    if !remote_matches_before(&still_before, plan) {
        let refused_result = ResultRecord {
            format: RESULT_FORMAT.into(),
            run_id: run_id.clone(),
            reservation_ref,
            c1_plan_sha256: intent.c1_plan_sha256,
            outcome: "refused-before-effect".into(),
            landed_sha256: None,
        };
        return Ok(
            if publish(lease, &run_id, "result", &refused_result).is_ok() {
                PublicationTagInspection::RefusedBeforeEffect { run_id }
            } else {
                PublicationTagInspection::Uncertain {
                    run_id,
                    pending: true,
                    landed: false,
                }
            },
        );
    }
    let created =
        match port.create_tag(&plan.target, &still_before, &selected.tag, &selected.commit) {
            Ok(receipt) => receipt,
            Err(_) => {
                return Ok(PublicationTagInspection::Uncertain {
                    run_id,
                    pending: true,
                    landed: false,
                });
            }
        };
    if created.tag_ref != intent.tag_ref || created.commit != intent.desired_commit {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: false,
        });
    }
    let after = match port.observe() {
        Ok(value) => value,
        Err(_) => {
            return Ok(PublicationTagInspection::Uncertain {
                run_id,
                pending: true,
                landed: false,
            });
        }
    };
    let PublicationObservedTag::Present(observed_tag) = after.tag else {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: false,
        });
    };
    if after.generation_ref.is_empty()
        || after.generation_ref == intent.remote_generation_ref
        || observed_tag.object_kind != "commit"
        || observed_tag.reference != intent.tag_ref
        || observed_tag.object_sha != intent.desired_commit
        || after.release != plan.observed_release
    {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: false,
        });
    }
    let landed = Partition {
        format: PARTITION_FORMAT.into(),
        run_id: run_id.clone(),
        c1_plan_sha256: intent.c1_plan_sha256.clone(),
        kind: "landed".into(),
        remote_generation_ref: after.generation_ref,
        tag_ref: intent.tag_ref.clone(),
        desired_commit: intent.desired_commit.clone(),
    };
    let landed_bytes = match publish(lease, &run_id, "landed", &landed) {
        Ok(bytes) => bytes,
        Err(_) => {
            let landed_observed = read_bytes(lease, &run_id, "landed")
                .ok()
                .flatten()
                .is_some();
            return Ok(PublicationTagInspection::Uncertain {
                run_id,
                pending: true,
                landed: landed_observed,
            });
        }
    };
    let result = ResultRecord {
        format: RESULT_FORMAT.into(),
        run_id: run_id.clone(),
        reservation_ref,
        c1_plan_sha256: intent.c1_plan_sha256,
        outcome: "completed".into(),
        landed_sha256: Some(sha(&landed_bytes)),
    };
    if publish(lease, &run_id, "result", &result).is_err() {
        return Ok(PublicationTagInspection::Uncertain {
            run_id,
            pending: true,
            landed: true,
        });
    }
    Ok(PublicationTagInspection::Completed {
        run_id,
        tag_ref: landed.tag_ref,
        commit: landed.desired_commit,
        replayed: false,
    })
}

#[cfg(test)]
#[path = "publication_operation/tests.rs"]
mod tests;
