//! MCP `requirements_query` — the ONE read-only requirements answer over
//! MCP (PROP-054 `##FACT-QUERY-CONTRACT`; R7 architecture §5, §6.2–§6.3).
//!
//! The grammar is three OPTIONAL members — `address_prefix`, `limit`,
//! `relations` — and nothing else. There is deliberately no `path`: the
//! node this tool answers for is [`ServerContext::project_root`], the
//! surface's own trusted authority, so no caller can retarget the server
//! at another tree. There is no provider, model, lifecycle, sync or write
//! option either — the tool is algorithmic: it reads no `[llm]` table,
//! opens no socket, spends no token and touches no secret.
//!
//! Order is the whole argument law. Decode through a private
//! `deny_unknown_fields` struct, resolve the three defaults, and build the
//! validated [`RequirementsQuery`] — all BEFORE the first filesystem or
//! state access. A wrong type, an unknown member, a limit outside
//! `1..=256` or a prefix that is not a `spec://` URI therefore refuses
//! text-only, with no `.vibe` directory, no lock and no state byte
//! created.
//!
//! Everything after the grammar is composition, never assembly: the
//! selected node comes from the read-only
//! [`Workspace::discover_selected`], the optional lifecycle run id from
//! the read-only [`LifecycleStateStore::peek`] (joined ONLY when the
//! durable run header names this same selected node — a sibling's run is
//! not this node's join key), the clock from `chrono::Utc::now()`, and the
//! relation provider is exactly [`SpecmapRelationProvider`] when — and
//! only when — relations were requested. `vibe_requirements::query` is
//! then called once: its generated root IS the `structuredContent` and
//! `vibe_requirements::text::render` IS the text channel. This surface
//! interprets neither and constructs no report member of its own.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use serde::Deserialize;
use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_lifecycle::LifecycleStateStore;
use vibe_requirements::{QueryContext, QueryError, RelationProvider, RequirementsQuery};
use vibe_trace::SpecmapRelationProvider;
use vibe_wire::behaviour::requirements_report::{ADDRESS_CAP_BYTES, LIMIT_MAX, LIMIT_MIN};
use vibe_workspace::{SelectedWorkspace, Workspace};

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

/// MCP `requirements_query`: answer ONE bounded, read-only question about
/// what this project's specs declare and what it recorded about them —
/// the same generated report the CLI's `vibe requirements --json` returns,
/// because both surfaces call the same `vibe_requirements::query`.
///
/// ```
/// use vibe_mcp::tools::{McpTool, RequirementsQueryMcpTool};
///
/// let descriptor = RequirementsQueryMcpTool.descriptor();
/// assert_eq!(descriptor.name, "requirements_query");
/// // Three members, all optional — a bare `{}` is a valid call.
/// assert!(descriptor.input_schema["required"].is_null());
/// let properties = descriptor.input_schema["properties"].as_object().unwrap();
/// assert_eq!(properties.len(), 3);
/// // …and none of them is a path: the selected node is the server's.
/// assert!(properties.get("path").is_none());
/// // The advertised defaults/caps are the library's and the wire's own.
/// assert_eq!(descriptor.input_schema["properties"]["limit"]["default"], 100);
/// assert_eq!(descriptor.input_schema["properties"]["limit"]["maximum"], 256);
/// ```
#[cell(seam = "McpTool", variant = "requirements_query")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
pub struct RequirementsQueryMcpTool;

/// The runtime argument authority: exactly the three optional members,
/// decoded BEFORE anything on disk is touched. `deny_unknown_fields` is
/// what makes `path`, `provider`, `model`, `sync` and every other
/// smuggled spelling a refusal rather than a silently ignored member.
///
/// The two defaults are the LIBRARY's own — [`RequirementsQuery::default`]
/// — read through the same functions the descriptor advertises, so the
/// documented grammar and the enforced one cannot drift.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementsQueryArgs {
    #[serde(default)]
    address_prefix: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default = "default_relations")]
    relations: bool,
}

/// The effective row bound when the caller states none — the library's
/// default, never a second constant typed here.
fn default_limit() -> u32 {
    RequirementsQuery::default().limit()
}

