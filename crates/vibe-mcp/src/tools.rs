//! Built-in MCP tools (PROP-015 §2.2). Each tool is a cell behind the
//! [`McpTool`] seam — it describes itself and runs against parsed
//! arguments plus the read-only [`ServerContext`]. The server registers
//! them at one point ([`default_tools`]) and dispatches by name.
//!
//! Slice 1 ships three lockfile-derived tools: two read-only
//! (`query_package`, `read_subskill`) and one writing
//! (`materialise_subskill`). Subsequent slices add `list_capabilities`
//! and PROP-003 §F virtual-capability emission once `vibe-llm` is real.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_core::{Group, PackageRef};

use crate::{ServerContext, ToolDescriptor, ToolError};

/// The MCP tool seam (PROP-015 §2.2): a tool describes itself (name,
/// human description, JSON-Schema input shape) and runs against parsed
/// `arguments` plus the read-only [`ServerContext`]. Every tool is a cell
/// behind this seam; the dispatcher routes by registered name and does
/// not know a tool's identity beyond it.
///
/// ```
/// use vibe_mcp::tools::{McpTool, QueryPackage};
///
/// // A tool's descriptor names it and shapes its arguments.
/// let tool = QueryPackage;
/// let d = tool.descriptor();
/// assert_eq!(d.name, "query_package");
/// assert_eq!(d.input_schema["required"][0], "name");
/// ```
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#tools")]
pub trait McpTool {
    /// The tool's `tools/list` descriptor.
    fn descriptor(&self) -> ToolDescriptor;
    /// Run the tool against parsed `arguments` and the server context.
    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError>;
}

/// The built-in tool set — the one registration point (PROP-015 §2.2).
/// A new tool is a new cell added here, not an edit to the dispatcher.
pub fn default_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(QueryPackage),
        Box::new(ReadSubskill),
        Box::new(MaterialiseSubskill),
        Box::new(AgenticExplain),
        Box::new(crate::tcg::TcgValidate),
        Box::new(crate::tcg::TcgScope),
        Box::new(crate::tcg::TcgComplete),
        Box::new(crate::tcg::TcgType),
    ]
}

// ---------------------------------------------------------------------------
// query_package
// ---------------------------------------------------------------------------

/// Look up an installed package in the lockfile and return its full
/// entry. Read-only.
///
/// ```
/// use vibe_mcp::tools::{McpTool, QueryPackage};
/// assert_eq!(QueryPackage.descriptor().name, "query_package");
/// ```
#[cell(seam = "McpTool", variant = "query_package")]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct QueryPackage;

impl McpTool for QueryPackage {
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

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
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
            .map(|p| Value::String(p.to_string_lossy().replace('\\', "/")))
            .collect();

        Ok(json!({
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
        }))
    }
}

// ---------------------------------------------------------------------------
// read_subskill
// ---------------------------------------------------------------------------

/// Read the materialised content of an active subskill — project tree
/// for eager / lazy-push, package cache for lazy-pull. Read-only.
///
/// ```
/// use vibe_mcp::tools::{McpTool, ReadSubskill};
/// assert_eq!(ReadSubskill.descriptor().name, "read_subskill");
/// ```
#[cell(seam = "McpTool", variant = "read_subskill")]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ReadSubskill;

impl McpTool for ReadSubskill {
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

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
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

        // Per PROP-003 §2.5.0: eager / lazy-push subskills read from the
        // project tree; lazy-pull subskills never touch it and read from
        // the package cache. The lockfile carries `files_written`
        // (project-relative) and `cache_files` (subskill-root-relative);
        // we use whichever matches the delivery mode, so the agent gets
        // bytes regardless of how the package author shipped them.
        let mut content = String::new();
        let mut paths_returned: Vec<Value> = Vec::new();
        if sub.delivery == "lazy-pull" {
            let cache_root = ctx
                .project_root
                .join(".vibe/cache")
                .join(entry.kind.as_str())
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
                let rel_str = rel.to_string_lossy().replace('\\', "/");
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
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                content.push_str(&format!("--- {rel_str}\n"));
                content.push_str(&text);
                if !text.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
                paths_returned.push(Value::String(rel_str));
            }
        }

        Ok(json!({
            "package": package,
            "subskill_path": sub.path,
            "delivery": sub.delivery,
            "describes": sub.describes,
            "paths": paths_returned,
            "content": content,
        }))
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
/// use vibe_mcp::tools::{McpTool, MaterialiseSubskill};
/// assert_eq!(MaterialiseSubskill.descriptor().name, "materialise_subskill");
/// ```
#[cell(seam = "McpTool", variant = "materialise_subskill")]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct MaterialiseSubskill;

impl McpTool for MaterialiseSubskill {
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

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
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
        if sub.delivery != "lazy-pull" {
            return Ok(json!({
                "package": package,
                "subskill_path": sub.path,
                "delivery": sub.delivery,
                "status": "no-op",
                "note": format!(
                    "subskill delivery `{}` is materialised at install time; nothing to do",
                    sub.delivery
                ),
                "written": Vec::<Value>::new(),
            }));
        }
        let cache_root = ctx
            .project_root
            .join(".vibe/cache")
            .join(entry.kind.as_str())
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
                    rel.to_string_lossy()
                )));
                continue;
            }
            if target.exists() && !force {
                skipped.push(Value::String(format!(
                    "{} (already exists; pass force=true to overwrite)",
                    rel.to_string_lossy()
                )));
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source, &target)?;
            written.push(Value::String(rel.to_string_lossy().replace('\\', "/")));
        }
        Ok(json!({
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
        }))
    }
}

// ---------------------------------------------------------------------------
// agentic_explain
// ---------------------------------------------------------------------------

/// Compose the "explain this project" instruction and return it inline —
/// the MCP-transport face of `vibe agentic explain` (PROP-018 §2.8, §2.10).
/// Shares the [`crate::agentic::explain_intent`] core with the CLI relay,
/// but where the CLI one-shot parks the intent in `.vibe/agentic/`, the MCP
/// path returns it synchronously and writes no mailbox file.
///
/// ```
/// use vibe_mcp::tools::{McpTool, AgenticExplain};
/// assert_eq!(AgenticExplain.descriptor().name, "agentic_explain");
/// ```
#[cell(seam = "McpTool", variant = "agentic_explain")]
#[spec(implements = "spec://vibevm/common/PROP-018#transports")]
pub struct AgenticExplain;

impl McpTool for AgenticExplain {
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

    fn run(&self, _args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
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
        Ok(json!({
            "source": intent.source,
            "title": intent.title,
            "instruction": intent.body,
            "delivery": "inline",
            "note": "Carry out this instruction yourself on your own model; nothing was written to disk.",
        }))
    }
}

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
