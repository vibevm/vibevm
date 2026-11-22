//! Built-in MCP tools (PROP-015 §2.2). Each tool is a cell behind the
//! [`McpTool`] seam — it describes itself and runs against parsed
//! arguments plus the read-only [`ServerContext`]. The server registers
//! them at one point ([`default_tools`]) and dispatches by name.
//!
//! Slice 1 ships three lockfile-derived tools: two read-only
//! (`query_package`, `read_subskill`) and one writing
//! (`materialise_subskill`). Subsequent slices add `list_capabilities`
//! and PROP-003 §F virtual-capability emission once `vibe-llm` is real.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_core::{Group, PackageRef, machine_json_path};

use crate::{ServerContext, ToolDescriptor, ToolError};

pub mod migration_operation;
pub mod migration_plan;
pub use migration_operation::ManifestMigrationOperationMcpTool;
pub use migration_plan::PlanManifestMigrationMcpTool;

pub mod output;
pub use output::ToolOutput;

/// The ordinary configuration preview (PROP-069 `##SURFACE-PARITY`) and its
/// version-two integer/profile sibling: both render the library operation
/// the CLI renders, against the project root the host supplies.
pub mod configuration_preview;
pub use configuration_preview::ConfigurationPreviewMcpTool;
pub mod configuration_preview_integer;
pub use configuration_preview_integer::ConfigurationPreviewV2McpTool;
/// The documentation page reader (PROP-057 `##OBS-AUDIENCE-AGENT`),
/// split out beside `explain`, `query` and `select` for the same reason
/// they are: this registry file sits at its size ceiling.
pub mod read_doc;
pub use read_doc::ReadDocMcpTool;

/// The MCP tool seam (PROP-015 §2.2): a tool describes itself (name,
/// human description, JSON-Schema input shape) and runs against parsed
/// `arguments` plus the read-only [`ServerContext`]. Every tool is a cell
/// behind this seam; the dispatcher routes by registered name and does
/// not know a tool's identity beyond it.
///
/// ```
/// use vibe_mcp::tools::{McpTool, QueryPackageMcpTool};
///
/// // A tool's descriptor names it and shapes its arguments.
/// let tool = QueryPackageMcpTool;
/// let d = tool.descriptor();
/// assert_eq!(d.name, "query_package");
/// assert_eq!(d.input_schema["required"][0], "name");
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub trait McpTool {
    /// The tool's `tools/list` descriptor.
    fn descriptor(&self) -> ToolDescriptor;
    /// Run the tool against parsed `arguments` and the server context.
    ///
    /// `Ok(ToolOutput)` means the operation **executed** — ordinary
    /// success, or an executed structured failure whose report survives
    /// in `structuredContent` under `isError: true`. `Err(ToolError)`
    /// is the preflight / tool-failure channel: the operation never
    /// ran. A typed structured-preflight error retains its registered report;
    /// other failures render text only. See [`ToolOutput`] for the full distinction.
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError>;
}

/// The built-in tool set — the one registration point (PROP-015 §2.2).
/// A new tool is a new cell added here, not an edit to the dispatcher.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub fn default_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(PlanManifestMigrationMcpTool),
        Box::new(ManifestMigrationOperationMcpTool),
        Box::new(QueryPackageMcpTool),
        Box::new(ConfigurationPreviewMcpTool),
        Box::new(ConfigurationPreviewV2McpTool),
        Box::new(ReadSubskillMcpTool),
        Box::new(MaterialiseSubskillMcpTool),
        Box::new(AgenticExplainMcpTool),
        Box::new(ExplainMcpTool),
        Box::new(QueryMcpTool),
        Box::new(SelectMcpTool),
        Box::new(ListToolsMcpTool),
        Box::new(LifecycleTasksMcpTool),
        Box::new(LifecycleRunMcpTool),
        Box::new(RequirementsQueryMcpTool),
        Box::new(ReadDocMcpTool),
    ]
}

// ---------------------------------------------------------------------------
// list_tools
// ---------------------------------------------------------------------------

/// The registry of what this project can invoke. Read-only.
///
/// The **second surface** over `vibe_workspace::tools::collect_tools`, the
/// first being `vibe tools`. Neither computes: both call the library and
/// render. That is `flow:omnichannel` dogfooded rather than described —
/// delete either surface on paper and only presentation is lost.
///
/// Why an agent needs it: the boot lane already names which language
/// disciplines a project follows, so "which guides do I hold?" is answered
/// before it is asked. What those packages brought that can be RUN is a
/// different question, and it reached no agent until this tool existed.
///
/// ```
/// use vibe_mcp::tools::{McpTool, ListToolsMcpTool};
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

