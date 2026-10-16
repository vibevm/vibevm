//! One typed client operation for API, MCP and framed CLI.
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
use vibe_orchestrator::machine_operation::client_deploy::{
    ClientDeployRoute, DOMAIN_OWNER, OPERATION_ID,
};
use vibe_wire::generated::machine::e1::operation_envelope as e;
pub struct ClientDeployOperationMcpTool;
impl McpTool for ClientDeployOperationMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"client_deploy_operation".into(),description:"Plan, apply, verify, update, recover, remove, inspect or cooperatively cancel a trusted client deployment. Wire evidence grants no authority.".into(),input_schema:json!({"type":"object","additionalProperties":false,"required":["schema","action","plan_id","key"],"properties":{"schema":{"const":1},"action":{"enum":["plan","apply","verify","update","recover","remove","inspect","cancel"]},"plan_id":{"type":"string"},"key":{"type":"string"},"run_id":{"type":"string"}}})}
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let payload =
            serde_json::to_string(args).map_err(|e| ToolError::Internal(e.to_string()))?;
        let input = json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,"source_id":null,"evidence_id":null,"body":{"stage":"input","format_id":"client-deploy-request","payload_json":payload}});
        let envelope = ClientDeployRoute::exchange_local(
            ctx.client_deploy_route().map(|r| r.as_ref()),
            input.to_string().as_bytes(),
            &ctx.project_root,
        )
        .map_err(ToolError::Internal)?;
        match envelope.body {
            e::Body::Result(row) => {
                let result: Value = serde_json::from_str(
                    row.payload_json
                        .as_deref()
                        .ok_or_else(|| ToolError::Internal("payload".into()))?,
                )
                .map_err(|e| ToolError::Internal(e.to_string()))?;
                if row.effect_state == e::BodyResultEffectState::NoEffects
                    && result["status"] != "planned"
                    && result["status"] != "completed"
                {
                    return Err(ToolError::StructuredPreflight {
                        report: result,
                        message: "Client deployment requires preparation or authority".into(),
                    });
                }
                if result["status"] == "completed" || result["status"] == "planned" {
                    Ok(ToolOutput::executed(
                        result,
                        "Client deployment owner result",
                    ))
                } else {
                    Ok(ToolOutput::executed_failure(
                        result,
                        "Client deployment partial result or refusal",
                    ))
                }
            }
            e::Body::Error(row) => {
                let report = serde_json::from_str(row.payload_json.as_deref().unwrap_or("{}"))
                    .map_err(|e| ToolError::Internal(e.to_string()))?;
                if row.effect_state != e::BodyErrorEffectState::NoEffects {
                    return Ok(ToolOutput::executed_failure(
                        report,
                        "Client deployment effect or publication uncertainty",
                    ));
                }
                Err(ToolError::StructuredPreflight {
                    report,
                    message: "Client deployment unavailable".into(),
                })
            }
            _ => Err(ToolError::Internal("unexpected owner envelope".into())),
        }
    }
}
#[cfg(test)]
mod m_08_d_2_cases;
