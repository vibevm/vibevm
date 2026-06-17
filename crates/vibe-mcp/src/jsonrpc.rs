//! JSON-RPC 2.0 message shapes per <https://www.jsonrpc.org/specification>.
//!
//! MCP framing on stdio is line-delimited JSON-RPC 2.0; one request
//! per line, one response per line. We parse the input via
//! [`parse`] which decides whether the message is a request
//! (carries `id`) or a notification (no `id`).

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#server");

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use specmark::spec;
use thiserror::Error;

/// Wire form of a request (`id` present) or notification (`id` absent
/// or null).
///
/// ```
/// use vibe_mcp::jsonrpc::JsonRpcRequest;
/// let r: JsonRpcRequest =
///     serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).unwrap();
/// assert_eq!(r.method, "ping");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    /// `id` is present on requests. May be a string or a number; we
    /// preserve it as a `Value` so we can echo it verbatim in the
    /// response.
    pub id: Value,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Notifications carry no `id` — the JSON-RPC spec disallows
/// responses to them. vibe-mcp slice 1 doesn't emit notifications;
/// we accept inbound ones and silently ignore.
///
/// ```
/// use vibe_mcp::jsonrpc::JsonRpcNotification;
/// let n: JsonRpcNotification =
///     serde_json::from_str(r#"{"jsonrpc":"2.0","method":"bar"}"#).unwrap();
/// assert_eq!(n.method, "bar");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A parsed inbound message: a request (carries `id`) or a notification
/// (no `id`). [`parse`] decides which.
///
/// ```
/// use vibe_mcp::jsonrpc::{parse, JsonRpcMessage};
/// let msg = parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).unwrap();
/// assert!(matches!(msg, JsonRpcMessage::Request(_)));
/// ```
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

/// JSON-RPC error object — `code`, `message`, optional `data`. Codes
/// follow the spec's reserved range (-32700 to -32603) plus any
/// implementation-defined codes outside it.
///
/// ```
/// use vibe_mcp::jsonrpc::JsonRpcError;
/// let e = JsonRpcError::method_not_found("tools/foo");
/// assert_eq!(e.code, -32601);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error(msg: &str) -> Self {
        JsonRpcError {
            code: -32700,
            message: format!("Parse error: {msg}"),
            data: None,
        }
    }

    pub fn invalid_request(msg: &str) -> Self {
        JsonRpcError {
            code: -32600,
            message: format!("Invalid request: {msg}"),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        JsonRpcError {
            code: -32601,
            message: format!("Method not found: {method}"),
            data: None,
        }
    }

    pub fn invalid_params(msg: &str) -> Self {
        JsonRpcError {
            code: -32602,
            message: format!("Invalid params: {msg}"),
            data: None,
        }
    }

    pub fn internal(msg: &str) -> Self {
        JsonRpcError {
            code: -32603,
            message: format!("Internal error: {msg}"),
            data: None,
        }
    }
}

/// Wire form of a response. Exactly one of `result` / `error` is
/// populated; the JSON-RPC spec disallows both at once.
///
/// ```
/// use vibe_mcp::jsonrpc::JsonRpcResponse;
/// let r = JsonRpcResponse::ok(serde_json::json!(1), serde_json::json!({ "ok": true }));
/// assert_eq!(r.jsonrpc, "2.0");
/// assert!(r.error.is_none());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, error: JsonRpcError) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// Why a line failed to parse as a JSON-RPC 2.0 message (PROP-015 §server).
///
/// ```
/// use vibe_mcp::jsonrpc::{parse, ParseError};
/// assert!(matches!(parse("{not json").unwrap_err(), ParseError::Json(_)));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#server")]
pub enum ParseError {
    #[error(
        "invalid JSON: {0} \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#server; \
          fix: send one well-formed JSON-RPC 2.0 object per line)"
    )]
    Json(#[from] serde_json::Error),

    #[error(
        "missing `jsonrpc` field \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#server; \
          fix: include `\"jsonrpc\": \"2.0\"` in the request)"
    )]
    MissingJsonrpc,

    #[error(
        "unsupported jsonrpc version `{0}` (expected `2.0`) \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#server; \
          fix: set `\"jsonrpc\": \"2.0\"`)"
    )]
    UnsupportedVersion(String),

    #[error(
        "missing `method` field \
         (violates spec://vibevm/modules/vibe-mcp/PROP-015#server; \
          fix: include a `method` string in the request)"
    )]
    MissingMethod,
}

pub fn parse(line: &str) -> Result<JsonRpcMessage, ParseError> {
    let v: Value = serde_json::from_str(line)?;
    let obj = v.as_object().ok_or_else(|| {
        ParseError::Json(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "JSON-RPC message must be an object",
        )))
    })?;

    let jsonrpc = obj
        .get("jsonrpc")
        .and_then(|v| v.as_str())
        .ok_or(ParseError::MissingJsonrpc)?;
    if jsonrpc != "2.0" {
        return Err(ParseError::UnsupportedVersion(jsonrpc.to_string()));
    }
    let method = obj
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or(ParseError::MissingMethod)?
        .to_string();
    let params = obj.get("params").cloned();

    if let Some(id) = obj.get("id")
        && !id.is_null()
    {
        return Ok(JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: jsonrpc.to_string(),
            id: id.clone(),
            method,
            params,
        }));
    }
    Ok(JsonRpcMessage::Notification(JsonRpcNotification {
        jsonrpc: jsonrpc.to_string(),
        method,
        params,
    }))
}

/// Helper for tools that emit structured records. Wraps a typed
/// payload as a serde JSON object.
pub fn record_object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k.to_string(), v);
    }
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_request_with_id() {
        let msg = parse(r#"{"jsonrpc":"2.0","id":7,"method":"foo","params":{"x":1}}"#).unwrap();
        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.id, json!(7));
                assert_eq!(req.method, "foo");
                assert_eq!(req.params, Some(json!({"x": 1})));
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn parses_notification_no_id() {
        let msg = parse(r#"{"jsonrpc":"2.0","method":"bar"}"#).unwrap();
        match msg {
            JsonRpcMessage::Notification(n) => assert_eq!(n.method, "bar"),
            _ => panic!("expected notification"),
        }
    }

    #[test]
    fn parses_notification_null_id() {
        let msg = parse(r#"{"jsonrpc":"2.0","id":null,"method":"baz"}"#).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn rejects_wrong_version() {
        let err = parse(r#"{"jsonrpc":"1.0","id":1,"method":"x"}"#).unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedVersion(_)));
    }

    #[test]
    fn rejects_missing_method() {
        let err = parse(r#"{"jsonrpc":"2.0","id":1}"#).unwrap_err();
        assert!(matches!(err, ParseError::MissingMethod));
    }

    #[test]
    fn rejects_invalid_json() {
        let err = parse("{not json").unwrap_err();
        assert!(matches!(err, ParseError::Json(_)));
    }

    #[test]
    fn ok_response_round_trips() {
        let resp = JsonRpcResponse::ok(json!(1), json!({"x": 42}));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains(r#""result":{"x":42}"#));
        assert!(!s.contains(r#""error""#));
    }

    #[test]
    fn error_response_round_trips() {
        let resp = JsonRpcResponse::error(json!(2), JsonRpcError::method_not_found("foo"));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains(r#""error""#));
        assert!(s.contains("Method not found"));
    }
}
