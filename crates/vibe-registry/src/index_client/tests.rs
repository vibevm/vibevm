//! Unit tests for the index client's construction, its
//! authorization-attachment decision and its Debug redaction.
//!
//! Split out of `mod.rs` on 2026-08-06 for the file-length budget:
//! the module crossed 600 lines after `cargo fmt` when the
//! attachment truth table gained a test. The budget decides where
//! code sits, never what a public type looks like — nothing here
//! changed shape to fit, it moved.

use super::*;

#[test]
fn registry_env_suffix_uppercases() {
    assert_eq!(registry_env_suffix("vibespecs"), "VIBESPECS");
    assert_eq!(
        registry_env_suffix("vibespecs-gitverse"),
        "VIBESPECS_GITVERSE"
    );
}

#[test]
fn at_strips_trailing_slash_and_defaults_to_no_auth() {
    let c = IndexClient::at("https://example.com/foo/");
    assert_eq!(c.file_base(), "https://example.com/foo");
    assert_eq!(c.server_base(), "https://example.com/foo");
    assert!(matches!(c.auth, IndexAuth::None));
}

#[test]
fn at_with_auth_carries_the_plan() {
    let c = IndexClient::at_with_auth(
        "https://example.com",
        IndexAuth::Bearer(BearerToken::new("sekret".into())),
    );
    assert!(matches!(c.auth, IndexAuth::Bearer(_)));
}

#[test]
fn index_client_debug_redacts_bearer_token() {
    // The client derives Debug; its `auth` field's Debug must not
    // leak the secret, or the derived impl would print it.
    let c = IndexClient::at_with_auth(
        "https://example.com",
        IndexAuth::Bearer(BearerToken::new("hunter2-supersecret".into())),
    );
    let rendered = format!("{c:?}");
    assert!(
        rendered.contains("<redacted>"),
        "Debug should mark the token redacted: {rendered}"
    );
    assert!(
        !rendered.contains("hunter2"),
        "Debug must not leak the token: {rendered}"
    );
}

/// The whole truth table of the attachment decision, including the
/// row nothing else covers: a bearer plan over an `https://` base
/// DOES attach.
///
/// Why this test rather than an end-to-end one. The mock servers this
/// crate's integration tests spawn are plain HTTP, so after the
/// scheme gate moved into the attachment step no integration test can
/// exercise the positive case at all — one arrived asserting the
/// header goes out, and the gate correctly suppressed it. The
/// alternative is a TLS fixture, which would test rustls rather than
/// this decision. So the composition is tested where it lives: the
/// header VALUE is proven by `auth::tests::header_map_attaches_bearer_authorization`,
/// the PLAN by `auth::tests::plan_*`, and the join of the two by this.
#[test]
fn authorization_attaches_only_for_a_bearer_plan_over_https() {
    let bearer = || IndexAuth::Bearer(BearerToken::new("sekret".into()));
    assert!(
        attaches_authorization("https://example.com", &bearer()),
        "a bearer plan over https is the ONLY case that attaches — \
         if this row ever goes false, private-index reads stop working \
         and every other assertion in this file still passes"
    );
    assert!(
        !attaches_authorization("http://example.com", &bearer()),
        "plaintext never carries the token, whatever the plan says"
    );
    assert!(!attaches_authorization(
        "https://example.com",
        &IndexAuth::None
    ));
    assert!(!attaches_authorization(
        "https://example.com",
        &IndexAuth::HttpIncapable("ssh")
    ));
}
