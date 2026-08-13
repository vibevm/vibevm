//! The blocking serve loop (MCP-CORE §4): read one line, dispatch,
//! write one line, until the host closes stdin. Methods: `initialize`
//! (answers the §2 revision + serverInfo + tools capability),
//! `tools/list`, `tools/call`, `ping`; notifications are absorbed;
//! unknown methods answer method-not-found; malformed frames answer
//! parse-error. The loop NEVER exits on a bad frame — only on
//! end-of-input or a dead channel.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/MCP-CORE-v0.1#server");

use std::io::{BufRead, Write};

use serde_json::Value;

use crate::{
    JsonRpcError, JsonRpcMessage, JsonRpcResponse, McpCoreError, PROTOCOL_VERSION, ToolSet,
    parse_line,
};

/// The line transport seam: production is stdio; tests inject scripts.
pub trait Transport {
    /// `Ok(None)` is end-of-input — the host is done with this server.
    fn read_line(&mut self) -> Result<Option<String>, McpCoreError>;
    fn write_line(&mut self, line: &str) -> Result<(), McpCoreError>;
}

/// The production transport: locked stdin/stdout of this process.
///
/// ```no_run
/// let mut t = core_ai_native_mcp::StdioTransport::new();
/// let mut server = core_ai_native_mcp::Server::new("rust-ai-native", "0.7.0", core_ai_native_mcp::ToolSet::new());
/// server.run(&mut t).unwrap();
/// ```
#[derive(Default)]
pub struct StdioTransport;

impl StdioTransport {
    pub fn new() -> Self {
        StdioTransport
    }
}

impl Transport for StdioTransport {
    fn read_line(&mut self) -> Result<Option<String>, McpCoreError> {
        let mut line = String::new();
        let n = std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|source| McpCoreError::Io {
                op: "read".into(),
                source,
            })?;
        Ok(if n == 0 { None } else { Some(line) })
    }

    fn write_line(&mut self, line: &str) -> Result<(), McpCoreError> {
        let mut out = std::io::stdout().lock();
        writeln!(out, "{line}")
            .and_then(|()| out.flush())
            .map_err(|source| McpCoreError::Io {
                op: "write".into(),
                source,
            })
    }
}

/// The server: identity + a [`ToolSet`], driven over a [`Transport`].
///
/// ```
/// use core_ai_native_mcp::{Server, ToolSet};
///
/// let mut server = Server::new("rust-ai-native", "0.7.0", ToolSet::new());
/// let mut script = core_ai_native_mcp::testing::Scripted::new(vec![
///     r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.into(),
///     r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.into(),
/// ]);
/// server.run(&mut script).unwrap();
/// assert!(script.outbound[0].contains("2024-11-05"));
/// assert!(script.outbound[1].contains("\"tools\":[]"));
/// ```
pub struct Server {
    name: String,
    version: String,
    tools: ToolSet,
}

impl Server {
    pub fn new(name: impl Into<String>, version: impl Into<String>, tools: ToolSet) -> Self {
        Server {
            name: name.into(),
            version: version.into(),
            tools,
        }
    }

    /// Drive the loop until end-of-input.
    pub fn run(&mut self, transport: &mut dyn Transport) -> Result<(), McpCoreError> {
        while let Some(raw) = transport.read_line()? {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let response = match parse_line(line) {
                Ok(JsonRpcMessage::Request(req)) => Some(self.answer(req)),
                Ok(JsonRpcMessage::Notification(_)) => None,
                Err(detail) => Some(JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcError::parse_error(&detail),
                )),
            };
            if let Some(resp) = response {
                transport.write_line(&resp.render()?)?;
            }
        }
        Ok(())
    }

    fn answer(&mut self, req: crate::JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => JsonRpcResponse::ok(
                req.id,
                serde_json::json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "serverInfo": { "name": self.name, "version": self.version },
                    "capabilities": { "tools": { "listChanged": false } },
                }),
            ),
            "ping" => JsonRpcResponse::ok(req.id, Value::Object(serde_json::Map::new())),
            "tools/list" => JsonRpcResponse::ok(
                req.id,
                serde_json::json!({ "tools": self.tools.descriptors() }),
            ),
            "tools/call" => {
                let params = req.params.unwrap_or(Value::Null);
                let Some(name) = params.get("name").and_then(Value::as_str) else {
                    return JsonRpcResponse::error(
                        req.id,
                        JsonRpcError::invalid_params("missing `name`"),
                    );
                };
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);
                match self.tools.run(name, &args) {
                    Some(output) => JsonRpcResponse::ok(req.id, output.into_result_value()),
                    None => JsonRpcResponse::error(
                        req.id,
                        JsonRpcError::method_not_found(&format!("tools/{name}")),
                    ),
                }
            }
            other => JsonRpcResponse::error(req.id, JsonRpcError::method_not_found(other)),
        }
    }
}

