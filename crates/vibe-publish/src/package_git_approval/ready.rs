//! Exact cold Ready seal and C.1 plan binding for approval admission.
use super::*;

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
        let status = package_git_ready::inspect_ready_seal_cold(
            lease,
            namespace.id(),
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
        );
        let ReadyInspection::ReadyForApproval {
            run_id,
            seal_sha256,
        } = status
        else {
            return Err(match status {
                ReadyInspection::RecoveryRequired { reason, .. } => {
                    ApprovalError::RecoveryRequired(reason)
                }
                other => ApprovalError::Refused(format!(
                    "exact package-Git Ready seal unavailable: {other:?}"
                )),
            });
        };
        let seal_bytes = journal::read_bytes(lease, &run_id, "ready")
            .map_err(|e| ApprovalError::RecoveryRequired(format!("Ready seal read: {e:#}")))?
            .ok_or_else(|| ApprovalError::RecoveryRequired("Ready seal disappeared".into()))?;
        if sha(&seal_bytes) != seal_sha256 {
            return Err(ApprovalError::RecoveryRequired("Ready seal changed".into()));
        }
        let seal: journal::ReadySeal =
            canonical(&seal_bytes).map_err(|e| ApprovalError::RecoveryRequired(e.to_string()))?;
        let (_, proof) = package_git_operation::ready_proof_cold(
            lease,
            namespace.id(),
            scope,
            key,
            package_source,
        )
        .map_err(|e| ApprovalError::RecoveryRequired(format!("Ready plan proof: {e:#}")))?;
        let proof = proof.ok_or_else(|| {
            ApprovalError::RecoveryRequired("Ready plan proof disappeared".into())
        })?;
        if proof.run_id != run_id
            || proof.plan.digest().ok().as_deref() != Some(seal.semantic_plan_sha256.as_str())
            || seal.run_id != run_id
            || seal.namespace_id != namespace.id()
            || seal.scope_context != scope.context
            || seal.scope_target != scope.target
            || seal.scope_principal != scope.principal
            || seal.key_sha256 != sha(key.as_bytes())
        {
            return Err(ApprovalError::RecoveryRequired(
                "Ready plan/seal/C.1 identity differs".into(),
            ));
        }
        let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
            .map_err(|e| ApprovalError::RecoveryRequired(format!("C.1 inspect: {e}")))?
            .ok_or_else(|| ApprovalError::RecoveryRequired("C.1 row absent".into()))?;
        if row.retired
            || row.journal_admission != JournalAdmission::Ready
            || row.domain_run_id != run_id
            || row.plan_sha256 != seal.c1_plan_sha256
            || row.request_sha256 != seal.request_sha256
        {
            return Err(ApprovalError::RecoveryRequired(
                "C.1 Ready row differs".into(),
            ));
        }
        if !matches!(
            package_git_ready::inspect_ready_seal_cold(
                lease,
                namespace.id(),
                scope,
                key,
                package_source,
                local_bare_source,
                git_binary,
                limits,
            ),
            ReadyInspection::ReadyForApproval { run_id: same, seal_sha256: hash }
                if same == run_id && hash == seal_sha256
        ) {
            return Err(ApprovalError::RecoveryRequired(
                "Ready proof changed during approval inspection".into(),
            ));
        }
        Ok(ReadyBinding {
            plan: proof.plan,
            seal,
            seal_sha256,
        })
    }
}
