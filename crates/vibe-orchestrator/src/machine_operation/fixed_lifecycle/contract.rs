//! Size gates and generated data publication. These routines never grant authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");
use serde::{Serialize, de::DeserializeOwned};
use std::io::Write;
pub use vibe_wire::generated::machine::e1::{
    lifecycle_action_error::*, lifecycle_action_reply::*, lifecycle_action_request::*,
};
pub const REQUEST_CAP: usize = 128 * 1024;
pub const PUBLICATION_CAP: usize = 256 * 1024;
pub fn size<T: Serialize>(value: &T, cap: usize) -> Result<usize, String> {
    struct Counter {
        count: usize,
        cap: usize,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.count = self
                .count
                .checked_add(bytes.len())
                .filter(|n| *n <= self.cap)
                .ok_or_else(|| std::io::Error::other("frame cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { count: 0, cap };
    serde_json::to_writer(&mut counter, value)
        .map_err(|_| "serialized frame exceeds cap".to_owned())?;
    Ok(counter.count)
}
pub fn encode<T: Serialize>(v: &T, cap: usize) -> Result<String, String> {
    let n = size(v, cap)?;
    let mut out = Vec::with_capacity(n);
    serde_json::to_writer(&mut out, v).map_err(|e| e.to_string())?;
    String::from_utf8(out).map_err(|e| e.to_string())
}
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() > REQUEST_CAP {
        return Err("serialized frame exceeds cap".into());
    }
    serde_json::from_slice(bytes).map_err(|_| "generated request shape refused".into())
}
pub fn key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 256 && key.trim() == key && !key.chars().any(char::is_control)
}
pub fn options(o: &RequestedOptions) -> crate::machine_operation::input_decision::RequestedOptions {
    use crate::machine_operation::input_decision::RequestedOptionsControl as C;
    crate::machine_operation::input_decision::RequestedOptions {
        force: o.force,
        trace: o.trace,
        deploy_profile: o.deploy_profile.clone(),
        control: match o.control {
            RequestedOptionsControl::None => C::None,
            RequestedOptionsControl::Detach => C::Detach,
            RequestedOptionsControl::CancelPreDispatch => C::CancelPreDispatch,
            RequestedOptionsControl::CancelDeferredBoundary => C::CancelDeferredBoundary,
        },
    }
}
pub fn question(
    q: &crate::machine_operation::input_decision::InputDecisionMessage,
) -> Result<Reply, String> {
    use crate::machine_operation::input_decision::{self, ExpectedBinding};
    let binding = ExpectedBinding {
        question_id: q.question_id.as_deref(),
        operation_id: q.operation_id.as_deref(),
        domain_owner: q.domain_owner.as_deref(),
        context_id: q.context_id.as_deref(),
        target_id: q.target_id.as_deref(),
        principal_id: q.principal_id.as_deref(),
        plan_id: q.plan_id.as_deref(),
        source_id: q.source_id.as_deref(),
        policy_generation: q.policy_generation.as_deref(),
        requested_options: &q.requested_options,
    };
    let bytes = input_decision::serialize(q, binding).map_err(|e| e.to_string())?;
    input_decision::parse(&bytes, binding).map_err(|e| e.to_string())?;
    Ok(Reply::Question(Box::new(ReplyQuestion {
        format_id: ReplyQuestionFormatId::MachineInputDecisionMessage,
        payload_json: String::from_utf8(bytes).map_err(|e| e.to_string())?,
    })))
}
pub fn view(
    s: &crate::machine_submission::FixedOwnerEvidenceSnapshot,
) -> Result<
    (
        crate::machine_operation::lifecycle_view::LifecycleSubmissionView,
        String,
    ),
    String,
> {
    use crate::machine_operation::lifecycle_view as v;
    let row = v::project_snapshot(s).map_err(|e| e.to_string())?;
    let bytes = v::serialize(&row, s).map_err(|e| e.to_string())?;
    v::parse(&bytes, s).map_err(|e| e.to_string())?;
    Ok((row, String::from_utf8(bytes).map_err(|e| e.to_string())?))
}
pub fn error(
    code: LifecycleActionErrorCode,
    message: &str,
    action: Option<&str>,
    phase: Option<&str>,
    host: Option<&crate::machine_submission::FixedLifecycleDecisionHost>,
) -> LifecycleActionError {
    let mut end = message.len().min(4096);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    LifecycleActionError {
        schema: 1,
        code,
        message: message[..end].into(),
        message_truncated: end < message.len(),
        requested_action: action.map(str::to_owned),
        requested_phase: phase.map(str::to_owned),
        requested_scope: host.map(|h| {
            let s = h.readonly_scope();
            Scope {
                context: s.context.clone(),
                target: s.target.clone(),
                principal: s.principal.clone(),
            }
        }),
        scope_unavailable_reason: host
            .is_none()
            .then(|| "no explicitly granted host scope".into()),
        known_run_id: None,
        view_json: None,
        effect_observation: LifecycleActionErrorEffectObservation::Unavailable,
    }
}
pub fn enrich(
    e: &mut LifecycleActionError,
    s: &crate::machine_submission::FixedOwnerEvidenceSnapshot,
) {
    e.known_run_id = s.bound_run_id().map(str::to_owned);
    e.requested_phase = Some(s.requested_phase().as_str().into());
    if let Ok((row, json)) = view(s) {
        use crate::machine_operation::lifecycle_view::{
            Status, StatusResultAvailableEffectState as E,
        };
        if let Status::ResultAvailable(r) = row.status {
            e.effect_observation = match r.effect_state {
                E::None => LifecycleActionErrorEffectObservation::None,
                E::Landed => LifecycleActionErrorEffectObservation::Landed,
                E::Uncertain => LifecycleActionErrorEffectObservation::Uncertain,
            };
        }
        e.view_json = Some(json);
    }
}
