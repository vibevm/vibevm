//! Read-only installed-tool projection through the shared workspace library.
use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};
use serde_json::{Value, json};
use specmark::{cell, spec};

/// The registry of what this project can invoke. Read-only.
///
/// The second surface over `vibe_workspace::tools::collect_tools`, beside
/// `vibe tools`: both call the same library and render its result.
///
/// ```
/// use vibe_mcp::tools::{ListToolsMcpTool, McpTool};
/// assert_eq!(ListToolsMcpTool.descriptor().name, "list_tools");
/// ```
#[cell(seam = "McpTool", variant = "list_tools")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ListToolsMcpTool;

impl McpTool for ListToolsMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "list_tools".to_string(),
            description:
                "List every tool the installed packages declare in this project: each `[[binary]]` (a PATH-facing executable) and each `[[mcp_server]]` (an agent-facing server), with the declaring package and the author's own description. Use this to find out what the project lets you RUN — the boot lane already says which language disciplines it follows, but not what they brought. Read-only; a project with nothing installed returns an empty list rather than an error."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, _args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let tools = vibe_workspace::tools::collect_tools(&ctx.project_root)
            .map_err(|e| ToolError::Internal(format!("collecting tools: {e}")))?;
        let value = serde_json::to_value(&tools)
            .map_err(|e| ToolError::Internal(format!("serialising tools: {e}")))?;
        Ok(ToolOutput::ok(value))
    }
}
