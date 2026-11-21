//! The `agentic_explain` MCP tool — the inline transport of the agentic
//! "explain this project" operation (PROP-018 §2.8, §2.10), split out of
//! `tools.rs` beside `explain`, `query` and `select` so the registry file
//! stays inside its line budget. Registration is unchanged: the cell is
//! still added at `default_tools`.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Compose the "explain this project" instruction and return it inline —
/// the MCP-transport face of `vibe agentic explain` (PROP-018 §2.8, §2.10).
/// Shares the [`crate::agentic::explain_intent`] core with the CLI relay,
/// but where the CLI one-shot parks the intent in `.vibe/agentic/`, the MCP
/// path returns it synchronously and writes no mailbox file.
///
/// ```
/// use vibe_mcp::tools::{McpTool, AgenticExplainMcpTool};
/// assert_eq!(AgenticExplainMcpTool.descriptor().name, "agentic_explain");
/// ```
#[cell(seam = "McpTool", variant = "agentic_explain")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#transports")]
pub struct AgenticExplainMcpTool;

impl McpTool for AgenticExplainMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "agentic_explain".to_string(),
            description:
                "Return a domain-grounded instruction for explaining this vibevm project (at most three short paragraphs, summarising README.md and folding in what vibe.toml reveals). vibevm composes the instruction from its algorithmic knowledge of the project model, so it is more informative and more reliable than a prompt improvised from scratch; you carry it out, because in agent mode you hold the live context and tools. Treat the returned `instruction` field as the authoritative description of the task and follow it. This is the zero-latency MCP face of the CLI `vibe agentic explain` + `vibe command` relay: the instruction is returned inline and nothing is written to disk."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, _args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        use crate::agentic::{
            ActiveBackend, BackendOutcome, EXPLAIN_AFFINITY, InferenceBackend, InlineBackend,
            check_affinity, explain_intent,
        };
        // The MCP transport reaches vibevm as an agent subprocess, so the
        // relay is the active backend; `explain` is agentic-only and passes
        // the affinity dispatcher (PROP-018 §2.3).
        check_affinity(EXPLAIN_AFFINITY, ActiveBackend::Relay)
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let intent = explain_intent(&ctx.project_root);
        // The MCP path is the inline §2.8 transport — the same op as the CLI
        // one-shot, behind the same InferenceBackend seam, but returned in the
        // tool result with no mailbox written.
        let outcome = InlineBackend
            .submit(&intent)
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let BackendOutcome::Inline { intent } = outcome else {
            return Err(ToolError::Internal(
                "inline backend did not return an inline outcome".into(),
            ));
        };
        Ok(ToolOutput::ok(json!({
            "source": intent.source,
            "title": intent.title,
            "instruction": intent.body,
            "delivery": "inline",
            "note": "Carry out this instruction yourself on your own model; nothing was written to disk.",
        })))
    }
}
