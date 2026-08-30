//! The OpenCode MCP translation — §6.3.0.8's "documented different
//! adapter", as one pure function over the already-validated canonical
//! `mcpServers` map.
//!
//! **The target shape.** OpenCode 1.17's configuration declares servers
//! under a root `mcp` member. A local server carries `type = "local"`, a
//! `command` ARGV ARRAY, `enabled`, and an optional `environment` map; a
//! remote one carries `type = "remote"`, a `url`, `enabled`, and optional
//! `headers`. That is the whole vocabulary this cell emits, and §6.3.0.4
//! fixes the fragment's root to `mcp` and nothing else — an adapter that
//! wrote a second root member would be merging a document the deploy
//! adapter has to merge into, which is the one thing a projection may not
//! decide for it.
//!
//! **The two shapes are not interchangeable, so neither is their config.**
//! The canonical portable v1 entry admits `args`/`env` beside a `command`
//! and `headers` beside a `url`; OpenCode's local and remote servers do
//! not each take all four. A local `headers`, a remote `args` and a remote
//! `env` therefore refuse with the capability report §6.2 demands, rather
//! than being dropped into an output that would look complete.
//!
//! **Nothing is substituted.** `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` are
//! carried through byte-for-value: §6.2 keeps their "specified single-pass
//! meaning", and the pass that gives them a value is the DEPLOY adapter's,
//! which is the only place that knows a home. A projection that expanded
//! them would bake one machine into a reproducible artifact.
//!
//! **Every emitted object is key-sorted.** §6.3.0.4 requires deterministic
//! server-key ordering; this cell applies the same rule to every map it
//! writes, so one canonical document always renders to one byte string
//! regardless of how the source spelled its keys.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use serde_json::{Map, Value};

use super::client::ProjectionClient;
use super::error::ClientProjectionError;
use crate::mechanism::MechanismError;
use crate::mechanism::error::preview;
use crate::mechanism::plugin::manifest::McpServers;

/// The root member the fragment carries — and, by §6.3.0.4, its only one.
const ROOT_MEMBER: &str = "mcp";

/// The two transports OpenCode distinguishes.
const LOCAL: &str = "local";
const REMOTE: &str = "remote";

/// Render the canonical servers as one OpenCode configuration fragment.
///
/// Pure: it reads nothing, writes nothing, and depends only on the map it
/// is handed — which `validate_mcp_manifest` already judged, so this cell
/// never re-decides whether the source is well formed, only whether
/// OpenCode can express it.
pub(crate) fn render(target: &str, servers: &McpServers) -> Result<Vec<u8>, MechanismError> {
    let mut projected = Map::new();
    // §6.3.0.4's "deterministic server-key ordering": the emitted order is
    // sorted, never the order the source file happened to use.
    let mut names: Vec<&String> = servers.keys().collect();
    names.sort();
    for name in names {
        let Some(entry) = servers.get(name).and_then(Value::as_object) else {
            // The validated document proved every entry is an object; this
            // arm keeps the law a refusal rather than a silent skip.
            return Err(unrepresentable(
                target,
                &format!("mcpServers.{}", preview(name)),
                "it is not a server object".to_owned(),
            ));
        };
        projected.insert(name.clone(), server(target, name, entry)?);
    }
    let mut document = Map::new();
    document.insert(ROOT_MEMBER.to_owned(), Value::Object(projected));
    let mut bytes = match serde_json::to_vec_pretty(&Value::Object(document)) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(unrepresentable(
                target,
                ROOT_MEMBER,
                format!("the fragment could not be encoded: {error}"),
            ));
        }
    };
    bytes.push(b'\n');
    Ok(bytes)
}

/// One canonical server entry as OpenCode declares it.
fn server(target: &str, name: &str, entry: &Map<String, Value>) -> Result<Value, MechanismError> {
    let member = |leaf: &str| format!("mcpServers.{}.{leaf}", preview(name));
    let mut projected = Map::new();
    if let Some(command) = entry.get("command") {
        // Local: the canonical `command` followed by its optional `args`
        // becomes ONE argv array, because that is the member OpenCode
        // spells a local server's program with.
        if entry.contains_key("headers") {
            return Err(unrepresentable(
                target,
                &member("headers"),
                "an OpenCode local server runs a program and speaks no HTTP, so it carries no \
                 headers; headers belong to a `url` server"
                    .to_owned(),
            ));
        }
        let mut argv = vec![command.clone()];
        if let Some(Value::Array(args)) = entry.get("args") {
            argv.extend(args.iter().cloned());
        }
        projected.insert("command".to_owned(), Value::Array(argv));
        projected.insert("enabled".to_owned(), Value::Bool(true));
        if let Some(env) = entry.get("env") {
            projected.insert("environment".to_owned(), sorted(env));
        }
        projected.insert("type".to_owned(), Value::String(LOCAL.to_owned()));
        return Ok(Value::Object(projected));
    }
    // Remote: the canonical `url` and its optional `headers`, and neither
    // of the two members a program would need.
    for (leaf, reason) in [
        (
            "args",
            "an OpenCode remote server is reached over its URL, not spawned, so it takes no \
             argument vector",
        ),
        (
            "env",
            "an OpenCode remote server runs in nobody's process here, so it takes no environment",
        ),
    ] {
        if entry.contains_key(leaf) {
            return Err(unrepresentable(target, &member(leaf), reason.to_owned()));
        }
    }
    let Some(url) = entry.get("url") else {
        return Err(unrepresentable(
            target,
            &member("url"),
            "it declares neither `command` nor `url`".to_owned(),
        ));
    };
    projected.insert("enabled".to_owned(), Value::Bool(true));
    if let Some(headers) = entry.get("headers") {
        projected.insert("headers".to_owned(), sorted(headers));
    }
    projected.insert("type".to_owned(), Value::String(REMOTE.to_owned()));
    projected.insert("url".to_owned(), url.clone());
    Ok(Value::Object(projected))
}

/// One JSON value with every object key it carries in sorted order.
///
/// The VALUES are untouched — this is a canonical rendering of the same
/// document, which is what keeps `${PLUGIN_ROOT}` and `${PLUGIN_DATA}`
/// byte-for-value while making the bytes reproducible.
fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut canonical = Map::new();
            for key in keys {
                if let Some(inner) = map.get(key) {
                    canonical.insert(key.clone(), sorted(inner));
                }
            }
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        scalar => scalar.clone(),
    }
}

/// One OpenCode capability refusal.
fn unrepresentable(target: &str, member: &str, reason: String) -> MechanismError {
    ClientProjectionError::Unrepresentable {
        target: target.to_owned(),
        client: ProjectionClient::OpenCode.as_str(),
        member: preview(member),
        reason,
    }
    .into()
}