// ---------------------------------------------------------------------------
// query_package
// ---------------------------------------------------------------------------

/// Look up an installed package in the lockfile and return its full
/// entry. Read-only.
///
/// ```
/// use vibe_mcp::tools::{McpTool, QueryPackageMcpTool};
/// assert_eq!(QueryPackageMcpTool.descriptor().name, "query_package");
/// ```
#[cell(seam = "McpTool", variant = "query_package")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct QueryPackageMcpTool;

impl McpTool for QueryPackageMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "query_package".to_string(),
            description:
                "Look up an installed package in the project's lockfile and return its full lockfile entry: kind, name, version, content_hash, registry, source_url, source_ref, resolved_commit, files_written, features, subskills_active, describes (PURL), language. Use this when the agent needs precise version/identity information about something the project already depends on. The response is JSON; the `content_hash` field is the canonical identity per PROP-002 §2.1."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Group-qualified package reference in `<group>/<name>` form (e.g. `org.vibevm/wal`)."
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("`name` must be a string".into()))?;
        let (group, pname) = parse_pkgref(name)?;
        let lockfile = ctx
            .load_lockfile()
            .map_err(|e| ToolError::Internal(format!("loading lockfile: {e}")))?;
        let entry = lockfile
            .find(&group, &pname)
            .ok_or_else(|| ToolError::NotFound(format!("package `{name}` not in lockfile")))?;

        let subskills: Vec<Value> = entry
            .subskills_active
            .iter()
            .map(|s| {
                let mut obj = serde_json::Map::new();
                obj.insert("path".into(), Value::String(s.path.clone()));
                obj.insert("delivery".into(), Value::String(s.delivery.clone()));
                if let Some(d) = &s.describes {
                    obj.insert("describes".into(), Value::String(d.clone()));
                }
                Value::Object(obj)
            })
            .collect();
        let files: Vec<Value> = entry
            .files_written
            .iter()
            .map(|path| Value::String(machine_json_path(path)))
            .collect();

        Ok(ToolOutput::ok(json!({
            "kind": entry.kind.as_str(),
            "name": entry.name,
            "version": entry.version.to_string(),
            "registry": entry.registry,
            "source_url": entry.source_url,
            "source_ref": entry.source_ref,
            "resolved_commit": entry.resolved_commit,
            "content_hash": entry.content_hash,
            "boot_snippet": entry.boot_snippet,
            "files_written": files,
            "features": entry.features,
            "subskills_active": subskills,
            "describes": entry.describes,
            "language": entry.language,
        })))
    }
}

// ---------------------------------------------------------------------------
// read_subskill
// ---------------------------------------------------------------------------

/// Read the materialised content of an active subskill — project tree
/// for eager / lazy-push, package cache for lazy-pull. Read-only.
///
/// ```
/// use vibe_mcp::tools::{McpTool, ReadSubskillMcpTool};
/// assert_eq!(ReadSubskillMcpTool.descriptor().name, "read_subskill");
/// ```
#[cell(seam = "McpTool", variant = "read_subskill")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ReadSubskillMcpTool;

