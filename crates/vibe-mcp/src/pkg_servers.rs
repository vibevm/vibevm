//! Package-declared MCP servers (PROP-027 §2.4–§2.5): the pure halves
//! of registration — launch-entry payloads per agent, the closed-set
//! `{project_root}` substitution, and the vibevm-managed sidecar that
//! lets re-installs rewrite ONLY our entries while operator-owned
//! servers stay untouched (the `<vibevm>` block convention of the boot
//! files, applied to agent configs). Discovery lives in
//! `vibe_workspace::bins::collect_mcp_servers` (lockfile/slot
//! knowledge); consent rides the SAME gate as building the binary
//! (`consent_to_build` — one trust model, two verbs); the CLI composes
//! all three and owns the file writes.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-027#registration");

use std::path::Path;

use serde_json::{Map, Value as JsonValue};

use crate::agents::{Agent, ConfigPayload};

/// The top-level sidecar key in a JSON agent config. Its `managed`
/// array names the server entries vibevm owns; everything else in the
/// file is the operator's.
pub const MANAGED_KEY: &str = "vibevm";

/// Strip Windows' `\\?\` verbatim prefix — agent hosts and the
/// registered command lines must never see it (the canonicalize
/// artefact; the same lesson PROP-019's `derive_self` and both oracle
/// bridges carry).
///
/// ```
/// let p = vibe_mcp::pkg_servers::verbatim_free(std::path::Path::new(
///     r"\\?\C:\proj\vibedeps\mcp-x\0.1.0\target\release\x.exe",
/// ));
/// assert_eq!(p.to_string_lossy(), r"C:\proj\vibedeps\mcp-x\0.1.0\target\release\x.exe");
/// ```
pub fn verbatim_free(path: &Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(stripped) => std::path::PathBuf::from(stripped),
        None => path.to_path_buf(),
    }
}

/// Resolve a declaration's `args` against the closed substitution set
/// (`{project_root}` — the absolute, verbatim-free project root).
/// Unknown tokens cannot reach here: `Manifest::validate` refused them
/// at install time (PROP-027 §2.2).
///
/// ```
/// let args = vibe_mcp::pkg_servers::substituted_args(
///     &["--path".to_string(), "{project_root}".to_string()],
///     std::path::Path::new("C:/proj"),
/// );
/// assert_eq!(args, ["--path", "C:/proj"]);
/// ```
pub fn substituted_args(decl_args: &[String], project_root: &Path) -> Vec<String> {
    let root = verbatim_free(project_root).to_string_lossy().into_owned();
    decl_args
        .iter()
        .map(|a| a.replace("{project_root}", &root))
        .collect()
}

/// The launch entry for one package server in `agent`'s config shape.
/// `command` is the ABSOLUTE slot-artifact path — a real executable, so
/// no `cmd /c` shim wrapper is needed on any platform (unlike the
/// product server's `vibe.cmd` entry).
///
/// ```
/// use vibe_mcp::agents::{Agent, ConfigPayload};
///
/// let ConfigPayload::Json(v) = vibe_mcp::pkg_servers::entry_payload(
///     Agent::ClaudeCode,
///     "C:/p/vibedeps/mcp-d/0.5.0/target/release/d.exe",
///     &["--path".to_string(), "C:/p".to_string()],
/// ) else {
///     panic!("Claude Code uses JSON");
/// };
/// assert!(v["command"].as_str().unwrap().ends_with("d.exe"));
/// assert_eq!(v["args"][0], "--path");
/// ```
pub fn entry_payload(agent: Agent, command: &str, args: &[String]) -> ConfigPayload {
    match agent {
        Agent::ClaudeCode | Agent::ClaudeCodeDesktop | Agent::Cursor => {
            ConfigPayload::Json(serde_json::json!({
                "command": command,
                "args": args,
            }))
        }
        Agent::OpenCode => {
            let mut argv = vec![command.to_string()];
            argv.extend(args.iter().cloned());
            ConfigPayload::Json(serde_json::json!({
                "type": "local",
                "command": argv,
                "enabled": true,
            }))
        }
        Agent::Codex => {
            let mut tbl = toml::value::Table::new();
            tbl.insert("command".into(), toml::Value::String(command.to_string()));
            tbl.insert(
                "args".into(),
                toml::Value::Array(
                    args.iter()
                        .map(|a| toml::Value::String(a.clone()))
                        .collect(),
                ),
            );
            ConfigPayload::Toml(toml::Value::Table(tbl))
        }
    }
}

