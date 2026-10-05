//! Real CLI fixed cargo grant and framed owner exchange.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Barrier, mpsc},
    thread,
    time::Duration,
};
use vibe_core::manifest::TargetOs;
use vibe_mcp::ServerContext;
use vibe_orchestrator::{
    machine_operation::fixed_cargo::FixedCargoRoute,
    machine_submission::cargo_grant::CargoGrantService,
};
use vibe_wire::generated::{format_id::FormatId, machine::e1::session_frame as f};

#[path = "machine_session_cargo/local_route.rs"]
mod local_route;
#[path = "machine_session_cargo/process_mcp.rs"]
mod process_mcp;

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
        fs::write(dir.path().join("project/Cargo.toml"), b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n").unwrap();
        fs::create_dir_all(dir.path().join("project/src")).unwrap();
        fs::write(
            dir.path().join("project/src/main.rs"),
            b"fn main() {\n    println!(\"r8\");\n}\n",
        )
        .unwrap();
        Self { dir }
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("project")
    }
    fn pointer(&self) -> PathBuf {
        let entries = fs::read_dir(self.root().join(".vibe/machine-cargo-host-grants"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        entries[0].clone()
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        let rustup_home = std::env::var_os("RUSTUP_HOME").or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".rustup").into_os_string())
        });
        command
            .env_clear()
            .env("HOME", self.dir.path().join("home"))
            .env("USERPROFILE", self.dir.path().join("home"))
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .current_dir(self.dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(rustup_home) = rustup_home {
            command.env("RUSTUP_HOME", rustup_home);
        }
        if let Some(toolchain) = std::env::var_os("RUSTUP_TOOLCHAIN") {
            command.env("RUSTUP_TOOLCHAIN", toolchain);
        }
        command
    }
    fn cargo_command(&self, mode: &str, ttl: Option<&str>) -> Command {
        let mut command = self.command();
        command
            .args(["machine", "session", "--root"])
            .arg(self.root())
            .args(["--fixed-cargo", mode]);
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
        FormatId::MachineFixedCargoActionRequest,
        FormatId::MachineFixedCargoActionReply,
        FormatId::MachineFixedCargoActionError,
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
            panic!("cargo initialization refused: {response:?}")
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
            "operation_id":vibe_orchestrator::machine_operation::fixed_cargo::OPERATION_ID,
            "domain_owner":vibe_orchestrator::machine_operation::fixed_cargo::DOMAIN_OWNER,
            "source_id":null,"evidence_id":null,
            "body":{"stage":"input","format_id":FormatId::MachineFixedCargoActionRequest.id(),
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

fn mcp_cargo_context(root: &Path) -> ServerContext {
    let service = CargoGrantService::from_trusted_builtin_host(
        Arc::new(vibe_safefs::Project::open(root).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::current().unwrap(),
    )
    .unwrap();
    let route = Arc::new(FixedCargoRoute::new(Arc::new(service.reattach().unwrap())));
    ServerContext::with_store_root(root, root.join("unused"))
        .with_fixed_cargo_route(route)
        .unwrap()
}

fn mcp_cargo_submit(context: ServerContext, key: &str) -> Value {
    let request = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{
        "name":"fixed_cargo_operation","arguments":{"schema":1,"action":{"kind":"submit","key":key}}
    }});
    serde_json::from_str(&vibe_mcp::dispatch_one(context, &request.to_string()).unwrap()).unwrap()
}

#[test]
fn issue_and_reattach_keep_one_cargo_effect_and_owner_identity() {
    let fixture = Fixture::new();
    let mut first = Client::new(
        fixture
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    let submitted = first.call(json!({"kind":"submit","key":"once"}));
    assert_eq!(
        outcome(&submitted).effect_state,
        f::OutcomeEffectState::Completed,
        "{submitted:?}"
    );
    let retained = envelope(&submitted);
    let pointer = fs::read(fixture.pointer()).unwrap();
    let owner: Value =
        serde_json::from_str(retained["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(owner["reply"]["kind"], "completed");
    assert!(
        owner["reply"]["warning"]
            .as_str()
            .unwrap()
            .contains("rustup")
    );
    let output = fixture
        .root()
        .join(owner["reply"]["output_relative"].as_str().unwrap());
    let output_bytes = fs::read(&output).unwrap();
    let run = owner["reply"]["run_id"].as_str().unwrap();
    assert!(
        fixture
            .root()
            .join(format!(".vibe/build-operations/{run}.cargo-result.json"))
            .is_file()
    );
    let old_session = first.session.clone();
    first.finish();
    let mut second = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    assert_ne!(second.session, old_session);
    let inspected = second.call(json!({"kind":"inspect","key":"once"}));
    assert_eq!(envelope(&inspected), retained);
    let replay = second.call(json!({"kind":"submit","key":"once"}));
    assert_eq!(envelope(&replay), retained);
    second.finish();
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    assert_eq!(fs::read(output).unwrap(), output_bytes);
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
    Client::new(first.cargo_command("issue", Some("60000")).spawn().unwrap()).finish();
    Client::new(
        second
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    fs::write(second.pointer(), fs::read(first.pointer()).unwrap()).unwrap();
    let output = second.cargo_command("reattach", None).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ForeignBinding"));
    assert!(!second.root().join("target").exists());
    let fresh = Fixture::new();
    for (mode, ttl, extra) in [
        ("issue", None, vec![]),
        ("reattach", Some("60000"), vec![]),
        ("issue", Some("0"), vec![]),
        ("issue", Some("60000"), vec!["--approve-captured-script"]),
    ] {
        let mut command = fresh.cargo_command(mode, ttl);
        command.args(extra);
        assert!(!command.output().unwrap().status.success());
        assert!(
            !fresh
                .root()
                .join(".vibe/machine-cargo-host-grants")
                .exists()
        );
    }
}

#[test]
fn damaged_retained_output_is_not_reported_as_no_effects() {
    let fixture = Fixture::new();
    let mut first = Client::new(
        fixture
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    let submitted = first.call(json!({"kind":"submit","key":"once"}));
    let retained = envelope(&submitted);
    let owner: Value =
        serde_json::from_str(retained["body"]["payload_json"].as_str().unwrap()).unwrap();
    let run = owner["reply"]["run_id"].as_str().unwrap();
    first.finish();
    fs::remove_file(
        fixture
            .root()
            .join(format!(".vibe/build-operations/{run}.cargo-result.json")),
    )
    .unwrap();
    let mut second = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    let inspected = second.call(json!({"kind":"inspect","key":"once"}));
    assert_ne!(
        outcome(&inspected).effect_state,
        f::OutcomeEffectState::NoEffects
    );
    assert_eq!(
        outcome(&inspected).effect_state,
        f::OutcomeEffectState::Uncertain
    );
    let uncertain = envelope(&inspected);
    let view: Value =
        serde_json::from_str(uncertain["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(view["reply"]["kind"], "uncertain");
    assert_eq!(
        envelope(&second.call(json!({"kind":"submit","key":"once"}))),
        uncertain
    );
    second.finish();
}

#[test]
fn cli_issued_scope_mcp_submit_and_local_reattach_share_one_cargo_run() {
    let fixture = Fixture::new();
    let issued = Client::new(
        fixture
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    issued.finish();
    let pointer = fs::read(fixture.pointer()).unwrap();
    let service = CargoGrantService::from_trusted_builtin_host(
        Arc::new(vibe_safefs::Project::open(&fixture.root()).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::current().unwrap(),
    )
    .unwrap();
    let route = Arc::new(FixedCargoRoute::new(Arc::new(service.reattach().unwrap())));
    let context = ServerContext::with_store_root(fixture.root(), fixture.root().join("unused"))
        .with_fixed_cargo_route(route)
        .unwrap();
    let request = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{
        "name":"fixed_cargo_operation","arguments":{"schema":1,"action":{"kind":"submit","key":"shared"}}
    }});
    let response: Value =
        serde_json::from_str(&vibe_mcp::dispatch_one(context, &request.to_string()).unwrap())
            .unwrap();
    let mcp = &response["result"];
    assert_eq!(mcp["isError"], false, "{response}");
    assert_eq!(mcp["structuredContent"]["reply"]["kind"], "completed");
    assert!(
        mcp["structuredContent"]["reply"]["warning"]
            .as_str()
            .unwrap()
            .contains("rustup")
    );
    let run = mcp["structuredContent"]["reply"]["run_id"]
        .as_str()
        .unwrap();
    let output = fixture.root().join(
        mcp["structuredContent"]["reply"]["output_relative"]
            .as_str()
            .unwrap(),
    );
    let output_bytes = fs::read(&output).unwrap();
    let result_file = fixture
        .root()
        .join(format!(".vibe/build-operations/{run}.cargo-result.json"));
    let retained = fs::read(&result_file).unwrap();

    let mut local = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    let inspected = local.call(json!({"kind":"inspect","key":"shared"}));
    let view = envelope(&inspected);
    let owner: Value =
        serde_json::from_str(view["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(owner, mcp["structuredContent"]);
    let replay = local.call(json!({"kind":"submit","key":"shared"}));
    assert_eq!(envelope(&replay), view);
    local.finish();
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    assert_eq!(fs::read(&result_file).unwrap(), retained);
    assert_eq!(fs::read(output).unwrap(), output_bytes);
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    assert_eq!(
        index["namespaces"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()["keys"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn cli_and_registered_mcp_race_one_scoped_cargo_key() {
    let fixture = Fixture::new();
    Client::new(
        fixture
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    let pointer = fs::read(fixture.pointer()).unwrap();
    let root = fixture.root();
    let context = mcp_cargo_context(&root);
    let mut local = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    let barrier = Arc::new(Barrier::new(3));
    let (tx, rx) = mpsc::channel::<(&'static str, Value)>();

    let local_start = Arc::clone(&barrier);
    let local_tx = tx.clone();
    let local_thread = thread::spawn(move || {
        local_start.wait();
        let response = local.call(json!({"kind":"submit","key":"raced"}));
        local.finish();
        local_tx
            .send(("cli", serde_json::to_value(response).unwrap()))
            .unwrap();
    });
    let mcp_start = Arc::clone(&barrier);
    let mcp_thread = thread::spawn(move || {
        mcp_start.wait();
        let response = mcp_cargo_submit(context, "raced");
        tx.send(("mcp", response)).unwrap();
    });
    barrier.wait();
    let first = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("first racing surface timed out");
    let second = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("second racing surface timed out");
    local_thread.join().unwrap();
    mcp_thread.join().unwrap();
    assert_ne!(first.0, second.0);
    let (local, mcp) = if first.0 == "cli" {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };
    let local: f::SessionFrame = serde_json::from_value(local).unwrap();
    let local_view = envelope(&local);
    let local_payload: Value =
        serde_json::from_str(local_view["body"]["payload_json"].as_str().unwrap()).unwrap();
    let mcp_payload = &mcp["result"]["structuredContent"];
    let owner = match (
        outcome(&local).effect_state.clone(),
        mcp["result"]["isError"].as_bool(),
    ) {
        (f::OutcomeEffectState::Completed, Some(false)) => {
            assert_eq!(local_payload, *mcp_payload);
            local_payload
        }
        (f::OutcomeEffectState::Completed, Some(true)) => {
            assert_eq!(mcp_payload["code"], "submission_refused", "{mcp}");
            assert_eq!(mcp_payload["effect_observation"], "none", "{mcp}");
            assert!(mcp_payload["known_run_id"].is_null(), "{mcp}");
            assert!(
                mcp_payload["message"]
                    .as_str()
                    .unwrap()
                    .contains("Lease(Busy"),
                "{mcp}"
            );
            let retry = mcp_cargo_submit(mcp_cargo_context(&root), "raced");
            assert_eq!(retry["result"]["isError"], false, "{retry}");
            assert_eq!(retry["result"]["structuredContent"], local_payload);
            local_payload
        }
        (f::OutcomeEffectState::NoEffects, Some(false)) => {
            assert_eq!(local_payload["code"], "submission_refused", "{local:?}");
            assert_eq!(local_payload["effect_observation"], "none", "{local:?}");
            assert!(local_payload["known_run_id"].is_null(), "{local:?}");
            assert!(
                local_payload["message"]
                    .as_str()
                    .unwrap()
                    .contains("Lease(Busy"),
                "{local:?}"
            );
            let mut retry = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
            let replay = retry.call(json!({"kind":"submit","key":"raced"}));
            assert_eq!(
                outcome(&replay).effect_state,
                f::OutcomeEffectState::Completed
            );
            let replay_view = envelope(&replay);
            let replay_owner: Value =
                serde_json::from_str(replay_view["body"]["payload_json"].as_str().unwrap())
                    .unwrap();
            retry.finish();
            assert_eq!(replay_owner, *mcp_payload);
            replay_owner
        }
        other => panic!("unexpected race outcome {other:?}: local={local:?}; mcp={mcp}"),
    };
    assert_eq!(owner["reply"]["kind"], "completed");
    let run = owner["reply"]["run_id"].as_str().unwrap();
    let result_dir = root.join(".vibe/build-operations");
    let results = fs::read_dir(&result_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".cargo-result.json"))
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1, "one retained effect result: {results:?}");
    assert_eq!(
        results[0],
        result_dir.join(format!("{run}.cargo-result.json"))
    );
    assert!(
        root.join(owner["reply"]["output_relative"].as_str().unwrap())
            .is_file()
    );
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    let index: Value =
        serde_json::from_slice(&fs::read(root.join(".vibe/machine-submissions.json")).unwrap())
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
}
