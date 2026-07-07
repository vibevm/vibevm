//! The wire cell: line-delimited JSON-RPC 2.0 message model (MCP-CORE
//! §2 — the MCP stdio form; one JSON object per line, protocol revision
//! "2024-11-05"). Parsing distinguishes requests from notifications by
//! the presence of `id`; responses are built, never parsed (a server
//! only answers).

specmark::scope!("spec://core-ai-native/mechanisms/MCP-CORE-v0.1#wire");

use serde::Serialize;
use serde_json::Value;

/// The protocol revision this transport answers `initialize` with —
/// the same revision the vibe-mcp product server has spoken against
/// every supported agent host since PROP-015.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// One inbound frame, already classified.
///
/// ```
/// use mcp_core::JsonRpcMessage;
///
/// let m = mcp_core::parse_line(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).unwrap();
/// assert!(matches!(m, JsonRpcMessage::Request(r) if r.method == "ping"));
/// let n = mcp_core::parse_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
///     .unwrap();
/// assert!(matches!(n, JsonRpcMessage::Notification(_)));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcMessage {
    /// Carries an `id` — must be answered.
    Request(JsonRpcRequest),
    /// No `id` — absorbed, never answered (MCP hosts send
    /// `notifications/initialized` and cancellation notices).
    Notification(JsonRpcNotification),
}

/// A request: `id` + `method` (+ optional `params`).
#[derive(Debug, Clone, PartialEq)]
pub struct JsonRpcRequest {
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

/// A notification: `method` (+ optional `params`), no `id`.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonRpcNotification {
    pub method: String,
    pub params: Option<Value>,
}

/// Parse one line into a classified message. Malformed JSON or a frame
/// without a `method` is an error the server answers with a JSON-RPC
/// `parse error` / `invalid request` (never a crash).
pub fn parse_line(line: &str) -> Result<JsonRpcMessage, String> {
    let v: Value =
        serde_json::from_str(line).map_err(|e| format!("malformed JSON-RPC frame: {e}"))?;
    let Some(method) = v.get("method").and_then(Value::as_str).map(str::to_string) else {
        return Err("frame carries no `method`".to_string());
    };
    let params = v.get("params").cloned();
    match v.get("id") {
        Some(id) if !id.is_null() => Ok(JsonRpcMessage::Request(JsonRpcRequest {
            id: id.clone(),
            method,
            params,
        })),
        _ => Ok(JsonRpcMessage::Notification(JsonRpcNotification {
            method,
            params,
        })),
    }
}

/// An outbound response frame. Exactly one of `result` / `error` is
/// set; serialisation renders one line, no trailing newline.
///
/// ```
/// use mcp_core::JsonRpcResponse;
///
/// let ok = JsonRpcResponse::ok(1.into(), serde_json::json!({"pong": true}));
/// assert!(ok.render().unwrap().contains("\"pong\":true"));
/// let err = JsonRpcResponse::error(2.into(), mcp_core::JsonRpcError::method_not_found("x/y"));
/// assert!(err.render().unwrap().contains("-32601"));
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, error: JsonRpcError) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(error),
        }
    }

    /// The single wire line for this response.
    pub fn render(&self) -> Result<String, crate::McpCoreError> {
        serde_json::to_string(self).map_err(|source| crate::McpCoreError::Serialize { source })
    }
}

/// A JSON-RPC error object with the standard codes.
///
/// ```
/// let e = mcp_core::JsonRpcError::invalid_params("missing `name`");
/// assert_eq!(e.code, -32602);
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

impl JsonRpcError {
    /// -32700 — the frame was not valid JSON-RPC.
    pub fn parse_error(detail: &str) -> Self {
        JsonRpcError {
            code: -32700,
            message: format!("parse error: {detail}"),
        }
    }

    /// -32601 — no such method (or no such tool).
    pub fn method_not_found(method: &str) -> Self {
        JsonRpcError {
            code: -32601,
            message: format!("method not found: {method}"),
        }
    }

    /// -32602 — the params shape is wrong.
    pub fn invalid_params(detail: &str) -> Self {
        JsonRpcError {
            code: -32602,
            message: format!("invalid params: {detail}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_requests_and_notifications_by_id() {
        let req = parse_line(r#"{"jsonrpc":"2.0","id":"a-1","method":"tools/list"}"#).unwrap();
        assert!(matches!(req, JsonRpcMessage::Request(r) if r.id == "a-1"));
        // A null id is a notification per the MCP stdio practice.
        let null_id = parse_line(r#"{"jsonrpc":"2.0","id":null,"method":"x"}"#).unwrap();
        assert!(matches!(null_id, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn malformed_frames_are_errors_not_panics() {
        assert!(parse_line("not json").is_err());
        assert!(parse_line(r#"{"jsonrpc":"2.0","id":1}"#).is_err()); // no method
    }

    #[test]
    fn responses_render_one_line() {
        let line = JsonRpcResponse::ok(7.into(), serde_json::json!({"a": [1, 2]}))
            .render()
            .unwrap();
        assert!(!line.contains('\n'));
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert_eq!(v["result"]["a"][1], 2);
        assert!(v.get("error").is_none(), "ok frames carry no error key");
    }
}
