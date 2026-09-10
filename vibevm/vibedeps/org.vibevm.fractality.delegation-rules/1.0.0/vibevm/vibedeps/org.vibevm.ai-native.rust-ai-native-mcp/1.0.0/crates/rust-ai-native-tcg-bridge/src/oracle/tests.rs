//! Oracle-layer replay tests: op semantics over scripted transports —
//! version law, effective-text order, position conversion on
//! non-ASCII content, hover splitting. No rust-analyzer here.

use std::path::PathBuf;
use std::time::Instant;

use super::{RustOracle, split_hover};
use crate::TcgBridgeError;
use crate::client::{Capabilities, LspClient, Transport};
use crate::position::{OuterPosition, PositionEncoding};

/// Inbound frames on demand + a full outbound recording.
struct Scripted {
    inbound: std::collections::VecDeque<serde_json::Value>,
    outbound: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
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

fn caps_utf8() -> Capabilities {
    Capabilities {
        position_encoding: PositionEncoding::Utf8,
        pull_diagnostics: true,
        server_version: "test".to_string(),
    }
}

fn oracle_with(
    frames: Vec<serde_json::Value>,
) -> (
    RustOracle<Scripted>,
    std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
) {
    let outbound = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let transport = Scripted {
        inbound: frames.into(),
        outbound: outbound.clone(),
    };
    let oracle = RustOracle::from_parts(
        LspClient::new(transport),
        caps_utf8(),
        PathBuf::from("C:/proj"),
        true,
    );
    (oracle, outbound)
}

fn diagnostic_response(id: u64, items: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id,
        "result": {"kind": "full", "items": items}})
}

#[test]
fn validate_opens_v1_then_changes_v2_never_reuses() {
    let (mut oracle, outbound) = oracle_with(vec![
        diagnostic_response(1, serde_json::json!([])),
        diagnostic_response(2, serde_json::json!([])),
    ]);
    oracle
        .validate("src/lib.rs", Some("fn a() {}".to_string()))
        .expect("first");
    oracle
        .validate("src/lib.rs", Some("fn b() {}".to_string()))
        .expect("second");
    let sent = outbound
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let opens: Vec<_> = sent
        .iter()
        .filter(|f| f["method"] == "textDocument/didOpen")
        .collect();
    let changes: Vec<_> = sent
        .iter()
        .filter(|f| f["method"] == "textDocument/didChange")
        .collect();
    assert_eq!(opens.len(), 1);
    assert_eq!(opens[0]["params"]["textDocument"]["version"], 1);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["params"]["textDocument"]["version"], 2);
    let uri = opens[0]["params"]["textDocument"]["uri"]
        .as_str()
        .expect("uri");
    assert!(uri.starts_with("file:///C:/proj/"), "{uri}");
    assert!(
        !uri.contains("\\\\?\\"),
        "verbatim leaked into a URI: {uri}"
    );
}

#[test]
fn validate_converts_positions_against_cyrillic_text() {
    // Diagnostic at utf-8 column 12 on a line whose prefix is
    // Cyrillic: "пример x" → 'п','р','и','м','е','р' are 2 bytes each.
    let (mut oracle, _outbound) = oracle_with(vec![diagnostic_response(
        1,
        serde_json::json!([{
            "range": {"start": {"line": 0, "character": 12},
                       "end": {"line": 0, "character": 13}},
            "severity": 1,
            "code": "E0308",
            "message": "mismatched types\nexpected i32",
        }]),
    )]);
    let out = oracle
        .validate("src/lib.rs", Some("пример x = 1;".to_string()))
        .expect("validated");
    assert_eq!(out.diagnostics.len(), 1);
    let d = &out.diagnostics[0];
    assert_eq!(d.code, "E0308");
    assert_eq!(d.category, "error");
    assert_eq!(d.line, 1);
    // 6 two-byte chars = utf-8 column 12 → outer character 6.
    assert_eq!(d.character, 6);
    assert_eq!(d.message, "mismatched types", "first line only");
    assert!(!out.degraded);
}

#[test]
fn effective_text_prefers_overlay_then_open_doc() {
    let (mut oracle, _outbound) = oracle_with(vec![
        diagnostic_response(1, serde_json::json!([])),
        diagnostic_response(2, serde_json::json!([])),
    ]);
    oracle
        .validate("src/x.rs", Some("fn one() {}".to_string()))
        .expect("overlay in");
    // No inline content now: the OPEN DOCUMENT's text serves — no
    // disk read of a file that does not exist.
    let out = oracle.validate("src/x.rs", None).expect("open doc serves");
    assert!(out.diagnostics.is_empty());
}

#[test]
fn missing_file_without_overlay_is_a_protocol_error() {
    let (mut oracle, _outbound) = oracle_with(vec![]);
    let err = oracle
        .validate("src/nope.rs", None)
        .expect_err("nothing to read");
    assert_eq!(err.wire_kind(), "protocol");
}

#[test]
fn update_null_closes_and_falls_back() {
    let (mut oracle, outbound) = oracle_with(vec![]);
    oracle
        .update("src/x.rs", Some("fn a() {}".to_string()))
        .expect("open");
    let v = oracle.update("src/x.rs", None).expect("close");
    assert_eq!(v, 0);
    let sent = outbound
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(
        sent.iter().any(|f| f["method"] == "textDocument/didClose"),
        "didClose sent"
    );
}

#[test]
fn completion_position_converts_through_the_line() {
    let (mut oracle, outbound) = oracle_with(vec![serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "result": {"items": [
            {"label": "greet", "kind": 3, "detail": "fn(&str) -> String"},
        ]},
    })]);
    let entries = oracle
        .complete(
            "src/x.rs",
            OuterPosition {
                line: 1,
                character: 11,
            },
            Some("let x = gre".to_string()),
        )
        .expect("completed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "greet");
    assert_eq!(entries[0].type_text.as_deref(), Some("fn(&str) -> String"));
    let sent = outbound
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let req = sent
        .iter()
        .find(|f| f["method"] == "textDocument/completion")
        .expect("request sent");
    assert_eq!(req["params"]["position"]["line"], 0);
    assert_eq!(req["params"]["position"]["character"], 11);
}

#[test]
fn hover_splits_display_from_docs() {
    let (display, docs) =
        split_hover("```rust\nfn greet(name: &str) -> String\n```\n\nSays hello.\n");
    assert_eq!(display, "fn greet(name: &str) -> String");
    assert_eq!(docs, "Says hello.");
    let (bare, empty) = split_hover("plain text only");
    assert_eq!(bare, "plain text only");
    assert!(empty.is_empty());
}
