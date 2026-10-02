//! Actual registered MCP adapter for the trusted fixed lifecycle owner.
//! Opaque scoped observers delegate replay and controls to the incumbent owner.
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::Value;
pub(crate) mod bridge;
mod contract;
mod descriptor;
mod observers;
use LifecycleActionErrorCode as Code;
use contract::*;
use vibe_orchestrator::machine_submission::{DecisionOutcome, FixedDecisionError};
pub struct LifecycleOperationMcpTool;
fn failure(e: LifecycleActionError, admitted: bool) -> Result<ToolOutput, ToolError> {
    let text = format!("Lifecycle action refused: {}", e.message);
    match value(&e) {
        Ok(report) if admitted => Ok(ToolOutput::executed_failure(report, text)),
        Ok(report) => Err(ToolError::StructuredPreflight {
            report,
            message: text,
        }),
        Err(reason) => Ok(ToolOutput::executed_failure_text(format!(
            "Lifecycle publication unavailable: {reason}; {text}"
        ))),
    }
}
fn owner_code(e: &FixedDecisionError, fallback: Code) -> Code {
    match e {
        FixedDecisionError::Capacity => Code::Capacity,
        FixedDecisionError::ImplementationGap(_) => Code::ImplementationGap,
        _ => fallback,
    }
}
fn snapshot_reply(
    s: &vibe_orchestrator::machine_submission::FixedOwnerEvidenceSnapshot,
    key: &str,
    submitted: bool,
) -> Result<Reply, (Code, String, bool)> {
    let (row, payload_json) = view(s).map_err(|e| (Code::PublicationUnavailable, e, true))?;
    use vibe_orchestrator::machine_operation::lifecycle_view::Status;
    match &row.status {
        Status::RecoveryRequired(r) => {
            return Err((Code::InspectionUnavailable, r.reason.clone(), true));
        }
        Status::InspectionUnavailable(r) => {
            return Err((Code::InspectionUnavailable, r.reason.clone(), true));
        }
        _ => {}
    }
    Ok(if submitted {
        Reply::Submitted(Box::new(ReplySubmitted {
            key: key.into(),
            format_id: ReplySubmittedFormatId::MachineLifecycleSubmissionView,
            payload_json,
        }))
    } else {
        Reply::View(Box::new(ReplyView {
            key: key.into(),
            format_id: ReplyViewFormatId::MachineLifecycleSubmissionView,
            payload_json,
        }))
    })
}
impl McpTool for LifecycleOperationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"lifecycle_operation".into(),description:"Prepare, answer, submit, inspect, attach, replay, control or detach the explicitly granted fixed lifecycle owner. Questions and references are ephemeral; a submission may be Starting.".into(),input_schema:descriptor::input_schema()}
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let grant = ctx.fixed_lifecycle_bridge();
        let host = grant.map(|b| b.host.as_ref());
        let request: LifecycleActionRequest = match decode(args) {
            Ok(r) => r,
            Err(m) => return failure(error(Code::InvalidRequest, &m, None, None, host), false),
        };
        let (action, phase, key) = match &request.action {
            Action::Prepare(p) => ("prepare", Some(p.phase.as_str()), None),
            Action::Answer(_) => ("answer", None, None),
            Action::Submit(s) => ("submit", Some("validate"), Some(s.key.as_str())),
            Action::Inspect(i) => ("inspect", Some("validate"), Some(i.key.as_str())),
            Action::Attach(a) => ("attach", Some("validate"), Some(a.key.as_str())),
            Action::Replay(_) => ("replay", None, None),
            Action::Control(_) => ("control", None, None),
            Action::Detach(_) => ("detach", None, None),
        };
        let refuse =
            |code, m: &str, admitted| failure(error(code, m, Some(action), phase, host), admitted);
        if request.schema != 1 {
            return refuse(Code::InvalidRequest, "schema must equal 1", false);
        }
        let Some(bridge) = grant else {
            return refuse(
                Code::UnavailableHost,
                "no explicitly granted lifecycle host",
                false,
            );
        };
        if bridge.host.compare_context_root(&ctx.project_root).is_err() {
            return refuse(
                Code::ContextRootMismatch,
                "physical context root differs from retained host project",
                false,
            );
        }
        if key.is_some_and(|k| !contract::key(k)) {
            return refuse(
                Code::InvalidRequest,
                "key must be nonempty, trimmed, <=256 bytes and contain no controls",
                false,
            );
        }
        let result: Result<Reply, (Code, String, bool)> = match &request.action {
            Action::Prepare(p) => {
                if p.phase != "validate"
                    || p.requested_options.force
                    || p.requested_options.trace
                    || p.requested_options.deploy_profile.is_some()
                {
                    return refuse(
                        Code::ImplementationGap,
                        "fixed owner implements Validate without force, trace or deploy_profile",
                        false,
                    );
                }
                bridge
                    .host
                    .prepare_requested_options(options(&p.requested_options))
                    .map_err(|e| (owner_code(&e, Code::DecisionRefused), e.to_string(), true))
                    .and_then(|q| question(&q).map_err(|e| (Code::PublicationUnavailable, e, true)))
            }
            Action::Answer(a) => {
                if a.message_json.len() > 65_536 {
                    return refuse(Code::InvalidRequest, "decision frame exceeds 64KiB", false);
                }
                let answer:vibe_orchestrator::machine_operation::input_decision::InputDecisionMessage=match serde_json::from_str(&a.message_json) {Ok(a)=>a,Err(_)=>return refuse(Code::InvalidRequest,"generated decision shape refused",false)};
                match bridge.host.answer_typed(&answer) {
                    Err(e) => Err((owner_code(&e, Code::DecisionRefused), e.to_string(), true)),
                    Ok(DecisionOutcome::Question(q)) => {
                        question(&q).map_err(|e| (Code::PublicationUnavailable, e, true))
                    }
                    Ok(DecisionOutcome::Validated(e)) => {
                        let Some(id) = answer.question_id.clone() else {
                            return refuse(
                                Code::PublicationUnavailable,
                                "owner validated without original question id",
                                true,
                            );
                        };
                        let plan_id = e.identity().to_owned();
                        bridge
                            .retain(id.clone(), e)
                            .map(|()| {
                                Reply::Validated(Box::new(ReplyValidated {
                                    accepted_ref: id,
                                    plan_id,
                                }))
                            })
                            .map_err(|e| {
                                (owner_code(&e, Code::DecisionRefused), e.to_string(), true)
                            })
                    }
                }
            }
            Action::Submit(s) => {
                let Some(e) = bridge.accepted(&s.accepted_ref) else {
                    return refuse(
                        Code::UnknownReference,
                        "accepted reference is not retained by this owner bridge",
                        false,
                    );
                };
                bridge
                    .submit(&e, &s.key)
                    .map_err(|e| (owner_code(&e, Code::SubmissionRefused), e.to_string(), true))
                    .and_then(|j| snapshot_reply(&j.evidence(), &s.key, true))
            }
            Action::Attach(a) => bridge.attach_observer(&a.key, &ctx.project_root),
            Action::Replay(r) => bridge.stream_observer(&r.observer_ref, &r.message_json, false),
            Action::Control(c) => bridge.stream_observer(&c.observer_ref, &c.message_json, true),
            Action::Detach(d) => bridge.detach_observer(&d.observer_ref),
            Action::Inspect(i) => bridge
                .inspect(&i.key)
                .map_err(|e| {
                    (
                        owner_code(&e, Code::InspectionUnavailable),
                        e.to_string(),
                        true,
                    )
                })
                .and_then(|snapshot| match snapshot {
                    None => Ok(Reply::Absent(Box::new(ReplyAbsent { key: i.key.clone() }))),
                    Some(s) => snapshot_reply(&s, &i.key, false),
                }),
        };
        match result {
            Ok(reply) => match value(&LifecycleActionReply { schema: 1, reply }) {
                Ok(v) => Ok(ToolOutput::executed(
                    v,
                    format!(
                        "Lifecycle {action} returned owner data; use lifecycle_operation inspect for a fresh submitted view."
                    ),
                )),
                Err(m) => {
                    let mut e = error(Code::PublicationUnavailable, &m, Some(action), phase, host);
                    if let Some(k) = key {
                        if let Ok(Some(s)) = bridge.inspect(k) {
                            enrich(&mut e, &s);
                        }
                    }
                    failure(e, true)
                }
            },
            Err((code, m, admitted)) => {
                let mut e = error(code, &m, Some(action), phase, host);
                if admitted {
                    if let Some(k) = key {
                        if let Ok(Some(s)) = bridge.inspect(k) {
                            enrich(&mut e, &s);
                        }
                    }
                }
                failure(e, admitted)
            }
        }
    }
}
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
