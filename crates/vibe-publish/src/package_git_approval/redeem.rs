//! One-use settings marker redemption after exact C.1 Ready profile proof.
use super::*;
use crate::package_git_provenance_populated::ApprovedGitdir;
use crate::package_git_ready::profile::ReadyProfile;
use crate::package_git_submodule::SubmoduleLimits;

impl ApprovalHome {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn redeem_at_profile(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        git_binary: &Path,
        profile: ReadyProfile<'_>,
        verified: &VerifiedApproval,
        now_override: Option<u64>,
        lose_reply: bool,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        let ready = self.ready_profile(
            lease,
            namespace,
            scope,
            key,
            package_source,
            git_binary,
            profile,
        )?;
        if ready.seal.run_id != verified.record.run_id
            || ready.seal_sha256 != verified.record.ready_seal_sha256
            || ready.seal.semantic_plan_sha256 != verified.record.semantic_plan_sha256
            || ready.plan.target != verified.record.repository
            || effects(&ready.plan)? != verified.record.effects
        {
            return Err(ApprovalError::Refused("selected Ready run changed".into()));
        }
        let settings_root = self.checked_settings_root()?;
        if verified.record.settings_root_token != settings_root {
            return Err(ApprovalError::RecoveryRequired(
                "settings root changed".into(),
            ));
        }
        let approval_dir = self
            .settings
            .subproject(&[HOME, APPROVAL_DIR])
            .map_err(|e| ApprovalError::RecoveryRequired(format!("approval home: {e:#}")))?;
        let snapshot = approval_dir
            .read_file_snapshot_bounded(&verified.record_name, CAP)
            .map_err(|e| ApprovalError::RecoveryRequired(format!("approval recheck: {e:#}")))?
            .ok_or_else(|| ApprovalError::RecoveryRequired("approval disappeared".into()))?;
        if sha(&snapshot.bytes) != verified.record_sha256
            || canonical::<ApprovalRecord>(&snapshot.bytes).ok().as_ref() != Some(&verified.record)
        {
            return Err(ApprovalError::RecoveryRequired("approval changed".into()));
        }
        let redeemed = self
            .settings
            .ensure_subproject(&[HOME, REDEEMED])
            .map_err(|e| ApprovalError::RecoveryRequired(format!("redemption home: {e:#}")))?;
        let marker_name = marker_name(&verified.record.approval_id);
        let dir = redeemed
            .root_dir()
            .map_err(|e| ApprovalError::RecoveryRequired(format!("redemption dir: {e:#}")))?;
        if redeemed
            .read_file_snapshot_bounded(&marker_name, CAP)
            .map_err(|e| ApprovalError::RecoveryRequired(format!("prior marker: {e:#}")))?
            .is_some()
        {
            return Err(ApprovalError::RecoveryRequired(
                "approval ID already redeemed".into(),
            ));
        }
        let (marker, bytes) = marker_candidate(verified, namespace, scope, &settings_root)?;
        // The production clock is read at the last pre-publication boundary,
        // after potentially long cold Git/object verification.
        let now = match now_override {
            Some(value) => value,
            None => host_now()?,
        };
        if now < verified.record.issued_at_unix || now >= verified.record.expires_at_unix {
            return Err(ApprovalError::Refused(
                "approval expired before redemption".into(),
            ));
        }
        if self.checked_settings_root()? != marker.settings_root_token {
            return Err(ApprovalError::RecoveryRequired(
                "settings root changed before redemption".into(),
            ));
        }
        redeemed
            .publish_new_in(&dir, &marker_name, &bytes)
            .map_err(|e| {
                ApprovalError::RecoveryRequired(format!("create-new marker: {:#}", e.into_report()))
            })?;
        if lose_reply {
            return Err(ApprovalError::RecoveryRequired(
                "redemption response lost after publication".into(),
            ));
        }
        let readback = redeemed
            .read_file_snapshot_bounded(&marker_name, CAP)
            .map_err(|e| ApprovalError::RecoveryRequired(format!("marker readback: {e:#}")))?
            .ok_or_else(|| ApprovalError::RecoveryRequired("marker disappeared".into()))?;
        if readback.bytes != bytes {
            return Err(ApprovalError::RecoveryRequired("marker changed".into()));
        }
        if self.checked_settings_root()? != marker.settings_root_token {
            return Err(ApprovalError::RecoveryRequired(
                "settings root changed after redemption".into(),
            ));
        }
        Ok(RedemptionReceipt {
            run_id: marker.run_id,
            marker_sha256: sha(&bytes),
            approval_record_sha256: verified.record_sha256.clone(),
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn redeem_populated(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        approved_gitdirs: Option<&[ApprovedGitdir<'_>]>,
        git_binary: &Path,
        limits: SubmoduleLimits,
        verified: &VerifiedApproval,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        self.redeem_at_profile(
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
            verified,
            None,
            false,
        )
    }
}
