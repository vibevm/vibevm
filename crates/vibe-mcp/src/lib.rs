//! `vibe-mcp` — Model Context Protocol server for vibevm.
//!
//! Spec: PROP-004 §5.1 + ROADMAP §M1.7. Targets the official protocol
//! at <https://modelcontextprotocol.io>.
//!
//! ## What ships
//!
//! - JSON-RPC 2.0 over line-delimited stdin/stdout (the MCP wire form
//!   for stdio servers).
//! - MCP message shapes — `initialize` handshake, `tools/list`,
//!   `tools/call` — modelled as plain Rust types serialised via serde.
//! - Four tools (see [`tools::default_tools`]): `query_package` (lockfile
//!   metadata), `read_subskill` and `materialise_subskill` (subskill
//!   content for an activated package), and `agentic_explain` (the
//!   PROP-018 in-project inference transport).
//!
//! ## Architecture
//!
//! The crate is **transport-agnostic** at the type level: [`Server`]
//! reads request strings via a [`Transport`] trait and writes
//! responses through it. The bundled [`StdioTransport`] is what
//! production uses; tests inject a [`MemoryTransport`] for
//! deterministic round-trip checks.
//!
//! Each tool is a [`tools::McpTool`] implementation (one `#[cell]` per
//! tool); [`Server::register_default_tools`] installs the set returned by
//! [`tools::default_tools`], and the dispatcher routes by each tool's
//! declared name.

#![forbid(unsafe_code)]
specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#server");

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::Lockfile;

pub mod agent_config;
pub mod agentic;
pub mod agents;
pub mod install;
pub mod jsonrpc;
pub mod pkgskill;
pub mod tools;
pub mod transport;

pub use agents::{Agent, ConfigFormat, ConfigPayload, SKILL_NAME, Scope, What, detect_agents};
pub use jsonrpc::{JsonRpcError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse};
pub use tools::{McpTool, default_tools};
pub use transport::{MemoryTransport, StdioTransport, Transport};

/// MCP protocol version this server speaks. The shipped MCP spec uses
/// date-stamped versions for the wire form; we report a fixed string
/// the client compares against. Kept as a `const` so any update is
/// one-line.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name + version — surfaced to clients during the
/// `initialize` handshake.
pub const SERVER_NAME: &str = "vibe-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Read-only snapshot of the project the server is exposing. Carried
/// per request so every tool call sees the same lockfile state during
/// its scope; reload happens at the start of each tool dispatch so
/// concurrent `vibe install` runs surface their changes on the next
/// invocation without a server restart.
///
/// ```
/// use vibe_mcp::ServerContext;
/// let ctx = ServerContext::new("/some/project");
/// assert!(ctx.project_root.ends_with("project"));
/// ```
pub struct ServerContext {
    /// Project root — the directory containing `vibe.toml` and
    /// `vibe.lock`.
    pub project_root: PathBuf,
}

impl ServerContext {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        ServerContext {
            project_root: project_root.into(),
        }
    }

    /// Load the project's lockfile fresh on every call. Returns an
    /// empty lockfile if `vibe.lock` does not exist yet — callers
    /// surface the empty-state through their normal output rather
    /// than aborting with `Lockfile not found`.
    pub fn load_lockfile(&self) -> Result<Lockfile, vibe_core::Error> {
        let path = self.project_root.join(Lockfile::FILENAME);
        if !path.exists() {
            return Ok(Lockfile::empty(SERVER_NAME, "0"));
        }
        Lockfile::read(&path)
    }
}

