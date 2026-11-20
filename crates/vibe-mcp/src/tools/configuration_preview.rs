//! Ordinary preview shares the library operation with the CLI.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SURFACE-PARITY");
use super::{McpTool, ServerContext, ToolDescriptor, ToolError, ToolOutput};
use serde_json::{Value, json};
use specmark::cell;
use vibe_package_source::configuration_preview::{MAX_INPUT_BYTES, configuration_preview};
use vibe_wire::behaviour::ordinary_configuration_preview::parse_request;

/// Read-only inline answers; the host, not the caller, supplies project scope.
#[cell(seam = "McpTool", variant = "configuration_preview")]
pub struct ConfigurationPreviewMcpTool;
impl McpTool for ConfigurationPreviewMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "configuration_preview".into(),
            description: "Describe or evaluate one compulsory exact ordinary flow from its admitted project-local source without installing, solving or authorizing effects.".into(),
            input_schema: json!({
                "type":"object", "additionalProperties":false, "required":["package"],
                "properties": {
                    "package":{"type":"string"},
                    "input":{"type":"object", "additionalProperties":false,
                        "required":["selections","parameters"], "properties":{
                            "selections":{"type":"object","additionalProperties":{"type":"string"}},
                            "parameters":{"type":"object","additionalProperties":{"type":"boolean"}}
                        }
                    }
                }
            }),
        }
    }
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        if serde_json::to_vec(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?
            .len()
            > MAX_INPUT_BYTES
        {
            return Err(ToolError::InvalidArguments(
                "preview request exceeds bounded input limit".into(),
            ));
        }
        let request = parse_request(args.clone())
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let result =
            configuration_preview(&ctx.project_root, &request).map_err(|error| {
                match error.downcast::<vibe_core::manifest::PackageConfigurationError>() {
                    Ok(error) => ToolError::ConfigurationPreview(Box::new(error)),
                    Err(error) => ToolError::PreExecution(format!(
                        "ordinary configuration preview refused: {error:#}"
                    )),
                }
            })?;
        Ok(ToolOutput::ok(
            serde_json::to_value(result).map_err(|error| ToolError::Internal(error.to_string()))?,
        ))
    }
}
