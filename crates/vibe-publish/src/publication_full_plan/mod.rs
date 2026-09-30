//! Private, fake-remote full-plan owner. Every partition has its own durable
//! Pending/Landed pair under one C.1 run. No live adapter or public entry exists.

use serde_json::json;
use vibe_lifecycle::process::propose_run_id;
use vibe_lifecycle::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::publication_effect_plan::{
    PublicationEffectPlan, PublicationObservedRelease, PublicationObservedTag,
    PublicationPartition, PublicationRemoteTarget,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_effect_plan::{
    PublicationEffectPlanExpectations, publication_effect_plan_digest,
    require_apply_ready_effect_plan,
};

mod journal;
use journal::{FinalResult, Intent, Step, landed_name, pending_name, sha};

const C1_CAP: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("full-plan fake publication refused: {0}")]
    Refused(String),
    #[error(transparent)]
    Submission(#[from] vibe_lifecycle::SubmissionError),
    #[error(transparent)]
    Scratch(#[from] vibe_lifecycle::process::ScratchError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
impl Error {
    fn io(error: impl std::fmt::Display) -> Self {
        Self::Refused(error.to_string())
    }
}
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inspection {
    Completed {
        run_id: String,
        partitions: usize,
        replayed: bool,
    },
    Partial {
        run_id: String,
        landed: usize,
        pending: Option<usize>,
        uncertain: bool,
    },
    RecoveryRequired {
        run_id: String,
    },
    Retired {
        run_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartitionReceipt {
    pub index: usize,
    pub partition_sha256: String,
    pub resulting_generation_ref: String,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteObservation {
    pub generation_ref: String,
    pub tag: PublicationObservedTag,
    pub release: PublicationObservedRelease,
    pub last: Option<PartitionReceipt>,
}

/// Only a test fake implements this crate-private port. Its apply call must
/// compare the exact expected generation atomically before changing state.
/// GitHub's multi-object API provides no equivalent blanket guarantee.
pub(crate) trait FakePlanPort {
    fn observe(&mut self) -> std::result::Result<RemoteObservation, String>;
    fn apply(
        &mut self,
        target: &PublicationRemoteTarget,
        index: usize,
        partition: &PublicationPartition,
        expected: &RemoteObservation,
    ) -> std::result::Result<PartitionReceipt, String>;
}

struct Selected {
    request: Vec<u8>,
    plan_bytes: Vec<u8>,
    intent: Intent,
}

fn select(
    scope: &SubmissionScope,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<Selected> {
    require_apply_ready_effect_plan(plan, subject, expected).map_err(Error::io)?;
    let semantic_plan_sha256 =
        publication_effect_plan_digest(plan, subject, expected).map_err(Error::io)?;
    if scope.target != "github.com/vibevm/vibevm"
        || scope.context.is_empty()
        || scope.principal.is_empty()
    {
        return Err(Error::Refused(
            "host scope differs from fixed publication target".into(),
        ));
    }
    let plan_bytes = serde_json::to_vec(plan)?;
    let partition_sha256 = plan
        .partitions
        .iter()
        .map(|row| serde_json::to_vec(row).map(|bytes| sha(&bytes)))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let request = serde_json::to_vec(&json!({
        "format": "vibevm.publication.full-plan-request/1",
        "scope": { "context": scope.context, "target": scope.target, "principal": scope.principal },
        "subject_sha256": plan.subject_sha256,
        "semantic_plan_sha256": semantic_plan_sha256,
        "remote_generation_ref": plan.remote_generation_ref,
        "partition_sha256": partition_sha256,
    }))?;
    if request.len() > C1_CAP || plan_bytes.len() > C1_CAP {
        return Err(Error::Refused("C.1 payload cap".into()));
    }
    let intent = Intent {
        format: journal::INTENT_FORMAT.into(),
        run_id: String::new(),
        request_sha256: sha(&request),
        c1_plan_sha256: sha(&plan_bytes),
        semantic_plan_sha256,
        subject_sha256: plan.subject_sha256.clone(),
        remote_generation_ref: plan.remote_generation_ref.clone(),
        target: scope.target.clone(),
        partition_sha256,
    };
    Ok(Selected {
        request,
        plan_bytes,
        intent,
    })
}

fn initial_matches(observed: &RemoteObservation, plan: &PublicationEffectPlan) -> bool {
    observed.generation_ref == plan.remote_generation_ref
        && observed.tag == plan.observed_tag
        && observed.release == plan.observed_release
        && observed.last.is_none()
}

fn partial(run_id: String, landed: usize, pending: Option<usize>, uncertain: bool) -> Inspection {
    Inspection::Partial {
        run_id,
        landed,
        pending,
        uncertain,
    }
}

fn expected_pending(intent: &Intent, index: usize, generation: &str) -> Step {
    Step {
        format: journal::STEP_FORMAT.into(),
        run_id: intent.run_id.clone(),
        c1_plan_sha256: intent.c1_plan_sha256.clone(),
        index,
        partition_sha256: intent.partition_sha256[index].clone(),
        expected_generation_ref: generation.into(),
        resulting_generation_ref: None,
        receipt_ref: None,
    }
}

fn inspect(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    selected: &Selected,
    run_id: String,
    admission: JournalAdmission,
    reservation_ref: Option<String>,
) -> Result<Inspection> {
    let intent = match journal::read::<Intent>(lease, &run_id, "intent") {
        Ok(Some(value)) => value,
        Ok(None) | Err(_) => return Ok(Inspection::RecoveryRequired { run_id }),
    };
    let mut expected = selected.intent.clone();
    expected.run_id = run_id.clone();
    if admission != JournalAdmission::Ready
        || intent != expected
        || intent.c1_plan_sha256
            != SubmissionReservations::inspect_reserved_plan_sha256(
                lease, namespace, scope, key, &run_id,
            )?
    {
        return Ok(Inspection::RecoveryRequired { run_id });
    }
    let mut generation = intent.remote_generation_ref.clone();
    let mut landed_hashes = Vec::with_capacity(intent.partition_sha256.len());
    for index in 0..intent.partition_sha256.len() {
        let pending = match journal::read::<Step>(lease, &run_id, &pending_name(index)) {
            Ok(value) => value,
            Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        let Some(pending) = pending else {
            return Ok(partial(run_id, index, None, false));
        };
        if pending != expected_pending(&intent, index, &generation) {
            return Ok(Inspection::RecoveryRequired { run_id });
        }
        let bytes = match journal::read_bytes(lease, &run_id, &landed_name(index)) {
            Ok(Some(bytes)) => bytes,
            Ok(None) | Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        let landed: Step = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        if landed.format != journal::STEP_FORMAT
            || landed.run_id != run_id
            || landed.c1_plan_sha256 != intent.c1_plan_sha256
            || landed.index != index
            || landed.partition_sha256 != intent.partition_sha256[index]
            || landed.expected_generation_ref != generation
            || landed
                .resulting_generation_ref
                .as_deref()
                .is_none_or(str::is_empty)
            || landed.receipt_ref.as_deref().is_none_or(str::is_empty)
        {
            return Ok(partial(run_id, index, Some(index), true));
        }
        generation = landed.resulting_generation_ref.clone().unwrap();
        landed_hashes.push(sha(&bytes));
    }
    let result = match journal::read::<FinalResult>(lease, &run_id, "result") {
        Ok(Some(value)) => value,
        Ok(None) | Err(_) => return Ok(partial(run_id, landed_hashes.len(), None, true)),
    };
    let expected_summary = sha(&serde_json::to_vec(&landed_hashes)?);
    if result.format != journal::RESULT_FORMAT
        || result.run_id != run_id
        || result.c1_plan_sha256 != intent.c1_plan_sha256
        || result.reservation_ref != reservation_ref.unwrap_or_default()
        || result.landed_sha256 != expected_summary
    {
        return Ok(partial(run_id, landed_hashes.len(), None, true));
    }
    Ok(Inspection::Completed {
        run_id,
        partitions: landed_hashes.len(),
        replayed: true,
    })
}

/// Fake-only full plan. A duplicate never calls the port, including after
/// response loss or an owner process exit. Later partitions remain absent
/// after uncertainty and cannot be inferred from matching remote bytes.
pub(crate) fn submit_fake_full_plan(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    port: &mut dyn FakePlanPort,
) -> Result<Inspection> {
    let selected = select(scope, subject, plan, expected)?;
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &selected.request,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(Inspection::Retired { run_id: None });
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
    let before = port.observe().map_err(Error::io)?;
    if !initial_matches(&before, plan) {
        return Err(Error::Refused(
            "captured remote precondition changed before reservation".into(),
        ));
    }
    let run_id = propose_run_id(lease.root())?;
    if !matches!(SubmissionReservations::reserve(lease, namespace, scope, key,
        &selected.request, &selected.plan_bytes, &run_id)?,
        ReservationDecision::Created { domain_run_id } if domain_run_id == run_id)
    {
        return Err(Error::Refused(
            "fresh full-plan reservation was not Created".into(),
        ));
    }
    let mut intent = selected.intent;
    intent.run_id = run_id.clone();
    if journal::publish(lease, &run_id, "intent", &intent).is_err()
        || !matches!(journal::read::<Intent>(lease, &run_id, "intent"), Ok(Some(ref row)) if row == &intent)
        || SubmissionReservations::mark_journal_ready(lease, namespace, scope, key, &run_id)
            .is_err()
        || !matches!(
            SubmissionReservations::journal_admission(lease, namespace, scope, key, &run_id),
            Ok(JournalAdmission::Ready)
        )
    {
        return Ok(Inspection::RecoveryRequired { run_id });
    }
    let reservation_ref =
        match SubmissionReservations::reservation_ref(lease, namespace, scope, key, &run_id) {
            Ok(value) => value,
            Err(_) => return Ok(Inspection::RecoveryRequired { run_id }),
        };
    let mut current = before;
    let mut landed_hashes = Vec::with_capacity(plan.partitions.len());
    for (index, partition) in plan.partitions.iter().enumerate() {
        let pending = expected_pending(&intent, index, &current.generation_ref);
        if journal::publish(lease, &run_id, &pending_name(index), &pending).is_err()
            || !matches!(journal::read::<Step>(lease, &run_id, &pending_name(index)), Ok(Some(ref row)) if row == &pending)
        {
            return Ok(partial(run_id, index, Some(index), true));
        }
        let observed = match port.observe() {
            Ok(value) => value,
            Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        if observed != current {
            return Ok(partial(run_id, index, Some(index), true));
        }
        let receipt = match port.apply(&plan.target, index, partition, &current) {
            Ok(value) => value,
            Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        if receipt.index != index
            || receipt.partition_sha256 != intent.partition_sha256[index]
            || receipt.resulting_generation_ref.is_empty()
            || receipt.resulting_generation_ref == current.generation_ref
            || receipt.receipt_ref.is_empty()
        {
            return Ok(partial(run_id, index, Some(index), true));
        }
        let after = match port.observe() {
            Ok(value) => value,
            Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        if after.generation_ref != receipt.resulting_generation_ref
            || after.last.as_ref() != Some(&receipt)
        {
            return Ok(partial(run_id, index, Some(index), true));
        }
        let landed = Step {
            format: journal::STEP_FORMAT.into(),
            run_id: run_id.clone(),
            c1_plan_sha256: intent.c1_plan_sha256.clone(),
            index,
            partition_sha256: receipt.partition_sha256,
            expected_generation_ref: current.generation_ref,
            resulting_generation_ref: Some(receipt.resulting_generation_ref),
            receipt_ref: Some(receipt.receipt_ref),
        };
        let landed_bytes = match journal::publish(lease, &run_id, &landed_name(index), &landed) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(partial(run_id, index, Some(index), true)),
        };
        landed_hashes.push(sha(&landed_bytes));
        current = after;
    }
    let result = FinalResult {
        format: journal::RESULT_FORMAT.into(),
        run_id: run_id.clone(),
        reservation_ref,
        c1_plan_sha256: intent.c1_plan_sha256,
        landed_sha256: sha(&serde_json::to_vec(&landed_hashes)?),
    };
    if journal::publish(lease, &run_id, "result", &result).is_err() {
        return Ok(partial(run_id, landed_hashes.len(), None, true));
    }
    Ok(Inspection::Completed {
        run_id,
        partitions: landed_hashes.len(),
        replayed: false,
    })
}

#[cfg(test)]
mod tests;
