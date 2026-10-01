//! Owner-settings Mode A load for a selected, strictly proved Ready profile.
use super::*;
use crate::package_git_provenance_populated::ApprovedGitdir;
use crate::package_git_ready::profile::ReadyProfile;
use crate::package_git_submodule::SubmoduleLimits;

impl ApprovalHome {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn load_at_profile(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        git_binary: &Path,
        profile: ReadyProfile<'_>,
        now: u64,
    ) -> Result<VerifiedApproval, ApprovalError> {
        let ready = self.ready_profile(
            lease,
            namespace,
            scope,
            key,
            package_source,
            git_binary,
            profile,
        )?;
        let name = approval_name(&ready.seal.semantic_plan_sha256)?;
        let directory = self
            .settings
            .subproject(&[HOME, APPROVAL_DIR])
            .map_err(|e| {
                ApprovalError::Refused(format!("owner approval directory unavailable: {e:#}"))
            })?;
        let snapshot = directory
            .read_file_snapshot_bounded(&name, CAP)
            .map_err(|e| ApprovalError::Refused(format!("approval read: {e:#}")))?
            .ok_or_else(|| ApprovalError::Refused("exact owner approval absent".into()))?;
        let record: ApprovalRecord = canonical(&snapshot.bytes)?;
        let settings_root = self.checked_settings_root()?;
        if record.format != FORMAT
            || !valid_human(&record.approval_id)
            || !valid_human(&record.approver)
            || record.issued_at_unix >= record.expires_at_unix
            || now < record.issued_at_unix
            || now >= record.expires_at_unix
            || record.semantic_plan_sha256 != ready.seal.semantic_plan_sha256
            || record.c1_plan_sha256 != ready.seal.c1_plan_sha256
            || record.request_sha256 != ready.seal.request_sha256
            || record.ready_seal_sha256 != ready.seal_sha256
            || record.namespace_id != namespace.id()
            || record.run_id != ready.seal.run_id
            || record.key_sha256 != ready.seal.key_sha256
            || record.context != scope.context
            || record.target != scope.target
            || record.principal != scope.principal
            || record.workspace_root_token != ready.seal.lease_root_token
            || record.settings_root_token != settings_root
            || record.repository != ready.plan.target
            || record.effects != effects(&ready.plan)?
        {
            return Err(ApprovalError::Refused(
                "Mode A record does not approve the exact Ready run/plan/scope/time".into(),
            ));
        }
        Ok(VerifiedApproval {
            record,
            record_sha256: sha(&snapshot.bytes),
            record_name: name,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load_mode_a_populated(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        approved_gitdirs: Option<&[ApprovedGitdir<'_>]>,
        git_binary: &Path,
        limits: SubmoduleLimits,
    ) -> Result<VerifiedApproval, ApprovalError> {
        self.load_at_profile(
            lease,
            namespace,
            scope,
            key,
            package_source,
            git_binary,
            ReadyProfile::ParentlessPopulated {
                limits,
                approved_gitdirs,
            },
            host_now()?,
        )
    }
}

#[cfg(test)]
mod tests;
