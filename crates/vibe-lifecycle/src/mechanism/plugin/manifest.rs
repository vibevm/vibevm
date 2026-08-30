//! Local, structural validation of the two Agent Plugins 1.0 manifests.
//!
//! §6.2 asks for exactly this and nothing more: "validates the published
//! 1.0.0 schemas locally". Nothing here fetches a schema, contacts a
//! client or resolves a registry — this cell reads two JSON files that are
//! already inside the workspace and judges their SHAPE.
//!
//! **`plugin.json`, the members validated.** The document is an object.
//! `name` — required, a string in the plugin-name grammar (lowercase
//! alphanumerics with inner `-`, `_` or `.`, at most 64 characters);
//! `version` — required, a non-blank control-free string;
//! `description` — optional, non-blank when present; `author` — optional,
//! either a non-blank string or an object carrying a non-blank `name`.
//! Every other member is preserved and not judged: the published
//! vocabulary is not this engine's to close.
//!
//! **`mcp.json`, the members validated.** The document is an object whose
//! ONLY member is `mcpServers`, an object of server entries — §6.2:
//! "Portable v1 components are Agent Skills and MCP servers only …
//! not invented portable fields", so an unknown top-level member refuses.
//! Each entry is an object with exactly one of `command` or `url` (both
//! strings), and optional `args` (array of strings), `env` and `headers`
//! (objects of strings); any other member refuses. Every string value's
//! `${…}` references must be `${PLUGIN_ROOT}` or `${PLUGIN_DATA}` — §6.2
//! keeps those two "their specified single-pass meaning", and a third
//! placeholder would be a substitution nobody defined.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::Path;

use serde_json::Value;

use crate::mechanism::MechanismError;
use crate::mechanism::contain::read_file_bounded;
use crate::mechanism::error::preview;

use super::shape::{MCP_MANIFEST, PLUGIN_MANIFEST};

/// The largest manifest this cell will read.
const MANIFEST_CAP: u64 = 1024 * 1024;

/// The two placeholders §6.2 defines, and the only ones admitted.
const PLACEHOLDERS: [&str; 2] = ["${PLUGIN_ROOT}", "${PLUGIN_DATA}"];

/// One canonical plugin's declared identity, as `plugin.json` states it.
///
/// Both members are validated strings rather than one, because §6.3's
/// projections bind "the parsed name/version" into their fingerprint and
/// evidence: a version this cell read and then discarded would have to be
/// re-parsed by every adapter, and three parsers of one member is three
/// answers waiting to disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIdentity {
    pub(crate) name: String,
    pub(crate) version: String,
}

/// Validate `plugin.json` and return the plugin's declared identity.
pub(crate) fn validate_plugin_manifest(
    target: &str,
    root: &Path,
) -> Result<PluginIdentity, MechanismError> {
    let document = read_json(target, root, PLUGIN_MANIFEST)?;
    let object = object(target, PLUGIN_MANIFEST, &document, "<document>")?;
    let name = required_string(target, PLUGIN_MANIFEST, object, "name")?;
    if !is_plugin_name(&name) {
        return Err(refuse(
            target,
            PLUGIN_MANIFEST,
            "name",
            format!(
                "`{}` is not a plugin name; use lowercase letters, digits and inner `-`, `_` or \
                 `.`, at most 64 characters",
                preview(&name)
            ),
        ));
    }
    let version = required_string(target, PLUGIN_MANIFEST, object, "version")?;
    if let Some(description) = object.get("description") {
        non_blank_string(target, PLUGIN_MANIFEST, "description", description)?;
    }
    match object.get("author") {
        None | Some(Value::String(_)) => {
            if let Some(author) = object.get("author") {
                non_blank_string(target, PLUGIN_MANIFEST, "author", author)?;
            }
        }
        Some(Value::Object(author)) => {
            let Some(inner) = author.get("name") else {
                return Err(refuse(
                    target,
                    PLUGIN_MANIFEST,
                    "author.name",
                    "an object author names itself".to_owned(),
                ));
            };
            non_blank_string(target, PLUGIN_MANIFEST, "author.name", inner)?;
        }
        Some(other) => {
            return Err(refuse(
                target,
                PLUGIN_MANIFEST,
                "author",
                format!("expected a string or an object, found {}", kind(other)),
            ));
        }
    }
    Ok(PluginIdentity { name, version })
}

/// The validated `mcpServers` map of one canonical `mcp.json`.
///
/// Returned rather than discarded so the §6.3 adapters translate the
/// document this cell already judged: a second parse for the translation
/// would be a second opinion about the same bytes, and the two could part
/// on a file that changed between them.
pub(crate) type McpServers = serde_json::Map<String, Value>;

