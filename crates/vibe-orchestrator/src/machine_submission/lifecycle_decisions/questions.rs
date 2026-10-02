use super::*;

pub(super) const OPERATION: &str = super::super::lifecycle_adapter::FIXED_OPERATION;
pub(super) const DOMAIN: &str = "rust:vibe-lifecycle";
pub(super) fn choices(facts: &HostPolicyFacts) -> Vec<Choice> {
    [
        RequestedOptionsControl::None,
        RequestedOptionsControl::Detach,
        RequestedOptionsControl::CancelPreDispatch,
        RequestedOptionsControl::CancelDeferredBoundary,
    ]
    .into_iter()
    .filter(|c| facts.permits(c))
    .map(|c| Choice {
        id: control_id(&c).into(),
        label: match c {
            RequestedOptionsControl::None => "Continue without a control request",
            RequestedOptionsControl::Detach => "Detach this observer; execution continues",
            RequestedOptionsControl::CancelPreDispatch => "Stop before execution begins",
            RequestedOptionsControl::CancelDeferredBoundary => {
                "Request stop after the provider returns naturally; may wait indefinitely"
            }
        }
        .into(),
    })
    .collect()
}
pub(super) fn control_id(control: &RequestedOptionsControl) -> &'static str {
    match control {
        RequestedOptionsControl::None => "none",
        RequestedOptionsControl::Detach => "detach",
        RequestedOptionsControl::CancelPreDispatch => "cancel_pre_dispatch",
        RequestedOptionsControl::CancelDeferredBoundary => "cancel_deferred_boundary",
    }
}
pub(super) fn selected(id: &str) -> Result<RequestedOptionsControl, FixedDecisionError> {
    match id {
        "none" => Ok(RequestedOptionsControl::None),
        "detach" => Ok(RequestedOptionsControl::Detach),
        "cancel_pre_dispatch" => Ok(RequestedOptionsControl::CancelPreDispatch),
        "cancel_deferred_boundary" => Ok(RequestedOptionsControl::CancelDeferredBoundary),
        _ => Err(FixedDecisionError::InvalidData),
    }
}
pub(super) fn approval_body() -> Body {
    Body::ApprovalQuestion(Box::new(BodyApprovalQuestion {
        prompt: "Approve this captured Validate script and selected control policy?".into(),
        effect_summary: "A later lifecycle admission may run the captured script. This answer does not submit it.".into(),
    }))
}
pub(super) fn expected(q: &InputDecisionMessage) -> ExpectedBinding<'_> {
    ExpectedBinding {
        question_id: q.question_id.as_deref(),
        operation_id: Some(OPERATION),
        domain_owner: Some(DOMAIN),
        context_id: q.context_id.as_deref(),
        target_id: q.target_id.as_deref(),
        principal_id: q.principal_id.as_deref(),
        plan_id: q.plan_id.as_deref(),
        source_id: q.source_id.as_deref(),
        policy_generation: q.policy_generation.as_deref(),
        requested_options: &q.requested_options,
    }
}
/// Private normalized resulting identity; v4 bytes stay unchanged. Slot IDs
/// never enter semantic identity. Policy generation and actual base hash do.
pub(super) fn resulting_id(
    base: &CapturedPlan,
    options: &RequestedOptions,
    generation: &str,
    facts: &HostPolicyFacts,
) -> String {
    digest(
        serde_json::to_vec(&serde_json::json!({
            "format": "vibevm.lifecycle.fixed-decision-plan/1",
            "base_v4": base.plan_id, "request": base.request_id,
            "options": options, "policy_generation": generation, "host_policy": facts.identity(),
        }))
        .expect("fixed normalized values serialize"),
    )
}