/// Metadata for a registered tool — surfaces in `tools/list` responses.
///
/// ```
/// use vibe_mcp::ToolDescriptor;
/// let d = ToolDescriptor {
///     name: "query_package".into(),
///     description: "Look up a package".into(),
///     input_schema: serde_json::json!({ "type": "object" }),
/// };
/// assert_eq!(d.name, "query_package");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    /// JSON Schema describing the tool's argument shape.
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Per-tool error surface. Distinct from `JsonRpcError` so the
/// dispatcher can decide whether to render the error as a tool-level
/// failure (`isError: true` in the result payload) or as a transport-
/// level JSON-RPC error.
///
/// ```
/// use vibe_mcp::ToolError;
/// let e = ToolError::NotFound("org.vibevm/wal".into());
/// assert!(e.to_string().contains("not found"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#errors")]
pub enum ToolError {
    #[error(
        "invalid arguments: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#tools; \
          fix: pass arguments matching the tool's inputSchema)"
    )]
    InvalidArguments(String),

    #[error(
        "not found: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#tools; \
          fix: install the package, or query one the lockfile carries)"
    )]
    NotFound(String),

    #[error(
        "io error: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#tools; \
          fix: check the project tree and cache are readable)"
    )]
    Io(#[from] std::io::Error),

    #[error(
        "vibe-core error: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#tools; \
          fix: act on the wrapped vibe-core error)"
    )]
    Core(#[from] vibe_core::Error),

    #[error(
        "internal error: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#tools; \
          fix: this is a server-side invariant break — report it)"
    )]
    Internal(String),
}

/// The MCP server itself. Construct with a `ServerContext` and a
/// `Transport`; call [`Server::run`] to drive the request/response
/// loop until the transport's input ends.
///
/// ```
/// use vibe_mcp::{Server, ServerContext, MemoryTransport};
/// // Construct over an in-memory transport (production uses stdio).
/// let _server = Server::new(MemoryTransport::with_input(""), ServerContext::new("."));
/// ```
pub struct Server<T: Transport> {
    transport: T,
    context: ServerContext,
    tools: BTreeMap<String, Box<dyn McpTool>>,
}

impl<T: Transport> Server<T> {
    pub fn new(transport: T, context: ServerContext) -> Self {
        let mut s = Server {
            transport,
            context,
            tools: BTreeMap::new(),
        };
        s.register_default_tools();
        s
    }

    /// Hot-add a tool behind the [`McpTool`] seam. Used by tests;
    /// production calls this once during construction via
    /// `register_default_tools`. Registering a tool whose
    /// `descriptor().name` already exists overwrites the previous entry.
    pub fn register_tool(&mut self, tool: Box<dyn McpTool>) {
        self.tools.insert(tool.descriptor().name, tool);
    }

    fn register_default_tools(&mut self) {
        for tool in tools::default_tools() {
            self.register_tool(tool);
        }
    }

    /// Drive the request/response loop. Reads lines from the
    /// transport, dispatches each as a JSON-RPC message, writes the
    /// response back. Returns when the transport reports end-of-input.
    pub fn run(&mut self) -> Result<(), ServerError> {
        while let Some(line) = self.transport.read_line()? {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let response = match jsonrpc::parse(line) {
                Ok(msg) => self.dispatch(msg),
                Err(e) => Some(JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcError::parse_error(&e.to_string()),
                )),
            };
            if let Some(resp) = response {
                let payload = serde_json::to_string(&resp).map_err(ServerError::Json)?;
                self.transport.write_line(&payload)?;
            }
        }
        Ok(())
    }

    fn dispatch(&self, msg: JsonRpcMessage) -> Option<JsonRpcResponse> {
        match msg {
            JsonRpcMessage::Request(req) => Some(self.dispatch_request(req)),
            // Notifications (request without `id`) carry no response.
            // We accept them for the protocol's compatibility but
            // nothing in slice 1 emits them.
            JsonRpcMessage::Notification(_) => None,
        }
    }

    fn dispatch_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req),
            "ping" => JsonRpcResponse::ok(req.id, Value::Object(serde_json::Map::new())),
            other => JsonRpcResponse::error(req.id, JsonRpcError::method_not_found(other)),
        }
    }

    fn handle_initialize(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let result = serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            },
            "capabilities": {
                "tools": { "listChanged": false }
            },
        });
        JsonRpcResponse::ok(req.id, result)
    }

    fn handle_tools_list(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let descriptors: Vec<ToolDescriptor> =
            self.tools.values().map(|t| t.descriptor()).collect();
        let result = serde_json::json!({
            "tools": descriptors,
        });
        JsonRpcResponse::ok(req.id, result)
    }

    fn handle_tools_call(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params = req.params.unwrap_or(Value::Null);
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                return JsonRpcResponse::error(
                    req.id,
                    JsonRpcError::invalid_params("missing `name`"),
                );
            }
        };
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);
        let tool = match self.tools.get(&name) {
            Some(t) => t,
            None => {
                return JsonRpcResponse::error(
                    req.id,
                    JsonRpcError::method_not_found(&format!("tools/{name}")),
                );
            }
        };
        match tool.run(&args, &self.context) {
            Ok(value) => {
                let text = match &value {
                    Value::String(s) => s.clone(),
                    other => {
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string())
                    }
                };
                let result = serde_json::json!({
                    "content": [
                        { "type": "text", "text": text }
                    ],
                    "isError": false,
                    "structuredContent": value,
                });
                JsonRpcResponse::ok(req.id, result)
            }
            Err(e) => {
                let result = serde_json::json!({
                    "content": [
                        { "type": "text", "text": e.to_string() }
                    ],
                    "isError": true,
                });
                JsonRpcResponse::ok(req.id, result)
            }
        }
    }
}

