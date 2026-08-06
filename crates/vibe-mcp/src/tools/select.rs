//! The `select` MCP tool — the query-language / graph-traversal cell
//! (E-A5B-QUERYLANG), split out of `tools.rs` for the same reason `query.rs`
//! is: that registry file is at its size ceiling. Registered in
//! `default_tools` like every other cell; the contract "a new tool is a new
//! cell added at `default_tools`, not an edit to the dispatcher" holds.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};

use super::McpTool;
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Search the project's code↔spec map with a **predicate query and graph
/// walk** — the traversal layer over `query` (E-A5B-QUERYLANG). Shares the
/// [`vibe_trace::select`] core with the CLI `vibe select`: the `--where` query
/// string is parsed, the map built fresh, the bipartite graph walked per
/// `depth`, and the capped, depth-ordered result returned — no logic is
/// duplicated in the tool.
///
/// When to reach for this vs `query` vs `explain`: `query` FINDS nodes by what
/// they are (a flat, AND-joined filter set — "every `fn`"); `explain` looks at
/// ONE target's one-hop subgraph ("what verifies this rule?"); `select` answers
/// RELATIONAL questions over the graph — "every spec rule with NO verifier"
/// (`lacks:verifies`), "the implementers of this rule and one hop around them"
/// (`uri:… depth:1`), "code under this spec namespace" (`scope:…`). Reach for
/// `select` when the answer needs the edges, not just the node attributes.
///
/// ```
/// use vibe_mcp::tools::{McpTool, SelectMcpTool};
/// assert_eq!(SelectMcpTool.descriptor().name, "select");
/// ```
#[cell(seam = "McpTool", variant = "select")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct SelectMcpTool;

impl McpTool for SelectMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "select".to_string(),
            description:
                "Search the project's code↔spec traceability map with a predicate query AND walk the bipartite graph — the traversal layer over `query`. Give `where` as a whitespace-AND-joined conjunction of `name:value` predicates: `uri:<exact spec://…#anchor>` (spec units only), `symbol:<code-symbol substring>` (code items only), `kind:<item_kind or spec kind>`, `scope:<spec:// uri prefix>` (spec units only), `has:<verb>` / `lacks:<verb>` (`implements|verifies|documents|deviates|informs` — keep seeds an edge of that verb does/does not touch), and `depth:<0..3>` (an UNDIRECTED walk from the seeds; seeds stay at depth 0). The ceiling (default 50, hard max 200; no unbounded mode) is applied AFTER the walk. Reach for `select` over `query` when the answer is RELATIONAL — needs the edges, not just node attributes: e.g. `lacks:verifies` (spec rules with no verifier), `uri:… depth:1` (a rule's implementers and one hop around them), `scope:…` (a spec namespace's code). Reach for `query` for a flat filter, and `explain` for ONE target's one-hop subgraph. The map is built fresh on every call. Returns JSON: `grammar`, `query`, `parsed`, `results` (each a flattened hit + `depth`), `count`, `total_matching`, `limit`, `truncated`."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "where": {
                        "type": "string",
                        "description": "The query: predicates joined by spaces (AND). Each is `name:value` — `uri:`, `symbol:`, `kind:`, `scope:`, `has:`, `lacks:`, `depth:`. Required: an empty query is an error, not \"everything\"."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results, applied AFTER the walk. At least 1; clamped to a hard max of 200. No unbounded mode. Default: 50.",
                        "minimum": 1,
                        "maximum": 200,
                        "default": 50
                    }
                },
                "required": ["where"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
        let query = args
            .get("where")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("`where` must be a string".into()))?;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(vibe_trace::search::DEFAULT_LIMIT);
        // Parse here so a malformed query is an argument error with the
        // parser's own message (it names the offending token) — the build
        // never runs for a query that cannot be walked.
        let parsed = vibe_trace::select::parse(query)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        // Routes through the SAME library function the CLI uses — no copy of
        // the grammar, traversal, or rendering lives in the tool.
        let out = vibe_trace::select::query(&ctx.project_root, &parsed, limit)
            .map_err(|e| ToolError::Internal(format!("building the map: {e}")))?;
        let vibe_trace::select::SelectView::Json(value) =
            vibe_trace::select::render(&out, &parsed, query, true)
        else {
            return Err(ToolError::Internal(
                "json render did not return the json view".into(),
            ));
        };
        Ok(value)
    }
}
