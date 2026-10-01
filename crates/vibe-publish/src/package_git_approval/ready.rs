//! Exact cold Ready seal and C.1 plan binding for owner approval selection.
use super::*;
use crate::package_git_ready::profile::{ReadyProfile, inspect_profile_cold};

impl ApprovalHome {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ready(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
    ) -> Result<ReadyBinding, ApprovalError> {
        self.ready_profile(
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn ready_profile(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        git_binary: &Path,
        profile: ReadyProfile<'_>,
    ) -> Result<ReadyBinding, ApprovalError> {
        let proof = inspect_profile_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            git_binary,
            profile,
        )
        .map_err(|status| match status {
            ReadyInspection::RecoveryRequired { reason, .. } => {
                ApprovalError::RecoveryRequired(reason)
            }
            other => ApprovalError::Refused(format!(
                "exact package-Git Ready profile unavailable: {other:?}"
            )),
        })?;
        Ok(ReadyBinding {
            plan: proof.plan,
            seal: proof.seal,
            seal_sha256: proof.seal_sha256,
        })
    }
}