/// Convenience constructor for the bundled stdio transport.
impl Server<StdioTransport> {
    pub fn stdio(context: ServerContext) -> Self {
        Server::new(StdioTransport::new(), context)
    }
}

/// The server's transport / protocol failure surface — distinct from
/// [`ToolError`] (per-tool, rendered in-band) and [`JsonRpcError`]
/// (per-request). Surfaces only from [`Server::run`].
///
/// ```
/// use vibe_mcp::ServerError;
/// let e: ServerError = std::io::Error::other("pipe closed").into();
/// assert!(e.to_string().contains("transport error"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#errors")]
pub enum ServerError {
    #[error(
        "transport error: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#server; \
          fix: check the stdio transport is connected)"
    )]
    Transport(#[from] std::io::Error),

    #[error(
        "json error: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#server; \
          fix: send well-formed JSON-RPC 2.0 messages)"
    )]
    Json(#[from] serde_json::Error),
}

/// Wire a one-shot request through an in-memory transport and return the
/// raw response line — the canonical way to drive the server in tests.
///
/// ```
/// use vibe_mcp::{dispatch_one, ServerContext};
/// let resp = dispatch_one(
///     ServerContext::new("."),
///     r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
/// )
/// .unwrap();
/// assert!(resp.contains("\"id\":1"));
/// ```
pub fn dispatch_one(context: ServerContext, request_line: &str) -> Result<String, ServerError> {
    let transport = MemoryTransport::with_input(request_line.to_string() + "\n");
    let mut server = Server::new(transport, context);
    server.run()?;
    let output = server.transport.take_output();
    Ok(output.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_project() -> (tempfile::TempDir, ServerContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            r#"[project]
name = "test"
version = "0.0.1"
"#,
        )
        .unwrap();
        let ctx = ServerContext::new(dir.path());
        (dir, ctx)
    }

    #[test]
    fn initialize_returns_protocol_version_and_server_info() {
        let (_dir, ctx) = empty_project();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        })
        .to_string();
        let response_line = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(v["result"]["serverInfo"]["name"], SERVER_NAME);
    }

    #[test]
    fn tools_list_returns_registered_tools() {
        let (_dir, ctx) = empty_project();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })
        .to_string();
        let response_line = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&response_line).unwrap();
        let tools = v["result"]["tools"].as_array().expect("tools array");
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"query_package"), "got {:?}", names);
        assert!(names.contains(&"read_subskill"), "got {:?}", names);
    }

    #[test]
    fn unknown_method_returns_jsonrpc_error() {
        let (_dir, ctx) = empty_project();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "no_such_method",
        })
        .to_string();
        let response_line = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&response_line).unwrap();
        assert!(v["error"].is_object(), "expected error; got {v}");
        assert_eq!(v["error"]["code"], -32601);
    }

    #[test]
    fn tools_call_unknown_tool_returns_method_not_found() {
        let (_dir, ctx) = empty_project();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": { "name": "made_up_tool", "arguments": {} }
        })
        .to_string();
        let response_line = dispatch_one(ctx, &req).unwrap();
        let v: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[test]
    fn parse_error_returns_negative_32700() {
        let (_dir, ctx) = empty_project();
        let response_line = dispatch_one(ctx, "{not json").unwrap();
        let v: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(v["error"]["code"], -32700);
    }
}
