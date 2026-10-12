//! E2 actual process/service integrity parity over owned fixtures.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-PURITY-EVIDENCE");
#[path = "m_01_e_2_cases/fixture.rs"]
mod fixture;
#[path = "m_01_e_2_cases/proof.rs"]
mod proof;
use fixture::*;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};
use vibe_wire::behaviour::source_query as wire;
use vibe_workspace::source_query::SourceQueryService;
fn request() -> Value {
    json!({"schema":1,"address":"spec://org.demo/dep/DOC#ITEM","mode":"read","format":"md","children":true})
}
fn command(f: &Fixture) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vibe"));
    c.env_clear()
        .env("PATH", "/usr/bin:/bin")
        .current_dir("/tmp")
        .env("HOME", &f.home)
        .env("VIBE_SETTINGS", f.home.join(".vibe"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("VIBE_NO_PROGRESS", "1");
    c
}
fn parity(f: &Fixture, case: &str, input: &Value, expected: Option<(&str, &str)>) -> Value {
    let before = state(f.root.path());
    eprintln!("E2_QUERY_BEGIN library-{case}");
    let value = SourceQueryService::execute_json(&f.host, input).into_value();
    eprintln!("E2_QUERY_END library-{case}");
    if let Some((code, boundary)) = expected {
        wire::read_error(&value).unwrap();
        assert_eq!(value["code"], code);
        assert_eq!(value["boundary"], boundary);
    } else {
        wire::read_result(&value).unwrap();
        assert_eq!(value["observation"]["strength"], "record-consistent");
    }
    let cli = command(f)
        .args(["--json", "spec", "read", "--path"])
        .arg(&f.host)
        .arg("--request-json")
        .arg(input.to_string())
        .output()
        .unwrap();
    assert_eq!(
        cli.status.code(),
        Some(if expected.is_some() { 1 } else { 0 }),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let cli_value: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(cli_value, value);
    let mut mcp = command(f)
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(&f.host)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2","version":"1"}}});
    let call = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_spec","arguments":input}});
    {
        let mut stdin = mcp.stdin.take().unwrap();
        writeln!(stdin, "{init}\n{call}").unwrap();
    }
    let out = mcp.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let replies: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let reply = replies.iter().find(|r| r["id"] == 2).unwrap();
    assert_eq!(reply["result"]["isError"], expected.is_some());
    assert_eq!(reply["result"]["structuredContent"], value);
    let after = state(f.root.path());
    assert_eq!(after, before);
    println!(
        "E2_SNAPSHOT {case} {}",
        json!({"before":before,"after":after})
    );
    println!("E2_PARITY {case} {}", value);
    value
}
#[test]
fn process_service_success_expectation_and_missing_capture_parity() {
    let f = Fixture::new(vibe_core::manifest::SpecFormat::Mixed);
    let req = request();
    let a = parity(&f, "success", &req, None);
    let b = parity(&f, "repeat", &req, None);
    assert_eq!(a, b);
    let mut stale = req.clone();
    stale["expected_document_hash"] = json!(format!("sha256:{}", "b".repeat(64)));
    parity(
        &f,
        "stale-document",
        &stale,
        Some(("stale_expectation", "document expectation")),
    );
    let mut strict = req.clone();
    strict["proof_strength"] = json!("source-proven");
    parity(
        &f,
        "unsupported-strict-grade",
        &strict,
        Some(("invalid_request", "request shape")),
    );
    std::fs::remove_file(f.slot.join(&f.output)).unwrap();
    parity(
        &f,
        "missing-capture",
        &req,
        Some(("unavailable_world", "selected descriptor/output map")),
    );
}
#[test]
fn process_service_tamper_matrix_cannot_self_authenticate() {
    for mode in ["record-only", "payload-only", "coordinated"] {
        let f = Fixture::new(vibe_core::manifest::SpecFormat::Mixed);
        let req = request();
        let original = parity(&f, &format!("{mode}-control"), &req, None);
        let poisoned = b"# Document {#root}\n\n@fact:ITEM POISONED. @status:impl/done\n";
        let mut record = vibe_workspace::vibedeps::read_slot_record(&f.slot).unwrap();
        if mode != "record-only" {
            std::fs::write(f.slot.join(&f.output), poisoned).unwrap();
        }
        if mode != "payload-only" {
            use sha2::{Digest, Sha256};
            record
                .files
                .iter_mut()
                .find(|r| r.path == f.output)
                .unwrap()
                .sha256 = format!("{:x}", Sha256::digest(poisoned));
            vibe_workspace::vibedeps::write_slot_record(&f.slot, &record).unwrap();
        }
        if mode == "coordinated" {
            let local = parity(&f, mode, &req, None);
            assert!(local["content"].as_str().unwrap().contains("POISONED"));
            assert_eq!(local["observation"]["strength"], "record-consistent");
            let mut expected = req.clone();
            expected["expected_document_hash"] = original["observation"]["document_hash"].clone();
            parity(
                &f,
                "coordinated-pristine-expectation",
                &expected,
                Some(("stale_expectation", "document expectation")),
            );
        } else {
            parity(
                &f,
                mode,
                &req,
                Some(("unavailable_world", "selected descriptor/output map")),
            );
        }
    }
}
