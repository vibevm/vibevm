//! Real CLI fixed package grant and framed owner exchange.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Barrier, mpsc},
    thread,
    time::Duration,
};
use vibe_wire::generated::{format_id::FormatId, machine::e1::session_frame as f};

struct Fixture {
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("project")).unwrap();
        fs::create_dir_all(dir.path().join("home")).unwrap();
        fs::write(
            dir.path().join("project/vibe.toml"),
            "[project]\nname='demo'\nversion='1.1.0'\n",
        )
        .unwrap();
        fs::write(dir.path().join("project/source.bin"), b"launcher bytes\n").unwrap();
        Self { dir }
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("project")
    }
    fn pointer(&self) -> PathBuf {
        let entries = fs::read_dir(self.root().join(".vibe/cli-package-host-grants"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        entries[0].clone()
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", self.dir.path().join("home"))
            .env("USERPROFILE", self.dir.path().join("home"))
            .current_dir(self.dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    fn package_command(&self, mode: &str, ttl: Option<&str>) -> Command {
        let mut command = self.command();
        command
            .args(["machine", "session", "--root"])
            .arg(self.root())
            .args(["--fixed-package", mode]);
        if let Some(ttl) = ttl {
            command.args(["--ttl-ms", ttl]);
        }
        command
    }
}

fn schemas() -> Vec<f::SchemaOffer> {
    [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MachineFixedPackageActionRequest,
        FormatId::MachineFixedPackageActionReply,
        FormatId::MachineFixedPackageActionError,
    ]
    .into_iter()
    .map(|id| f::SchemaOffer {
        format_id: id.id().into(),
        epoch: id.epoch().try_into().unwrap(),
    })
    .collect()
}
fn frame_bytes(frame: &f::SessionFrame) -> Vec<u8> {
    let body = serde_json::to_vec(frame).unwrap();
    let mut bytes = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
    bytes.extend(body);
    bytes
}
fn read_frame(reader: &mut impl Read) -> f::SessionFrame {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix).unwrap();
    let len = u32::from_be_bytes(prefix) as usize;
    assert!((1..=8 * 1024 * 1024).contains(&len));
    let mut body = vec![0; len];
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}
struct Client {
    child: Child,
    session: String,
    context: String,
    target: String,
    next: usize,
}
impl Client {
    fn new(mut child: Child) -> Self {
        let offer = f::SessionFrame {
            schema: 1,
            session_id: None,
            request_id: Some("init".into()),
            body: f::Body::InitializeOffer(Box::new(f::BodyInitializeOffer {
                api_versions: vec!["1".into()],
                position_encodings: vec!["raw-byte-offsets".into()],
                schema_offers: schemas(),
                requested_capabilities: f::Capabilities {
                    streaming: false,
                    replay: false,
                    control: false,
                },
            })),
        };
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&frame_bytes(&offer))
            .unwrap();
        let response = read_frame(child.stdout.as_mut().unwrap());
        let f::Body::Initialized(body) = response.body else {
            panic!("package initialization refused: {response:?}")
        };
        assert_eq!(body.selected_schemas, schemas());
        let context = &body.contexts[0];
        Self {
            child,
            session: response.session_id.unwrap(),
            context: context.context_id.clone(),
            target: context.target_ids[0].clone(),
            next: 0,
        }
    }
    fn call(&mut self, action: Value) -> f::SessionFrame {
        self.next += 1;
        let envelope = json!({"schema":1,
            "operation_id":vibe_orchestrator::machine_operation::fixed_package::OPERATION_ID,
            "domain_owner":vibe_orchestrator::machine_operation::fixed_package::DOMAIN_OWNER,
            "source_id":null,"evidence_id":null,
            "body":{"stage":"input","format_id":FormatId::MachineFixedPackageActionRequest.id(),
                "payload_json":json!({"schema":1,"action":action}).to_string()}});
        let frame = f::SessionFrame {
            schema: 1,
            session_id: Some(self.session.clone()),
            request_id: Some(format!("r-{}", self.next)),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: self.context.clone(),
                target_id: Some(self.target.clone()),
                envelope_json: envelope.to_string(),
            })),
        };
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&frame_bytes(&frame))
            .unwrap();
        read_frame(self.child.stdout.as_mut().unwrap())
    }
    fn finish(mut self) {
        drop(self.child.stdin.take());
        let output = self.child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
    }
}
fn outcome(frame: &f::SessionFrame) -> &f::Outcome {
    match &frame.body {
        f::Body::Complete(row) => &row.outcome,
        f::Body::Refusal(row) => &row.outcome,
        other => panic!("unexpected response: {other:?}"),
    }
}
fn envelope(frame: &f::SessionFrame) -> Value {
    let json = match &frame.body {
        f::Body::Complete(row) => row.envelope_json.as_ref().unwrap(),
        f::Body::Refusal(row) => row.envelope_json.as_ref().unwrap(),
        other => panic!("unexpected response: {other:?}"),
    };
    serde_json::from_str(json).unwrap()
}

