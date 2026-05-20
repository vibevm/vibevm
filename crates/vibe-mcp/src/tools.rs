//! Built-in MCP tools — first-cut PROP-004 §5.1 surface.
//!
//! Slice 1 ships two read-only tools that surface lockfile-derived
//! information to a connected agent without touching the project
//! tree. Subsequent slices will add `materialise_subskill` (writes),
//! `list_capabilities` (cross-package discovery), and PROP-003 §F
//! virtual-capability emission once `vibe-llm` is real.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use vibe_core::PackageKind;

use crate::{ServerContext, ToolDescriptor, ToolError, ToolHandler};

/// Build the default tool set. Returns a Vec of (descriptor,
/// handler) pairs suitable for `Server::register_tool`.
pub fn default_set() -> Vec<(ToolDescriptor, ToolHandler)> {
    vec![
        query_package_tool(),
        read_subskill_tool(),
        materialise_subskill_tool(),
    ]
}

// ---------------------------------------------------------------------------
// query_package
// ---------------------------------------------------------------------------

fn query_package_tool() -> (ToolDescriptor, ToolHandler) {
    let descriptor = ToolDescriptor {
        name: "query_package".to_string(),
        description:
            "Look up an installed package in the project's lockfile and return its full lockfile entry: kind, name, version, content_hash, registry, source_url, source_ref, resolved_commit, files_written, features, subskills_active, describes (PURL), language. Use this when the agent needs precise version/identity information about something the project already depends on. The response is JSON; the `content_hash` field is the canonical identity per PROP-002 §2.1."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Package reference in `<kind>:<name>` form (e.g. `flow:wal`)."
                }
            },
            "required": ["name"],
            "additionalProperties": false
        }),
    };
    let handler: ToolHandler = Arc::new(query_package_run);
    (descriptor, handler)
}

fn query_package_run(args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidArguments("`name` must be a string".into()))?;
    let (kind, pname) = parse_pkgref(name)?;
    let lockfile = ctx
        .load_lockfile()
        .map_err(|e| ToolError::Internal(format!("loading lockfile: {e}")))?;
    let entry = lockfile
        .find(kind, &pname)
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

// ---------------------------------------------------------------------------
// read_subskill
// ---------------------------------------------------------------------------

fn read_subskill_tool() -> (ToolDescriptor, ToolHandler) {
    let descriptor = ToolDescriptor {
        name: "read_subskill".to_string(),
        description:
            "Read the materialised content of a subskill that activated for an installed package. The agent gets back the concatenated text of every file the subskill's `[content].files_written` recorded, prefixed with each file's project-relative path. Use when an active subskill is mentioned by `query_package` and the agent wants the actual content. Subskills with `delivery = lazy-pull` are also visible through this tool — that's their primary access path."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "Package reference in `<kind>:<name>` form."
                },
                "subskill_path": {
                    "type": "string",
                    "description": "The subskill's canonical path (e.g. `stack/rust`)."
                }
            },
            "required": ["package", "subskill_path"],
            "additionalProperties": false
        }),
    };
    let handler: ToolHandler = Arc::new(read_subskill_run);
    (descriptor, handler)
}

