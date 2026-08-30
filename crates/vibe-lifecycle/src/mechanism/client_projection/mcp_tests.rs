//! §6.3.0.4 and §6.3.0.8's OpenCode MCP translation, value by value.
//!
//! The canonical portable v1 entry and OpenCode's configuration are two
//! different vocabularies over one idea, and every clause of the mapping is
//! measured here rather than described: the argv fold, the `env` →
//! `environment` rename, the two fixed members, the deterministic key
//! order, and the three fields whose absence from OpenCode's shape is a
//! capability refusal instead of a silent drop.

use serde_json::Value;
use specmark::verifies;

use super::client::OPENCODE_CONFIG;
use super::error::ClientProjectionError;
use super::support::*;
use crate::mechanism::package::support::{temp, write};

/// The projected configuration document of one OpenCode run.
fn projected(root: &std::path::Path, id: &str) -> Value {
    let text = staged(root, &format!("target/vibe-package/{id}/{OPENCODE_CONFIG}"));
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("the projected fragment parses: {error}\n{text}"),
    }
}

/// One server entry of that document.
fn server<'a>(document: &'a Value, name: &str) -> &'a Value {
    match document.get("mcp").and_then(|mcp| mcp.get(name)) {
        Some(entry) => entry,
        None => panic!("`mcp.{name}` is declared: {document}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_local_server_becomes_one_argv_array_with_its_environment() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    project(root, "local", "package:opencode-plugin", &["mcp"]);

    let document = projected(root, "local");
    let alpha = server(&document, "alpha");
    assert_eq!(alpha.get("type").and_then(Value::as_str), Some("local"));
    assert_eq!(alpha.get("enabled").and_then(Value::as_bool), Some(true));
    assert_eq!(
        alpha.get("command"),
        Some(&Value::Array(vec![
            Value::String("${PLUGIN_ROOT}/bin/demo".to_owned()),
            Value::String("--data".to_owned()),
            Value::String("${PLUGIN_DATA}".to_owned()),
        ])),
        "the canonical `command` leads its `args` in one argv array",
    );
    let environment = alpha.get("environment").and_then(Value::as_object);
    let environment = match environment {
        Some(map) => map,
        None => panic!("`env` becomes `environment`: {alpha}"),
    };
    assert_eq!(environment.len(), 2, "no declared variable is dropped");
    assert_eq!(
        environment.get("DEMO_MODE").and_then(Value::as_str),
        Some("on"),
    );
    assert_eq!(environment.get("AAA").and_then(Value::as_str), Some("1"));
    assert!(alpha.get("env").is_none(), "the canonical spelling is gone");
    assert!(alpha.get("url").is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_remote_server_keeps_its_url_and_every_header() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    project(root, "remote", "package:opencode-plugin", &["mcp"]);

    let document = projected(root, "remote");
    let zeta = server(&document, "zeta");
    assert_eq!(zeta.get("type").and_then(Value::as_str), Some("remote"));
    assert_eq!(zeta.get("enabled").and_then(Value::as_bool), Some(true));
    assert_eq!(
        zeta.get("url").and_then(Value::as_str),
        Some("https://example.test/mcp"),
    );
    let headers = match zeta.get("headers").and_then(Value::as_object) {
        Some(map) => map,
        None => panic!("a remote server keeps its headers: {zeta}"),
    };
    assert_eq!(headers.len(), 2, "no declared header is dropped");
    assert_eq!(headers.get("X-Trace").and_then(Value::as_str), Some("on"));
    assert_eq!(
        headers.get("Authorization").and_then(Value::as_str),
        Some("Bearer ${PLUGIN_DATA}"),
    );
    assert!(zeta.get("command").is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_root_carries_only_mcp_and_the_server_keys_are_sorted() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    project(root, "ordered", "package:opencode-plugin", &["mcp"]);

    let text = staged(
        root,
        &format!("target/vibe-package/ordered/{OPENCODE_CONFIG}"),
    );
    let document: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("the fragment parses: {error}"),
    };
    let members = match document.as_object() {
        Some(map) => map,
        None => panic!("the fragment is an object: {text}"),
    };
    assert_eq!(
        members.keys().collect::<Vec<_>>(),
        vec!["mcp"],
        "§6.3.0.4 fixes the fragment's root to `mcp` and nothing else",
    );
    // The source declares `zeta` first and `alpha` second; the projection
    // is sorted, so its bytes do not depend on how the author typed it.
    let alpha = text.find("\"alpha\"").unwrap_or(usize::MAX);
    let zeta = text.find("\"zeta\"").unwrap_or(0);
    assert!(
        alpha < zeta,
        "server keys are emitted in sorted order: {text}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_local_server_declaring_headers_refuses_as_unsupported() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    write(
        root,
        "plugin/mcp.json",
        "{ \"mcpServers\": { \"demo\": { \"command\": \"demo\", \
         \"headers\": { \"X-Token\": \"t\" } } } }\n",
    );

    let error = run(
        root,
        vec![projection_target(
            "bad-local",
            "package:opencode-plugin",
            &["mcp"],
        )],
    )
    .expect_err("an OpenCode local server has nowhere to put headers");

    match capability(error) {
        ClientProjectionError::Unrepresentable { member, client, .. } => {
            assert_eq!(client, "opencode");
            assert_eq!(member, "mcpServers.demo.headers");
        }
        other => panic!("expected a capability refusal, got {other}"),
    }
    assert!(
        !root.join("target/vibe-package/bad-local").exists(),
        "a refusal leaves no projection",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_remote_server_declaring_args_or_env_refuses_as_unsupported() {
    for (spelling, member) in [
        (
            "{ \"mcpServers\": { \"demo\": { \"url\": \"https://x.test\", \
             \"args\": [\"--flag\"] } } }\n",
            "mcpServers.demo.args",
        ),
        (
            "{ \"mcpServers\": { \"demo\": { \"url\": \"https://x.test\", \
             \"env\": { \"K\": \"v\" } } } }\n",
            "mcpServers.demo.env",
        ),
    ] {
        let home = temp();
        let root = home.path();
        write_full_plugin(root);
        write(root, "plugin/mcp.json", spelling);

        let error = run(
            root,
            vec![projection_target(
                "bad-remote",
                "package:opencode-plugin",
                &["mcp"],
            )],
        )
        .expect_err("an OpenCode remote server is reached, never spawned");

        match capability(error) {
            ClientProjectionError::Unrepresentable {
                member: named,
                reason,
                ..
            } => {
                assert_eq!(named, member);
                assert!(!reason.is_empty(), "the capability report says why");
            }
            other => panic!("expected a capability refusal, got {other}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_same_unsupported_fields_are_carried_unharmed_by_the_copying_clients() {
    // The capability gap is OpenCode's own, not the plugin's: Claude and
    // Codex take the identical document byte-for-byte, which is what makes
    // the refusal above a report about a CLIENT rather than about a source.
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    let declaration = "{ \"mcpServers\": { \"demo\": { \"command\": \"demo\", \
                       \"headers\": { \"X-Token\": \"t\" } } } }\n";
    write(root, "plugin/mcp.json", declaration);

    project(root, "tolerant", "package:claude-plugin", &["mcp"]);

    assert_eq!(
        staged(root, "target/vibe-package/tolerant/.mcp.json"),
        declaration,
    );
}
