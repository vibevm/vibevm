//! Registered adapter for one explicitly embedded fixed vibe-bin Deploy host.
//! Project/settings/home roots and profile are held by the route, not by wire.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde::Serialize;
use serde_json::{Value, json};
use std::io::Write;
use vibe_orchestrator::machine_operation::fixed_deploy::{
    DOMAIN_OWNER, FixedDeployRoute, FixedDeployRouteError, OPERATION_ID, PUBLICATION_CAP,
    REQUEST_CAP,
};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        fixed_deploy_action_error::FixedDeployActionError,
        fixed_deploy_action_reply::{FixedDeployActionReply, Reply},
        operation_envelope as e,
    },
};

/// One closed Deploy action over an independently supplied trusted host.
///
/// ```
/// use vibe_mcp::tools::{FixedDeployOperationMcpTool, McpTool};
/// assert_eq!(FixedDeployOperationMcpTool.descriptor().name, "fixed_deploy_operation");
/// ```
pub struct FixedDeployOperationMcpTool;

fn bounded<T: Serialize>(value: &T, cap: usize) -> Result<String, String> {
    struct Writer {
        bytes: Vec<u8>,
        cap: usize,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let size = self
                .bytes
                .len()
                .checked_add(bytes.len())
                .filter(|size| *size <= self.cap)
                .ok_or_else(|| std::io::Error::other("serialized request exceeds cap"))?;
            self.bytes.reserve(size - self.bytes.len());
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        cap,
    };
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| "serialized request exceeds cap".to_owned())?;
    String::from_utf8(writer.bytes).map_err(|error| error.to_string())
}

fn preflight(refusal: FixedDeployRouteError) -> Result<ToolOutput, ToolError> {
    let report = serde_json::to_value(&refusal.error).map_err(|error| {
        ToolError::Internal(format!(
            "fixed Deploy preflight serialization: {error}; original={:?}",
            refusal.error
        ))
    })?;
    Err(ToolError::StructuredPreflight {
        report,
        message: refusal.error.message,
    })
}

fn diagnostic(refusal: FixedDeployRouteError) -> Result<ToolOutput, ToolError> {
    Ok(ToolOutput::executed_failure_text(format!(
        "UNVALIDATED fixed Deploy owner publication; may_effect={}; error={:?}; observation={:?}",
        refusal.effects_may_have_begun, refusal.error, refusal.owner_observation,
    )))
}

fn published(envelope: e::OperationEnvelope) -> Result<ToolOutput, ToolError> {
    match envelope.body {
        e::Body::Result(row) => {
            let reply: FixedDeployActionReply =
                serde_json::from_str(row.payload_json.as_deref().ok_or_else(|| {
                    ToolError::Internal("validated Deploy result lacks payload".into())
                })?)
                .map_err(|error| {
                    ToolError::Internal(format!("validated Deploy reply reader: {error}"))
                })?;
            let kind = match &reply.reply {
                Reply::Completed(_) => "completed",
                Reply::Uncertain(_) => "uncertain",
                Reply::RecoveryRequired(_) => "recovery_required",
                Reply::Retired(_) => "retired",
                Reply::Absent(_) => "absent",
            };
            let report = serde_json::to_value(&reply).map_err(|error| {
                ToolError::Internal(format!(
                    "fixed Deploy reply serialization: {error}; original={reply:?}"
                ))
            })?;
            Ok(ToolOutput::executed(
                report,
                format!(
                    "Fixed Deploy owner observation: {kind}; inspect the typed reply for run and effect truth."
                ),
            ))
        }
        e::Body::Error(row) => {
            let error: FixedDeployActionError =
                serde_json::from_str(row.payload_json.as_deref().ok_or_else(|| {
                    ToolError::Internal("validated Deploy error lacks payload".into())
                })?)
                .map_err(|reason| {
                    ToolError::Internal(format!("validated Deploy error reader: {reason}"))
                })?;
            let report = serde_json::to_value(&error).map_err(|reason| {
                ToolError::Internal(format!(
                    "fixed Deploy error serialization: {reason}; original={error:?}"
                ))
            })?;
            let text = format!("Fixed Deploy action refused: {}", error.message);
            if row.effect_state == e::BodyErrorEffectState::NoEffects {
                Err(ToolError::StructuredPreflight {
                    report,
                    message: text,
                })
            } else {
                Ok(ToolOutput::executed_failure(report, text))
            }
        }
        _ => Err(ToolError::Internal(
            "fixed Deploy route published a non-result stage".into(),
        )),
    }
}

impl McpTool for FixedDeployOperationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "fixed_deploy_operation".into(),
            description: "Submit or inspect only an explicitly embedded, host-bound fixed vibe-bin Deploy route. The request cannot select project/settings/home roots, profile, target, generation, namespace or destination. Ordinary MCP startup has no grant; explicit fixed Deploy issue or reattach requires an absolute project root and authored profile.".into(),
            input_schema: json!({
                "type":"object", "properties":{
                    "schema":{"type":"integer","const":1},
                    "action":{"oneOf":[
                        {"type":"object","properties":{"kind":{"type":"string","const":"submit"},"key":{"type":"string"}},"required":["kind","key"],"additionalProperties":false},
                        {"type":"object","properties":{"kind":{"type":"string","const":"inspect"},"key":{"type":"string"}},"required":["kind","key"],"additionalProperties":false}
                    ]}
                },"required":["schema","action"],"additionalProperties":false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let grant = ctx.fixed_deploy_bridge().map(|route| route.as_ref());
        let payload = match bounded(args, REQUEST_CAP) {
            Ok(payload) => payload,
            Err(reason) => {
                return preflight(FixedDeployRouteError::invalid_request(&reason, grant));
            }
        };
        let input = json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
            "source_id":null,"evidence_id":null,"body":{"stage":"input",
                "format_id":FormatId::MachineFixedDeployActionRequest.id(),"payload_json":payload}});
        let input = match bounded(&input, PUBLICATION_CAP) {
            Ok(input) => input,
            Err(reason) => {
                return preflight(FixedDeployRouteError::invalid_request(&reason, grant));
            }
        };
        match FixedDeployRoute::exchange_local(grant, input.as_bytes(), &ctx.project_root) {
            Ok(envelope) => published(envelope),
            Err(refusal) => diagnostic(refusal),
        }
    }
}

#[cfg(test)]
mod tests;
