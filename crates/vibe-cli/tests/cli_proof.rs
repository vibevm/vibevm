//! The actual CLI and MCP dispatch project one selected-proof engine result.

use serde_json::{Value, json};
use vibe_mcp::{ServerContext, dispatch_one};
use vibe_test_support::UserScratch;
use vibe_trace::proof::{Context, Request, execute};

const SUBJECT: &str = "spec://org.example/same-subject/Protocol#identity";

#[test]
fn cli_and_mcp_keep_the_same_engine_refusal_body() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let context = Context {
        project_root: root.path().to_path_buf(),
        lean_toolchain_dir: None,
        rust_toolchain_dir: None,
    };
    for (verb, tool, request) in [
        (
            "proof-check",
            "proof_check",
            Request::Check {
                subject: SUBJECT.into(),
                witness: "proof.json".into(),
            },
        ),
        (
            "proof-verify",
            "proof_verify",
            Request::Verify {
                subject: SUBJECT.into(),
                witness: "proof.json".into(),
            },
        ),
    ] {
        let expected = execute(&context, request);
        assert!(!expected.ok);
        let expected = serde_json::to_value(&expected).unwrap();
        let output = user
            .vibe()
            .args([
                verb,
                "--subject",
                SUBJECT,
                "--witness",
                "proof.json",
                "--json",
                "--invoked-by",
                "proof-parity",
                "--unattended",
            ])
            .arg("--path")
            .arg(root.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        let cli: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "one Reply was required: {error}: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
        assert_eq!(
            cli, expected,
            "CLI must not stamp or rebuild the engine body"
        );

        let rpc = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
            "name":tool, "arguments":{"subject":SUBJECT, "witness":"proof.json"},
        }});
        let frame = dispatch_one(ServerContext::new(root.path()), &rpc.to_string()).unwrap();
        let mcp: Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(mcp["result"]["structuredContent"], cli);
        assert_eq!(mcp["result"]["isError"], true);
        assert!(!root.path().join("proof.json").exists());
    }
}

#[test]
fn quiet_proof_refusal_still_returns_one_complete_json_value() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let output = user
        .vibe()
        .args([
            "proof-verify",
            "--quiet",
            "--subject",
            SUBJECT,
            "--witness",
            "proof.json",
        ])
        .arg("--path")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        std::str::from_utf8(&output.stdout).unwrap().lines().count(),
        1
    );
    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply["ok"], false);
    assert!(reply["error"]["code"].is_string());
}

#[test]
fn proof_help_names_both_required_inputs_and_trusted_overrides() {
    let user = UserScratch::new();
    for command in ["proof-check", "proof-verify"] {
        let output = user.vibe().args([command, "--help"]).output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        for flag in [
            "--subject",
            "--witness",
            "--path",
            "--lean-toolchain-dir",
            "--rust-toolchain-dir",
        ] {
            assert!(text.contains(flag), "missing {flag} from {command} help");
        }
    }
}

fn selected_fixture() -> tempfile::TempDir {
    use std::{fs, path::Path};
    let root = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/lean-composition");
    for file in [
        "vibe.toml",
        "specmap.toml",
        "proof-policy.toml",
        "lean-proof.toml",
        "Example.lean",
        "rust/Cargo.toml",
        "rust/Cargo.lock",
        "rust/src/lib.rs",
        "vibevm/vibespecs/Protocol.md",
    ] {
        let destination = root.path().join(file);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source.join(file), destination).unwrap();
    }
    root
}

#[test]
#[ignore = "requires the pinned real Lean and Rust toolchains in trusted startup environment"]
fn real_selected_proof_cli_mcp_positive_parity() {
    use std::fs;
    use vibe_trace::proof::Reply;

    let lean = std::env::var_os("LEAN_AI_NATIVE_TOOLCHAIN_DIR")
        .expect("set the real Lean toolchain directory for this explicitly selected test");
    let rust = std::env::var_os("RUST_LEAN_RUST_TOOLCHAIN_DIR")
        .expect("set the real Rust toolchain directory for this explicitly selected test");
    let root = selected_fixture();
    let user = UserScratch::new();
    let run_cli = |verb: &str| -> Reply {
        let output = user
            .vibe()
            .env("LEAN_AI_NATIVE_TOOLCHAIN_DIR", &lean)
            .env("RUST_LEAN_RUST_TOOLCHAIN_DIR", &rust)
            .args([
                verb,
                "--subject",
                SUBJECT,
                "--witness",
                "joined-proof.json",
                "--json",
            ])
            .arg("--path")
            .arg(root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let checked = run_cli("proof-check");
    assert!(checked.ok);
    let checked_result = checked.result.as_ref().unwrap();
    assert_eq!(checked_result.status, "checked");
    let witness_bytes = fs::read(root.path().join("joined-proof.json")).unwrap();
    assert!(!witness_bytes.is_empty());

    let request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
        "name":"proof_verify", "arguments":{"subject":SUBJECT, "witness":"joined-proof.json"},
    }});
    let frame = dispatch_one(ServerContext::new(root.path()), &request.to_string()).unwrap();
    let frame: Value = serde_json::from_str(&frame).unwrap();
    assert_eq!(frame["result"]["isError"], false, "{frame}");
    let hosted: Reply =
        serde_json::from_value(frame["result"]["structuredContent"].clone()).unwrap();
    let hosted_text: Reply =
        serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(hosted, hosted_text);
    let verified = run_cli("proof-verify");
    assert_eq!(
        verified, hosted,
        "both surfaces must retain the entire typed verified body"
    );
    assert!(verified.ok);
    let result = verified.result.unwrap();
    assert_eq!(result.status, "matched");
    assert_eq!(result.witness, checked_result.witness);
    assert_eq!(result.witness_sha256, checked_result.witness_sha256);
    assert_eq!(result.witness.grade, "g1_model");
    assert_eq!(result.witness.lean.proof.grade, "g1_model");
    assert_eq!(result.witness.correspondence, "not_established");
    assert_eq!(result.witness.rust.outcome, "passed");
    assert_eq!(result.witness.lean.outcome, "passed");
    assert_ne!(
        result.witness.rust.execution_id,
        result.witness.lean.execution_id
    );
    assert_eq!(
        fs::read(root.path().join("joined-proof.json")).unwrap(),
        witness_bytes
    );
}

#[test]
fn selected_fact_query_preserves_rust_and_lean_relations_without_writing() {
    let root = selected_fixture();
    let user = UserScratch::new();
    let output = user
        .vibe()
        .args([
            "requirements",
            "--relations",
            "--json",
            "--address-prefix",
            SUBJECT,
        ])
        .arg("--path")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{report}");
    assert_eq!(rows[0]["address"], SUBJECT);
    let relations = rows[0]["relations"].as_array().unwrap();
    assert_eq!(relations.len(), 2, "{report}");
    assert!(
        relations
            .iter()
            .any(|r| r["verb"] == "implements" && r["symbol"] == "rust::identity")
    );
    assert!(
        relations
            .iter()
            .any(|r| r["verb"] == "verifies" && r["symbol"] == "lean::Example.identity")
    );
    assert_eq!(report["relation_sources"][0]["state"], "current");
    assert!(
        !root.path().join("specmap.json").exists(),
        "query wrote a map"
    );
}
