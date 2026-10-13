//! Generated read request/result delegated to the same M16 domain service.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-READ-SERVICE");
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::Value;
use vibe_wire::behaviour::change_query as wire;
pub struct ChangeQueryMcpTool;
impl McpTool for ChangeQueryMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"change_query".into(),description:"Read check/show/coverage for one immutable local change through captured shared grammar, source scope and observation owners. No argument root, writes, adoption, lifecycle or terminal verdict.".into(),input_schema:wire::request_json_schema()}
    }
    fn run(&self, args: &Value, context: &ServerContext) -> Result<ToolOutput, ToolError> {
        let response =
            vibe_orchestrator::machine_operation::MachineOperationService::read_change_json(
                &context.project_root,
                None,
                args,
            );
        wire::validate_response(&response)
            .map_err(|e| ToolError::Internal(format!("change query output invariant: {e}")))?;
        let value =
            serde_json::to_value(&response).map_err(|e| ToolError::Internal(e.to_string()))?;
        wire::read_response(&value).map_err(ToolError::Internal)?;
        if let wire::Response::Refused(error) = response {
            Err(ToolError::StructuredPreflight {
                message: error.message,
                report: value,
            })
        } else {
            Ok(ToolOutput::ok(value))
        }
    }
}
#[cfg(test)]
mod m_02_d_2_cases {
    use super::*;
    #[test]
    fn generated_request_refuses_extra_root_before_preparation() {
        let root = tempfile::tempdir().unwrap();
        let ctx = ServerContext::with_store_root(root.path(), root.path().join("empty-cache"));
        for (request, code) in [
            (
                serde_json::json!({"schema":1,"mode":"show","id":"chg-0123456789abcdef0123456789abcdef","path":"/elsewhere"}),
                "invalid-request",
            ),
            (
                serde_json::json!({"schema":9,"mode":"show","id":"chg-0123456789abcdef0123456789abcdef"}),
                "unknown-epoch",
            ),
        ] {
            let error = ChangeQueryMcpTool.run(&request, &ctx).unwrap_err();
            let ToolError::StructuredPreflight { report, .. } = error else {
                panic!("wrong channel")
            };
            assert_eq!(report["code"], code);
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
