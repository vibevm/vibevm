//! Private package-Git C.1 Pending owner. Local payload custody is the only
//! effect here; Gitlink/object proof, Ready, RepoCreator and Git remain absent.

use serde_json::json;
use vibe_lifecycle::process::propose_run_id;
use vibe_lifecycle::{
    ExistingSubmissionInspection, JournalAdmission, LifecycleLease, ReservationDecision,
    SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;

use crate::package_git_payload::{
    LocalPayloadInspection, PayloadLimits, PayloadStageError, ProvenanceStatus,
    capture_source_identity, inspect_local_payload, stage_local_payload,
};
use crate::package_git_plan::{PackageGitPlan, RepositoryTarget};

pub(crate) mod journal;
mod proof;
use journal::{Intent, Request, StageReceipt, sha};
#[cfg(test)]
use proof::pending_proof_cold_inner;
pub(crate) use proof::{pending_proof_cold, ready_proof_cold};

/// One cold-proved local Pending run. This retains the exact byte vault and
/// parsed plan for the next local object stage; it grants no remote effect.
#[derive(Debug)]
pub(crate) struct PendingProof {
    pub run_id: String,
    pub plan: PackageGitPlan,
    pub manifest: crate::package_git_payload::CompleteManifest,
    pub vault: Project,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingInspection {
    Absent,
    PendingIncomplete {
        run_id: String,
    },
    VaultCompleteButNotReady {
        run_id: String,
        tree_sha256: String,
    },
    /// Internal byte/C.1 witness only. A Ready seal, approval and effect owner
    /// are still required; this variant never permits publication dispatch.
    VaultCompleteReady {
        run_id: String,
        tree_sha256: String,
    },
    /// Duplicate preparation found the same Ready reservation and seal, but
    /// this API lacks held Git inputs for full object cold proof. No effect.
    ReadyRetainedNeedsColdProof {
        run_id: String,
    },
    Uncertain {
        run_id: String,
        reason: String,
    },
    RecoveryRequired {
        run_id: String,
    },
    Retired {
        run_id: Option<String>,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OperationError {
    #[error("protected package Git Pending owner refused: {0}")]
    Refused(String),
    #[error(transparent)]
    Submission(#[from] vibe_lifecycle::SubmissionError),
    #[error(transparent)]
    Scratch(#[from] vibe_lifecycle::process::ScratchError),
}
fn refused(error: impl std::fmt::Display) -> OperationError {
    OperationError::Refused(error.to_string())
}
fn target(plan: &PackageGitPlan) -> String {
    match &plan.target {
        RepositoryTarget::HostedCreate { repository, .. } => format!(
            "package-git:{}/{}/{}",
            repository.host, repository.organization, repository.name,
        ),
        RepositoryTarget::DirectGitExisting {
            remote_identity_sha256,
        } => format!("package-git-direct:{remote_identity_sha256}"),
    }
}

/// Reserve a unique C.1 run, retain intent, and stage local bytes while C.1
/// remains Pending. An existing key is inspected, never staged a second time.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_pending(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    plan: &PackageGitPlan,
    limits: PayloadLimits,
) -> Result<PendingInspection, OperationError> {
    prepare_pending_inner(lease, namespace, scope, key, source, plan, limits, || false)
}

#[allow(clippy::too_many_arguments)]
fn prepare_pending_inner(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    plan: &PackageGitPlan,
    limits: PayloadLimits,
    after_intent: impl FnOnce() -> bool,
) -> Result<PendingInspection, OperationError> {
    let semantic_plan_sha256 = plan.digest().map_err(refused)?;
    if scope.target != target(plan)
        || scope.context.is_empty()
        || scope.principal.is_empty()
        || key.is_empty()
    {
        return Err(refused(
            "C.1 scope/key differs from selected package Git target",
        ));
    }
    let selected = capture_source_identity(source, limits).map_err(refused)?;
    if selected.tree_sha256 != plan.payload_sha256 {
        return Err(refused(
            "selected exact source tree differs from package Git plan",
        ));
    }
    let lease_root_token = lease
        .package_git_journal_project()
        .root_correlation_token()
        .map_err(refused)?;
    let request = Request {
        format: journal::REQUEST_FORMAT.into(),
        semantic_plan_sha256: semantic_plan_sha256.clone(),
        source_root_token: selected.root_token.clone(),
        source_tree_sha256: selected.tree_sha256.clone(),
        skipped_gitmodules: selected.skipped_gitmodules,
        lease_root_token: lease_root_token.clone(),
        scope_target: scope.target.clone(),
        limits,
    };
    let request_bytes = serde_json::to_vec(&request).map_err(refused)?;
    let plan_bytes = serde_json::to_vec(plan).map_err(refused)?;
    match SubmissionReservations::inspect_existing_request(
        lease,
        namespace,
        scope,
        key,
        &request_bytes,
    )? {
        ExistingSubmissionInspection::RetiredUnreserved => {
            return Ok(PendingInspection::Retired { run_id: None });
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission: JournalAdmission::Pending,
            ..
        } => {
            let _ = domain_run_id;
            return inspect_pending_cold(lease, namespace.id(), scope, key, Some(source));
        }
        ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission: JournalAdmission::Ready,
            retired: false,
            ..
        } => {
            let observed = inspect_ready_cold(lease, namespace.id(), scope, key, Some(source))?;
            if !matches!(observed, PendingInspection::VaultCompleteReady { ref run_id, .. } if run_id == &domain_run_id)
            {
                return Ok(PendingInspection::RecoveryRequired {
                    run_id: domain_run_id,
                });
            }
            if !journal::ready_duplicate_matches(
                lease,
                &domain_run_id,
                namespace.id(),
                scope,
                key,
                &request_bytes,
                plan,
                &plan_bytes,
                &lease_root_token,
                &selected.root_token,
            ) {
                return Ok(PendingInspection::RecoveryRequired {
                    run_id: domain_run_id,
                });
            }
            return Ok(PendingInspection::ReadyRetainedNeedsColdProof {
                run_id: domain_run_id,
            });
        }
        ExistingSubmissionInspection::Reserved { domain_run_id, .. } => {
            return Ok(PendingInspection::RecoveryRequired {
                run_id: domain_run_id,
            });
        }
        ExistingSubmissionInspection::Absent => {}
    }
    let run_id = propose_run_id(lease.root())?;
    if !matches!(SubmissionReservations::reserve(
        lease, namespace, scope, key, &request_bytes, &plan_bytes, &run_id,
    )?, ReservationDecision::Created { domain_run_id } if domain_run_id == run_id)
    {
        return Err(refused("fresh package Git C.1 reservation was not Created"));
    }
    let intent = Intent {
        format: journal::INTENT_FORMAT.into(),
        run_id: run_id.clone(),
        request_json: String::from_utf8(request_bytes.clone()).map_err(refused)?,
        plan_json: String::from_utf8(plan_bytes.clone()).map_err(refused)?,
        request_sha256: sha(&request_bytes),
        c1_plan_sha256: sha(&plan_bytes),
        semantic_plan_sha256,
        source_root_token: selected.root_token,
        source_tree_sha256: selected.tree_sha256,
        skipped_gitmodules: selected.skipped_gitmodules,
        lease_root_token,
        scope_context: scope.context.clone(),
        scope_target: scope.target.clone(),
        scope_principal: scope.principal.clone(),
        key_sha256: sha(key.as_bytes()),
        limits,
    };
    let intent_bytes = match journal::publish(lease, &run_id, "intent", &intent) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(PendingInspection::RecoveryRequired { run_id }),
    };
    if after_intent() {
        return Ok(PendingInspection::PendingIncomplete { run_id });
    }
    let vault = match journal::vault(lease, &run_id, true) {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(PendingInspection::PendingIncomplete { run_id }),
        Err(error) => {
            return Ok(PendingInspection::Uncertain {
                run_id,
                reason: error.to_string(),
            });
        }
    };
    let manifest = match stage_local_payload(source, &vault, limits) {
        Ok(value) => value,
        Err(PayloadStageError::PossiblyPublished { reason, .. }) => {
            return Ok(PendingInspection::Uncertain { run_id, reason });
        }
        Err(_) => return Ok(PendingInspection::PendingIncomplete { run_id }),
    };
    if manifest.tree_sha256 != plan.payload_sha256
        || manifest.source_root_token != intent.source_root_token
        || manifest.skipped_gitmodules != intent.skipped_gitmodules
        || manifest.provenance != ProvenanceStatus::NeedsGitlinkCheck
    {
        return Ok(PendingInspection::Uncertain {
            run_id,
            reason: "staged payload differs from bound plan".into(),
        });
    }
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(value) => value,
        Err(error) => {
            return Ok(PendingInspection::Uncertain {
                run_id,
                reason: error.to_string(),
            });
        }
    };
    let receipt = StageReceipt {
        format: journal::STAGE_FORMAT.into(),
        run_id: run_id.clone(),
        intent_sha256: sha(&intent_bytes),
        manifest_sha256: sha(&manifest_bytes),
        tree_sha256: manifest.tree_sha256,
        source_root_token: manifest.source_root_token,
        destination_root_token: manifest.destination_root_token,
        provenance: ProvenanceStatus::NeedsGitlinkCheck,
    };
    if journal::publish(lease, &run_id, "stage", &receipt).is_err() {
        return Ok(PendingInspection::Uncertain {
            run_id,
            reason: "stage receipt publication uncertain".into(),
        });
    }
    inspect_pending_cold(lease, namespace.id(), scope, key, Some(source))
}

/// Historical local inspection. `source` is a fresh host-held selection when
/// available; no source path from an authored record is opened as authority.
pub(crate) fn inspect_pending_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
) -> Result<PendingInspection, OperationError> {
    inspect_local_cold(
        lease,
        namespace_token,
        scope,
        key,
        source,
        JournalAdmission::Pending,
    )
}

pub(crate) fn inspect_ready_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
) -> Result<PendingInspection, OperationError> {
    // This only rehashes local bytes under a Ready C.1 row. The package-Git
    // Ready seal owner alone may return ReadyForApproval.
    inspect_local_cold(
        lease,
        namespace_token,
        scope,
        key,
        source,
        JournalAdmission::Ready,
    )
}