/// Test doubles for consumers of this crate (and its own suite): a
/// scripted transport with canned inbound lines and recorded outbound.
pub mod testing {
    use super::*;

    /// Canned inbound, recorded outbound — the replay posture: the whole
    /// loop tests without an agent host anywhere near the suite.
    pub struct Scripted {
        inbound: std::collections::VecDeque<String>,
        pub outbound: Vec<String>,
    }

    impl Scripted {
        pub fn new(lines: Vec<String>) -> Self {
            Scripted {
                inbound: lines.into(),
                outbound: Vec::new(),
            }
        }
    }

    impl Transport for Scripted {
        fn read_line(&mut self) -> Result<Option<String>, McpCoreError> {
            Ok(self.inbound.pop_front())
        }
        fn write_line(&mut self, line: &str) -> Result<(), McpCoreError> {
            self.outbound.push(line.to_string());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::Scripted;
    use super::*;
    use crate::{Tool, ToolDescriptor, ToolOutput};

    struct Echo;
    impl Tool for Echo {
        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: "echo".into(),
                description: "returns its `text` argument".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } },
                    "required": ["text"],
                }),
            }
        }
        fn run(&mut self, args: &Value) -> ToolOutput {
            match args.get("text").and_then(Value::as_str) {
                Some(t) => ToolOutput::ok(t),
                None => ToolOutput::failed("no `text` given"),
            }
        }
    }

    fn server() -> Server {
        let mut tools = ToolSet::new();
        tools.register(Box::new(Echo));
        Server::new("test-server", "0.0.0", tools)
    }

    fn run_script(lines: &[&str]) -> Vec<Value> {
        let mut t = Scripted::new(lines.iter().map(|s| s.to_string()).collect());
        server().run(&mut t).unwrap();
        t.outbound
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn handshake_list_call_ping_round_trip() {
        let out = run_script(&[
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"text":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
        ]);
        // The notification produced no frame: 4 answers for 5 lines.
        assert_eq!(out.len(), 4);
        assert_eq!(out[0]["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(out[0]["result"]["serverInfo"]["name"], "test-server");
        assert_eq!(out[1]["result"]["tools"][0]["name"], "echo");
        assert_eq!(
            out[1]["result"]["tools"][0]["inputSchema"]["required"][0],
            "text"
        );
        assert_eq!(out[2]["result"]["content"][0]["text"], "hi");
        assert_eq!(out[2]["result"]["isError"], false);
        assert_eq!(out[3]["id"], 3);
    }

    #[test]
    fn tool_level_failure_is_a_result_not_a_protocol_error() {
        let out = run_script(&[
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{}}}"#,
        ]);
        assert!(out[0].get("error").is_none(), "{:?}", out[0]);
        assert_eq!(out[0]["result"]["isError"], true);
    }

    #[test]
    fn unknown_tool_and_method_answer_not_found() {
        let out = run_script(&[
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ghost"}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"resources/list"}"#,
        ]);
        assert_eq!(out[0]["error"]["code"], -32601);
        assert_eq!(out[1]["error"]["code"], -32601);
    }

    #[test]
    fn malformed_and_blank_lines_never_kill_the_loop() {
        let out = run_script(&[
            "   ",
            "not json at all",
            r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#,
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["error"]["code"], -32700);
        assert_eq!(out[1]["id"], 9, "the loop survived to answer the ping");
    }

    #[test]
    fn missing_call_name_is_invalid_params() {
        let out = run_script(&[r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{}}"#]);
        assert_eq!(out[0]["error"]["code"], -32602);
    }
}
