//! Oracle-surface replay tests: overlay version law, pull vs push
//! validate, and the outer-position conversion of diagnostics — all
//! gopls-free.

use std::path::PathBuf;

use super::GoOracle;
use crate::client::tests::Script;
use crate::client::{Capabilities, LspClient};
use crate::position::PositionEncoding;

fn caps(pull: bool) -> Capabilities {
    Capabilities {
        position_encoding: PositionEncoding::Utf8,
        pull_diagnostics: pull,
        server_version: "test".into(),
    }
}

fn oracle_with(pull: bool, inbound: Vec<serde_json::Value>) -> GoOracle<Script> {
    GoOracle::from_parts(
        LspClient::new(Script::new(inbound)),
        caps(pull),
        PathBuf::from("/demo"),
        true,
    )
}

#[test]
fn overlay_versions_are_monotonic_and_close_resets() {
    let mut oracle = oracle_with(true, vec![]);
    assert_eq!(oracle.update("a.go", Some("package a".into())).expect("v1"), 1);
    assert_eq!(oracle.update("a.go", Some("package a2".into())).expect("v2"), 2);
    assert_eq!(oracle.update("a.go", None).expect("close"), 0);
    // Re-open starts a fresh document at v1 (didOpen again).
    assert_eq!(oracle.update("a.go", Some("package a3".into())).expect("v1 again"), 1);
}

#[test]
fn validate_uses_the_pull_channel_when_granted() {
    let mut oracle = oracle_with(
        true,
        vec![serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "result": { "kind": "full", "items": [{
                "range": { "start": { "line": 1, "character": 4 } },
                "severity": 1,
                "code": "UndeclaredName",
                "message": "undeclared name: hell\nsecond line",
            }] },
        })],
    );
    let outcome = oracle
        .validate("a.go", Some("package a\nfunc F() { hell() }\n".into()))
        .expect("validate");
    assert_eq!(outcome.diagnostics.len(), 1);
    let d = &outcome.diagnostics[0];
    assert_eq!(d.code, "UndeclaredName");
    assert_eq!(d.category, "error");
    assert_eq!((d.line, d.character), (2, 4));
    assert_eq!(d.message, "undeclared name: hell");
    assert!(!outcome.degraded);
}

#[test]
fn validate_falls_back_to_the_push_channel_with_a_settle_wait() {
    // No pull grant: the diagnostics arrive as a notification AFTER
    // the didOpen — the settle wait must catch them.
    let mut oracle = oracle_with(
        false,
        vec![serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///demo/a.go",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 } },
                    "severity": 2,
                    "code": "unusedvariable",
                    "message": "x declared and not used",
                }],
            },
        })],
    );
    let outcome = oracle
        .validate("a.go", Some("package a\n".into()))
        .expect("validate");
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(outcome.diagnostics[0].category, "warning");
}

#[test]
fn hover_splits_fences_from_prose() {
    let raw = "```go\nfunc Hello(name string) string\n```\n\nHello greets.\n";
    let (display, docs) = super::split_hover(raw);
    assert_eq!(display, "func Hello(name string) string");
    assert_eq!(docs, "Hello greets.");
    let (plain, empty) = super::split_hover("just text");
    assert_eq!(plain, "just text");
    assert!(empty.is_empty());
}

#[test]
fn missing_overlay_and_disk_is_a_protocol_error() {
    let mut oracle = oracle_with(true, vec![]);
    let err = oracle
        .validate("definitely/not/here.go", None)
        .expect_err("no state");
    assert_eq!(err.wire_kind(), "protocol");
}
