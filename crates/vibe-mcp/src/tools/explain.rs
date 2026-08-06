//! The `explain` MCP tool — the traceability-explain cell (PROP-014 §2.6),
//! extracted from `tools.rs` to keep that registry file under the 600-line
//! budget (the same reason `query.rs` and `select.rs` are split out beside
//! it). The contract "a new tool is a new cell added at `default_tools`, not
//! an edit to the dispatcher" is unchanged — only the impl's address moved.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};

use super::McpTool;
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Answer a traceability question over THIS project's tree — the MCP face
/// of `vibe explain` (PROP-014 §2.6): build the specmap fresh in memory
/// and return what implements, verifies, documents, or deviates from one
/// spec unit or code symbol. Shares the [`vibe_trace::explain`] core with
/// the CLI; `target` selects the unit/symbol, `json` selects the raw
/// subgraph over the text view.
///
/// ```
/// use vibe_mcp::tools::{McpTool, ExplainMcpTool};
/// assert_eq!(ExplainMcpTool.descriptor().name, "explain");
/// ```
#[cell(seam = "McpTool", variant = "explain")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ExplainMcpTool;

impl McpTool for ExplainMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "explain".to_string(),
            description:
                "Answer a traceability question over this project's tree: build the specmap fresh in memory and return what implements, verifies, documents, or deviates from one spec unit (`spec://…#anchor`) or code symbol. This is the canonical \"which test verifies this spec rule?\" lookup — give an address, get back the code-side edges with `file:line`. The index is built fresh on every call, never read from a stale committed artefact. `json=true` returns the raw one-hop subgraph; the default is the deterministic text view. This is the MCP face of the CLI `vibe explain`."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "A `spec://…#anchor` URI or a code symbol to explain."
                    },
                    "json": {
                        "type": "boolean",
                        "description": "Return the raw one-hop subgraph instead of the text view. Default: false."
                    }
                },
                "required": ["target"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
        let target = args
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("`target` must be a string".into()))?;
        let json = args.get("json").and_then(|v| v.as_bool()).unwrap_or(false);
        let out = vibe_trace::explain(&ctx.project_root, target, json)
            .map_err(|e| ToolError::NotFound(e.to_string()))?;
        Ok(match out {
            vibe_trace::Explain::Text(text) => Value::String(text),
            vibe_trace::Explain::Json(value) => value,
        })
    }
}
