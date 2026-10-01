//! One strict internal Ready profile selector. A caller with a current
//! populated source must present the complete approved Gitdir set; historical
//! source-less inspection never silently becomes a current-source claim.

use super::*;
use crate::package_git_operation::journal::ReadyProvenance;
use crate::package_git_plan::PackageGitPlan;
use crate::package_git_provenance_populated::ApprovedGitdir;
use crate::package_git_submodule::SubmoduleLimits;

#[derive(Clone, Copy)]
pub(crate) enum ReadyProfile<'a> {
    NoGitlinks {
        limits: ReadyLimits,
        local_bare_source: Option<&'a Project>,
    },
    ParentlessPopulated {
        limits: SubmoduleLimits,
        approved_gitdirs: Option<&'a [ApprovedGitdir<'a>]>,
    },
}

pub(crate) struct VerifiedReady {
    pub plan: PackageGitPlan,
    pub seal: ReadySeal,
    pub seal_sha256: String,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_profile_cold(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    git_binary: &Path,
    profile: ReadyProfile<'_>,
) -> Result<VerifiedReady, ReadyInspection> {
    let selected = |profile: ReadyProfile<'_>| match profile {
        ReadyProfile::NoGitlinks {
            limits,
            local_bare_source,
        } => super::inspect_ready_seal_cold(
            lease,
            namespace.id(),
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
        ),
        ReadyProfile::ParentlessPopulated {
            limits,
            approved_gitdirs,
        } => {
            if package_source.is_some() != approved_gitdirs.is_some() {
                return unavailable(
                    None,
                    "current populated proof needs the complete approved Gitdir set",
                );
            }
            match super::populated::inspect_ready_cold(
                lease,
                namespace.id(),
                scope,
                key,
                package_source,
                approved_gitdirs,
                git_binary,
                limits,
            ) {
                super::populated::PopulatedInspection::ReadyForLocalProof {
                    run_id,
                    seal_sha256,
                } => ReadyInspection::ReadyForApproval {
                    run_id,
                    seal_sha256,
                },
                super::populated::PopulatedInspection::NotReady { run_id } => {
                    ReadyInspection::NotReady { run_id }
                }
                super::populated::PopulatedInspection::SealRetainedC1Pending {
                    run_id,
                    seal_sha256,
                } => ReadyInspection::SealRetainedC1Pending {
                    run_id,
                    seal_sha256,
                },
                super::populated::PopulatedInspection::Unavailable { run_id, reason } => {
                    ReadyInspection::Unavailable { run_id, reason }
                }
                super::populated::PopulatedInspection::RecoveryRequired { run_id, reason } => {
                    ReadyInspection::RecoveryRequired { run_id, reason }
                }
            }
        }
    };
    let initial = selected(profile);
    let ReadyInspection::ReadyForApproval {
        run_id,
        seal_sha256,
    } = initial
    else {
        return Err(initial);
    };
    let bytes = journal::read_bytes(lease, &run_id, SEAL)
        .map_err(|e| recovery(Some(&run_id), e.to_string()))?
        .ok_or_else(|| recovery(Some(&run_id), "Ready seal disappeared"))?;
    let seal: ReadySeal =
        serde_json::from_slice(&bytes).map_err(|e| recovery(Some(&run_id), e.to_string()))?;
    if sha(&bytes) != seal_sha256
        || seal.format != journal::READY_FORMAT
        || serde_json::to_vec(&seal).ok().as_deref() != Some(bytes.as_slice())
        || !matches!(
            (&profile, &seal.provenance),
            (
                ReadyProfile::NoGitlinks { .. },
                ReadyProvenance::NoGitlinks { .. }
            ) | (
                ReadyProfile::ParentlessPopulated { .. },
                ReadyProvenance::Populated { .. }
            )
        )
    {
        return Err(recovery(
            Some(&run_id),
            "Ready profile/seal arm or bytes changed",
        ));
    }
    let (_, proof) =
        package_git_operation::ready_proof_cold(lease, namespace.id(), scope, key, package_source)
            .map_err(|e| recovery(Some(&run_id), e.to_string()))?;
    let proof = proof.ok_or_else(|| recovery(Some(&run_id), "Ready plan proof absent"))?;
    let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
        .map_err(|e| recovery(Some(&run_id), e.to_string()))?
        .ok_or_else(|| recovery(Some(&run_id), "C.1 row absent"))?;
    if proof.run_id != run_id
        || proof.plan.digest().ok().as_deref() != Some(seal.semantic_plan_sha256.as_str())
        || row.retired
        || row.journal_admission != JournalAdmission::Ready
        || row.domain_run_id != run_id
        || row.plan_sha256 != seal.c1_plan_sha256
        || row.request_sha256 != seal.request_sha256
        || seal.run_id != run_id
        || seal.namespace_id != namespace.id()
        || seal.scope_context != scope.context
        || seal.scope_target != scope.target
        || seal.scope_principal != scope.principal
        || seal.key_sha256 != sha(key.as_bytes())
    {
        return Err(recovery(Some(&run_id), "Ready plan/C.1/scope differs"));
    }
    if !matches!(selected(profile), ReadyInspection::ReadyForApproval {
        run_id: ref again, seal_sha256: ref digest,
    } if again == &run_id && digest == &seal_sha256)
        || !matches!(journal::read_bytes(lease, &run_id, SEAL), Ok(Some(ref now)) if now == &bytes)
    {
        return Err(recovery(
            Some(&run_id),
            "Ready proof changed during profile inspection",
        ));
    }
    Ok(VerifiedReady {
        plan: proof.plan,
        seal,
        seal_sha256,
    })
}
