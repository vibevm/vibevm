//! Private atomic package-Git push journal. Production exposes retained
//! inspection and an unconnected private owner; no public-host push port is
//! connected in this campaign.

use std::path::Path;

use vibe_lifecycle::{LifecycleLease, SubmissionNamespace, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_approval::{ApprovalHome, LinkInspection};
use crate::package_git_operation::{self, journal};
use crate::package_git_plan::{
    PackageGitPlan, RawRef, RefSnapshot, RepositoryIdentity, RepositoryObservation,
    RepositoryTarget, StagedCommit, TagRef,
};
use crate::package_git_ready::{
    ReadyLimits,
    profile::{ReadyProfile, inspect_profile_cold},
};
use crate::package_git_repo_create::{self, CreateInspection};
use journal::{
    ATOMIC_PUSH_FORMAT, ATOMIC_PUSH_PENDING_FORMAT, AtomicPushLanded, AtomicPushPending,
    RefUpdateKind, sha,
};

pub(crate) mod driver;

const PENDING: &str = "atomic-push-pending";
const LANDED: &str = "atomic-push";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PushReceipt {
    pub(crate) main_update: RefUpdateKind,
    pub(crate) tag_update: RefUpdateKind,
}

fn exact_updates(plan: &PackageGitPlan, receipt: PushReceipt) -> bool {
    let main = match (&plan.before_refs.main, &plan.staged.commit) {
        (
            RawRef::Absent,
            StagedCommit::New {
                parent_oid: None, ..
            },
        ) => RefUpdateKind::New,
        (
            RawRef::Present { oid },
            StagedCommit::New {
                parent_oid: Some(parent),
                ..
            },
        ) if oid == parent => RefUpdateKind::FastForward,
        _ => return false,
    };
    let tag = match &plan.before_refs.version_tag {
        TagRef::Absent => RefUpdateKind::New,
        TagRef::Present { .. } => RefUpdateKind::Forced,
    };
    receipt.main_update == main && receipt.tag_update == tag
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PushInspection {
    NotStarted {
        run_id: String,
    },
    PendingRecoveryRequired {
        run_id: String,
        pending_sha256: String,
    },
    Landed {
        run_id: String,
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

fn recovery(run: Option<&str>, reason: impl Into<String>) -> PushInspection {
    PushInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> PushInspection {
    PushInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Proved {
    plan: PackageGitPlan,
    seal: journal::ReadySeal,
    seal_sha256: String,
    approval_link_landed_sha256: String,
    repo_create_landed_sha256: Option<String>,
    repository: RepositoryIdentity,
    desired_refs: RefSnapshot,
}

impl Proved {
    fn run(&self) -> &str {
        &self.seal.run_id
    }
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
) -> Result<Proved, PushInspection> {
    proved_profile(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        git_binary,
        ReadyProfile::NoGitlinks {
            limits,
            local_bare_source,
        },
        approval_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn proved_profile(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    git_binary: &Path,
    profile: ReadyProfile<'_>,
    approval_id: &str,
) -> Result<Proved, PushInspection> {
    let ready = inspect_profile_cold(
        lease,
        namespace,
        scope,
        key,
        package_source,
        git_binary,
        profile,
    )
    .map_err(|status| {
        recovery(
            None,
            format!("exact C.1 Ready profile unavailable: {status:?}"),
        )
    })?;
    let run_id = ready.seal.run_id.clone();
    let seal_sha256 = ready.seal_sha256;
    let link = match profile {
        ReadyProfile::NoGitlinks {
            limits,
            local_bare_source,
        } => home.inspect_link_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            approval_id,
        ),
        ReadyProfile::ParentlessPopulated {
            limits,
            approved_gitdirs,
        } => home.inspect_link_populated_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            approved_gitdirs,
            git_binary,
            limits,
            approval_id,
        ),
    };
    if !matches!(link, LinkInspection::Landed { run_id: linked, .. } if linked == run_id) {
        return Err(recovery(
            Some(&run_id),
            "approval-link Landed cold proof unavailable",
        ));
    }
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
    let (_, pending_proof) =
        package_git_operation::ready_proof_cold(lease, namespace.id(), scope, key, package_source)
            .map_err(|e| recovery(Some(&run_id), e.to_string()))?;
    let pending_proof =
        pending_proof.ok_or_else(|| recovery(Some(&run_id), "Ready plan proof absent"))?;
    if pending_proof.run_id != run_id
        || pending_proof.plan.digest().ok().as_deref() != Some(seal.semantic_plan_sha256.as_str())
    {
        return Err(recovery(Some(&run_id), "Ready plan differs"));
    }
    let plan = pending_proof.plan;
    if plan != ready.plan || seal != ready.seal {
        return Err(recovery(
            Some(&run_id),
            "Ready profile differs from retained plan/seal",
        ));
    }
    if matches!(plan.staged.commit, StagedCommit::ReusedMain { .. }) {
        return Err(unavailable(
            Some(&run_id),
            "ReusedMain no-op has no protected terminal owner",
        ));
    }
    let desired_refs = plan
        .desired_refs()
        .ok_or_else(|| unavailable(Some(&run_id), "annotated tag object absent"))?;
    let link_landed = journal::read_bytes(lease, &run_id, "approval-link")
        .map_err(|e| recovery(Some(&run_id), e.to_string()))?
        .ok_or_else(|| recovery(Some(&run_id), "approval-link Landed disappeared"))?;
    let (repository, repo_create_landed_sha256) = match (&plan.target, &plan.before_repo) {
        (RepositoryTarget::HostedCreate { repository, .. }, RepositoryObservation::Absent) => {
            let CreateInspection::Landed {
                run_id: created,
                repository_id,
                landed_sha256,
            } = package_git_repo_create::inspect_cold_profile(
                home,
                lease,
                namespace,
                scope,
                key,
                package_source,
                git_binary,
                profile,
                approval_id,
            )
            else {
                return Err(recovery(
                    Some(&run_id),
                    "hosted repo-create Landed cold proof unavailable",
                ));
            };
            if created != run_id || repository_id == 0 {
                return Err(recovery(
                    Some(&run_id),
                    "created repository identity differs",
                ));
            }
            (
                RepositoryIdentity::Hosted {
                    repository: repository.clone(),
                    id: repository_id,
                },
                Some(landed_sha256),
            )
        }
        (
            RepositoryTarget::DirectGitExisting {
                remote_identity_sha256,
            },
            RepositoryObservation::Present(RepositoryIdentity::Direct {
                remote_identity_sha256: observed,
            }),
        ) if remote_identity_sha256 == observed => {
            if !matches!(
                journal::read_bytes(lease, &run_id, "repo-create-pending"),
                Ok(None)
            ) || !matches!(journal::read_bytes(lease, &run_id, "repo-create"), Ok(None))
            {
                return Err(recovery(
                    Some(&run_id),
                    "DirectGitExisting carries unexpected create records",
                ));
            }
            (
                RepositoryIdentity::Direct {
                    remote_identity_sha256: observed.clone(),
                },
                None,
            )
        }
        (RepositoryTarget::HostedCreate { .. }, RepositoryObservation::Present(_)) => {
            return Err(unavailable(
                Some(&run_id),
                "existing hosted repository Ready route is unavailable",
            ));
        }
        _ => {
            return Err(unavailable(
                Some(&run_id),
                "unsupported repository identity for atomic push",
            ));
        }
    };
    if !matches!(inspect_profile_cold(
        lease, namespace, scope, key, package_source, git_binary, profile,
    ), Ok(ref current) if current.seal == seal && current.seal_sha256 == seal_sha256 && current.plan == plan)
        || !matches!(journal::read_bytes(lease, &run_id, "approval-link"), Ok(Some(ref now)) if now == &link_landed)
    {
        return Err(recovery(
            Some(&run_id),
            "local authority changed during push proof",
        ));
    }
    Ok(Proved {
        plan,
        seal,
        seal_sha256,
        approval_link_landed_sha256: sha(&link_landed),
        repo_create_landed_sha256,
        repository,
        desired_refs,
    })
}

fn expected_pending(proof: &Proved) -> AtomicPushPending {
    AtomicPushPending {
        format: ATOMIC_PUSH_PENDING_FORMAT.into(),
        run_id: proof.run().into(),
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
        approval_link_landed_sha256: proof.approval_link_landed_sha256.clone(),
        repo_create_landed_sha256: proof.repo_create_landed_sha256.clone(),
        repository: proof.repository.clone(),
        before_refs: proof.plan.before_refs.clone(),
        desired_refs: proof.desired_refs.clone(),
        object_root_token: proof.seal.object_root_token.clone(),
        object_receipt_sha256: proof.seal.object_receipt_sha256.clone(),
        selected_tree_oid: proof.seal.selected_tree_oid.clone(),
        selected_commit_oid: proof.seal.selected_commit_oid.clone(),
        selected_tag_oid: proof.seal.selected_tag_oid.clone(),
    }
}

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
) -> PushInspection {
    inspect_cold_profile(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        git_binary,
        ReadyProfile::NoGitlinks {
            limits,
            local_bare_source,
        },
        approval_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_cold_profile(
    home: &ApprovalHome,
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    git_binary: &Path,
    profile: ReadyProfile<'_>,
    approval_id: &str,
) -> PushInspection {
    let proof = match proved_profile(
        home,
        lease,
        namespace,
        scope,
        key,
        package_source,
        git_binary,
        profile,
        approval_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run = proof.run();
    let pending_bytes = match journal::read_bytes(lease, run, PENDING) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return match journal::read_bytes(lease, run, LANDED) {
                Ok(None) => PushInspection::NotStarted { run_id: run.into() },
                _ => recovery(Some(run), "atomic-push Landed lacks Pending"),
            };
        }
        Err(e) => return recovery(Some(run), e.to_string()),
    };
    let pending: AtomicPushPending = match serde_json::from_slice(&pending_bytes) {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), e.to_string()),
    };
    if pending != expected_pending(&proof)
        || serde_json::to_vec(&pending).ok().as_deref() != Some(pending_bytes.as_slice())
    {
        return recovery(
            Some(run),
            "atomic-push Pending differs from exact plan/authority",
        );
    }
    let landed_bytes = match journal::read_bytes(lease, run, LANDED) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return PushInspection::PendingRecoveryRequired {
                run_id: run.into(),
                pending_sha256: sha(&pending_bytes),
            };
        }
        Err(e) => return recovery(Some(run), e.to_string()),
    };
    let landed: AtomicPushLanded = match serde_json::from_slice(&landed_bytes) {
        Ok(value) => value,
        Err(e) => return recovery(Some(run), e.to_string()),
    };
    if landed.format != ATOMIC_PUSH_FORMAT
        || landed.run_id != run
        || landed.pending_sha256 != sha(&pending_bytes)
        || !landed.callback_exit_zero
        || !exact_updates(
            &proof.plan,
            PushReceipt {
                main_update: landed.main_update,
                tag_update: landed.tag_update,
            },
        )
        || landed.observed_repository != RepositoryObservation::Present(proof.repository.clone())
        || landed.observed_refs != proof.desired_refs
        || serde_json::to_vec(&landed).ok().as_deref() != Some(landed_bytes.as_slice())
    {
        return recovery(
            Some(run),
            "atomic-push Landed lacks exit-zero/exact poststate",
        );
    }
    if !matches!(journal::read_bytes(lease, run, PENDING), Ok(Some(ref now)) if now == &pending_bytes)
        || !matches!(journal::read_bytes(lease, run, LANDED), Ok(Some(ref now)) if now == &landed_bytes)
    {
        return recovery(Some(run), "atomic-push records changed during cold proof");
    }
    PushInspection::Landed {
        run_id: run.into(),
        landed_sha256: sha(&landed_bytes),
    }
}

#[cfg(test)]
pub(crate) mod tests;