/// The effective relation posture when the caller states none — likewise
/// the library's own default (`false`: no map is loaded or built).
fn default_relations() -> bool {
    RequirementsQuery::default().relations()
}

impl McpTool for RequirementsQueryMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        // Descriptor and runtime read the SAME defaults (the library's
        // `RequirementsQuery::default()`, through the same two functions
        // the private decoder uses) and the SAME caps (the wire owner's
        // `LIMIT_MIN`/`LIMIT_MAX`/`ADDRESS_CAP_BYTES`, which is also what
        // `RequirementsQuery::try_new` enforces). A future change to
        // either lands in both or in neither.
        let limit = default_limit();
        ToolDescriptor {
            name: "requirements_query".into(),
            description: format!(
                "Answer one bounded, read-only question about this project's requirements: every \
                 addressed fact its own specs and its installed packages' specs declare, each with \
                 its authoring status, its separately-recorded consumer adoption, and — only if \
                 asked — its relation edges. Returns the generated requirements report as \
                 structuredContent (identical to the CLI's `vibe requirements --json`) plus a \
                 bounded text projection. Metadata ONLY: no fact prose, no code body, no prompt, no \
                 recommendation, no ranking and no next task, so an orchestrator reads observations \
                 rather than advice. All three arguments are optional — `{{}}` answers for the whole \
                 selected project with limit {limit} and no relations. There is deliberately no \
                 `path`: the project is fixed when this MCP server starts. Read-only and \
                 algorithmic — nothing is synced, written, installed or sent anywhere, and no model \
                 is called."
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address_prefix": {
                        "type": "string",
                        "maxLength": ADDRESS_CAP_BYTES,
                        "description": format!(
                            "Optional `spec://` address prefix scoping the answer — a full URI \
                             prefix such as `spec://org.example/demo/RULE`, never a bare fact id. \
                             At most {ADDRESS_CAP_BYTES} bytes. Default: absent, meaning every \
                             enumerated source answers."
                        )
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": LIMIT_MIN,
                        "maximum": LIMIT_MAX,
                        "default": limit,
                        "description": format!(
                            "Optional row bound, {LIMIT_MIN}..={LIMIT_MAX} inclusive. Default: \
                             {limit}. When the bound cuts the row set the report says so in its \
                             own `truncated` member — a cut answer never reads as a complete one."
                        )
                    },
                    "relations": {
                        "type": "boolean",
                        "default": default_relations(),
                        "description": format!(
                            "Optional relation enrichment. Default: {}. False loads and builds no \
                             map at all and every source answers `not-requested`; true asks the \
                             read-only specmap provider once, and a missing, stale or malformed map \
                             is a typed per-source state — the fact rows still return either way.",
                            default_relations()
                        )
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        // GRAMMAR FIRST. Nothing below this line has read a directory,
        // opened a lock, created `.vibe` or peeked at any state: an
        // unacceptable question refuses having changed nothing.
        let query = parse_query(args)?;

        // The trusted selected node — the server's own root, never an
        // argument. Discovery is read-only and may legitimately fail (the
        // server was started outside a workspace node); that refusal is
        // NOT rendered here, because `vibe_requirements::query` produces
        // the same typed workspace error from the same call below, and one
        // authority for it is the point. All that is lost meanwhile is the
        // lifecycle join key, which such a project does not have anyway.
        let selected = Workspace::discover_selected(&ctx.project_root).ok();
        let (selected_root, lifecycle_run_id) = match &selected {
            Some(selected) => (selected.selected_root.clone(), joined_run_id(selected)?),
            None => (ctx.project_root.clone(), None),
        };

        // Exactly one provider value, injected only for a query that asked
        // for relations — the `false` case must reach the library as
        // `None`, which is what makes `not-requested` an honest statement
        // that no map was loaded rather than an empty scan result.
        let specmap = SpecmapRelationProvider;
        let provider: Option<&dyn RelationProvider> = if query.relations() {
            Some(&specmap)
        } else {
            None
        };

        // The ONE call. The surface owns the clock and the trusted roots;
        // the library owns every member of the answer.
        let report = vibe_requirements::query(
            &query,
            &QueryContext {
                selected_root,
                observed_at: chrono::Utc::now(),
                lifecycle_run_id,
            },
            provider,
        )
        .map_err(refuse)?;

        // The generated root crosses as `structuredContent` exactly as the
        // library produced it, and the text channel is the library's own
        // bounded projection — no second assembly, no reinterpretation and
        // no surface-minted prose.
        let text = vibe_requirements::text::render(&report);
        let structured = serde_json::to_value(&report).map_err(|error| {
            ToolError::Internal(format!("serialising the requirements report: {error}"))
        })?;
        Ok(ToolOutput::executed(structured, text))
    }
}

