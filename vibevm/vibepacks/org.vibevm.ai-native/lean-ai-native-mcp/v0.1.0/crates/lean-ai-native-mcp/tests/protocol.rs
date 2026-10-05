//! Protocol and engine parity, exercised without starting an agent host.

use lean_ai_native_check::{Context, Reply, Request, execute};
use lean_ai_native_mcp::{SERVER_NAME, TOOL_NAMES, tool_set};
use mcp_core::{Server, testing::Scripted};
use serde_json::{Value, json};

mod common;

fn context(root: &std::path::Path) -> Context {
    Context {
        project_root: root.to_path_buf(),
        toolchain_dir: Some(root.join("missing-toolchain")),
    }
}

fn call(name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
        "name":name, "arguments":arguments,
    }})
}

fn replay(context: &Context, inputs: Vec<Value>) -> Vec<Value> {
    let mut server = Server::new(SERVER_NAME, env!("CARGO_PKG_VERSION"), tool_set(context));
    let mut transport = Scripted::new(inputs.into_iter().map(|v| v.to_string()).collect());
    server.run(&mut transport).unwrap();
    transport
        .outbound
        .iter()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect()
}

fn reply(frame: &Value) -> Reply {
    assert!(
        frame.get("error").is_none(),
        "unexpected protocol error: {frame}"
    );
    serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn handshake_lists_only_the_three_strict_tools() {
    let root = tempfile::tempdir().unwrap();
    let frames = replay(
        &context(root.path()),
        vec![
            json!({"jsonrpc":"2.0","id":1,"method":"initialize"}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        ],
    );
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0]["result"]["serverInfo"]["name"], SERVER_NAME);
    let tools = frames[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(
        tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        TOOL_NAMES
    );
    for tool in tools {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        let properties = tool["inputSchema"]["properties"].as_object().unwrap();
        for forbidden in ["operation", "project_root", "toolchain_dir", "path"] {
            assert!(!properties.contains_key(forbidden));
        }
    }
}

#[test]
fn malformed_arguments_refuse_in_the_shared_reply_shape() {
    let root = tempfile::tempdir().unwrap();
    for tool in TOOL_NAMES {
        for bad in [
            Value::Null,
            json!([]),
            json!(false),
            json!({}),
            json!({"carrier":42}),
            json!(["proofs.toml"]),
            json!(["proofs.toml", "spec://org.example/demo/RULE#R"]),
            json!([
                "proofs.toml",
                "spec://org.example/demo/RULE#R",
                "witness.json"
            ]),
        ] {
            let frames = replay(&context(root.path()), vec![call(tool, bad)]);
            let answer = reply(&frames[0]);
            assert!(!answer.ok);
            assert_eq!(frames[0]["result"]["isError"], true);
            assert_eq!(
                serde_json::to_value(answer).unwrap()["error"]["code"],
                "invalid_request"
            );
        }
    }
}

#[test]
fn unknown_members_cannot_override_startup_authority_or_operation() {
    let root = tempfile::tempdir().unwrap();
    for tool in TOOL_NAMES {
        let mut valid = json!({"carrier":"proofs.toml"});
        if tool != "lean_ai_native_scan" {
            valid["subject"] = json!("spec://org.example/demo/RULE#R");
        }
        if tool == "lean_ai_native_verify_witness" {
            valid["witness"] = json!("witness.json");
        }
        for key in [
            "operation",
            "project_root",
            "toolchain_dir",
            "path",
            "extra",
        ] {
            let mut bad = valid.clone();
            bad[key] = json!("other");
            let frames = replay(&context(root.path()), vec![call(tool, bad)]);
            assert_eq!(
                serde_json::to_value(reply(&frames[0])).unwrap()["error"]["code"],
                "invalid_request"
            );
        }
    }
}

#[test]
fn subject_and_witness_types_are_enforced_before_execution() {
    let root = tempfile::tempdir().unwrap();
    for (tool, arguments) in [
        (
            "lean_ai_native_check",
            json!({"carrier":"proofs.toml", "subject":null}),
        ),
        (
            "lean_ai_native_check",
            json!({"carrier":"proofs.toml", "subject":42}),
        ),
        (
            "lean_ai_native_verify_witness",
            json!({"carrier":"proofs.toml", "subject":"spec://org.example/demo/RULE#R", "witness":[]}),
        ),
        (
            "lean_ai_native_verify_witness",
            json!({"carrier":"proofs.toml", "subject":"spec://org.example/demo/RULE#R"}),
        ),
    ] {
        let frames = replay(&context(root.path()), vec![call(tool, arguments)]);
        assert_eq!(
            serde_json::to_value(reply(&frames[0])).unwrap()["error"]["code"],
            "invalid_request"
        );
        assert_eq!(frames[0]["result"]["isError"], true);
    }
}

#[test]
fn every_valid_operation_preserves_the_exact_engine_failure_reply() {
    let root = tempfile::tempdir().unwrap();
    let context = context(root.path());
    let cases = [
        (
            "lean_ai_native_scan",
            "scan",
            json!({"carrier":"absent.toml"}),
        ),
        (
            "lean_ai_native_check",
            "check",
            json!({"carrier":"absent.toml", "subject":"spec://org.example/demo/RULE#R"}),
        ),
        (
            "lean_ai_native_verify_witness",
            "verify_witness",
            json!({"carrier":"absent.toml", "subject":"spec://org.example/demo/RULE#R", "witness":"absent.json"}),
        ),
    ];
    for (tool, operation, arguments) in cases {
        let mut request = arguments.clone();
        request["operation"] = json!(operation);
        let request: Request = serde_json::from_value(request).unwrap();
        let expected = execute(&context, request);
        assert!(!expected.ok);
        let frames = replay(&context, vec![call(tool, arguments)]);
        assert!(reply(&frames[0]) == expected);
        assert_eq!(frames[0]["result"]["isError"], !expected.ok);
    }
}

#[test]
fn declared_inventory_success_is_the_exact_shared_engine_reply() {
    let root = common::consumer();
    let context = context(root.path());
    let expected = execute(
        &context,
        Request::Scan {
            carrier: "lean-proof.toml".into(),
        },
    );
    assert!(expected.ok, "{}", serde_json::to_string(&expected).unwrap());
    let frames = replay(
        &context,
        vec![call(
            "lean_ai_native_scan",
            json!({"carrier":"lean-proof.toml"}),
        )],
    );
    assert!(reply(&frames[0]) == expected);
    assert_eq!(frames[0]["result"]["isError"], false);
}

#[test]
fn absent_checker_is_a_typed_failure_without_a_witness() {
    let root = common::consumer();
    let context = context(root.path());
    let expected = execute(
        &context,
        Request::Check {
            carrier: "lean-proof.toml".into(),
            subject: common::SUBJECT.into(),
        },
    );
    assert!(!expected.ok);
    let encoded = serde_json::to_value(&expected).unwrap();
    assert_eq!(encoded["error"]["code"], "toolchain_unavailable");
    assert!(encoded["result"].is_null());
    let frames = replay(
        &context,
        vec![call(
            "lean_ai_native_check",
            json!({
                "carrier":"lean-proof.toml", "subject":common::SUBJECT,
            }),
        )],
    );
    assert!(reply(&frames[0]) == expected);
    assert_eq!(frames[0]["result"]["isError"], true);
}

#[test]
fn unknown_tools_and_bad_protocol_frames_remain_transport_errors() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::new(
        SERVER_NAME,
        env!("CARGO_PKG_VERSION"),
        tool_set(&context(root.path())),
    );
    let mut transport = Scripted::new(vec![
        "not JSON".into(),
        call("lean_ai_native_install", json!({})).to_string(),
        json!({"jsonrpc":"2.0","id":2,"method":"ping"}).to_string(),
    ]);
    server.run(&mut transport).unwrap();
    let frames: Vec<Value> = transport
        .outbound
        .iter()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect();
    assert_eq!(frames[0]["error"]["code"], -32700);
    assert_eq!(frames[1]["error"]["code"], -32601);
    assert_eq!(frames[2]["result"], json!({}));
}

#[test]
fn stdio_uses_the_startup_root_from_a_different_working_directory() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let root = tempfile::tempdir().unwrap();
    let unrelated = tempfile::tempdir().unwrap();
    let context = context(root.path());
    let expected = execute(
        &context,
        Request::Scan {
            carrier: "absent.toml".into(),
        },
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_lean-ai-native-mcp"))
        .args([
            "--path",
            root.path().to_str().unwrap(),
            "--toolchain-dir",
            context.toolchain_dir.as_ref().unwrap().to_str().unwrap(),
        ])
        .current_dir(unrelated.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "{}",
        call("lean_ai_native_scan", json!({"carrier":"absent.toml"}))
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let frames: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(frames.len(), 1, "stdout must contain only protocol frames");
    assert!(reply(&frames[0]) == expected);
}