fn read_subskill_run(args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
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
    let (kind, pname) = parse_pkgref(package)?;
    let lockfile = ctx
        .load_lockfile()
        .map_err(|e| ToolError::Internal(format!("loading lockfile: {e}")))?;
    let entry = lockfile
        .find(kind, &pname)
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

    // Per PROP-003 §2.5.0:
    //
    // - `eager` / `lazy-push` subskills materialised into the project
    //   tree at install time. Read from `<project_root>/<path>`.
    // - `lazy-pull` subskills never touch the project tree. Read from
    //   the package cache at
    //   `<.vibe/cache>/<kind>/<name>/<version>/subskills/<path>/<file>`.
    //
    // The lockfile carries both `files_written` (project-relative,
    // for eager / lazy-push) and `cache_files` (subskill-root-
    // relative, for lazy-pull). The MCP server uses whichever
    // matches the subskill's delivery mode, so the agent gets
    // bytes regardless of how the package author chose to ship them.
    let mut content = String::new();
    let mut paths_returned: Vec<Value> = Vec::new();
    if sub.delivery == "lazy-pull" {
        let cache_root = ctx
            .project_root
            .join(".vibe/cache")
            .join(entry.kind.as_str())
            .join(&entry.name)
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

// ---------------------------------------------------------------------------
// materialise_subskill
// ---------------------------------------------------------------------------

fn materialise_subskill_tool() -> (ToolDescriptor, ToolHandler) {
    let descriptor = ToolDescriptor {
        name: "materialise_subskill".to_string(),
        description:
            "Copy a `delivery=lazy-pull` subskill's content into the project tree (under `spec/`). Use when the agent (or the operator at the agent's prompting) decides a lazy-pull subskill should become persistent. The tool does nothing for `eager` / `lazy-push` subskills — those are materialised at install time. Returns the list of paths actually written. Refuses to overwrite existing files unless `force` is true."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "Package reference in `<kind>:<name>` form."
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
    };
    let handler: ToolHandler = Arc::new(materialise_subskill_run);
    (descriptor, handler)
}

fn materialise_subskill_run(args: &Value, ctx: &ServerContext) -> Result<Value, ToolError> {
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
    let force = args
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let (kind, pname) = parse_pkgref(package)?;
    let lockfile = ctx
        .load_lockfile()
        .map_err(|e| ToolError::Internal(format!("loading lockfile: {e}")))?;
    let entry = lockfile
        .find(kind, &pname)
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
        .join(&entry.name)
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

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn parse_pkgref(s: &str) -> Result<(PackageKind, String), ToolError> {
    let (kind_s, name) = s
        .split_once(':')
        .ok_or_else(|| ToolError::InvalidArguments(format!("`{s}`: expected `<kind>:<name>`")))?;
    if name.is_empty() {
        return Err(ToolError::InvalidArguments(format!(
            "`{s}`: empty package name"
        )));
    }
    use std::str::FromStr;
    let kind = PackageKind::from_str(kind_s)
        .map_err(|e| ToolError::InvalidArguments(format!("`{s}`: invalid kind — {e}")))?;
    Ok((kind, name.to_string()))
}

/// Convenience for tests: write a lockfile fixture into a fresh
/// project root and return the [`ServerContext`].
#[doc(hidden)]
pub fn _test_context_with_fixture(project_root: PathBuf, lockfile_text: &str) -> ServerContext {
    std::fs::write(project_root.join("vibe.toml"), "[project]\nname=\"x\"\nversion=\"0.0.1\"\n")
        .unwrap();
    std::fs::write(project_root.join("vibe.lock"), lockfile_text).unwrap();
    ServerContext::new(project_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch_one;
    use serde_json::json;

    fn project_with_locked(text: &str) -> (tempfile::TempDir, ServerContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("vibe.lock"), text).unwrap();
        let ctx = ServerContext::new(dir.path().to_path_buf());
        (dir, ctx)
    }

    const LOCKFILE_FIXTURE: &str = r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-05-05T00:00:00Z"
schema_version = 4

[[package]]
kind = "flow"
name = "wal"
version = "0.1.0"
registry = "vibespecs"
source_url = "https://github.com/vibespecs/flow-wal.git"
source_ref = "v0.1.0"
content_hash = "sha256:deadbeef"
boot_snippet = "10-flow-wal.md"
files_written = [
    "spec/flows/wal/PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]
features = ["default", "base-discipline"]
describes = "pkg:cargo/sqlx@0.8.0"
language = "ru"

[[package.subskills_active]]
path = "stack/rust"
delivery = "lazy-push"
files_written = [
    "spec/flows/wal/PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]
cache_files = [
    "spec/flows/wal/PROTOCOL.md",
    "spec/boot/10-flow-wal.md",
]

[[package.subskills_active]]
path = "sqlx/v08"
delivery = "lazy-pull"
describes = "pkg:cargo/sqlx@^0.8"
cache_files = [
    "spec/flows/wal/SQLX-NOTES.md",
]
"#;

    #[test]
    fn query_package_returns_full_lockfile_entry() {
        let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "query_package",
                "arguments": { "name": "flow:wal" }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], false);
        let payload = &v["result"]["structuredContent"];
        assert_eq!(payload["kind"], "flow");
        assert_eq!(payload["name"], "wal");
        assert_eq!(payload["version"], "0.1.0");
        assert_eq!(payload["content_hash"], "sha256:deadbeef");
        assert_eq!(payload["describes"], "pkg:cargo/sqlx@0.8.0");
        assert_eq!(payload["language"], "ru");
        let subs: Vec<&str> = payload["subskills_active"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["path"].as_str().unwrap())
            .collect();
        assert!(subs.contains(&"stack/rust"));
        assert!(subs.contains(&"sqlx/v08"));
    }

    #[test]
    fn query_package_unknown_returns_error_payload() {
        let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "query_package",
                "arguments": { "name": "flow:nonexistent" }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], true);
        assert!(
            v["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not in lockfile")
        );
    }

    #[test]
    fn query_package_invalid_pkgref_format_errors() {
        let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
        let req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "query_package",
                "arguments": { "name": "no-colon" }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], true);
    }

    #[test]
    fn read_subskill_returns_paths_and_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("vibe.lock"), LOCKFILE_FIXTURE).unwrap();
        // Materialise a couple of files matching files_written.
        let p = dir.path().join("spec/flows/wal/PROTOCOL.md");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "Russian-localised PROTOCOL bytes here.").unwrap();
        let b = dir.path().join("spec/boot/10-flow-wal.md");
        std::fs::create_dir_all(b.parent().unwrap()).unwrap();
        std::fs::write(&b, "boot snippet bytes here.").unwrap();

        let ctx = ServerContext::new(dir.path().to_path_buf());
        let req = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "read_subskill",
                "arguments": {
                    "package": "flow:wal",
                    "subskill_path": "stack/rust",
                }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], false);
        let content = v["result"]["structuredContent"]["content"]
            .as_str()
            .unwrap();
        assert!(content.contains("PROTOCOL bytes"));
        assert!(content.contains("boot snippet bytes"));
        let paths: Vec<&str> = v["result"]["structuredContent"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect();
        assert!(paths.iter().any(|p| p.ends_with("PROTOCOL.md")));
    }

    #[test]
    fn materialise_subskill_copies_lazy_pull_content_into_project() {
        // Lockfile says sqlx/v08 is lazy-pull; populate the package
        // cache with a file under subskills/sqlx/v08/...; call
        // materialise_subskill; expect the file to land at the
        // project-relative path.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("vibe.lock"), LOCKFILE_FIXTURE).unwrap();

        let cache_root = dir
            .path()
            .join(".vibe/cache/flow/wal/v0.1.0/subskills/sqlx/v08/spec/flows/wal");
        std::fs::create_dir_all(&cache_root).unwrap();
        std::fs::write(
            cache_root.join("SQLX-NOTES.md"),
            "sqlx 0.8.x specific notes",
        )
        .unwrap();

        let ctx = ServerContext::new(dir.path().to_path_buf());
        let req = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "materialise_subskill",
                "arguments": {
                    "package": "flow:wal",
                    "subskill_path": "sqlx/v08",
                }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], false);
        let payload = &v["result"]["structuredContent"];
        assert_eq!(payload["status"], "materialised");
        let written: Vec<&str> = payload["written"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect();
        assert!(written.contains(&"spec/flows/wal/SQLX-NOTES.md"));
        let materialised = dir
            .path()
            .join("spec/flows/wal/SQLX-NOTES.md");
        assert!(materialised.is_file());
        let body = std::fs::read_to_string(&materialised).unwrap();
        assert!(body.contains("sqlx 0.8.x"));
    }

    #[test]
    fn materialise_subskill_no_op_for_eager_delivery() {
        // The fixture has no eager subskill, but stack/rust is
        // lazy-push (which is "materialised at install time" by
        // current degradation). The tool returns no-op for
        // anything other than lazy-pull.
        let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
        let req = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "materialise_subskill",
                "arguments": {
                    "package": "flow:wal",
                    "subskill_path": "stack/rust",
                }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], false);
        assert_eq!(
            v["result"]["structuredContent"]["status"], "no-op"
        );
    }

    #[test]
    fn materialise_subskill_refuses_overwrite_without_force() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("vibe.lock"), LOCKFILE_FIXTURE).unwrap();
        let cache_root = dir
            .path()
            .join(".vibe/cache/flow/wal/v0.1.0/subskills/sqlx/v08/spec/flows/wal");
        std::fs::create_dir_all(&cache_root).unwrap();
        std::fs::write(cache_root.join("SQLX-NOTES.md"), "from-cache").unwrap();
        // Plant an existing file at the target path.
        let target_dir = dir.path().join("spec/flows/wal");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("SQLX-NOTES.md"), "user-edit").unwrap();

        let ctx = ServerContext::new(dir.path().to_path_buf());
        let req = json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "materialise_subskill",
                "arguments": {
                    "package": "flow:wal",
                    "subskill_path": "sqlx/v08",
                }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        let payload = &v["result"]["structuredContent"];
        assert_eq!(payload["status"], "skipped");
        let body =
            std::fs::read_to_string(target_dir.join("SQLX-NOTES.md")).unwrap();
        assert_eq!(
            body, "user-edit",
            "user file must survive when force is unset"
        );
    }

    #[test]
    fn read_subskill_unknown_subskill_errors() {
        let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
        let req = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "read_subskill",
                "arguments": {
                    "package": "flow:wal",
                    "subskill_path": "made/up",
                }
            }
        })
        .to_string();
        let resp = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["result"]["isError"], true);
        assert!(
            v["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not active")
        );
    }
}