/// The strict argument parse. Omitted/null `arguments` normalise to `{}`
/// and answer with every default; a scalar or array refuses; an unknown
/// member (`path`, `provider`, `model`, `project`, `sync`, …), a wrong
/// type, a limit of `0` or `257`, and a prefix that is not a `spec://`
/// URI all refuse HERE — before the filesystem, the lifecycle state or
/// any `.vibe` byte is reached.
fn parse_query(args: &Value) -> Result<RequirementsQuery, ToolError> {
    let normalized = match args {
        Value::Null => Value::Object(serde_json::Map::new()),
        Value::Object(_) => args.clone(),
        other => {
            return Err(ToolError::InvalidArguments(format!(
                "`requirements_query` takes an object with the optional members \
                 `address_prefix`, `limit` and `relations` — got {other}"
            )));
        }
    };
    let decoded: RequirementsQueryArgs = serde_json::from_value(normalized).map_err(|error| {
        ToolError::InvalidArguments(format!(
            "`requirements_query` takes only the optional members `address_prefix`, `limit` and \
             `relations`; there is deliberately no `path`, provider, model, sync or write option: \
             {error}"
        ))
    })?;
    // The library's own grammar has the last word on the DECODED values,
    // so the surface never re-implements the range or the prefix law.
    RequirementsQuery::try_new(
        decoded.address_prefix.as_deref(),
        decoded.limit,
        decoded.relations,
    )
    .map_err(|error| ToolError::InvalidArguments(typed_chain(&error)))
}

/// The optional lifecycle run id for THIS selected node, through the
/// read-only lock-free peek: no begin, no adopt, no lease, no write, and
/// no `.vibe` directory created when none exists.
///
/// The join is by node identity, not by mere presence. A durable header
/// whose `selected` names a sibling belongs to that node's run; borrowing
/// its id would attach this observation to a run that never observed this
/// node — and because the run id participates in `observation_id`, that
/// would mint a wrong identity rather than merely a wrong label.
fn joined_run_id(selected: &SelectedWorkspace) -> Result<Option<String>, ToolError> {
    let state = LifecycleStateStore::peek(&selected.workspace.root)
        .map_err(|error| ToolError::Io(std::io::Error::other(typed_chain(&error))))?;
    let Some(state) = state else {
        return Ok(None);
    };
    if state.run.selected.as_deref() != Some(selected.selected.as_str()) {
        return Ok(None);
    }
    Ok(state.run.run_id)
}

/// Map the library's typed refusal onto the MCP error channel, carrying
/// the FULL typed chain — each `QueryError` already names what it
/// violated and how to fix it, and that wording is what the agent acts on.
///
/// `InvalidQuery` can only be reached through a value the decoder let
/// past, so it is an argument problem; the two invariant breaks are
/// server-side bugs; everything else is a project the query could not
/// establish a scope over (an undiscoverable node, a malformed lock,
/// registry or durable state).
fn refuse(error: QueryError) -> ToolError {
    let text = typed_chain(&error);
    match error {
        QueryError::InvalidQuery { .. } => ToolError::InvalidArguments(text),
        QueryError::Invariant(_) | QueryError::Wire { .. } => ToolError::Internal(text),
        _ => ToolError::Io(std::io::Error::other(text)),
    }
}

/// Flatten an error and every `source` behind it into one line. The typed
/// remedies these errors carry live on the inner variants, so collapsing
/// to the outer `Display` alone would drop exactly the actionable half.
fn typed_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    text
}

#[cfg(test)]
#[path = "requirements_query/tests.rs"]
mod tests;
