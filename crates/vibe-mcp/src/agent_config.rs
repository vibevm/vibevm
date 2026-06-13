//! Agent config-file I/O (PROP-015 §2.5): read an agent's JSON or TOML
//! config, upsert vibevm's one entry under the MCP section preserving
//! every foreign key, and strip it back out the same way. Format-generic
//! primitives — the caller picks JSON vs TOML from the agent profile and
//! threads the section key + server name in.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#agent-config");

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_json::{Map, Value as JsonValue};

/// Parse a JSON config file. A missing-or-empty file is an empty object,
/// not an error — first-time installs write into a fresh config.
pub fn read_json(path: &Path) -> Result<JsonValue> {
    let text = fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    if text.trim().is_empty() {
        return Ok(JsonValue::Object(Map::new()));
    }
    let v: JsonValue = serde_json::from_str(&text)
        .with_context(|| format!("parsing JSON `{}`", path.display()))?;
    Ok(v)
}

/// Parse a TOML config file. Missing-or-empty → empty table.
pub fn read_toml(path: &Path) -> Result<toml::Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    if text.trim().is_empty() {
        return Ok(toml::Value::Table(toml::value::Table::new()));
    }
    let v: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing TOML `{}`", path.display()))?;
    Ok(v)
}

/// Upsert `new_entry` under `section_key[server_name]` in a JSON config,
/// returning the merged document. A missing file starts empty; every
/// other key under the section and the document survives untouched —
/// the operator's other MCP servers are never disturbed (PROP-015 §2.5).
pub fn merge_json(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
    new_entry: &JsonValue,
) -> Result<JsonValue> {
    let mut existing = if config_path.exists() {
        read_json(config_path)?
    } else {
        JsonValue::Object(Map::new())
    };
    let obj = existing
        .as_object_mut()
        .ok_or_else(|| anyhow!("`{}` is not a JSON object", config_path.display()))?;
    let servers = obj
        .entry(section_key.to_string())
        .or_insert_with(|| JsonValue::Object(Map::new()));
    let servers_obj = servers
        .as_object_mut()
        .ok_or_else(|| anyhow!("`{section_key}` is not a JSON object"))?;
    servers_obj.insert(server_name.to_string(), new_entry.clone());
    Ok(existing)
}

/// TOML counterpart of [`merge_json`].
pub fn merge_toml(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
    new_entry: &toml::Value,
) -> Result<toml::Value> {
    let mut existing = if config_path.exists() {
        read_toml(config_path)?
    } else {
        toml::Value::Table(toml::value::Table::new())
    };
    let root = existing
        .as_table_mut()
        .ok_or_else(|| anyhow!("`{}` root is not a TOML table", config_path.display()))?;
    let servers = root
        .entry(section_key.to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
    let servers_tbl = servers
        .as_table_mut()
        .ok_or_else(|| anyhow!("`[{section_key}]` is not a TOML table"))?;
    servers_tbl.insert(server_name.to_string(), new_entry.clone());
    Ok(existing)
}

/// Remove `section_key[server_name]` from a JSON config, leaving every
/// other key in place. The inverse of [`merge_json`].
pub fn strip_json_entry(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
) -> Result<JsonValue> {
    let mut existing = read_json(config_path)?;
    if let Some(obj) = existing.as_object_mut()
        && let Some(servers) = obj.get_mut(section_key).and_then(|v| v.as_object_mut())
    {
        servers.remove(server_name);
    }
    Ok(existing)
}

/// TOML counterpart of [`strip_json_entry`].
pub fn strip_toml_entry(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
) -> Result<toml::Value> {
    let mut existing = read_toml(config_path)?;
    if let Some(root) = existing.as_table_mut()
        && let Some(servers) = root.get_mut(section_key).and_then(|v| v.as_table_mut())
    {
        servers.remove(server_name);
    }
    Ok(existing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_json_preserves_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "theme": "dark", "mcpServers": { "other": { "command": "x" } } }"#,
        )
        .unwrap();
        let merged =
            merge_json(&path, "mcpServers", "vibevm", &json!({ "command": "vibe" })).unwrap();
        // Our entry landed...
        assert_eq!(merged["mcpServers"]["vibevm"]["command"], "vibe");
        // ...and the operator's other server + top-level key survived.
        assert_eq!(merged["mcpServers"]["other"]["command"], "x");
        assert_eq!(merged["theme"], "dark");
    }

    #[test]
    fn strip_json_entry_removes_only_ours() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "mcpServers": { "vibevm": { "command": "vibe" }, "other": { "command": "x" } } }"#,
        )
        .unwrap();
        let stripped = strip_json_entry(&path, "mcpServers", "vibevm").unwrap();
        assert!(stripped["mcpServers"].get("vibevm").is_none());
        assert_eq!(stripped["mcpServers"]["other"]["command"], "x");
    }

    #[test]
    fn merge_toml_preserves_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[mcp_servers.other]\ncommand = \"x\"\n").unwrap();
        let mut entry = toml::value::Table::new();
        entry.insert("command".into(), toml::Value::String("vibe".into()));
        let merged =
            merge_toml(&path, "mcp_servers", "vibevm", &toml::Value::Table(entry)).unwrap();
        let servers = merged["mcp_servers"].as_table().unwrap();
        assert!(servers.contains_key("vibevm"));
        assert!(servers.contains_key("other"));
    }
}
