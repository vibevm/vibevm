//! Real CLI fixed package grant and framed owner exchange.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
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
