//! A separate MCP stdio process races the fixed Cargo machine frame.
use super::*;
use std::io::{BufRead, BufReader};

fn mcp_submit(fixture: &Fixture, key: &str) -> Value {
    let mut child = fixture.command();
    child
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(fixture.root())
        .args(["--fixed-cargo", "reattach"]);
    let mut child = child.spawn().unwrap();
    let request = json!({"jsonrpc":"2.0","id":51,"method":"tools/call",
        "params":{"name":"fixed_cargo_operation","arguments":{
            "schema":1,"action":{"kind":"submit","key":key}}}});
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(format!("{request}\n").as_bytes())
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.as_mut().unwrap())
        .read_line(&mut line)
        .unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    if line.is_empty() {
        assert!(!output.status.success());
        return json!({"startup_error":String::from_utf8_lossy(&output.stderr)});
    }
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(&line).unwrap()
}

#[test]
fn machine_frame_and_stdio_mcp_process_race_one_cargo_key() {
    let fixture = Fixture::new();
    Client::new(
        fixture
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    let pointer = fs::read(fixture.pointer()).unwrap();
    let mut cli = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    let barrier = Arc::new(Barrier::new(3));
    let (tx, rx) = mpsc::channel::<(&'static str, Value)>();
    let cli_start = Arc::clone(&barrier);
    let cli_tx = tx.clone();
    let cli_thread = thread::spawn(move || {
        cli_start.wait();
        let reply = cli.call(json!({"kind":"submit","key":"process-race"}));
        cli.finish();
        cli_tx
            .send(("cli", serde_json::to_value(reply).unwrap()))
            .unwrap();
    });
    let mcp_start = Arc::clone(&barrier);
    let mcp_thread = thread::spawn(move || {
        mcp_start.wait();
        tx.send(("mcp", mcp_submit(&fixture, "process-race")))
            .unwrap();
        fixture
    });
    barrier.wait();
    let first = rx.recv_timeout(Duration::from_secs(90)).unwrap();
    let second = rx.recv_timeout(Duration::from_secs(90)).unwrap();
    cli_thread.join().unwrap();
    let fixture = mcp_thread.join().unwrap();
    let (cli, mcp) = if first.0 == "cli" {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };
    let cli: f::SessionFrame = serde_json::from_value(cli).unwrap();
    let view = envelope(&cli);
    let cli_payload: Value =
        serde_json::from_str(view["body"]["payload_json"].as_str().unwrap()).unwrap();
    let mcp_payload = &mcp["result"]["structuredContent"];
    let completed = match (
        outcome(&cli).effect_state.clone(),
        mcp["result"]["isError"].as_bool(),
    ) {
        (f::OutcomeEffectState::Completed, None) => {
            assert!(
                mcp["startup_error"]
                    .as_str()
                    .unwrap()
                    .contains("Lease(Busy"),
                "{mcp}"
            );
            let retry = mcp_submit(&fixture, "process-race");
            assert_eq!(retry["result"]["structuredContent"], cli_payload);
            cli_payload
        }
        (f::OutcomeEffectState::Completed, Some(false)) => {
            assert_eq!(cli_payload, *mcp_payload);
            cli_payload
        }
        (f::OutcomeEffectState::Completed, Some(true)) => {
            assert_eq!(mcp_payload["code"], "submission_refused", "{mcp}");
            assert_eq!(mcp_payload["effect_observation"], "none", "{mcp}");
            assert!(mcp_payload["known_run_id"].is_null());
            let retry = mcp_submit(&fixture, "process-race");
            assert_eq!(retry["result"]["structuredContent"], cli_payload);
            cli_payload
        }
        (f::OutcomeEffectState::NoEffects, Some(false)) => {
            assert_eq!(cli_payload["code"], "submission_refused", "{cli:?}");
            assert_eq!(cli_payload["effect_observation"], "none");
            assert!(cli_payload["known_run_id"].is_null());
            let mut retry = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
            let replay = retry.call(json!({"kind":"submit","key":"process-race"}));
            assert_eq!(
                outcome(&replay).effect_state,
                f::OutcomeEffectState::Completed
            );
            let replay_view = envelope(&replay);
            let payload: Value =
                serde_json::from_str(replay_view["body"]["payload_json"].as_str().unwrap())
                    .unwrap();
            retry.finish();
            assert_eq!(payload, *mcp_payload);
            payload
        }
        other => panic!("unexpected process race {other:?}: cli={cli:?}; mcp={mcp}"),
    };
    assert_eq!(completed["reply"]["kind"], "completed");
    let run = completed["reply"]["run_id"].as_str().unwrap();
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    let result_dir = fixture.root().join(".vibe/build-operations");
    let results = fs::read_dir(&result_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".cargo-result.json"))
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1, "one retained result: {results:?}");
    assert!(
        result_dir
            .join(format!("{run}.cargo-result.json"))
            .is_file()
    );
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["process-race"]["domain_run_id"], run);
}
