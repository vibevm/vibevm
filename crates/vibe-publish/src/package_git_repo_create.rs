//! Private package-Git repository-create journal. Production has cold
//! inspection only; the sole dispatch port is compiled for local fake tests.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vibe_lifecycle::{LifecycleLease, SubmissionNamespace, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_approval::{ApprovalHome, LinkInspection};
use crate::package_git_operation::{self, journal};
use crate::package_git_plan::{
    CreationOptions, HostedRepository, PackageGitPlan, RawRef, RefSnapshot, RepositoryIdentity,
    RepositoryObservation, RepositoryTarget, TagRef,
};
use crate::package_git_ready::{self, ReadyInspection, ReadyLimits};

use journal::{
    REPO_CREATE_FORMAT, REPO_CREATE_PENDING_FORMAT, RepoCreateLanded, RepoCreatePending, sha,
};

const PENDING: &str = "repo-create-pending";
const LANDED: &str = "repo-create";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateInspection {
    NotStarted {
        run_id: String,
    },
    Landed {
        run_id: String,
        repository_id: u64,
        landed_sha256: String,
    },
    Unavailable {
        run_id: Option<String>,
        reason: String,
    },
    RecoveryRequired {
        run_id: Option<String>,
        reason: String,
    },
}

fn recovery(run: Option<&str>, reason: impl Into<String>) -> CreateInspection {
    CreateInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> CreateInspection {
    CreateInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn empty_refs() -> RefSnapshot {
    RefSnapshot {
        main: RawRef::Absent,
        version_tag: TagRef::Absent,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Proved {
    plan: PackageGitPlan,
    run_id: String,
    seal_sha256: String,
    seal: journal::ReadySeal,
    link_pending_sha256: String,
    link_landed_sha256: String,
}

#[allow(clippy::too_many_arguments)]
fn proved(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ReadyLimits,
    approval_id: &str,
) -> Result<Proved, CreateInspection> {
    let ReadyInspection::ReadyForApproval {
        run_id,
        seal_sha256,
    } = package_git_ready::inspect_ready_seal_cold(
        lease,
        namespace.id(),
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
    )
    else {
        return Err(recovery(None, "exact C.1 Ready seal unavailable"));
    };
    let LinkInspection::Landed {
        run_id: linked,
        pending_sha256,
        ..
    } = home.inspect_link_cold(
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    )
    else {
        return Err(recovery(
            Some(&run_id),
            "approval-link Landed cold proof unavailable",
        ));
    };
    if linked != run_id {
        return Err(recovery(Some(&run_id), "approval-link run differs"));
    }
    let landed_bytes = journal::read_bytes(lease, &run_id, "approval-link")
        .map_err(|e| recovery(Some(&run_id), e.to_string()))?
        .ok_or_else(|| recovery(Some(&run_id), "approval-link Landed disappeared"))?;
    let seal_bytes = journal::read_bytes(lease, &run_id, "ready")
        .map_err(|e| recovery(Some(&run_id), e.to_string()))?
        .ok_or_else(|| recovery(Some(&run_id), "Ready seal disappeared"))?;
    let seal: journal::ReadySeal =
        serde_json::from_slice(&seal_bytes).map_err(|e| recovery(Some(&run_id), e.to_string()))?;
    if sha(&seal_bytes) != seal_sha256
        || serde_json::to_vec(&seal).ok().as_deref() != Some(seal_bytes.as_slice())
    {
        return Err(recovery(Some(&run_id), "Ready seal changed"));
    }
    let (_, proof) =
        package_git_operation::ready_proof_cold(lease, namespace.id(), scope, key, package_source)
            .map_err(|e| recovery(Some(&run_id), e.to_string()))?;
    let proof = proof.ok_or_else(|| recovery(Some(&run_id), "Ready plan proof absent"))?;
    if proof.run_id != run_id
        || proof.plan.digest().ok().as_deref() != Some(seal.semantic_plan_sha256.as_str())
        || !matches!(package_git_ready::inspect_ready_seal_cold(
            lease, namespace.id(), scope, key, package_source, local_bare_source,
            git_binary, limits,
        ), ReadyInspection::ReadyForApproval { run_id: ref current, seal_sha256: ref digest }
            if current == &run_id && digest == &seal_sha256)
        || !matches!(journal::read_bytes(lease, &run_id, "approval-link"), Ok(Some(ref now)) if now == &landed_bytes)
    {
        return Err(recovery(
            Some(&run_id),
            "Ready or approval link changed during proof",
        ));
    }
    Ok(Proved {
        plan: proof.plan,
        run_id,
        seal_sha256,
        seal,
        link_pending_sha256: pending_sha256,
        link_landed_sha256: sha(&landed_bytes),
    })
}

fn expected_pending(
    proof: &Proved,
    repository: &HostedRepository,
    creation: &CreationOptions,
) -> RepoCreatePending {
    RepoCreatePending {
        format: REPO_CREATE_PENDING_FORMAT.into(),
        run_id: proof.run_id.clone(),
        namespace_id: proof.seal.namespace_id.clone(),
        scope_context: proof.seal.scope_context.clone(),
        scope_target: proof.seal.scope_target.clone(),
        scope_principal: proof.seal.scope_principal.clone(),
        key_sha256: proof.seal.key_sha256.clone(),
        lease_root_token: proof.seal.lease_root_token.clone(),
        semantic_plan_sha256: proof.seal.semantic_plan_sha256.clone(),
        c1_plan_sha256: proof.seal.c1_plan_sha256.clone(),
        request_sha256: proof.seal.request_sha256.clone(),
        ready_seal_sha256: proof.seal_sha256.clone(),
        approval_link_pending_sha256: proof.link_pending_sha256.clone(),
        approval_link_landed_sha256: proof.link_landed_sha256.clone(),
        repository: repository.clone(),
        creation: creation.clone(),
        before_repo: proof.plan.before_repo.clone(),
        before_refs: proof.plan.before_refs.clone(),
    }
}

fn create_shape(proof: &Proved) -> Result<(&HostedRepository, &CreationOptions), CreateInspection> {
    let RepositoryTarget::HostedCreate {
        repository,
        creation,
    } = &proof.plan.target
    else {
        return Err(unavailable(
            Some(&proof.run_id),
            "DirectGitExisting has no create call",
        ));
    };
    if !matches!(proof.plan.before_repo, RepositoryObservation::Absent)
        || proof.plan.before_refs != empty_refs()
    {
        return Err(unavailable(
            Some(&proof.run_id),
            "existing hosted repository needs no create call; protected update remains unavailable",
        ));
    }
    Ok((repository, creation))
}

/// Source-less historical inspection. A Pending-only create is recovery work,
/// even if a remote later happens to match the desired repository name.
#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_cold(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    git_binary: &Path,
    limits: ReadyLimits,
    approval_id: &str,
) -> CreateInspection {
    let proof = match proved(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        local_bare_source,
        git_binary,
        limits,
        approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run = proof.run_id.as_str();
    let (repository, creation) = match create_shape(&proof) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let pending_bytes = match journal::read_bytes(lease, run, PENDING) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return match journal::read_bytes(lease, run, LANDED) {
                Ok(None) => CreateInspection::NotStarted { run_id: run.into() },
                _ => recovery(Some(run), "create Landed exists without Pending"),
            };
        }
        Err(error) => return recovery(Some(run), error.to_string()),
    };
    let pending: RepoCreatePending = match serde_json::from_slice(&pending_bytes) {
        Ok(value) => value,
        Err(error) => return recovery(Some(run), error.to_string()),
    };
    if pending != expected_pending(&proof, repository, creation)
        || serde_json::to_vec(&pending).ok().as_deref() != Some(pending_bytes.as_slice())
    {
        return recovery(Some(run), "create Pending differs from approved Ready plan");
    }
    let landed_bytes = match journal::read_bytes(lease, run, LANDED) {
        Ok(Some(value)) => value,
        Ok(None) => return recovery(Some(run), "create Pending has no Landed; no retry"),
        Err(error) => return recovery(Some(run), error.to_string()),
    };
    let landed: RepoCreateLanded = match serde_json::from_slice(&landed_bytes) {
        Ok(value) => value,
        Err(error) => return recovery(Some(run), error.to_string()),
    };
    let RepositoryIdentity::Hosted {
        repository: returned_repo,
        id,
    } = &landed.returned_identity
    else {
        return recovery(Some(run), "create response lacks hosted identity");
    };
    if landed.format != REPO_CREATE_FORMAT
        || landed.run_id != run
        || landed.pending_sha256 != sha(&pending_bytes)
        || returned_repo != repository
        || *id == 0
        || landed.observed_repo != RepositoryObservation::Present(landed.returned_identity.clone())
        || landed.observed_refs != empty_refs()
        || serde_json::to_vec(&landed).ok().as_deref() != Some(landed_bytes.as_slice())
    {
        return recovery(
            Some(run),
            "create Landed does not prove exact returned/observed identity",
        );
    }
    if !matches!(journal::read_bytes(lease, run, PENDING), Ok(Some(ref now)) if now == &pending_bytes)
        || !matches!(journal::read_bytes(lease, run, LANDED), Ok(Some(ref now)) if now == &landed_bytes)
    {
        return recovery(Some(run), "create records changed during cold inspection");
    }
    CreateInspection::Landed {
        run_id: run.into(),
        repository_id: *id,
        landed_sha256: sha(&landed_bytes),
    }
}

#[cfg(test)]
pub(crate) mod tests;