fn inspect_local_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    required: JournalAdmission,
) -> Result<PendingInspection, OperationError> {
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())?;
    let Some(row) = SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)?
    else {
        return Ok(PendingInspection::Absent);
    };
    let run_id = row.domain_run_id;
    if row.retired {
        return Ok(PendingInspection::Retired {
            run_id: Some(run_id),
        });
    }
    if row.journal_admission != required {
        return Ok(PendingInspection::RecoveryRequired { run_id });
    }
    let intent_bytes = match journal::read_bytes(lease, &run_id, "intent") {
        Ok(Some(value)) => value,
        _ => return Ok(PendingInspection::RecoveryRequired { run_id }),
    };
    let intent: Intent = match serde_json::from_slice(&intent_bytes) {
        Ok(value)
            if serde_json::to_vec(&value).ok().as_deref() == Some(intent_bytes.as_slice()) =>
        {
            value
        }
        _ => return Ok(PendingInspection::RecoveryRequired { run_id }),
    };
    let request: Request = match serde_json::from_str(&intent.request_json) {
        Ok(value) => value,
        Err(_) => return Ok(PendingInspection::RecoveryRequired { run_id }),
    };
    if intent.format != journal::INTENT_FORMAT
        || request.format != journal::REQUEST_FORMAT
        || intent.run_id != run_id
        || intent.request_sha256 != sha(intent.request_json.as_bytes())
        || intent.c1_plan_sha256 != sha(intent.plan_json.as_bytes())
        || intent.request_sha256 != row.request_sha256
        || intent.c1_plan_sha256 != row.plan_sha256
        || intent.semantic_plan_sha256 != request.semantic_plan_sha256
        || intent.source_root_token != request.source_root_token
        || intent.source_tree_sha256 != request.source_tree_sha256
        || intent.skipped_gitmodules != request.skipped_gitmodules
        || intent.lease_root_token != request.lease_root_token
        || intent.scope_target != request.scope_target
        || intent.limits != request.limits
        || intent.scope_context != scope.context
        || intent.scope_target != scope.target
        || intent.scope_principal != scope.principal
        || intent.key_sha256 != sha(key.as_bytes())
        || intent.lease_root_token
            != lease
                .package_git_journal_project()
                .root_correlation_token()
                .map_err(refused)?
        || serde_json::to_vec(&request).map_err(refused)?.as_slice()
            != intent.request_json.as_bytes()
    {
        return Ok(PendingInspection::RecoveryRequired { run_id });
    }
    let parsed_plan: PackageGitPlan = match serde_json::from_str(&intent.plan_json) {
        Ok(value) => value,
        Err(_) => return Ok(PendingInspection::RecoveryRequired { run_id }),
    };
    if serde_json::to_vec(&parsed_plan)
        .map_err(refused)?
        .as_slice()
        != intent.plan_json.as_bytes()
        || parsed_plan.digest().ok().as_deref() != Some(intent.semantic_plan_sha256.as_str())
        || target(&parsed_plan) != scope.target
        || parsed_plan.payload_sha256 != intent.source_tree_sha256
    {
        return Ok(PendingInspection::RecoveryRequired { run_id });
    }
    if let Some(source) = source {
        let observed = match capture_source_identity(source, intent.limits) {
            Ok(value) => value,
            Err(error) => {
                return Ok(PendingInspection::Uncertain {
                    run_id,
                    reason: error.to_string(),
                });
            }
        };
        if observed.root_token != intent.source_root_token
            || observed.tree_sha256 != intent.source_tree_sha256
            || observed.skipped_gitmodules != intent.skipped_gitmodules
        {
            return Ok(PendingInspection::Uncertain {
                run_id,
                reason: "host-selected source root or full census changed".into(),
            });
        }
    }
    let vault = match journal::vault(lease, &run_id, false) {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(PendingInspection::PendingIncomplete { run_id }),
        Err(error) => {
            return Ok(PendingInspection::Uncertain {
                run_id,
                reason: error.to_string(),
            });
        }
    };
    let manifest = match inspect_local_payload(
        &vault,
        intent.limits,
        &intent.source_root_token,
        &intent.source_tree_sha256,
    ) {
        LocalPayloadInspection::Missing | LocalPayloadInspection::Incomplete => {
            return Ok(PendingInspection::PendingIncomplete { run_id });
        }
        LocalPayloadInspection::Uncertain(reason) => {
            return Ok(PendingInspection::Uncertain { run_id, reason });
        }
        LocalPayloadInspection::Complete(value) => value,
    };
    if manifest.skipped_gitmodules != intent.skipped_gitmodules {
        return Ok(PendingInspection::Uncertain {
            run_id,
            reason: "Git metadata census changed during staging".into(),
        });
    }
    if manifest.tree_sha256 != parsed_plan.payload_sha256 {
        return Ok(PendingInspection::Uncertain {
            run_id,
            reason: "vault payload differs from parsed C.1 plan".into(),
        });
    }
    let receipt: StageReceipt = match journal::read(lease, &run_id, "stage") {
        Ok(Some(value)) => value,
        _ => {
            return Ok(PendingInspection::Uncertain {
                run_id,
                reason: "stage receipt absent or unreadable".into(),
            });
        }
    };
    let manifest_bytes = serde_json::to_vec(&manifest).map_err(refused)?;
    if receipt.format != journal::STAGE_FORMAT
        || receipt.run_id != run_id
        || receipt.intent_sha256 != sha(&intent_bytes)
        || receipt.manifest_sha256 != sha(&manifest_bytes)
        || receipt.tree_sha256 != manifest.tree_sha256
        || receipt.source_root_token != manifest.source_root_token
        || receipt.destination_root_token != manifest.destination_root_token
        || receipt.provenance != ProvenanceStatus::NeedsGitlinkCheck
    {
        return Ok(PendingInspection::Uncertain {
            run_id,
            reason: "stage receipt differs from exact local payload".into(),
        });
    }
    Ok(match required {
        JournalAdmission::Pending => PendingInspection::VaultCompleteButNotReady {
            run_id,
            tree_sha256: manifest.tree_sha256,
        },
        JournalAdmission::Ready => PendingInspection::VaultCompleteReady {
            run_id,
            tree_sha256: manifest.tree_sha256,
        },
    })
}

/// Reconstruct a held byte-vault proof from one C.1 Pending run. The second
/// cold scan brackets the borrowed proof; a changed record yields uncertainty.
#[cfg(test)]
#[path = "package_git_operation/tests.rs"]
mod tests;
