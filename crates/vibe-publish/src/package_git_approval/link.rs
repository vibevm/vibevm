//! Run-local link between an exact Ready seal and one settings redemption.
//! Pending is durable before the settings marker; only a returned receipt and
//! independent cold marker check may produce Landed. Neither permits Git.

use super::*;
use crate::package_git_operation::journal::{
    APPROVAL_LINK_FORMAT, APPROVAL_LINK_PENDING_FORMAT, ApprovalLinkLanded, ApprovalLinkPending,
};

const PENDING: &str = "approval-link-pending";
const LANDED: &str = "approval-link";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkInspection {
    NotLinked {
        run_id: String,
    },
    Landed {
        run_id: String,
        pending_sha256: String,
        marker_sha256: String,
    },
    RecoveryRequired {
        run_id: Option<String>,
        reason: String,
    },
}

fn recovery(run: Option<&str>, reason: impl Into<String>) -> LinkInspection {
    LinkInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

/// One canonical marker candidate shared by the pre-effect link and actual
/// create-new redemption. This does not read or write the settings marker.
pub(super) fn marker_candidate(
    verified: &VerifiedApproval,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    settings_root: &str,
) -> Result<(Redemption, Vec<u8>), ApprovalError> {
    let marker = Redemption {
        format: MARKER_FORMAT.into(),
        approval_id_sha256: sha(verified.record.approval_id.as_bytes()),
        approval_record_sha256: verified.record_sha256.clone(),
        semantic_plan_sha256: verified.record.semantic_plan_sha256.clone(),
        c1_plan_sha256: verified.record.c1_plan_sha256.clone(),
        request_sha256: verified.record.request_sha256.clone(),
        ready_seal_sha256: verified.record.ready_seal_sha256.clone(),
        namespace_id: namespace.id().into(),
        run_id: verified.record.run_id.clone(),
        key_sha256: verified.record.key_sha256.clone(),
        context: scope.context.clone(),
        target: scope.target.clone(),
        principal: scope.principal.clone(),
        workspace_root_token: verified.record.workspace_root_token.clone(),
        settings_root_token: settings_root.into(),
        repository: verified.record.repository.clone(),
        effects: verified.record.effects,
    };
    let bytes = serde_json::to_vec(&marker)
        .map_err(|e| ApprovalError::Refused(format!("marker encode: {e}")))?;
    if bytes.len() > CAP {
        return Err(ApprovalError::Refused("marker exceeds cap".into()));
    }
    Ok((marker, bytes))
}

fn exact_pending(
    ready: &ReadyBinding,
    verified: &VerifiedApproval,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    settings_root: &str,
    marker_sha256: &str,
) -> ApprovalLinkPending {
    ApprovalLinkPending {
        format: APPROVAL_LINK_PENDING_FORMAT.into(),
        run_id: ready.seal.run_id.clone(),
        namespace_id: namespace.id().into(),
        scope_context: scope.context.clone(),
        scope_target: scope.target.clone(),
        scope_principal: scope.principal.clone(),
        key_sha256: ready.seal.key_sha256.clone(),
        lease_root_token: ready.seal.lease_root_token.clone(),
        settings_root_token: settings_root.into(),
        semantic_plan_sha256: ready.seal.semantic_plan_sha256.clone(),
        c1_plan_sha256: ready.seal.c1_plan_sha256.clone(),
        request_sha256: ready.seal.request_sha256.clone(),
        ready_seal_sha256: ready.seal_sha256.clone(),
        approval_id_sha256: sha(verified.record.approval_id.as_bytes()),
        approval_record_sha256: verified.record_sha256.clone(),
        expected_marker_sha256: marker_sha256.into(),
        repository: ready.plan.target.clone(),
        create_repository: verified.record.effects.create_repository,
        atomic_push_main_and_version_tag: verified.record.effects.atomic_push_main_and_version_tag,
    }
}

impl ApprovalHome {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn link_mode_a(
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
    ) -> LinkInspection {
        self.link_inner(
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
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn link_inner(
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
        stop_after_pending: bool,
    ) -> LinkInspection {
        let ready = match self.ready(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
        ) {
            Ok(value) => value,
            Err(error) => return recovery(None, error.to_string()),
        };
        let run = ready.seal.run_id.as_str();
        match journal::read_bytes(lease, run, PENDING) {
            Ok(Some(_)) => {
                return recovery(
                    Some(run),
                    "approval-link Pending already exists; no re-redemption",
                );
            }
            Ok(None) => {}
            Err(error) => return recovery(Some(run), error.to_string()),
        }
        if !matches!(journal::read_bytes(lease, run, LANDED), Ok(None)) {
            return recovery(
                Some(run),
                "approval-link Landed without Pending or unreadable",
            );
        }
        let now = match now_override {
            Some(value) => value,
            None => match host_now() {
                Ok(value) => value,
                Err(error) => return recovery(Some(run), error.to_string()),
            },
        };
        let selected = match self.load_at(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            now,
        ) {
            Ok(value)
                if value.record == verified.record
                    && value.record_sha256 == verified.record_sha256 =>
            {
                value
            }
            Ok(_) => return recovery(Some(run), "selected owner approval changed"),
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let settings_root = match self.checked_settings_root() {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let (_, marker_bytes) = match marker_candidate(&selected, namespace, scope, &settings_root)
        {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let expected = exact_pending(
            &ready,
            &selected,
            namespace,
            scope,
            &settings_root,
            &sha(&marker_bytes),
        );
        let pending_bytes = match journal::publish(lease, run, PENDING, &expected) {
            Ok(value) => value,
            Err(error) => {
                return recovery(
                    Some(run),
                    format!("approval-link Pending publication uncertain: {error:#}"),
                );
            }
        };
        if !matches!(journal::read_bytes(lease, run, PENDING), Ok(Some(ref bytes)) if bytes == &pending_bytes)
        {
            return recovery(Some(run), "approval-link Pending changed before redemption");
        }
        if stop_after_pending {
            return recovery(Some(run), "injected stop after approval-link Pending");
        }
        let redeemed = match self.redeem_at(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            &selected,
            now_override,
            lose_reply,
        ) {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        if redeemed.run_id != run
            || redeemed.marker_sha256 != expected.expected_marker_sha256
            || redeemed.approval_record_sha256 != expected.approval_record_sha256
        {
            return recovery(Some(run), "returned redemption differs from Pending");
        }
        let cold = ColdExpectation {
            approval_id: &selected.record.approval_id,
            approval_record_sha256: &expected.approval_record_sha256,
            marker_sha256: &expected.expected_marker_sha256,
        };
        if self
            .inspect_redemption_cold(
                lease,
                namespace,
                scope,
                key,
                package_source,
                local_bare_source,
                git_binary,
                limits,
                &cold,
            )
            .is_err()
        {
            return recovery(
                Some(run),
                "redemption cold proof failed after returned receipt",
            );
        }
        if !matches!(journal::read_bytes(lease, run, PENDING), Ok(Some(ref bytes)) if bytes == &pending_bytes)
        {
            return recovery(Some(run), "approval-link Pending changed before Landed");
        }
        let landed = ApprovalLinkLanded {
            format: APPROVAL_LINK_FORMAT.into(),
            run_id: run.into(),
            pending_sha256: sha(&pending_bytes),
            marker_sha256: redeemed.marker_sha256,
            approval_record_sha256: redeemed.approval_record_sha256,
        };
        if let Err(error) = journal::publish(lease, run, LANDED, &landed) {
            return recovery(
                Some(run),
                format!("approval-link Landed uncertain: {error:#}"),
            );
        }
        self.inspect_link_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            &selected.record.approval_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn inspect_link_cold(
        &self,
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        package_source: Option<&Project>,
        local_bare_source: Option<&Project>,
        git_binary: &Path,
        limits: ReadyLimits,
        approval_id: &str,
    ) -> LinkInspection {
        let ready = match self.ready(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
        ) {
            Ok(value) => value,
            Err(error) => return recovery(None, error.to_string()),
        };
        let run = ready.seal.run_id.as_str();
        let pending_bytes = match journal::read_bytes(lease, run, PENDING) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return match journal::read_bytes(lease, run, LANDED) {
                    Ok(None) => LinkInspection::NotLinked { run_id: run.into() },
                    _ => recovery(Some(run), "approval-link Landed lacks Pending"),
                };
            }
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let pending: ApprovalLinkPending = match canonical(&pending_bytes) {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let settings_root = match self.checked_settings_root() {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let expected_effects = match effects(&ready.plan) {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        if !valid_human(approval_id)
            || pending.format != APPROVAL_LINK_PENDING_FORMAT
            || pending.run_id != run
            || pending.namespace_id != namespace.id()
            || pending.scope_context != scope.context
            || pending.scope_target != scope.target
            || pending.scope_principal != scope.principal
            || pending.key_sha256 != sha(key.as_bytes())
            || pending.lease_root_token != ready.seal.lease_root_token
            || pending.settings_root_token != settings_root
            || pending.semantic_plan_sha256 != ready.seal.semantic_plan_sha256
            || pending.c1_plan_sha256 != ready.seal.c1_plan_sha256
            || pending.request_sha256 != ready.seal.request_sha256
            || pending.ready_seal_sha256 != ready.seal_sha256
            || pending.approval_id_sha256 != sha(approval_id.as_bytes())
            || !valid_sha(&pending.approval_record_sha256)
            || !valid_sha(&pending.expected_marker_sha256)
            || pending.repository != ready.plan.target
            || pending.create_repository != expected_effects.create_repository
            || pending.atomic_push_main_and_version_tag
                != expected_effects.atomic_push_main_and_version_tag
        {
            return recovery(Some(run), "approval-link Pending differs from Ready/scope");
        }
        let landed_bytes = match journal::read_bytes(lease, run, LANDED) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return recovery(
                    Some(run),
                    "approval-link Pending has no Landed; do not re-redeem",
                );
            }
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        let landed: ApprovalLinkLanded = match canonical(&landed_bytes) {
            Ok(value) => value,
            Err(error) => return recovery(Some(run), error.to_string()),
        };
        if landed.format != APPROVAL_LINK_FORMAT
            || landed.run_id != run
            || landed.pending_sha256 != sha(&pending_bytes)
            || landed.marker_sha256 != pending.expected_marker_sha256
            || landed.approval_record_sha256 != pending.approval_record_sha256
        {
            return recovery(Some(run), "approval-link Landed differs from Pending");
        }
        let cold = ColdExpectation {
            approval_id,
            approval_record_sha256: &pending.approval_record_sha256,
            marker_sha256: &pending.expected_marker_sha256,
        };
        if let Err(error) = self.inspect_redemption_cold(
            lease,
            namespace,
            scope,
            key,
            package_source,
            local_bare_source,
            git_binary,
            limits,
            &cold,
        ) {
            return recovery(Some(run), format!("historical marker: {error}"));
        }
        if !matches!(journal::read_bytes(lease, run, PENDING), Ok(Some(ref b)) if b == &pending_bytes)
            || !matches!(journal::read_bytes(lease, run, LANDED), Ok(Some(ref b)) if b == &landed_bytes)
        {
            return recovery(Some(run), "approval-link records changed during cold proof");
        }
        LinkInspection::Landed {
            run_id: run.into(),
            pending_sha256: sha(&pending_bytes),
            marker_sha256: pending.expected_marker_sha256,
        }
    }
}
