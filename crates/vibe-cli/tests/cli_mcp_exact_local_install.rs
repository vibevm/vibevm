//! Cold CLI/MCP recovery of one exact local Install with a missing final report.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
};
use vibe_orchestrator::machine_operation::{
    exact_local_install::{DOMAIN_OWNER, OPERATION_ID},
    input_decision as decision,
};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        lifecycle_action_reply::{LifecycleActionReply, Reply},
        operation_envelope as e, session_frame as f,
    },
};

const FORMATS: [FormatId; 7] = [
    FormatId::MachineSessionFrame,
    FormatId::MachineOperationEnvelope,
    FormatId::MachineLifecycleActionRequest,
    FormatId::MachineLifecycleActionReply,
    FormatId::MachineLifecycleActionError,
    FormatId::MachineInputDecisionMessage,
    FormatId::MachineLifecycleSubmissionView,
];

struct Fixture {
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let package = root.path().join("registry/org.example/fixture/v1.1.0");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&package).unwrap();
        fs::write(
            project.join("vibe.toml"),
            "[project]\nname='consumer'\nversion='1.1.0'\n[requires.packages]\n'flow:org.example/fixture'='=1.1.0'\n",
        )
        .unwrap();
        fs::write(
            package.join("vibe.toml"),
            "[package]\ngroup='org.example'\nname='fixture'\nkind='flow'\nversion='1.1.0'\n",
        )
        .unwrap();
        fs::write(package.join("README.md"), b"# Fixture\n").unwrap();
        fs::create_dir_all(root.path().join("home")).unwrap();
        Self { root }
    }
    fn project(&self) -> PathBuf {
        self.root.path().join("project")
    }
    fn base(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", self.root.path().join("home"))
            .env("USERPROFILE", self.root.path().join("home"))
            .env("VIBE_SETTINGS", self.root.path().join("settings"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .current_dir(self.root.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    fn arguments(&self, command: &mut Command, mode: &str) {
        command
            .args(["--fixed-install", mode, "--registry-root"])
            .arg(self.root.path().join("registry"))
            .args([
                "--package-pin",
                "flow:org.example/fixture@=1.1.0",
                "--source-file",
                "vibe.toml",
                "--source-file",
                "README.md",
                "--approve-exact-local-install",
            ]);
        if mode == "issue" {
            command.args(["--ttl-ms", "60000"]);
        }
    }
    fn machine(&self, mode: &str) -> Child {
        let mut command = self.base();
        command
            .args(["machine", "session", "--root"])
            .arg(self.project());
        self.arguments(&mut command, mode);
        command.spawn().unwrap()
    }
    fn mcp(&self, mode: &str, action: Value) -> Output {
        let mut command = self.base();
        command
            .args(["--offline", "mcp", "serve", "--path"])
            .arg(self.project());
        self.arguments(&mut command, mode);
        let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"lifecycle_operation","arguments":{"schema":1,
                "operation_id":OPERATION_ID,"action":action}}});
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(request.to_string().as_bytes())
            .unwrap();
        drop(child.stdin.take());
        child.wait_with_output().unwrap()
    }
}

fn exchange(child: &mut Child, frame: f::SessionFrame) -> f::SessionFrame {
    let bytes = serde_json::to_vec(&frame).unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(&bytes).unwrap();
    let mut length = [0; 4];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut length)
        .unwrap();
    let count = u32::from_be_bytes(length) as usize;
    assert!((1..=8 * 1024 * 1024).contains(&count));
    let mut result = vec![0; count];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut result)
        .unwrap();
    serde_json::from_slice(&result).unwrap()
}

fn initialize(child: &mut Child) -> (String, String, String) {
    let response = exchange(
        child,
        f::SessionFrame {
            schema: 1,
            session_id: None,
            request_id: Some("init".into()),
            body: f::Body::InitializeOffer(Box::new(f::BodyInitializeOffer {
                api_versions: vec!["1".into()],
                position_encodings: vec!["raw-byte-offsets".into()],
                schema_offers: FORMATS
                    .iter()
                    .map(|id| f::SchemaOffer {
                        format_id: id.id().into(),
                        epoch: id.epoch().try_into().unwrap(),
                    })
                    .collect(),
                requested_capabilities: f::Capabilities {
                    streaming: false,
                    replay: false,
                    control: false,
                },
            })),
        },
    );
    let f::Body::Initialized(body) = response.body else {
        panic!("exact Install initialization refused: {response:?}")
    };
    assert_eq!(body.selected_schemas.len(), FORMATS.len());
    let context = &body.contexts[0];
    (
        response.session_id.unwrap(),
        context.context_id.clone(),
        context.target_ids[0].clone(),
    )
}