fn mcp_package_process(fixture: &Fixture) -> Child {
    let mut command = fixture.command();
    command
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(fixture.root())
        .args(["--fixed-package", "reattach"]);
    command.spawn().unwrap()
}

fn mcp_package_response(mut child: Child, kind: &str, key: &str) -> Value {
    let request = json!({"jsonrpc":"2.0","id":41,"method":"tools/call",
        "params":{"name":"fixed_package_operation","arguments":{
            "schema":1,"action":{"kind":kind,"key":key}}}});
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    serde_json::from_str(stdout.trim()).unwrap()
}

fn mcp_package_call(fixture: &Fixture, kind: &str, key: &str) -> Value {
    let response = mcp_package_response(mcp_package_process(fixture), kind, key);
    assert_eq!(response["result"]["isError"], false, "{response}");
    response["result"]["structuredContent"].clone()
}

#[test]
fn machine_submit_then_mcp_reattach_reuses_same_package_operation() {
    let fixture = Fixture::new();
    let mut machine = Client::new(
        fixture
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    let submitted = machine.call(json!({"kind":"submit","key":"cross-surface"}));
    assert_eq!(
        outcome(&submitted).effect_state,
        f::OutcomeEffectState::Completed
    );
    let machine_envelope = envelope(&submitted);
    let machine_reply: Value =
        serde_json::from_str(machine_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(machine_reply["reply"]["kind"], "completed");
    let run_id = machine_reply["reply"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();
    machine.finish();

    let pointer = fs::read(fixture.pointer()).unwrap();
    let inspected = mcp_package_call(&fixture, "inspect", "cross-surface");
    assert_eq!(inspected["reply"]["run_id"], run_id);
    assert_eq!(inspected, machine_reply);
    let replay = mcp_package_call(&fixture, "submit", "cross-surface");
    assert_eq!(replay, machine_reply);
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    assert_eq!(
        fs::read(
            fixture
                .root()
                .join("target/vibe-package/launcher/launcher.bin")
        )
        .unwrap(),
        b"launcher bytes\n"
    );
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    assert_eq!(
        fs::read_dir(fixture.root().join(".vibe/package-operations"))
            .unwrap()
            .count(),
        3,
        "one intent, capsule and result"
    );
}

#[test]
fn machine_frame_and_mcp_race_one_fixed_package_key() {
    let fixture = Fixture::new();
    Client::new(
        fixture
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    let pointer = fs::read(fixture.pointer()).unwrap();
    let mut machine = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
    let mut mcp = mcp_package_process(&fixture);
    mcp.stdin
        .as_mut()
        .unwrap()
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":40,\"method\":\"tools/list\",\"params\":{}}\n")
        .unwrap();
    let mut ready_line = String::new();
    BufReader::new(mcp.stdout.as_mut().unwrap())
        .read_line(&mut ready_line)
        .unwrap();
    let ready: Value = serde_json::from_str(&ready_line).unwrap();
    assert!(ready["error"].is_null(), "{ready}");
    assert!(
        ready["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "fixed_package_operation")
    );
    let barrier = Arc::new(Barrier::new(3));
    let (tx, rx) = mpsc::channel::<(&'static str, Value)>();

    let machine_start = Arc::clone(&barrier);
    let machine_tx = tx.clone();
    let machine_thread = thread::spawn(move || {
        machine_start.wait();
        let reply = machine.call(json!({"kind":"submit","key":"raced"}));
        machine.finish();
        machine_tx
            .send(("machine", serde_json::to_value(reply).unwrap()))
            .unwrap();
    });
    let mcp_start = Arc::clone(&barrier);
    let mcp_thread = thread::spawn(move || {
        mcp_start.wait();
        let reply = mcp_package_response(mcp, "submit", "raced");
        tx.send(("mcp", reply)).unwrap();
    });
    barrier.wait();
    let first = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("first fixed package surface timed out");
    let second = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("second fixed package surface timed out");
    machine_thread.join().unwrap();
    mcp_thread.join().unwrap();
    assert_ne!(first.0, second.0);
    let (machine, mcp) = if first.0 == "machine" {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };
    let machine: f::SessionFrame = serde_json::from_value(machine).unwrap();
    let machine_envelope = envelope(&machine);
    let machine_payload: Value =
        serde_json::from_str(machine_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    let mcp_payload = &mcp["result"]["structuredContent"];
    let owner = match (
        outcome(&machine).effect_state.clone(),
        mcp["result"]["isError"].as_bool(),
    ) {
        (f::OutcomeEffectState::Completed, Some(false)) => {
            assert_eq!(machine_payload, *mcp_payload);
            machine_payload
        }
        (f::OutcomeEffectState::Completed, Some(true)) => {
            assert_eq!(mcp_payload["code"], "submission_refused", "{mcp}");
            assert_eq!(mcp_payload["effect_observation"], "unavailable", "{mcp}");
            assert!(mcp_payload["known_run_id"].is_null(), "{mcp}");
            assert!(
                mcp_payload["message"]
                    .as_str()
                    .unwrap()
                    .contains("lifecycle lease: another mutating lifecycle command owns"),
                "{mcp}"
            );
            let replay = mcp_package_call(&fixture, "submit", "raced");
            assert_eq!(replay, machine_payload);
            machine_payload
        }
        (f::OutcomeEffectState::Uncertain, Some(false)) => {
            assert_eq!(
                outcome(&machine).completeness,
                f::OutcomeCompleteness::Unavailable
            );
            assert_eq!(machine_payload["code"], "submission_refused");
            assert_eq!(machine_payload["effect_observation"], "unavailable");
            assert!(machine_payload["known_run_id"].is_null());
            assert!(
                machine_payload["message"]
                    .as_str()
                    .unwrap()
                    .contains("lifecycle lease: another mutating lifecycle command owns"),
                "{machine:?}"
            );
            let mut retry = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
            let replay = retry.call(json!({"kind":"submit","key":"raced"}));
            assert_eq!(
                outcome(&replay).effect_state,
                f::OutcomeEffectState::Completed
            );
            let replay_envelope = envelope(&replay);
            let replay_payload: Value =
                serde_json::from_str(replay_envelope["body"]["payload_json"].as_str().unwrap())
                    .unwrap();
            retry.finish();
            assert_eq!(replay_payload, *mcp_payload);
            replay_payload
        }
        other => panic!("unexpected race outcome {other:?}: machine={machine:?}; mcp={mcp}"),
    };
    assert_eq!(owner["reply"]["kind"], "completed");
    assert!(owner["reply"]["run_id"].as_str().is_some());
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    assert_eq!(
        fs::read(
            fixture
                .root()
                .join("target/vibe-package/launcher/launcher.bin")
        )
        .unwrap(),
        b"launcher bytes\n"
    );
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    assert_eq!(
        namespaces.values().next().unwrap()["keys"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fs::read_dir(fixture.root().join(".vibe/package-operations"))
            .unwrap()
            .count(),
        3,
        "one intent, capsule and result"
    );
}

#[test]
fn issue_and_reattach_keep_one_package_effect_and_owner_identity() {
    let fixture = Fixture::new();
    let mut first = Client::new(
        fixture
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    let submitted = first.call(json!({"kind":"submit","key":"once"}));
    assert_eq!(
        outcome(&submitted).effect_state,
        f::OutcomeEffectState::Completed
    );
    let retained = envelope(&submitted);
    let pointer = fs::read(fixture.pointer()).unwrap();
    let output = fixture
        .root()
        .join("target/vibe-package/launcher/launcher.bin");
    assert_eq!(fs::read(&output).unwrap(), b"launcher bytes\n");
    let old_session = first.session.clone();
    first.finish();
    let mut second = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
    assert_ne!(second.session, old_session);
    let inspected = second.call(json!({"kind":"inspect","key":"once"}));
    assert_eq!(envelope(&inspected), retained);
    let replay = second.call(json!({"kind":"submit","key":"once"}));
    assert_eq!(envelope(&replay), retained);
    second.finish();
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    assert_eq!(fs::read(output).unwrap(), b"launcher bytes\n");
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn foreign_pointer_and_invalid_modes_refuse_without_copy() {
    let first = Fixture::new();
    let second = Fixture::new();
    Client::new(
        first
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    Client::new(
        second
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    fs::write(second.pointer(), fs::read(first.pointer()).unwrap()).unwrap();
    let output = second.package_command("reattach", None).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ForeignBinding"));
    assert!(
        !second
            .root()
            .join("target/vibe-package/launcher/launcher.bin")
            .exists()
    );
    let fresh = Fixture::new();
    for (mode, ttl, extra) in [
        ("issue", None, vec![]),
        ("reattach", Some("60000"), vec![]),
        ("issue", Some("0"), vec![]),
        ("issue", Some("60000"), vec!["--approve-captured-script"]),
    ] {
        let mut command = fresh.package_command(mode, ttl);
        command.args(extra);
        assert!(!command.output().unwrap().status.success());
        assert!(!fresh.root().join(".vibe/cli-package-host-grants").exists());
    }
}

#[test]
fn damaged_retained_output_is_not_reported_as_no_effects() {
    let fixture = Fixture::new();
    let mut first = Client::new(
        fixture
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    first.call(json!({"kind":"submit","key":"once"}));
    first.finish();
    fs::remove_file(
        fixture
            .root()
            .join("target/vibe-package/launcher/launcher.bin"),
    )
    .unwrap();
    let mut second = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
    let inspected = second.call(json!({"kind":"inspect","key":"once"}));
    assert_ne!(
        outcome(&inspected).effect_state,
        f::OutcomeEffectState::NoEffects
    );
    second.finish();
}
