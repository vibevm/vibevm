//! Thin MCP projection of the selected-source owning service.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#GENERATED-SOURCE-QUERY-CONTRACT");
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::Value;
use vibe_workspace::source_query::SourceQueryService;

pub struct ReadSpecMcpTool;
impl McpTool for ReadSpecMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"read_spec".into(),description:"Read an exact declared Vibe node or outline from this host's captured selected world. Epoch-one generated request; md/xml projection, children control and bound continuation. No argument root, newest-slot fallback, fetch or lifecycle effects.".into(),input_schema:vibe_wire::behaviour::source_query::request_json_schema()}
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        run_query(args, ctx)
    }
}

pub(super) fn run_query(args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
    let response = SourceQueryService::execute_json(&ctx.project_root, args);
    let is_error = response.is_error();
    let value = response.into_value();
    if is_error {
        let error = vibe_wire::behaviour::source_query::read_error(&value)
            .map_err(|reason| ToolError::Internal(format!("source refusal invariant: {reason}")))?;
        Err(ToolError::StructuredPreflight {
            message: error.message,
            report: value,
        })
    } else {
        vibe_wire::behaviour::source_query::read_result(&value)
            .map_err(|reason| ToolError::Internal(format!("source result invariant: {reason}")))?;
        Ok(ToolOutput::ok(value))
    }
}

#[cfg(test)]
mod m_01_d_2_cases {
    use super::*;
    use serde_json::json;

    #[test]
    fn actual_tool_returns_validated_self_solution() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
        std::fs::write(root.path().join("vibe.toml"),"[package]\ngroup='org.demo'\nname='self'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        std::fs::write(root.path().join("vibevm/vibespecs/DOC.md"),"# D {#root}\n\n## Solution {#solution}\n\n@fact:ONE Selected. @status:impl/done\n\n## Other {#other}\n\nUnselected.\n").unwrap();
        let context = ServerContext::with_store_root(root.path(), root.path().join("unused"));
        let request = json!({"schema":1,"address":"spec://org.demo/self/DOC#root.solution","mode":"read","format":"md","children":true});
        let output = ReadSpecMcpTool.run(&request, &context).unwrap();
        let result = vibe_wire::behaviour::source_query::read_result(output.structured()).unwrap();
        assert!(result.content.contains("Selected."));
        assert!(!result.content.contains("Unselected."));
        assert_eq!(result.heading_chain, ["D", "Solution"]);
        assert!(!root.path().join("unused").exists());
    }
}

#[cfg(test)]
#[path = "read_spec/m_01_e_2_cases.rs"]
mod m_01_e_2_cases;
