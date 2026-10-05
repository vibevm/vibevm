//! Schema, dispatch and exact shared-engine failure parity for proof tools.

use serde_json::{Value, json};
use vibe_mcp::{ServerContext, dispatch_one};
use vibe_trace::proof::{Context, Request, execute};

const SUBJECT: &str = "spec://org.example/same-subject/Protocol#identity";

fn call(root: &std::path::Path, name: &str, arguments: Value) -> Value {
    let request = json!({
        "jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":name, "arguments":arguments},
    });
    let frame = dispatch_one(ServerContext::new(root), &request.to_string()).unwrap();
    serde_json::from_str(&frame).unwrap()
}

#[test]
fn proof_tools_advertise_exact_subject_witness_schema() {
    let root = tempfile::tempdir().unwrap();
    let request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/list"});
    let frame = dispatch_one(ServerContext::new(root.path()), &request.to_string()).unwrap();
    let frame: Value = serde_json::from_str(&frame).unwrap();
    let tools = frame["result"]["tools"].as_array().unwrap();
    for name in ["proof_check", "proof_verify"] {
        let matches: Vec<_> = tools.iter().filter(|tool| tool["name"] == name).collect();
        assert_eq!(matches.len(), 1);
        let schema = &matches[0]["inputSchema"];
        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 2);
        assert_eq!(properties["subject"]["type"], "string");
        assert_eq!(properties["witness"]["type"], "string");
        assert_eq!(schema["required"], json!(["subject", "witness"]));
        assert_eq!(schema["additionalProperties"], false);
    }
}

#[test]
fn malformed_arguments_and_startup_overrides_refuse_before_execution() {
    let root = tempfile::tempdir().unwrap();
    let unopened = root.path().join("unopened");
    for name in ["proof_check", "proof_verify"] {
        let mut cases = vec![
            Value::Null,
            json!(false),
            json!([]),
            json!([SUBJECT, "proof.json"]),
            json!({}),
            json!({"subject":SUBJECT}),
            json!({"witness":"proof.json"}),
            json!({"subject":42, "witness":"proof.json"}),
            json!({"subject":SUBJECT, "witness":[]}),
        ];
        for key in [
            "path",
            "project_root",
            "lean_toolchain_dir",
            "rust_toolchain_dir",
            "operation",
            "force",
        ] {
            let mut args = json!({"subject":SUBJECT, "witness":"proof.json"});
            args[key] = json!("override");
            cases.push(args);
        }
        for args in cases {
            let frame = call(&unopened, name, args.clone());
            assert!(frame["error"].is_null(), "{frame}");
            assert_eq!(frame["result"]["isError"], true, "{args}");
            assert!(
                frame["result"].get("structuredContent").is_none(),
                "{frame}"
            );
            assert!(
                !unopened.exists(),
                "invalid tool arguments reached the engine"
            );
        }
    }
}

#[test]
fn actual_engine_refusals_keep_their_exact_body_and_failure_flag() {
    let root = tempfile::tempdir().unwrap();
    let context = Context {
        project_root: root.path().to_path_buf(),
        lean_toolchain_dir: None,
        rust_toolchain_dir: None,
    };
    for (name, request) in [
        (
            "proof_check",
            Request::Check {
                subject: SUBJECT.into(),
                witness: "proof.json".into(),
            },
        ),
        (
            "proof_verify",
            Request::Verify {
                subject: SUBJECT.into(),
                witness: "proof.json".into(),
            },
        ),
    ] {
        let reply = execute(&context, request);
        assert!(
            !reply.ok,
            "the empty root cannot establish a selected proof"
        );
        let expected = serde_json::to_value(&reply).unwrap();
        let frame = call(
            root.path(),
            name,
            json!({"subject":SUBJECT, "witness":"proof.json"}),
        );
        assert_eq!(frame["result"]["structuredContent"], expected);
        assert_eq!(frame["result"]["isError"], !reply.ok);
        let text: Value =
            serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, expected);
        assert!(!root.path().join("proof.json").exists());
    }
}
