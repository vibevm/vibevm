//! Explicit v2 read-only integer/profile adapter; no MCP writer.
use super::{McpTool, ServerContext, ToolDescriptor, ToolError, ToolOutput};
use serde_json::{Value, json};
use specmark::cell;
#[cell(seam = "McpTool", variant = "configuration_preview_v2")]
pub struct IntegerPreviewMcpTool;
impl McpTool for IntegerPreviewMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor{name:"configuration_preview_v2".into(),description:"Read-only version-two bounded ordinary public integer/profile description and evaluation.".into(),input_schema:json!({"type":"object","additionalProperties":false,"required":["protocol","package"],"properties":{"protocol":{"const":"vibe-ordinary-integer-preview/2"},"package":{"type":"string"},"input":{"type":"object","additionalProperties":false,"required":["selections","parameters"],"properties":{"profile":{"type":"string"},"selections":{"type":"object","additionalProperties":{"type":"string"}},"parameters":{"type":"object","additionalProperties":{"type":"integer"}}}}}})}
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        if serde_json::to_vec(args)
            .map_err(|_| ToolError::InvalidArguments("integer request invalid".into()))?
            .len()
            > 64 * 1024
        {
            return Err(ToolError::InvalidArguments(
                "integer request too large".into(),
            ));
        }
        let request = serde_json::from_value(args.clone()).map_err(|_| {
            ToolError::InvalidArguments("integer preview v2 invalid shape; values omitted".into())
        })?;
        let result = vibe_package_source::configuration_preview::integer::preview(
            &ctx.project_root,
            &request,
        )
        .map_err(|_| ToolError::PreExecution("integer preview v2 refused".into()))?;
        Ok(ToolOutput::ok(serde_json::to_value(result).map_err(
            |_| ToolError::Internal("integer result invalid".into()),
        )?))
    }
}
