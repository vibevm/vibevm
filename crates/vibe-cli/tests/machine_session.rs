//! Real binary frame transport, explicit roots and independent MCP lines.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        operation_envelope::{Body as EnvelopeBody, BodyResultEffectState, OperationEnvelope},
        session_frame::*,
    },
};

const BEFORE: &str = "[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\nmaterialization='snapshot'\n";
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                out.insert(key, Vec::new());
                walk(root, &path, out);
            } else {
                out.insert(key, std::fs::read(path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
struct Fixture {
    temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        for name in ["home/.vibe", "cwd", "project"] {
            std::fs::create_dir_all(temp.path().join(name)).unwrap();
        }
        for name in ["config.toml", "settings.toml", "registry.toml"] {
            std::fs::write(
                temp.path().join("home/.vibe").join(name),
                "[broken config\n",
            )
            .unwrap();
        }
        std::fs::write(temp.path().join("project/vibe.toml"), BEFORE).unwrap();
        // An ambient project must not rescue an invalid or missing explicit root.
        std::fs::write(temp.path().join("cwd/vibe.toml"), BEFORE).unwrap();
        Self { temp }
    }
    fn root(&self) -> std::path::PathBuf {
        self.temp.path().join("project")
    }
    fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe"));
        cmd.env_clear()
            .env("HOME", self.temp.path().join("home"))
            .env("USERPROFILE", self.temp.path().join("home"))
            .env("VIBE_SETTINGS", self.temp.path().join("home/.vibe"))
            .env("VIBE_LOG", "invalid=???")
            .env("VIBEVM_HOME", self.temp.path().join("stale-home"))
            .current_dir(self.temp.path().join("cwd"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }
    fn start(&self) -> Child {
        self.command()
            .args(["machine", "session", "--root"])
            .arg(self.root())
            .spawn()
            .unwrap()
    }
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        tree(self.temp.path())
    }
}
fn schemas() -> Vec<SchemaOffer> {
    [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MigrationPlanRequest,
        FormatId::MigrationPlanReport,
        FormatId::MigrationPlanError,
    ]
    .into_iter()
    .map(|f| SchemaOffer {
        format_id: f.id().to_owned(),
        epoch: u16::try_from(f.epoch()).unwrap(),
    })
    .collect()
}
fn offer() -> SessionFrame {
    SessionFrame {
        schema: 1,
        session_id: None,
        request_id: Some("init".into()),
        body: Body::InitializeOffer(Box::new(BodyInitializeOffer {
            api_versions: vec!["1".into()],
            position_encodings: vec!["raw-byte-offsets".into()],
            schema_offers: schemas(),
            requested_capabilities: Capabilities {
                streaming: true,
                replay: true,
                control: true,
            },
        })),
    }
}
fn bytes(frame: &SessionFrame) -> Vec<u8> {
    let body = serde_json::to_vec(frame).unwrap();
    let mut bytes = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
    bytes.extend(body);
    bytes
}
fn receive(reader: &mut impl Read) -> SessionFrame {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix).unwrap();
    let length = u32::from_be_bytes(prefix) as usize;
    assert!(
        (1..=8 * 1024 * 1024).contains(&length),
        "four-byte generated frame prefix, no prose"
    );
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}
fn send(child: &mut Child, frame: &SessionFrame) -> SessionFrame {
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&bytes(frame))
        .unwrap();
    receive(child.stdout.as_mut().unwrap())
}
fn initialize(child: &mut Child) -> (String, String, String) {
    let reply = send(child, &offer());
    assert_eq!(reply.request_id.as_deref(), Some("init"));
    let session = reply.session_id.unwrap();
    let Body::Initialized(body) = reply.body else {
        panic!("expected initialized")
    };
    assert_eq!(body.selected_schemas, schemas());
    assert_eq!(body.api_version, "1");
    assert_eq!(body.position_encoding, "raw-byte-offsets");
    assert_eq!(
        body.actual_capabilities,
        Capabilities {
            streaming: false,
            replay: false,
            control: false
        }
    );
    assert_eq!(body.contexts.len(), 3);
    let workspace = body
        .contexts
        .iter()
        .find(|c| c.kind == ContextKind::Workspace)
        .unwrap();
    assert_eq!(workspace.availability, ContextAvailability::Available);
    assert_eq!(workspace.target_ids.len(), 1);
    for context in body
        .contexts
        .iter()
        .filter(|c| c.kind != ContextKind::Workspace)
    {
        assert_eq!(context.availability, ContextAvailability::Unavailable);
        assert!(context.target_ids.is_empty());
    }
    (
        session,
        workspace.context_id.clone(),
        workspace.target_ids[0].clone(),
    )
}
fn envelope() -> OperationEnvelope {
    serde_json::from_value(json!({"schema":1, "operation_id":"rust:vibe-workspace:migration_planning", "domain_owner":"rust:vibe-workspace:migration_planning", "source_id":"pure-migration-transport-adapter", "evidence_id":null,
        "body":{"stage":"input", "format_id":"migration-plan-request", "payload_json": json!({"schema":1,"source_text":BEFORE,"source_profile":"materialization-legacy-absent-r1","target_profile":"materialization-current-absent-r1","recipe":"snapshot-to-copy-r1"}).to_string()}})).unwrap()
}
fn request(session: &str, context: &str, id: &str, envelope: &OperationEnvelope) -> SessionFrame {
    SessionFrame {
        schema: 1,
        session_id: Some(session.into()),
        request_id: Some(id.into()),
        body: Body::Request(Box::new(BodyRequest {
            context_id: context.into(),
            target_id: None,
            envelope_json: serde_json::to_string(envelope).unwrap(),
        })),
    }
}
fn finish(child: Child, fixture: &Fixture, before: BTreeMap<String, Vec<u8>>) {
    let output = child.wait_with_output().unwrap(); // closes piped stdin: EOF owns lifetime
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "no trailing stdout prose");
    assert!(
        output.stderr.is_empty(),
        "no startup diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "HOME/cwd/project remained unchanged"
    );
}
#[test]
fn initialized_planner_request_and_eof_are_generated_frames_without_side_effects() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let mut child = fixture.start();
    let (session, context, _) = initialize(&mut child);
    let reply = send(
        &mut child,
        &request(&session, &context, "plan", &envelope()),
    );
    assert_eq!(reply.request_id.as_deref(), Some("plan"));
    let Body::Complete(body) = reply.body else {
        panic!("expected planner completion: {:?}", reply.body)
    };
    assert_eq!(body.outcome.effect_state, OutcomeEffectState::NoEffects);
    assert_eq!(body.outcome.completeness, OutcomeCompleteness::Complete);
    let result: OperationEnvelope =
        serde_json::from_str(body.envelope_json.as_deref().unwrap()).unwrap();
    let EnvelopeBody::Result(result) = result.body else {
        panic!("expected generated result envelope")
    };
    assert_eq!(result.effect_state, BodyResultEffectState::NoEffects);
    assert_eq!(
        result.format_id.as_deref(),
        Some(FormatId::MigrationPlanReport.id())
    );
    let payload: Value = serde_json::from_str(result.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(payload["outcome"]["status"], "planned");
    finish(child, &fixture, before);
}
#[test]
fn unknown_context_target_and_inner_format_refuse_without_effects() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let mut child = fixture.start();
    let (session, context, target) = initialize(&mut child);
    let mut unsupported = envelope();
    let EnvelopeBody::Input(body) = &mut unsupported.body else {
        panic!()
    };
    body.format_id = "caller-invented-format".into();
    let mut target_request = request(&session, &context, "unknown-target", &envelope());
    let Body::Request(body) = &mut target_request.body else {
        panic!()
    };
    body.target_id = Some("caller-target".into());
    let mut bound_target = request(&session, &context, "bound-target", &envelope());
    let Body::Request(body) = &mut bound_target.body else {
        panic!()
    };
    body.target_id = Some(target);
    for (frame, reason) in [
        (
            request(&session, "caller-context", "unknown-context", &envelope()),
            BodyRefusalReason::UnknownContext,
        ),
        (target_request, BodyRefusalReason::UnknownTarget),
        (bound_target, BodyRefusalReason::InvalidRequest),
        (
            request(&session, &context, "unknown-format", &unsupported),
            BodyRefusalReason::InvalidRequest,
        ),
    ] {
        let reply = send(&mut child, &frame);
        let Body::Refusal(body) = reply.body else {
            panic!("expected refusal")
        };
        assert_eq!(body.reason, reason);
        assert_eq!(body.outcome.effect_state, OutcomeEffectState::NoEffects);
    }
    finish(child, &fixture, before);
}
#[test]
fn invalid_roots_and_output_flags_refuse_before_frame_processing() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let missing = fixture.temp.path().join("missing");
    let no_manifest = fixture.temp.path().join("home");
    let root = fixture.root();
    for (args, exit) in [
        (vec!["machine", "session"], 2),
        (vec!["machine", "session", "--root", "."], 2),
        (
            vec!["machine", "session", "--root", missing.to_str().unwrap()],
            1,
        ),
        (
            vec![
                "machine",
                "session",
                "--root",
                no_manifest.to_str().unwrap(),
            ],
            1,
        ),
        (
            vec![
                "--json",
                "machine",
                "session",
                "--root",
                root.to_str().unwrap(),
            ],
            2,
        ),
    ] {
        let output = fixture.command().args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(exit), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert_eq!(fixture.snapshot(), before);
    }
    for flags in [
        vec!["--json"],
        vec!["--quiet"],
        vec!["--verbose"],
        vec!["--invoked-by", "fixture"],
        vec!["--agent-mode", "agent"],
        vec!["--agent-mode", "cli"],
    ] {
        for first in [true, false] {
            let mut args = Vec::new();
            if first {
                args.extend(flags.clone());
            }
            args.extend(["machine", "session", "--root", root.to_str().unwrap()]);
            if !first {
                args.extend(flags.clone());
            }
            let output = fixture.command().args(&args).output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
            assert_eq!(fixture.snapshot(), before);
        }
    }
}
#[test]
fn malformed_and_oversized_frames_fail_on_stderr_without_stdout_or_effects() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let mut epoch = offer();
    epoch.schema = 2;
    for (input, reason) in [
        (vec![0, 0, 0, 0], "ZeroLength"),
        (vec![0, 0], "TruncatedPrefix"),
        (vec![0, 0, 0, 4, b'{'], "TruncatedBody"),
        (vec![0, 0, 0, 1, 255], "InvalidUtf8"),
        (vec![0, 0, 0, 1, b'{'], "InvalidJson"),
        (vec![0, 0, 0, 2, b'{', b'}'], "InvalidShape"),
        (bytes(&epoch), "UnsupportedEpoch"),
        (
            (8 * 1024 * 1024 + 1_u32).to_be_bytes().to_vec(),
            "OverLimit",
        ),
    ] {
        let mut child = fixture.start();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(reason));
        assert_eq!(fixture.snapshot(), before);
    }
}
#[test]
fn mcp_serve_initialize_remains_one_newline_json_rpc_response() {
    let fixture = Fixture::new();
    for name in ["config.toml", "settings.toml", "registry.toml"] {
        std::fs::remove_file(fixture.temp.path().join("home/.vibe").join(name)).unwrap();
    }
    let mut child = fixture
        .command()
        .env_remove("VIBE_LOG")
        .env_remove("VIBEVM_HOME")
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(fixture.root())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(text.ends_with('\n'));
    let reply: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(reply["jsonrpc"], "2.0");
    assert_eq!(reply["id"], 1);
    assert_eq!(reply["result"]["protocolVersion"], "2024-11-05");
}
