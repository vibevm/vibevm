//! Replay tests: the whole client layer against scripted transports —
//! no rust-analyzer anywhere near the unit suite (ORACLE-RUST's replay
//! posture).

specmark::scope!("spec://rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#session");

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{Capabilities, LspClient, Transport};
use crate::TcgBridgeError;
use crate::position::PositionEncoding;

/// A scripted transport: canned inbound frames, recorded outbound.
pub struct Scripted {
    pub inbound: VecDeque<serde_json::Value>,
    pub outbound: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl Scripted {
    pub fn new(frames: Vec<serde_json::Value>) -> Self {
        Self {
            inbound: frames.into(),
            outbound: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Transport for Scripted {
    fn send(&mut self, value: &serde_json::Value) -> Result<(), TcgBridgeError> {
        self.outbound
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(value.clone());
        Ok(())
    }

    fn recv(&mut self, _deadline: Instant) -> Result<Option<serde_json::Value>, TcgBridgeError> {
        Ok(self.inbound.pop_front())
    }
}

fn budget() -> Duration {
    Duration::from_millis(200)
}

#[test]
fn request_correlates_and_skips_interleaved_noise() {
    let script = vec![
        // a notification arrives first — must be absorbed, not matched
        serde_json::json!({"jsonrpc": "2.0", "method": "$/progress",
            "params": {"token": 1, "value": {"kind": "begin"}}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": {"answer": 42}}),
    ];
    let mut client = LspClient::new(Scripted::new(script));
    let out = client
        .request("x/op", serde_json::json!({}), budget())
        .expect("answered");
    assert_eq!(out["answer"], 42);
}

#[test]
fn server_requests_are_answered_with_the_config() {
    let script = vec![
        serde_json::json!({"jsonrpc": "2.0", "id": 77, "method": "workspace/configuration",
            "params": {"items": [{"section": "rust-analyzer"}, {"section": "rust-analyzer"}]}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": null}),
    ];
    let transport = Scripted::new(script);
    let outbound = transport.outbound.clone();
    let mut client = LspClient::new(transport);
    client
        .request("x/op", serde_json::json!({}), budget())
        .expect("answered");
    let sent = outbound
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let config_answer = sent
        .iter()
        .find(|f| f["id"] == 77)
        .expect("configuration answered");
    let items = config_answer["result"].as_array().expect("array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["diagnostics"]["experimental"]["enable"], true);
}

#[test]
fn a_server_error_response_is_a_protocol_error() {
    let script = vec![serde_json::json!({"jsonrpc": "2.0", "id": 1,
        "error": {"code": -32601, "message": "nope"}})];
    let mut client = LspClient::new(Scripted::new(script));
    let err = client
        .request("x/op", serde_json::json!({}), budget())
        .expect_err("refused");
    assert_eq!(err.wire_kind(), "protocol");
}

#[test]
fn server_cancelled_with_retrigger_is_resent_and_answered() {
    // LSP ServerCancelled + retriggerRequest: the client sends the
    // request AGAIN under the same deadline (live bench finding: the
    // diagnostics pull for a fresh overlay races r-a's own revision
    // bump and cancels nondeterministically).
    let script = vec![
        serde_json::json!({"jsonrpc": "2.0", "id": 1,
            "error": {"code": -32802, "message": "server cancelled the request",
                      "data": {"retriggerRequest": true}}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "result": {"answer": 7}}),
    ];
    let transport = Scripted::new(script);
    let outbound = transport.outbound.clone();
    let mut client = LspClient::new(transport);
    let out = client
        .request("x/op", serde_json::json!({"p": 1}), budget())
        .expect("retriggered and answered");
    assert_eq!(out["answer"], 7);
    let sent = outbound
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sends: Vec<_> = sent.iter().filter(|f| f["method"] == "x/op").collect();
    assert_eq!(sends.len(), 2, "the request was sent again");
    assert_eq!(sends[1]["params"]["p"], 1, "same params ride the retrigger");
    assert_eq!(sends[1]["id"], 2, "the retrigger carries a fresh id");
}

#[test]
fn server_cancelled_without_retrigger_stays_a_protocol_error() {
    let script = vec![serde_json::json!({"jsonrpc": "2.0", "id": 1,
        "error": {"code": -32802, "message": "server cancelled the request"}})];
    let mut client = LspClient::new(Scripted::new(script));
    let err = client
        .request("x/op", serde_json::json!({}), budget())
        .expect_err("refused");
    assert_eq!(err.wire_kind(), "protocol");
}

#[test]
fn eof_mid_request_is_oracle_crashed() {
    let mut client = LspClient::new(Scripted::new(vec![]));
    let err = client
        .request("x/op", serde_json::json!({}), budget())
        .expect_err("stream ended");
    assert_eq!(err.wire_kind(), "oracle-crashed");
}

#[test]
fn quiescence_via_server_status() {
    let script = vec![
        serde_json::json!({"jsonrpc": "2.0", "method": "experimental/serverStatus",
            "params": {"health": "ok", "quiescent": false}}),
        serde_json::json!({"jsonrpc": "2.0", "method": "experimental/serverStatus",
            "params": {"health": "ok", "quiescent": true}}),
    ];
    let mut client = LspClient::new(Scripted::new(script));
    assert!(client.wait_quiescent(budget()));
}

#[test]
fn progress_noise_never_counts_as_quiescence() {
    // The drain heuristic was falsified live: a fast token pair must
    // NOT satisfy the wait — only the serverStatus flag does.
    let script = vec![
        serde_json::json!({"jsonrpc": "2.0", "method": "$/progress",
            "params": {"token": "prime", "value": {"kind": "begin"}}}),
        serde_json::json!({"jsonrpc": "2.0", "method": "$/progress",
            "params": {"token": "prime", "value": {"kind": "end"}}}),
    ];
    let mut client = LspClient::new(Scripted::new(script));
    assert!(!client.wait_quiescent(Duration::from_millis(50)));
}

#[test]
fn quiescence_deadline_degrades_to_false() {
    let mut client = LspClient::new(Scripted::new(vec![]));
    assert!(!client.wait_quiescent(Duration::from_millis(30)));
}

#[test]
fn initialize_reads_the_granted_set() {
    let script = vec![serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": {
        "capabilities": {
            "positionEncoding": "utf-8",
            "diagnosticProvider": {"identifier": "rust-analyzer"},
        },
        "serverInfo": {"name": "rust-analyzer", "version": "1.93.1-test"},
    }})];
    let transport = Scripted::new(script);
    let outbound = transport.outbound.clone();
    let mut client = LspClient::new(transport);
    let caps: Capabilities = client
        .initialize("file:///C:/x", budget())
        .expect("handshake");
    assert_eq!(caps.position_encoding, PositionEncoding::Utf8);
    assert!(caps.pull_diagnostics);
    assert_eq!(caps.server_version, "1.93.1-test");
    let sent = outbound
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let init = &sent[0];
    assert_eq!(
        init["params"]["initializationOptions"]["diagnostics"]["experimental"]["enable"], true,
        "the config posture rides initializationOptions"
    );
    assert_eq!(sent[1]["method"], "initialized");
}