impl McpTool for ReadSubskillMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "read_subskill".to_string(),
            description:
                "Read the materialised content of a subskill that activated for an installed package. The agent gets back the concatenated text of every file the subskill's `[content].files_written` recorded, prefixed with each file's project-relative path. Use when an active subskill is mentioned by `query_package` and the agent wants the actual content. Subskills with `delivery = lazy-pull` are also visible through this tool — that's their primary access path."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "package": {
                        "type": "string",
                        "description": "Group-qualified package reference in `<group>/<name>` form."
                    },
                    "subskill_path": {
                        "type": "string",
                        "description": "The subskill's canonical path (e.g. `stack/rust`)."
                    }
                },
                "required": ["package", "subskill_path"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let package = args
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("`package` must be a string".into()))?;
        let subskill_path = args
            .get("subskill_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments("`subskill_path` must be a string".into())
            })?;
        let (group, pname) = parse_pkgref(package)?;
        let lockfile = ctx
            .load_lockfile()
            .map_err(|e| ToolError::Internal(format!("loading lockfile: {e}")))?;
        let entry = lockfile
            .find(&group, &pname)
            .ok_or_else(|| ToolError::NotFound(format!("package `{package}` not in lockfile")))?;
        let sub = entry
            .subskills_active
            .iter()
            .find(|s| s.path == subskill_path)
            .ok_or_else(|| {
                ToolError::NotFound(format!(
                    "subskill `{subskill_path}` is not active on `{package}`"
                ))
            })?;

        if sub.source_witness.is_some() || lockfile.configurations.contains_key(package) {
            let bytes = vibe_registry::subskill_witness::read_configured_subskill(
                &ctx.store_root,
                entry,
                sub,
            )
            .map_err(|error| {
                ToolError::Internal(format!(
                    "configured subskill verification refused: {error:#}"
                ))
            })?;
            let text = String::from_utf8(bytes)
                .map_err(|_| ToolError::Internal("configured subskill is not UTF-8".into()))?;
            let path = machine_json_path(&sub.cache_files[0]);
            let content = format!(
                "--- {path}\n{text}{}\n",
                if text.ends_with('\n') { "" } else { "\n" }
            );
            return Ok(ToolOutput::ok(
                json!({ "package": package, "subskill_path": sub.path, "delivery": sub.delivery, "describes": sub.describes, "paths": [path], "content": content }),
            ));
        }

        // Per PROP-003 §2.5.0: eager / lazy-push subskills read from the
        // project tree; lazy-pull subskills never touch it and read from
        // the machine-global package store. The lockfile carries
        // `files_written` (project-relative) and `cache_files`
        // (subskill-root-relative); we use whichever matches the
        // delivery mode, so the agent gets bytes regardless of how the
        // package author shipped them. The store entry is keyed by
        // package identity (`<store>/<group>/<name>/v<version>/`,
        // PROP-010 §2.7) — joined here from the documented layout
        // because this crate does not depend on `vibe-registry`.
        let mut content = String::new();
        let mut paths_returned: Vec<Value> = Vec::new();
        if sub.delivery == "lazy-pull" {
            let cache_root = ctx
                .store_root
                .join(entry.group.as_str())
                .join(entry.name.as_str())
                .join(format!("v{}", entry.version));
            let sub_root = cache_root.join("subskills").join(&sub.path);
            for rel in &sub.cache_files {
                let abs = sub_root.join(rel);
                if !abs.is_file() {
                    continue;
                }
                let bytes = std::fs::read(&abs)?;
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let rel_str = machine_json_path(rel);
                content.push_str(&format!("--- {rel_str}\n"));
                content.push_str(&text);
                if !text.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
                paths_returned.push(Value::String(rel_str));
            }
        } else {
            for rel in &sub.files_written {
                let abs = ctx.project_root.join(rel);
                if !abs.is_file() {
                    continue;
                }
                let bytes = std::fs::read(&abs)?;
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let rel_str = machine_json_path(rel);
                content.push_str(&format!("--- {rel_str}\n"));
                content.push_str(&text);
                if !text.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
                paths_returned.push(Value::String(rel_str));
            }
        }

        Ok(ToolOutput::ok(json!({
            "package": package,
            "subskill_path": sub.path,
            "delivery": sub.delivery,
            "describes": sub.describes,
            "paths": paths_returned,
            "content": content,
        })))
    }
}

// ---------------------------------------------------------------------------
// materialise_subskill
// ---------------------------------------------------------------------------

/// Copy a `lazy-pull` subskill's content into the project tree. No-op for
/// eager / lazy-push; refuses to overwrite without `force`. The one
/// writing tool.
///
/// ```
/// use vibe_mcp::tools::{McpTool, MaterialiseSubskillMcpTool};
/// assert_eq!(MaterialiseSubskillMcpTool.descriptor().name, "materialise_subskill");
/// ```
#[cell(seam = "McpTool", variant = "materialise_subskill")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct MaterialiseSubskillMcpTool;

