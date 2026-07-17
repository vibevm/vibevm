//! Replay tests over a scripted transport — the whole client layer,
//! gopls-free. The scripts are recorded LSP shapes; every dispatch
//! path (auto-answer, parked response, notifications) is pinned here.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{LspClient, Transport};
use crate::TcgBridgeError;
use crate::position::PositionEncoding;

/// A scripted transport: outbound frames are logged; inbound frames
/// pop from a queue (None = EOF).
pub(crate) struct Script {
    pub sent: Vec<serde_json::Value>,
    pub inbound: VecDeque<serde_json::Value>,
}

impl Script {
    pub(crate) fn new(inbound: Vec<serde_json::Value>) -> Self {
        Self {
            sent: Vec::new(),
            inbound: inbound.into(),
        }
    }
}

impl Transport for Script {
    fn send(&mut self, value: &serde_json::Value) -> Result<(), TcgBridgeError> {
        self.sent.push(value.clone());
        Ok(())
    }
    fn recv(&mut self, _deadline: Instant) -> Result<Option<serde_json::Value>, TcgBridgeError> {
        Ok(self.inbound.pop_front())
    }
}

#[test]
fn initialize_reads_the_granted_set_and_sends_initialized() {
    let script = Script::new(vec![serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "result": {
            "capabilities": {
                "positionEncoding": "utf-8",
                "diagnosticProvider": { "interFileDependencies": true },
            },
            "serverInfo": { "name": "gopls", "version": "v0.23.0" },
        },
    })]);
    let mut client = LspClient::new(script);
    let caps = client
        .initialize("file:///c:/demo", Duration::from_secs(1))
        .expect("handshake");
    assert_eq!(caps.position_encoding, PositionEncoding::Utf8);
    assert!(caps.pull_diagnostics);
    assert_eq!(caps.server_version, "v0.23.0");
    let sent = &client.transport.sent;
    assert_eq!(sent[0]["method"], "initialize");
    assert_eq!(sent[1]["method"], "initialized");
}

#[test]
fn server_configuration_request_is_answered_per_item_mid_request() {
    let script = Script::new(vec![
        // The server interleaves its own request before answering ours.
        serde_json::json!({
            "jsonrpc": "2.0", "id": 42, "method": "workspace/configuration",
            "params": { "items": [{"section": "gopls"}, {"section": "gopls"}] },
        }),
        serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": { "ok": true } }),
    ]);
    let mut client = LspClient::new(script);
    let result = client
        .request("textDocument/hover", serde_json::json!({}), Duration::from_secs(1))
        .expect("request");
    assert_eq!(result["ok"], true);
    let answer = client
        .transport
        .sent
        .iter()
        .find(|f| f["id"] == 42)
        .expect("configuration answered");
    assert_eq!(answer["result"].as_array().map(Vec::len), Some(2));
}

#[test]
fn server_cancelled_with_retrigger_is_resent_and_answered() {
    let script = Script::new(vec![
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "error": { "code": -32802, "message": "content modified",
                       "data": { "retriggerRequest": true } },
        }),
        serde_json::json!({ "jsonrpc": "2.0", "id": 2, "result": { "items": [] } }),
    ]);
    let mut client = LspClient::new(script);
    let result = client
        .request(
            "textDocument/diagnostic",
            serde_json::json!({}),
            Duration::from_secs(1),
        )
        .expect("retriggered");
    assert!(result["items"].as_array().is_some_and(Vec::is_empty));
}

#[test]
fn readiness_is_the_progress_end_event() {
    let script = Script::new(vec![
        serde_json::json!({
            "jsonrpc": "2.0", "method": "$/progress",
            "params": { "token": "t", "value": { "kind": "begin", "title": "load" } },
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "method": "$/progress",
            "params": { "token": "t", "value": { "kind": "end" } },
        }),
    ]);
    let mut client = LspClient::new(script);
    assert!(client.wait_ready(Duration::from_secs(1)));
}

#[test]
fn readiness_deadline_degrades_instead_of_crashing() {
    let script = Script::new(vec![serde_json::json!({
        "jsonrpc": "2.0", "method": "$/progress",
        "params": { "token": "t", "value": { "kind": "begin" } },
    })]);
    let mut client = LspClient::new(script);
    assert!(!client.wait_ready(Duration::from_millis(10)));
}

#[test]
fn published_diagnostics_are_tracked_and_waited_for() {
    let script = Script::new(vec![serde_json::json!({
        "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///c:/demo/a.go",
            "diagnostics": [{ "message": "undeclared name: x" }],
        },
    })]);
    let mut client = LspClient::new(script);
    let diags = client
        .wait_published("file:///c:/demo/a.go", Duration::from_secs(1))
        .expect("published");
    assert_eq!(diags[0]["message"], "undeclared name: x");
}