fn frame_action(
    child: &mut Child,
    ids: &(String, String, String),
    id: &str,
    action: Value,
) -> (e::OperationEnvelope, f::Outcome, bool) {
    let input = json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineLifecycleActionRequest.id(),
            "payload_json":json!({"schema":1,"action":action}).to_string()}});
    let response = exchange(
        child,
        f::SessionFrame {
            schema: 1,
            session_id: Some(ids.0.clone()),
            request_id: Some(id.into()),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: ids.1.clone(),
                target_id: Some(ids.2.clone()),
                envelope_json: input.to_string(),
            })),
        },
    );
    let (json, outcome, refused) = match response.body {
        f::Body::Complete(row) => (row.envelope_json.unwrap(), row.outcome, false),
        f::Body::Refusal(row) => (row.envelope_json.unwrap(), row.outcome, true),
        other => panic!("unexpected frame: {other:?}"),
    };
    (serde_json::from_str(&json).unwrap(), outcome, refused)
}

#[test]
fn cold_cli_and_mcp_keep_missing_install_report_partial_and_one_run() {
    let fixture = Fixture::new();
    let mut child = fixture.machine("issue");
    let ids = initialize(&mut child);
    let (prepared, _, refused) = frame_action(
        &mut child,
        &ids,
        "prepare",
        json!({"kind":"prepare","phase":"install","requested_options":{
            "force":false,"trace":false,"deploy_profile":null,"control":"none"}}),
    );
    assert!(!refused);
    let e::Body::Result(body) = prepared.body else {
        panic!("prepare")
    };
    let prepared: LifecycleActionReply =
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    let Reply::Question(question) = prepared.reply else {
        panic!("question")
    };
    let mut answer: decision::InputDecisionMessage =
        serde_json::from_str(&question.payload_json).unwrap();
    let reference = answer.question_id.clone().unwrap();
    answer.body =
        decision::Body::ApprovalAnswer(Box::new(decision::BodyApprovalAnswer { approved: true }));
    let (accepted, _, refused) = frame_action(
        &mut child,
        &ids,
        "answer",
        json!({
        "kind":"answer","message_json":serde_json::to_string(&answer).unwrap()}),
    );
    assert!(!refused);
    assert!(matches!(accepted.body, e::Body::Result(_)));
    let (submitted, _, refused) = frame_action(
        &mut child,
        &ids,
        "submit",
        json!({
        "kind":"submit","accepted_ref":reference,"key":"once"}),
    );
    assert!(!refused);
    let e::Body::Result(body) = submitted.body else {
        panic!("submit")
    };
    let submitted: LifecycleActionReply =
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    let Reply::Submitted(row) = submitted.reply else {
        panic!("submitted")
    };
    let view: Value = serde_json::from_str(&row.payload_json).unwrap();
    let run = view["run_id"].as_str().unwrap().to_owned();
    drop(child.stdin.take());
    let finished = child.wait_with_output().unwrap();
    assert!(
        finished.status.success(),
        "{}",
        String::from_utf8_lossy(&finished.stderr)
    );
    let root = fixture.project();
    let before_index = fs::read(root.join(".vibe/machine-submissions.json")).unwrap();
    let before_lock = fs::read(root.join("vibe.lock")).unwrap();
    let report = root.join(format!(".vibe/lifecycle-outcomes/{run}.report.json"));
    fs::remove_file(&report).unwrap();

    let mcp = fixture.mcp("reattach", json!({"kind":"inspect","key":"once"}));
    assert!(
        mcp.status.success(),
        "{}",
        String::from_utf8_lossy(&mcp.stderr)
    );
    let result: Value = serde_json::from_slice(&mcp.stdout).unwrap();
    assert_eq!(result["result"]["isError"], true, "{result}");
    let owner = &result["result"]["structuredContent"];
    assert_eq!(owner["requested_phase"], "install");
    assert_eq!(owner["known_run_id"], run);
    assert_eq!(
        owner["view_json"]
            .as_str()
            .map(|s| serde_json::from_str::<Value>(s).unwrap()["partial"].clone()),
        Some(json!(true))
    );

    let mut cold = fixture.machine("reattach");
    let ids = initialize(&mut cold);
    let (envelope, outcome, refused) = frame_action(
        &mut cold,
        &ids,
        "inspect",
        json!({"kind":"inspect","key":"once"}),
    );
    assert!(refused);
    let e::Body::Error(row) = envelope.body else {
        panic!("cold error")
    };
    let from_frame: Value = serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(&from_frame, owner);
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Uncertain);
    assert_eq!(outcome.completeness, f::OutcomeCompleteness::Unavailable);
    assert_eq!(outcome.provenance_id.as_deref(), Some(run.as_str()));
    drop(cold.stdin.take());
    assert!(cold.wait_with_output().unwrap().status.success());
    assert_eq!(
        fs::read(root.join(".vibe/machine-submissions.json")).unwrap(),
        before_index
    );
    assert_eq!(fs::read(root.join("vibe.lock")).unwrap(), before_lock);
    assert!(!report.exists());
}
