//! Strict MCP adapter over the read-only optimistic lifecycle-task reader.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use serde::Deserialize;
use serde_json::{Value, json};
use specmark::{cell, spec};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

/// MCP `lifecycle_tasks`: return the exact durable hosted handoff, or idle /
/// absent, without creating state or taking the lifecycle mutation lease.
///
/// ```
/// use vibe_mcp::tools::{LifecycleTasksMcpTool, McpTool};
///
/// let descriptor = LifecycleTasksMcpTool.descriptor();
/// assert_eq!(descriptor.name, "lifecycle_tasks");
/// assert_eq!(descriptor.input_schema["additionalProperties"], false);
/// ```
#[cell(seam = "McpTool", variant = "lifecycle_tasks")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub struct LifecycleTasksMcpTool;

/// Strict empty-object grammar. The MCP dispatcher represents omitted
/// `arguments` as JSON null; this tool normalises only that transport spelling
/// to `{}` before deserialising. Every other scalar/array/member refuses.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleTasksArgs {}

impl McpTool for LifecycleTasksMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "lifecycle_tasks".into(),
            description:
                "Read the exact durable lifecycle handoff for this selected workspace node. Returns generated structured status `absent`, `idle`, or `parked`; parked rows carry each exact bounded UTF-8 task document in lifecycle order. Read-only: creates no `.vibe` state or lock, enumerates no outbox, calls no provider, and accepts no arguments. Call again after completing a task to observe the new state."
                    .into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        parse_args(args)?;
        let report =
            vibe_lifecycle::pending_hosted_tasks(&ctx.project_root).map_err(ToolError::from)?;
        let value = serde_json::to_value(report).map_err(|error| {
            ToolError::Internal(format!("serialising lifecycle tasks: {error}"))
        })?;
        Ok(ToolOutput::ok(value))
    }
}

fn parse_args(args: &Value) -> Result<(), ToolError> {
    let normalized = match args {
        Value::Null => Value::Object(serde_json::Map::new()),
        Value::Object(_) => args.clone(),
        _ => {
            return Err(ToolError::InvalidArguments(
                "`lifecycle_tasks` takes exactly an empty object".into(),
            ));
        }
    };
    serde_json::from_value::<LifecycleTasksArgs>(normalized)
        .map(|_| ())
        .map_err(|error| {
            ToolError::InvalidArguments(format!(
                "`lifecycle_tasks` takes exactly an empty object: {error}"
            ))
        })
}
