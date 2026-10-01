//! Historical package-Git approval marker inspection. No new authority.
use super::*;
use crate::package_git_provenance_populated::ApprovedGitdir;
use crate::package_git_ready::profile::ReadyProfile;
use crate::package_git_submodule::SubmoduleLimits;

impl ApprovalHome {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn inspect_redemption_cold_profile(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        git_binary: &Path,
        profile: ReadyProfile<'_>,
        expected: &ColdExpectation<'_>,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        if !valid_human(expected.approval_id)
            || !valid_sha(expected.approval_record_sha256)
            || !valid_sha(expected.marker_sha256)
        {
            return Err(ApprovalError::Refused(
                "invalid cold marker identity".into(),
            ));
        }
        let ready = self.ready_profile(
            lease,
            namespace,
            scope,
            key,
            package_source,
            git_binary,
            profile,
        )?;
        self.inspect_redemption_with_ready(lease, namespace, scope, key, &ready, expected)
    }

    /// Read-only reuse within one inspection, never an effect grant. The deep
    /// Ready proof belongs to the caller; current identity is bracketed here.
    pub(super) fn inspect_redemption_with_ready(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        ready: &ReadyBinding,
        expected: &ColdExpectation<'_>,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        if !valid_human(expected.approval_id)
            || !valid_sha(expected.approval_record_sha256)
            || !valid_sha(expected.marker_sha256)
        {
            return Err(ApprovalError::Refused(
                "invalid cold marker identity".into(),
            ));
        }
        check_ready_identity(lease, namespace, scope, key, ready)?;
        let settings_root = self.checked_settings_root()?;
        let redeemed = self
            .settings
            .subproject(&[HOME, REDEEMED])
            .map_err(|e| ApprovalError::RecoveryRequired(format!("redemption home: {e:#}")))?;
        let marker_name = marker_name(expected.approval_id);
        let snapshot = redeemed
            .read_file_snapshot_bounded(&marker_name, CAP)
            .map_err(|e| ApprovalError::RecoveryRequired(format!("marker read: {e:#}")))?
            .ok_or_else(|| ApprovalError::RecoveryRequired("marker absent".into()))?;
        if sha(&snapshot.bytes) != expected.marker_sha256 {
            return Err(ApprovalError::RecoveryRequired(
                "marker digest changed".into(),
            ));
        }
        let marker: Redemption = canonical(&snapshot.bytes)
            .map_err(|e| ApprovalError::RecoveryRequired(e.to_string()))?;
        if marker.format != MARKER_FORMAT
            || marker.approval_id_sha256 != sha(expected.approval_id.as_bytes())
            || marker.approval_record_sha256 != expected.approval_record_sha256
            || marker.semantic_plan_sha256 != ready.seal.semantic_plan_sha256
            || marker.c1_plan_sha256 != ready.seal.c1_plan_sha256
            || marker.request_sha256 != ready.seal.request_sha256
            || marker.ready_seal_sha256 != ready.seal_sha256
            || marker.namespace_id != namespace.id()
            || marker.run_id != ready.seal.run_id
            || marker.key_sha256 != sha(key.as_bytes())
            || marker.context != scope.context
            || marker.target != scope.target
            || marker.principal != scope.principal
            || marker.workspace_root_token != ready.seal.lease_root_token
            || marker.settings_root_token != settings_root
            || marker.repository != ready.plan.target
            || marker.effects != effects(&ready.plan)?
        {
            return Err(ApprovalError::RecoveryRequired(
                "historical marker differs from exact Ready run".into(),
            ));
        }
        if self.checked_settings_root()? != settings_root {
            return Err(ApprovalError::RecoveryRequired(
                "settings root changed during cold inspection".into(),
            ));
        }
        check_ready_identity(lease, namespace, scope, key, ready)?;
        Ok(RedemptionReceipt {
            run_id: marker.run_id,
            marker_sha256: expected.marker_sha256.into(),
            approval_record_sha256: expected.approval_record_sha256.into(),
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn inspect_redemption_cold(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
        expected: &ColdExpectation<'_>,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        self.inspect_redemption_cold_profile(
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
            expected,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn inspect_redemption_populated_cold(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        approved_gitdirs: Option<&[ApprovedGitdir<'_>]>,
        git_binary: &Path,
        limits: SubmoduleLimits,
        expected: &ColdExpectation<'_>,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        self.inspect_redemption_cold_profile(
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
            expected,
        )
    }
}

/// Recheck only the live C.1 admission and exact raw seal/root, without
/// repeating retained payload, provenance and Git object hashing.
fn check_ready_identity(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    ready: &ReadyBinding,
) -> Result<(), ApprovalError> {
    let recovery = |reason: String| ApprovalError::RecoveryRequired(reason);
    let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
        .map_err(|e| recovery(format!("current C.1 row: {e:#}")))?
        .ok_or_else(|| recovery("current C.1 row absent".into()))?;
    let bytes = journal::read_bytes(lease, &ready.seal.run_id, "ready")
        .map_err(|e| recovery(format!("current Ready seal: {e:#}")))?
        .ok_or_else(|| recovery("current Ready seal absent".into()))?;
    let project = lease.package_git_journal_project();
    let root = project
        .root_correlation_token()
        .map_err(|e| recovery(format!("current workspace root: {e:#}")))?;
    let reopened = Project::open(project.root_path())
        .map_err(|e| recovery(format!("workspace root reopen: {e:#}")))?;
    if row.retired
        || row.journal_admission != JournalAdmission::Ready
        || row.domain_run_id != ready.seal.run_id
        || row.plan_sha256 != ready.seal.c1_plan_sha256
        || row.request_sha256 != ready.seal.request_sha256
        || ready.seal.namespace_id != namespace.id()
        || ready.seal.scope_context != scope.context
        || ready.seal.scope_target != scope.target
        || ready.seal.scope_principal != scope.principal
        || ready.seal.key_sha256 != sha(key.as_bytes())
        || sha(&bytes) != ready.seal_sha256
        || root != ready.seal.lease_root_token
        || reopened.root_correlation_token().ok().as_deref() != Some(root.as_str())
    {
        return Err(recovery("current Ready seal/C.1/scope/root changed".into()));
    }
    Ok(())
}
