//! Actual registered MCP adapter for the trusted fixed lifecycle owner.
//! Opaque scoped observers delegate replay and controls to the incumbent owner.
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::Value;
mod contract;
mod descriptor;
use contract::*;
use vibe_orchestrator::machine_operation::exact_local_install::ExactLocalInstallRoute;
use vibe_orchestrator::machine_operation::fixed_lifecycle::{
    FixedLifecycleRoute, FixedLifecycleRouteError,
};
#[cfg(test)]
use vibe_wire::generated::machine::e1::lifecycle_action_error::LifecycleActionErrorCode as Code;
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
impl McpTool for LifecycleOperationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"lifecycle_operation".into(),description:"Prepare, answer, submit or inspect an explicitly granted lifecycle owner. Select exact-local Install with operation_id; omitted operation_id retains fixed Validate. Questions and references are ephemeral.".into(),input_schema:descriptor::input_schema()}
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let selector = match args.get("operation_id") {
            None => None,
            Some(Value::String(id)) => Some(id.as_str()),
            Some(_) => {
                return Err(ToolError::InvalidArguments(
                    "operation_id must be a string".into(),
                ));
            }
        };
        let exact = match selector {
            None => false,
            Some(id)
                if id == vibe_orchestrator::machine_operation::fixed_lifecycle::OPERATION_ID =>
            {
                false
            }
            Some(id)
                if id
                    == vibe_orchestrator::machine_operation::exact_local_install::OPERATION_ID =>
            {
                true
            }
            _ => {
                return Err(ToolError::InvalidArguments(
                    "unknown lifecycle operation_id".into(),
                ));
            }
        };
        let mut request = args.clone();
        if let Some(map) = request.as_object_mut() {
            map.remove("operation_id");
        }
        let grant = ctx.fixed_lifecycle_bridge().map(|route| route.as_ref());
        let bytes = match encode(&request, REQUEST_CAP) {
            Ok(bytes) => bytes,
            Err(reason) => {
                if exact {
                    return Err(ToolError::InvalidArguments(reason));
                }
                let refusal = FixedLifecycleRouteError::invalid_request(&reason, grant);
                return failure(refusal.error, refusal.effects_may_have_begun);
            }
        };
        if exact {
            let route = ctx.exact_local_install_bridge().map(|route| route.as_ref());
            return match ExactLocalInstallRoute::dispatch(
                route,
                bytes.as_bytes(),
                &ctx.project_root,
            ) {
                Ok(reply) => match value(&reply) {
                    Ok(report) => Ok(ToolOutput::executed(
                        report,
                        "Exact-local Install returned owner data; inspect this key for a fresh result.",
                    )),
                    Err(reason) => Ok(ToolOutput::executed_failure_text(format!(
                        "Exact-local Install publication unavailable after owner dispatch: {reason}"
                    ))),
                },
                Err(refusal) => failure(refusal.error, refusal.effects_may_have_begun),
            };
        }
        match FixedLifecycleRoute::dispatch(grant, bytes.as_bytes(), &ctx.project_root) {
            Ok(reply) => match value(&reply) {
                Ok(v) => Ok(ToolOutput::executed(
                    v,
                    format!(
                        "Lifecycle {} returned owner data; use lifecycle_operation inspect for a fresh submitted view.",
                        args["action"]["kind"].as_str().unwrap_or_default()
                    ),
                )),
                Err(reason) => {
                    let refusal = FixedLifecycleRouteError::publication_unavailable(
                        &reason,
                        grant,
                        bytes.as_bytes(),
                    );
                    failure(refusal.error, refusal.effects_may_have_begun)
                }
            },
            Err(refusal) => failure(refusal.error, refusal.effects_may_have_begun),
        }
    }
}
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
