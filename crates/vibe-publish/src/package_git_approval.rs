//! Private owner-settings Mode A approval for one protected package-Git run.
//! A local redemption is evidence, never permission to publish public Git.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;

use crate::package_git_operation::{self, journal};
use crate::package_git_plan::{
    PackageGitPlan, RepositoryIdentity, RepositoryObservation, RepositoryTarget,
};
use crate::package_git_ready::{self, ReadyInspection, ReadyLimits};
use crate::publication_approval::{HOME, REDEEMED, marker_name};

const APPROVAL_DIR: &str = "package-git-e2";
const FORMAT: &str = "vibevm.package-git.mode-a-approval/e1";
const MARKER_FORMAT: &str = "vibevm.package-git.approval-redemption/e1";
const CAP: usize = 16 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApprovalError {
    #[error("package-Git Mode A approval refused: {0}")]
    Refused(String),
    #[error("package-Git approval requires recovery: {0}")]
    RecoveryRequired(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectSet {
    create_repository: bool,
    atomic_push_main_and_version_tag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalRecord {
    format: String,
    approval_id: String,
    approver: String,
    issued_at_unix: u64,
    expires_at_unix: u64,
    semantic_plan_sha256: String,
    c1_plan_sha256: String,
    request_sha256: String,
    ready_seal_sha256: String,
    namespace_id: String,
    run_id: String,
    key_sha256: String,
    context: String,
    target: String,
    principal: String,
    workspace_root_token: String,
    settings_root_token: String,
    repository: RepositoryTarget,
    effects: EffectSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Redemption {
    format: String,
    approval_id_sha256: String,
    approval_record_sha256: String,
    semantic_plan_sha256: String,
    c1_plan_sha256: String,
    request_sha256: String,
    ready_seal_sha256: String,
    namespace_id: String,
    run_id: String,
    key_sha256: String,
    context: String,
    target: String,
    principal: String,
    workspace_root_token: String,
    settings_root_token: String,
    repository: RepositoryTarget,
    effects: EffectSet,
}

/// A path selected by the host's settings policy, never by a publication
/// request. The root correlation detects ordinary replacement on all hosts;
/// same-user hostile writers remain outside this cooperative filesystem law.
pub(crate) struct ApprovalHome {
    settings: Project,
}

/// A checked owner-settings record. This is not an effect grant.
pub(crate) struct VerifiedApproval {
    record: ApprovalRecord,
    record_sha256: String,
    record_name: String,
}

impl VerifiedApproval {
    pub(crate) fn authority_id(&self) -> &str {
        &self.record.approval_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedemptionReceipt {
    pub run_id: String,
    pub marker_sha256: String,
    pub approval_record_sha256: String,
}

/// An owner journal may retain these exact historical claims for read-only
/// attach. Callers cannot create a marker through this structure.
pub(crate) struct ColdExpectation<'a> {
    pub approval_id: &'a str,
    pub approval_record_sha256: &'a str,
    pub marker_sha256: &'a str,
}

#[derive(Debug)]
struct ReadyBinding {
    plan: PackageGitPlan,
    seal: journal::ReadySeal,
    seal_sha256: String,
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn valid_human(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_sha(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|c| c.is_ascii_digit() || matches!(c, b'a'..=b'f'))
    })
}

fn canonical<T: Serialize + for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ApprovalError> {
    let row: T = serde_json::from_slice(bytes)
        .map_err(|e| ApprovalError::Refused(format!("canonical JSON decode: {e}")))?;
    if serde_json::to_vec(&row).ok().as_deref() != Some(bytes) {
        return Err(ApprovalError::Refused("noncanonical approval JSON".into()));
    }
    Ok(row)
}

fn host_now() -> Result<u64, ApprovalError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApprovalError::Refused("host clock predates Unix epoch".into()))?
        .as_secs())
}

fn approval_name(digest: &str) -> Result<String, ApprovalError> {
    if !valid_sha(digest) {
        return Err(ApprovalError::Refused(
            "invalid semantic plan digest".into(),
        ));
    }
    Ok(format!("{}.json", &digest[7..]))
}

fn effects(plan: &PackageGitPlan) -> Result<EffectSet, ApprovalError> {
    plan.validate()
        .map_err(|e| ApprovalError::Refused(format!("selected package-Git plan: {e}")))?;
    match (&plan.target, &plan.before_repo) {
        (RepositoryTarget::HostedCreate { .. }, RepositoryObservation::Absent) => Ok(EffectSet {
            create_repository: true,
            atomic_push_main_and_version_tag: true,
        }),
        (
            RepositoryTarget::HostedCreate { repository, .. },
            RepositoryObservation::Present(RepositoryIdentity::Hosted {
                repository: observed,
                id,
            }),
        ) if repository == observed && *id > 0 => Ok(EffectSet {
            create_repository: false,
            atomic_push_main_and_version_tag: true,
        }),
        (RepositoryTarget::DirectGitExisting { .. }, RepositoryObservation::Present(_)) => {
            Ok(EffectSet {
                create_repository: false,
                atomic_push_main_and_version_tag: true,
            })
        }
        _ => Err(ApprovalError::Refused(
            "unsupported repository/effect posture for package-Git approval".into(),
        )),
    }
}

impl ApprovalHome {
    pub(crate) fn open_from_settings() -> Result<Self, ApprovalError> {
        let selected = vibe_core::settings::settings_dir()
            .ok_or_else(|| ApprovalError::Refused("settings directory unavailable".into()))?;
        let absolute = if selected.is_absolute() {
            selected
        } else {
            std::env::current_dir()
                .map_err(|e| ApprovalError::Refused(format!("current directory: {e}")))?
                .join(selected)
        };
        Self::open_selected(&absolute)
    }

    fn open_selected(path: &Path) -> Result<Self, ApprovalError> {
        let settings = Project::open(path)
            .map_err(|e| ApprovalError::Refused(format!("pin settings root: {e:#}")))?;
        Ok(Self { settings })
    }

    fn checked_settings_root(&self) -> Result<String, ApprovalError> {
        let held = self
            .settings
            .root_correlation_token()
            .map_err(|e| ApprovalError::RecoveryRequired(format!("settings root: {e:#}")))?;
        let reopened = Project::open(self.settings.root_path())
            .map_err(|e| ApprovalError::RecoveryRequired(format!("settings reopen: {e:#}")))?;
        if reopened.root_correlation_token().ok().as_deref() != Some(held.as_str()) {
            return Err(ApprovalError::RecoveryRequired(
                "settings pathname no longer denotes held root".into(),
            ));
        }
        Ok(held)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load_mode_a(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
    ) -> Result<VerifiedApproval, ApprovalError> {
        self.load_at(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            host_now()?,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn load_at(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
        now: u64,
    ) -> Result<VerifiedApproval, ApprovalError> {
        let ready = self.ready(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
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
    pub(crate) fn redeem(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
        verified: &VerifiedApproval,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        self.redeem_at(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            verified,
            None,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn redeem_at(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
        verified: &VerifiedApproval,
        now_override: Option<u64>,
        lose_reply: bool,
    ) -> Result<RedemptionReceipt, ApprovalError> {
        let ready = self.ready(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
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
}

mod cold;
mod link;
pub(crate) use link::LinkInspection;
use link::marker_candidate;
mod ready;
#[cfg(test)]
pub(crate) mod tests;
