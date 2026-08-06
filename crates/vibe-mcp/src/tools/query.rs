//! The `query` MCP tool — the map-search cell (A5A-MAPSEARCH), split out of
//! `tools.rs` only because that registry file is at its size ceiling. It is
//! registered in `default_tools` in the parent like every other cell; the
//! split keeps both files under the 600-line budget without touching the six
//! existing cells. The contract "a new tool is a new cell added at
//! `default_tools`, not an edit to the dispatcher" still holds — only the
//! impl's address changed.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};

use super::McpTool;
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Search the project's code↔spec map for the **many** nodes that fit
/// criteria — the grep-like counterpart to `explain` (A5A-MAPSEARCH). Shares
/// the [`vibe_trace::search`] core with the CLI `vibe query`: the three
/// independent filters (all optional, AND-joined) become a
/// [`vibe_trace::search::Filters`] and the same build-fresh → search → render
/// pipeline answers — no logic is duplicated in the tool.
///
/// When to reach for this vs `explain`: `query` **finds** nodes by what they
/// are (a set, capped at a hard ceiling); `explain` **looks at** one target's
/// subgraph (the canonical "which test verifies this rule?" lookup). Ask
/// "show me every `fn` that implements something" with `query`; ask "what
/// verifies `spec://…#anchor`?" with `explain`.
///
/// ```
/// use vibe_mcp::tools::{McpTool, QueryMcpTool};
/// assert_eq!(QueryMcpTool.descriptor().name, "query");
/// ```
#[cell(seam = "McpTool", variant = "query")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct QueryMcpTool;

impl McpTool for QueryMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "query".to_string(),
            description:
                "Search the project's code↔spec traceability map for the MANY nodes that fit criteria — the grep-like counterpart to `explain`. Give independent filters, all optional and AND-joined: `uri` (exact `spec://…#anchor`; matches spec units only), `symbol` (substring of a code item's symbol, case-sensitive like grep; matches code items only), `kind` (a code item's `item_kind` — `fn`, `struct`, `mod`, `trait`, `enum`, … — or a spec unit's own kind — `req`, `prop`, …; the two vocabularies never overlap), and `limit` (default 50, hard maximum 200; there is no unbounded mode). Use `query` to FIND nodes by what they are; use `explain` to look at ONE target's subgraph (which test verifies this rule?). Bare `query` with no filters returns a bounded slice of the whole map. The map is built fresh on every call, never from a stale artefact. Returns JSON: `results`, `count`, `total_matching`, `limit`, `truncated`, and the echo of the `filters`."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Exact `spec://…#anchor` URI to match. Only spec units carry an address."
                    },
                    "symbol": {
                        "type": "string",
                        "description": "Substring of a code item's symbol to match (case-sensitive). Only code items carry a symbol."
                    },
                    "kind": {
                        "type": "string",
                        "description": "Element kind to match exactly — a code item's `item_kind` (`fn`, `struct`, `mod`, …) or a spec unit's kind (`req`, `prop`, …)."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results. At least 1; clamped to a hard max of 200. No unbounded mode. Default: 50.",
                        "minimum": 1,
                        "maximum": 200,
                        "default": 50
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
        let uri = args.get("uri").and_then(|v| v.as_str()).map(str::to_owned);
        let symbol = args
            .get("symbol")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        let kind = args.get("kind").and_then(|v| v.as_str()).map(str::to_owned);
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(vibe_trace::search::DEFAULT_LIMIT);
        let filters = vibe_trace::search::Filters {
            uri,
            symbol,
            kind,
            limit,
        };
        // Routes through the SAME library function the CLI uses — no copy of
        // the build-or-search logic lives in the tool (acceptance 7).
        let out = vibe_trace::search::query(&ctx.project_root, &filters)
            .map_err(|e| ToolError::Internal(format!("building the map: {e}")))?;
        let vibe_trace::search::SearchView::Json(value) =
            vibe_trace::search::render(&out, &filters, true)
        else {
            return Err(ToolError::Internal(
                "json render did not return the json view".into(),
            ));
        };
        Ok(value)
    }
}