impl McpTool for MaterialiseSubskillMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "materialise_subskill".to_string(),
            description:
                "Copy a `delivery=lazy-pull` subskill's content into the project tree (under `spec/`). Use when the agent (or the operator at the agent's prompting) decides a lazy-pull subskill should become persistent. The tool does nothing for `eager` / `lazy-push` subskills — those are materialised at install time. Returns the list of paths actually written. Refuses to overwrite existing files unless `force` is true."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "package": {
                        "type": "string",
                        "description": "Group-qualified package reference in `<group>/<name>` form."
                    },
                    "subskill_path": {
                        "type": "string",
                        "description": "The subskill's canonical path (e.g. `sqlx/v08`)."
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Overwrite existing files at the target paths. Default: false."
                    }
                },
                "required": ["package", "subskill_path"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let package = args
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("`package` must be a string".into()))?;
        let subskill_path = args
            .get("subskill_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments("`subskill_path` must be a string".into())
            })?;
        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
        let (group, pname) = parse_pkgref(package)?;
        let lockfile = ctx
            .load_lockfile()
            .map_err(|e| ToolError::Internal(format!("loading lockfile: {e}")))?;
        let entry = lockfile
            .find(&group, &pname)
            .ok_or_else(|| ToolError::NotFound(format!("package `{package}` not in lockfile")))?;
        let sub = entry
            .subskills_active
            .iter()
            .find(|s| s.path == subskill_path)
            .ok_or_else(|| {
                ToolError::NotFound(format!(
                    "subskill `{subskill_path}` is not active on `{package}`"
                ))
            })?;
        if sub.source_witness.is_some() || lockfile.configurations.contains_key(package) {
            return Err(ToolError::InvalidArguments(
                "configured lazy-pull selection grants read-only delivery, not materialisation"
                    .into(),
            ));
        }
        if sub.delivery != "lazy-pull" {
            return Ok(ToolOutput::ok(json!({
                "package": package,
                "subskill_path": sub.path,
                "delivery": sub.delivery,
                "status": "no-op",
                "note": format!(
                    "subskill delivery `{}` is materialised at install time; nothing to do",
                    sub.delivery
                ),
                "written": Vec::<Value>::new(),
            })));
        }
        // The machine store entry for this package identity
        // (`<store>/<group>/<name>/v<version>/`, PROP-010 §2.7) — the
        // same layout the fetch path inserts into; the layout is the
        // index, so the reader joins it rather than consulting a
        // second representation.
        let cache_root = ctx
            .store_root
            .join(entry.group.as_str())
            .join(entry.name.as_str())
            .join(format!("v{}", entry.version));
        let sub_root = cache_root.join("subskills").join(&sub.path);

        let mut written: Vec<Value> = Vec::new();
        let mut skipped: Vec<Value> = Vec::new();
        for rel in &sub.cache_files {
            let source = sub_root.join(rel);
            let target = ctx.project_root.join(rel);
            if !source.is_file() {
                skipped.push(Value::String(format!(
                    "{} (cache miss)",
                    machine_json_path(rel)
                )));
                continue;
            }
            if target.exists() && !force {
                skipped.push(Value::String(format!(
                    "{} (already exists; pass force=true to overwrite)",
                    machine_json_path(rel)
                )));
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source, &target)?;
            written.push(Value::String(machine_json_path(rel)));
        }
        Ok(ToolOutput::ok(json!({
            "package": package,
            "subskill_path": sub.path,
            "delivery": sub.delivery,
            "status": if written.is_empty() && skipped.is_empty() {
                "empty"
            } else if written.is_empty() {
                "skipped"
            } else {
                "materialised"
            },
            "written": written,
            "skipped": skipped,
        })))
    }
}

pub mod agentic_explain;
pub use agentic_explain::AgenticExplainMcpTool;
pub mod explain;
pub use explain::ExplainMcpTool;
pub mod query;
pub use query::QueryMcpTool;
pub mod select;
pub use select::SelectMcpTool;
pub mod lifecycle_tasks;
pub use lifecycle_tasks::LifecycleTasksMcpTool;
pub mod lifecycle_run;
pub use lifecycle_run::LifecycleRunMcpTool;
pub mod requirements_query;
pub use requirements_query::RequirementsQueryMcpTool;
// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Parse a package reference into its `(group, name)` identity. The
/// reference must be group-qualified (`<group>/<name>`, e.g.
/// `org.vibevm/wal`); an optional `<kind>:` prefix is tolerated but
/// ignored — `kind` is metadata, not identity (PROP-008 §2.3).
fn parse_pkgref(s: &str) -> Result<(Group, String), ToolError> {
    let pkgref = PackageRef::parse(s).map_err(|e| {
        ToolError::InvalidArguments(format!("`{s}`: invalid package reference — {e}"))
    })?;
    let group = pkgref.group.ok_or_else(|| {
        ToolError::InvalidArguments(format!(
            "`{s}`: package reference must be group-qualified — write `<group>/<name>`"
        ))
    })?;
    Ok((group, pkgref.name.to_string()))
}
