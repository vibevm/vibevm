//! Local Mode A approval provenance and one-use redemption. No token, network
//! client, release operation or request-selected path is available here.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::process::is_valid_run_id;
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;
use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_authority::{
    PublicationApprovalReceipt, PublicationAuthorityDecision, PublicationAuthorityMode,
    PublicationAuthoritySelection, PublicationRecordProvenance, evaluate_publication_authority,
};
use crate::publication_effect_plan::{
    PublicationEffectPlanExpectations, publication_effect_plan_digest,
};

const HOME: &str = "publication-approvals";
const REDEEMED: &str = "redeemed";
const FORMAT: &str = "vibevm.publication.mode-a-approval/e1";
const REDEMPTION_FORMAT: &str = "vibevm.publication.approval-redemption/e1";
const CAP: usize = 16 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApprovalError {
    #[error("Mode A approval refused: {0}")]
    Refused(String),
    #[error("Mode A approval redemption requires recovery: {0}")]
    RecoveryRequired(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalRecord {
    format: String,
    schema: u32,
    context: String,
    root_correlation_token: String,
    receipt: PublicationApprovalReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Redemption {
    format: String,
    approval_id_sha256: String,
    approval_record_sha256: String,
    plan_sha256: String,
    c1_plan_sha256: String,
    request_sha256: String,
    namespace_id: String,
    run_id: String,
    context: String,
    target: String,
    principal: String,
    root_correlation_token: String,
    settings_root_token: String,
}

/// Host-pinned settings root. A request cannot supply a different directory.
/// This is a same-user cooperative boundary on Linux, macOS and Windows;
/// it does not authenticate against another process already running with the
/// owner's filesystem and environment privileges.
pub(crate) struct ApprovalHome {
    settings: Project,
}

/// Sealed result of a canonical owner-settings read and exact Mode A check.
/// It is not a remote effect grant; only redemption ties it to C.1 Pending.
pub(crate) struct VerifiedApproval {
    approval_id: String,
    record_name: String,
    record_sha256: String,
    plan_sha256: String,
    c1_plan_sha256: String,
    context: String,
    target: String,
    principal: String,
    root_correlation_token: String,
    settings_root_token: String,
    issued_at_unix: u64,
    expires_at_unix: u64,
}

impl VerifiedApproval {
    pub(crate) fn authority_id(&self) -> &str {
        &self.approval_id
    }
    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
    pub(crate) fn record_sha256(&self) -> &str {
        &self.record_sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedemptionReceipt {
    pub run_id: String,
    pub plan_sha256: String,
    pub marker_sha256: String,
}

/// Exact owner-journal values used only to inspect a prior redemption. The
/// caller cannot use these claims to create a marker or perform an effect.
pub(crate) struct ColdRedemptionExpectation<'a> {
    pub authority_id: &'a str,
    pub run_id: &'a str,
    pub marker_sha256: &'a str,
    pub plan_sha256: &'a str,
    pub c1_plan_sha256: &'a str,
    pub request_sha256: &'a str,
    pub approval_record_sha256: &'a str,
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn host_now() -> Result<u64, ApprovalError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApprovalError::Refused("host clock predates Unix epoch".into()))?
        .as_secs())
}

fn plan_name(digest: &str) -> Result<String, ApprovalError> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(ApprovalError::Refused("invalid plan digest".into()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(ApprovalError::Refused("invalid plan digest".into()));
    }
    Ok(format!("{hex}.json"))
}

fn marker_name(approval_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"vibevm.publication.approval-id/e1\0");
    hasher.update(approval_id.as_bytes());
    format!("{:x}.json", hasher.finalize())
}

fn canonical<T: Serialize + for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ApprovalError> {
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| ApprovalError::Refused(format!("approval JSON: {error}")))?;
    let emitted = serde_json::to_vec(&value)
        .map_err(|error| ApprovalError::Refused(format!("approval encoding: {error}")))?;
    if emitted != bytes {
        return Err(ApprovalError::Refused(
            "approval JSON is not canonical".into(),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
pub(crate) use test_support::lose_next_redemption_reply;

impl ApprovalHome {
    /// Resolve the one canonical per-user settings directory from the trusted
    /// host process environment, then pin it. `$VIBE_SETTINGS` is honored;
    /// the approval record never supplies a path or a provenance flag.
    pub(crate) fn open_from_settings() -> Result<Self, ApprovalError> {
        let selected = vibe_core::settings::settings_dir().ok_or_else(|| {
            ApprovalError::Refused("user settings directory is unavailable".into())
        })?;
        let absolute = if selected.is_absolute() {
            selected
        } else {
            std::env::current_dir()
                .map_err(|error| ApprovalError::Refused(format!("current directory: {error}")))?
                .join(selected)
        };
        Self::open_selected(&absolute)
    }

    fn open_selected(path: &Path) -> Result<Self, ApprovalError> {
        let settings = Project::open(path)
            .map_err(|error| ApprovalError::Refused(format!("pinning settings root: {error:#}")))?;
        Ok(Self { settings })
    }

    pub(crate) fn load_mode_a(
        &self,
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        subject: &PublicationSubject,
        plan: &PublicationEffectPlan,
        expected: &PublicationEffectPlanExpectations<'_>,
    ) -> Result<VerifiedApproval, ApprovalError> {
        let now = host_now()?;
        self.load_at(lease, scope, subject, plan, expected, now)
    }

    fn load_at(
        &self,
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        subject: &PublicationSubject,
        plan: &PublicationEffectPlan,
        expected: &PublicationEffectPlanExpectations<'_>,
        now_unix: u64,
    ) -> Result<VerifiedApproval, ApprovalError> {
        let plan_sha256 = publication_effect_plan_digest(plan, subject, expected)
            .map_err(|error| ApprovalError::Refused(format!("selected plan: {error}")))?;
        let record_name = plan_name(&plan_sha256)?;
        let approvals = self
            .settings
            .subproject(&[HOME])
            .map_err(|error| ApprovalError::Refused(format!("approval home: {error:#}")))?;
        let snapshot = approvals
            .read_file_snapshot_bounded(&record_name, CAP)
            .map_err(|error| ApprovalError::Refused(format!("approval read: {error:#}")))?
            .ok_or_else(|| ApprovalError::Refused("exact Mode A approval is absent".into()))?;
        let record: ApprovalRecord = canonical(&snapshot.bytes)?;
        let root_token = lease
            .publication_journal_project()
            .root_correlation_token()
            .map_err(|error| ApprovalError::Refused(format!("held workspace root: {error:#}")))?;
        if record.format != FORMAT
            || record.schema != 1
            || record.context != scope.context
            || record.root_correlation_token != root_token
            || scope.target != "github.com/vibevm/vibevm"
        {
            return Err(ApprovalError::Refused(
                "approval context, root or target differs".into(),
            ));
        }
        let selection = PublicationAuthoritySelection {
            principal: &scope.principal,
            now_unix,
            record_provenance: PublicationRecordProvenance::OwnerControlled,
            mode: PublicationAuthorityMode::ModeA,
            approval: Some(&record.receipt),
            policy_epoch: 0,
            policy_generation: "",
        };
        let PublicationAuthorityDecision::Approved {
            mode: "A",
            authority_ref,
            plan_sha256: approved_plan,
        } = evaluate_publication_authority(subject, plan, expected, &selection)
        else {
            return Err(ApprovalError::Refused(
                "Mode A receipt did not approve the exact plan/scope/time".into(),
            ));
        };
        if approved_plan != plan_sha256 {
            return Err(ApprovalError::Refused(
                "approval plan digest changed".into(),
            ));
        }
        let settings_root_token = self
            .settings
            .root_correlation_token()
            .map_err(|error| ApprovalError::Refused(format!("held settings root: {error:#}")))?;
        let c1_plan_sha256 = sha(&serde_json::to_vec(plan)
            .map_err(|error| ApprovalError::Refused(format!("plan encoding: {error}")))?);
        Ok(VerifiedApproval {
            approval_id: authority_ref,
            record_name,
            record_sha256: sha(&snapshot.bytes),
            plan_sha256,
            c1_plan_sha256,
            context: scope.context.clone(),
            target: scope.target.clone(),
            principal: scope.principal.clone(),
            root_correlation_token: root_token,
            settings_root_token,
            issued_at_unix: record.receipt.issued_at_unix,
            expires_at_unix: record.receipt.expires_at_unix,
        })
    }

    /// Consume one verified approval ID against one exact Pending C.1 run.
    /// A survived or uncertain marker is never overwritten or reissued.
    pub(crate) fn redeem(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        run_id: &str,
        canonical_request: &[u8],
        verified: &VerifiedApproval,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        #[cfg(test)]
        let lose_reply = test_support::take_reply_loss();
        #[cfg(not(test))]
        let lose_reply = false;
        self.redeem_inner(
            lease,
            namespace,
            scope,
            key,
            run_id,
            canonical_request,
            verified,
            host_now()?,
            lose_reply,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn redeem_inner(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        run_id: &str,
        canonical_request: &[u8],
        verified: &VerifiedApproval,
        now_unix: u64,
        inject_post_publish_loss: bool,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        if now_unix < verified.issued_at_unix || now_unix >= verified.expires_at_unix {
            return Err(ApprovalError::Refused(
                "Mode A approval is not currently valid".into(),
            ));
        }
        if !is_valid_run_id(run_id) {
            return Err(ApprovalError::Refused("invalid reserved run ID".into()));
        }
        let current_root = lease
            .publication_journal_project()
            .root_correlation_token()
            .map_err(|error| ApprovalError::Refused(format!("held workspace root: {error:#}")))?;
        if verified.context != scope.context
            || verified.target != scope.target
            || verified.principal != scope.principal
            || verified.root_correlation_token != current_root
            || verified.settings_root_token
                != self.settings.root_correlation_token().map_err(|error| {
                    ApprovalError::Refused(format!("held settings root: {error:#}"))
                })?
        {
            return Err(ApprovalError::Refused(
                "approval scope or held root changed".into(),
            ));
        }
        let reopened = Project::open(self.settings.root_path()).map_err(|error| {
            ApprovalError::RecoveryRequired(format!("settings root reobserve: {error:#}"))
        })?;
        if reopened.root_correlation_token().ok().as_deref() != Some(&verified.settings_root_token)
        {
            return Err(ApprovalError::RecoveryRequired(
                "settings pathname no longer denotes held root".into(),
            ));
        }
        let approvals = self.settings.subproject(&[HOME]).map_err(|error| {
            ApprovalError::RecoveryRequired(format!("approval home: {error:#}"))
        })?;
        let current = approvals
            .read_file_snapshot_bounded(&verified.record_name, CAP)
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("approval recheck: {error:#}"))
            })?
            .ok_or_else(|| {
                ApprovalError::RecoveryRequired("approval disappeared before redemption".into())
            })?;
        if sha(&current.bytes) != verified.record_sha256 {
            return Err(ApprovalError::RecoveryRequired(
                "approval changed before redemption".into(),
            ));
        }
        let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
            .map_err(|error| ApprovalError::Refused(format!("C.1 inspection: {error}")))?
            .ok_or_else(|| ApprovalError::Refused("C.1 key is absent".into()))?;
        if row.domain_run_id != run_id
            || row.retired
            || row.journal_admission != JournalAdmission::Pending
            || row.plan_sha256 != verified.c1_plan_sha256
            || row.request_sha256 != sha(canonical_request)
        {
            return Err(ApprovalError::Refused(
                "C.1 Pending run or exact request/plan differs".into(),
            ));
        }
        let marker_name = marker_name(&verified.approval_id);
        let redeemed = self
            .settings
            .ensure_subproject(&[HOME, REDEEMED])
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("redemption home: {error:#}"))
            })?;
        let dir = redeemed.root_dir().map_err(|error| {
            ApprovalError::RecoveryRequired(format!("redemption handle: {error:#}"))
        })?;
        if redeemed
            .read_file_snapshot_bounded(&marker_name, CAP)
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("prior redemption: {error:#}"))
            })?
            .is_some()
        {
            return Err(ApprovalError::RecoveryRequired(
                "approval ID was already redeemed".into(),
            ));
        }
        let marker = Redemption {
            format: REDEMPTION_FORMAT.into(),
            approval_id_sha256: sha(verified.approval_id.as_bytes()),
            approval_record_sha256: verified.record_sha256.clone(),
            plan_sha256: verified.plan_sha256.clone(),
            c1_plan_sha256: verified.c1_plan_sha256.clone(),
            request_sha256: row.request_sha256,
            namespace_id: namespace.id().into(),
            run_id: run_id.into(),
            context: scope.context.clone(),
            target: scope.target.clone(),
            principal: scope.principal.clone(),
            root_correlation_token: current_root,
            settings_root_token: verified.settings_root_token.clone(),
        };
        let bytes = serde_json::to_vec(&marker)
            .map_err(|error| ApprovalError::Refused(format!("redemption encoding: {error}")))?;
        if bytes.len() > CAP {
            return Err(ApprovalError::Refused(
                "redemption marker exceeds cap".into(),
            ));
        }
        redeemed
            .publish_new_in(&dir, &marker_name, &bytes)
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!(
                    "create-new redemption: {:#}",
                    error.into_report()
                ))
            })?;
        if inject_post_publish_loss {
            return Err(ApprovalError::RecoveryRequired(
                "injected response loss after redemption publication".into(),
            ));
        }
        let observed = redeemed
            .read_file_snapshot_bounded(&marker_name, CAP)
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("redemption readback: {error:#}"))
            })?
            .ok_or_else(|| ApprovalError::RecoveryRequired("redemption disappeared".into()))?;
        if observed.bytes != bytes {
            return Err(ApprovalError::RecoveryRequired(
                "redemption changed on readback".into(),
            ));
        }
        Ok(RedemptionReceipt {
            run_id: run_id.into(),
            plan_sha256: verified.plan_sha256.clone(),
            marker_sha256: sha(&bytes),
        })
    }

    /// Read a survived redemption after process/lease exit. This never
    /// re-evaluates today's approval expiry and never creates or redeems a
    /// marker. The C.1 row may be Pending or Ready, but must be the same run.
    pub(crate) fn inspect_redemption_cold(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        expected: &ColdRedemptionExpectation<'_>,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        if !is_valid_run_id(expected.run_id)
            || expected.authority_id.is_empty()
            || expected.authority_id.chars().any(char::is_control)
            || plan_name(expected.plan_sha256).is_err()
            || plan_name(expected.c1_plan_sha256).is_err()
            || plan_name(expected.request_sha256).is_err()
            || plan_name(expected.marker_sha256).is_err()
            || plan_name(expected.approval_record_sha256).is_err()
        {
            return Err(ApprovalError::Refused(
                "invalid cold redemption identity".into(),
            ));
        }
        let held_settings = self.settings.root_correlation_token().map_err(|error| {
            ApprovalError::RecoveryRequired(format!("held settings root: {error:#}"))
        })?;
        let reopened = Project::open(self.settings.root_path()).map_err(|error| {
            ApprovalError::RecoveryRequired(format!("settings root reobserve: {error:#}"))
        })?;
        if reopened.root_correlation_token().ok().as_deref() != Some(held_settings.as_str()) {
            return Err(ApprovalError::RecoveryRequired(
                "settings pathname no longer denotes held root".into(),
            ));
        }
        let workspace_root = lease
            .publication_journal_project()
            .root_correlation_token()
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("held workspace root: {error:#}"))
            })?;
        let row = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
            .map_err(|error| ApprovalError::Refused(format!("C.1 inspection: {error}")))?
            .ok_or_else(|| ApprovalError::Refused("C.1 key is absent".into()))?;
        if row.retired
            || row.domain_run_id != expected.run_id
            || row.plan_sha256 != expected.c1_plan_sha256
            || row.request_sha256 != expected.request_sha256
        {
            return Err(ApprovalError::Refused(
                "cold C.1 run/request/plan differs".into(),
            ));
        }
        let redeemed = self
            .settings
            .subproject(&[HOME, REDEEMED])
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("redemption home: {error:#}"))
            })?;
        let marker_name = marker_name(expected.authority_id);
        let snapshot = redeemed
            .read_file_snapshot_bounded(&marker_name, CAP)
            .map_err(|error| {
                ApprovalError::RecoveryRequired(format!("redemption read: {error:#}"))
            })?
            .ok_or_else(|| ApprovalError::RecoveryRequired("redemption marker is absent".into()))?;
        if sha(&snapshot.bytes) != expected.marker_sha256 {
            return Err(ApprovalError::RecoveryRequired(
                "redemption marker digest changed".into(),
            ));
        }
        let marker: Redemption = canonical(&snapshot.bytes)
            .map_err(|error| ApprovalError::RecoveryRequired(error.to_string()))?;
        if marker.format != REDEMPTION_FORMAT
            || marker.approval_id_sha256 != sha(expected.authority_id.as_bytes())
            || marker.approval_record_sha256 != expected.approval_record_sha256
            || marker.plan_sha256 != expected.plan_sha256
            || marker.c1_plan_sha256 != expected.c1_plan_sha256
            || marker.request_sha256 != expected.request_sha256
            || marker.namespace_id != namespace.id()
            || marker.run_id != expected.run_id
            || marker.context != scope.context
            || marker.target != scope.target
            || marker.principal != scope.principal
            || marker.root_correlation_token != workspace_root
            || marker.settings_root_token != held_settings
        {
            return Err(ApprovalError::RecoveryRequired(
                "redemption marker differs from exact C.1 or held roots".into(),
            ));
        }
        Ok(RedemptionReceipt {
            run_id: marker.run_id,
            plan_sha256: marker.plan_sha256,
            marker_sha256: expected.marker_sha256.into(),
        })
    }
}

#[cfg(test)]
#[path = "publication_approval/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "publication_approval/tests_cold.rs"]
mod tests_cold;