/// Validate `mcp.json` and return its declared servers.
pub(crate) fn validate_mcp_manifest(
    target: &str,
    root: &Path,
) -> Result<McpServers, MechanismError> {
    let document = read_json(target, root, MCP_MANIFEST)?;
    let root_members = object(target, MCP_MANIFEST, &document, "<document>")?;
    for member in root_members.keys() {
        if member != "mcpServers" {
            return Err(refuse(
                target,
                MCP_MANIFEST,
                member,
                "the portable v1 MCP declaration carries `mcpServers` and nothing else".to_owned(),
            ));
        }
    }
    let declared = root_members.get("mcpServers").ok_or_else(|| {
        refuse(
            target,
            MCP_MANIFEST,
            "mcpServers",
            "required; a present `mcp.json` declares at least the member".to_owned(),
        )
    })?;
    let servers = object(target, MCP_MANIFEST, declared, "mcpServers")?;
    for (name, declared_entry) in servers {
        let member = format!("mcpServers.{}", preview(name));
        let entry = object(target, MCP_MANIFEST, declared_entry, &member)?;
        let transports = ["command", "url"]
            .into_iter()
            .filter(|key| entry.contains_key(*key))
            .count();
        if transports != 1 {
            return Err(refuse(
                target,
                MCP_MANIFEST,
                &member,
                "declares exactly one of `command` or `url`".to_owned(),
            ));
        }
        for (key, value) in entry {
            let inner = format!("{member}.{}", preview(key));
            let key: &str = key;
            match key {
                "command" | "url" => {
                    let text = non_blank_string(target, MCP_MANIFEST, &inner, value)?;
                    placeholders(target, MCP_MANIFEST, &inner, &text)?;
                }
                "args" => {
                    let Value::Array(items) = value else {
                        return Err(refuse(
                            target,
                            MCP_MANIFEST,
                            &inner,
                            format!("expected an array of strings, found {}", kind(value)),
                        ));
                    };
                    for item in items {
                        let text = non_blank_string(target, MCP_MANIFEST, &inner, item)?;
                        placeholders(target, MCP_MANIFEST, &inner, &text)?;
                    }
                }
                "env" | "headers" => {
                    let Value::Object(map) = value else {
                        return Err(refuse(
                            target,
                            MCP_MANIFEST,
                            &inner,
                            format!("expected an object of strings, found {}", kind(value)),
                        ));
                    };
                    for (entry_key, entry_value) in map {
                        let leaf = format!("{inner}.{}", preview(entry_key));
                        let text = non_blank_string(target, MCP_MANIFEST, &leaf, entry_value)?;
                        placeholders(target, MCP_MANIFEST, &leaf, &text)?;
                    }
                }
                _ => {
                    return Err(refuse(
                        target,
                        MCP_MANIFEST,
                        &inner,
                        "unknown member; a portable v1 server declares `command`/`url`, `args`, \
                         `env` and `headers`"
                            .to_owned(),
                    ));
                }
            }
        }
    }
    Ok(servers.clone())
}

/// Read and parse one manifest.
fn read_json(target: &str, root: &Path, file: &str) -> Result<Value, MechanismError> {
    let bytes = read_file_bounded(&root.join(file), MANIFEST_CAP).map_err(|fault| {
        refuse(
            target,
            file,
            "<document>",
            format!("it could not be read: {}", fault.reason()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        refuse(
            target,
            file,
            "<document>",
            format!("it is not JSON: {error}"),
        )
    })
}

/// One value as an object, or the refusal that says what it was.
fn object<'a>(
    target: &str,
    file: &str,
    value: &'a Value,
    member: &str,
) -> Result<&'a serde_json::Map<String, Value>, MechanismError> {
    value.as_object().ok_or_else(|| {
        refuse(
            target,
            file,
            member,
            format!("expected an object, found {}", kind(value)),
        )
    })
}

/// One required, non-blank string member.
fn required_string(
    target: &str,
    file: &str,
    object: &serde_json::Map<String, Value>,
    member: &str,
) -> Result<String, MechanismError> {
    let value = object
        .get(member)
        .ok_or_else(|| refuse(target, file, member, "required".to_owned()))?;
    non_blank_string(target, file, member, value)
}

/// One non-blank, control-free string value.
fn non_blank_string(
    target: &str,
    file: &str,
    member: &str,
    value: &Value,
) -> Result<String, MechanismError> {
    let Value::String(text) = value else {
        return Err(refuse(
            target,
            file,
            member,
            format!("expected a string, found {}", kind(value)),
        ));
    };
    if text.trim().is_empty() || text.chars().any(char::is_control) {
        return Err(refuse(
            target,
            file,
            member,
            "must be non-blank and free of control bytes".to_owned(),
        ));
    }
    Ok(text.clone())
}

/// Every `${…}` reference in one value must be one §6.2 defines.
fn placeholders(target: &str, file: &str, member: &str, text: &str) -> Result<(), MechanismError> {
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        let tail = &rest[start..];
        let Some(end) = tail.find('}') else {
            return Err(refuse(
                target,
                file,
                member,
                "an unterminated `${` reference has no single-pass meaning".to_owned(),
            ));
        };
        let reference = &tail[..=end];
        if !PLACEHOLDERS.contains(&reference) {
            return Err(refuse(
                target,
                file,
                member,
                format!(
                    "`{}` is not a defined placeholder; §6.2 defines `${{PLUGIN_ROOT}}` and \
                     `${{PLUGIN_DATA}}`, and visible `env`/headers are never a credential \
                     mechanism",
                    preview(reference)
                ),
            ));
        }
        rest = &tail[end + 1..];
    }
    Ok(())
}

/// The plugin-name grammar.
fn is_plugin_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    bytes.iter().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || *byte == b'-'
            || *byte == b'_'
            || *byte == b'.'
    })
}

/// The JSON kind a refusal names.
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// One manifest refusal.
fn refuse(target: &str, file: &str, member: &str, reason: String) -> MechanismError {
    MechanismError::PluginManifest {
        target: target.to_owned(),
        file: file.to_owned(),
        member: preview(member),
        reason,
    }
}