/// The names the sidecar records as vibevm-managed in a JSON config
/// document. An absent or malformed sidecar reads as "none managed" —
/// never an error, so a hand-edited config degrades soft.
///
/// ```
/// let doc = serde_json::json!({
///     "mcpServers": { "discipline-rust": {}, "mine": {} },
///     "vibevm": { "managed": ["discipline-rust"] },
/// });
/// assert_eq!(vibe_mcp::pkg_servers::managed_entries(&doc), ["discipline-rust"]);
/// assert!(vibe_mcp::pkg_servers::managed_entries(&serde_json::json!({})).is_empty());
/// ```
pub fn managed_entries(doc: &JsonValue) -> Vec<String> {
    doc.get(MANAGED_KEY)
        .and_then(|v| v.get("managed"))
        .and_then(JsonValue::as_array)
        .map(|a| {
            a.iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Record `name` in the document's managed sidecar (idempotent; keeps
/// the list sorted so config diffs stay stable).
///
/// ```
/// let mut doc = serde_json::json!({});
/// vibe_mcp::pkg_servers::mark_managed(&mut doc, "discipline-rust").unwrap();
/// vibe_mcp::pkg_servers::mark_managed(&mut doc, "discipline-rust").unwrap();
/// assert_eq!(vibe_mcp::pkg_servers::managed_entries(&doc), ["discipline-rust"]);
/// ```
pub fn mark_managed(doc: &mut JsonValue, name: &str) -> anyhow::Result<()> {
    let obj = doc
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("agent config root is not a JSON object"))?;
    let sidecar = obj
        .entry(MANAGED_KEY.to_string())
        .or_insert_with(|| JsonValue::Object(Map::new()));
    let sidecar_obj = sidecar
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("`{MANAGED_KEY}` is not a JSON object"))?;
    let mut names = sidecar_obj
        .get("managed")
        .and_then(JsonValue::as_array)
        .map(|a| {
            a.iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !names.iter().any(|n| n == name) {
        names.push(name.to_string());
        names.sort();
    }
    sidecar_obj.insert(
        "managed".to_string(),
        JsonValue::Array(names.into_iter().map(JsonValue::String).collect()),
    );
    Ok(())
}

/// Drop `name` from the managed sidecar; an emptied sidecar is removed
/// whole, so an uninstalled config carries no vibevm residue.
///
/// ```
/// let mut doc = serde_json::json!({
///     "vibevm": { "managed": ["a", "b"] },
/// });
/// vibe_mcp::pkg_servers::unmark_managed(&mut doc, "a");
/// assert_eq!(vibe_mcp::pkg_servers::managed_entries(&doc), ["b"]);
/// vibe_mcp::pkg_servers::unmark_managed(&mut doc, "b");
/// assert!(doc.get("vibevm").is_none());
/// ```
pub fn unmark_managed(doc: &mut JsonValue, name: &str) {
    let Some(obj) = doc.as_object_mut() else {
        return;
    };
    let emptied = {
        let Some(sidecar) = obj.get_mut(MANAGED_KEY).and_then(JsonValue::as_object_mut) else {
            return;
        };
        let names: Vec<String> = sidecar
            .get("managed")
            .and_then(JsonValue::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(JsonValue::as_str)
                    .filter(|n| *n != name)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let empty = names.is_empty();
        sidecar.insert(
            "managed".to_string(),
            JsonValue::Array(names.into_iter().map(JsonValue::String).collect()),
        );
        empty
    };
    if emptied {
        obj.remove(MANAGED_KEY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_entry_carries_the_local_argv_shape() {
        let ConfigPayload::Json(v) = entry_payload(
            Agent::OpenCode,
            "/slot/target/release/discipline-mcp-rust",
            &["--path".to_string(), "/proj".to_string()],
        ) else {
            panic!("OpenCode uses JSON");
        };
        assert_eq!(v["type"], "local");
        assert_eq!(v["command"][0], "/slot/target/release/discipline-mcp-rust");
        assert_eq!(v["command"][2], "/proj");
        assert_eq!(v["enabled"], true);
    }

    #[test]
    fn codex_entry_is_a_toml_table() {
        let ConfigPayload::Toml(v) = entry_payload(
            Agent::Codex,
            "/slot/target/release/x",
            &["--path".to_string()],
        ) else {
            panic!("Codex uses TOML");
        };
        assert_eq!(
            v.get("command").and_then(toml::Value::as_str),
            Some("/slot/target/release/x")
        );
    }

    #[test]
    fn substitution_only_touches_the_closed_set() {
        let args = substituted_args(
            &[
                "{project_root}/x".to_string(),
                "literal-{brace}".to_string(),
            ],
            Path::new("/p"),
        );
        assert_eq!(args[0], "/p/x");
        assert_eq!(args[1], "literal-{brace}");
    }
}
