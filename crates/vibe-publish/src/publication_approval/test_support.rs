//! Test-only owner-settings approval fixture.

use super::*;
use crate::publication_authority::PublicationAuthorityScope;
use std::cell::Cell;

thread_local! {
    static LOSE_NEXT_REDEMPTION_REPLY: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn lose_next_redemption_reply() {
    LOSE_NEXT_REDEMPTION_REPLY.with(|value| value.set(true));
}

pub(super) fn take_reply_loss() -> bool {
    LOSE_NEXT_REDEMPTION_REPLY.with(|value| value.replace(false))
}

impl ApprovalHome {
    #[cfg(test)]
    pub(crate) fn open_for_test(path: &Path) -> Result<Self, ApprovalError> {
        Self::open_selected(path)
    }

    #[cfg(test)]
    pub(crate) fn seed_mode_a_for_test(
        &self,
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        subject: &PublicationSubject,
        plan: &PublicationEffectPlan,
        expected: &PublicationEffectPlanExpectations<'_>,
        approval_id: &str,
    ) -> Result<(), ApprovalError> {
        let digest = publication_effect_plan_digest(plan, subject, expected)
            .map_err(|error| ApprovalError::Refused(error.to_string()))?;
        let now = host_now()?;
        let record = ApprovalRecord {
            format: FORMAT.into(),
            schema: 1,
            context: scope.context.clone(),
            root_correlation_token: lease
                .publication_journal_project()
                .root_correlation_token()
                .map_err(|error| ApprovalError::Refused(error.to_string()))?,
            receipt: PublicationApprovalReceipt {
                approval_id: approval_id.into(),
                scope: PublicationAuthorityScope {
                    plan_sha256: digest.clone(),
                    subject_sha256: plan.subject_sha256.clone(),
                    host: plan.target.host.clone(),
                    owner: plan.target.owner.clone(),
                    repository: plan.target.repository.clone(),
                    principal: scope.principal.clone(),
                    remote_generation_ref: plan.remote_generation_ref.clone(),
                    smoke_policy_id: plan.smoke_policy_id.clone(),
                    smoke_policy_epoch: expected.subject.smoke_policy_epoch,
                    publish: expected.publish,
                    allow_replace_published: expected.allow_replace_published,
                },
                issued_at_unix: now.saturating_sub(1),
                expires_at_unix: now.saturating_add(3600),
            },
        };
        let bytes = serde_json::to_vec(&record)
            .map_err(|error| ApprovalError::Refused(error.to_string()))?;
        let approvals = self
            .settings
            .ensure_subproject(&[HOME])
            .map_err(|error| ApprovalError::Refused(error.to_string()))?;
        let dir = approvals
            .root_dir()
            .map_err(|error| ApprovalError::Refused(error.to_string()))?;
        approvals
            .publish_new_in(&dir, &plan_name(&digest)?, &bytes)
            .map_err(|error| ApprovalError::Refused(error.to_string()))?;
        Ok(())
    }
}
