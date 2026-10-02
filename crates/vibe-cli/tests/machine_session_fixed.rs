//! Real CLI fixed Validate frames, retained C1 identity and process-loss truth.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-HOST-REATTACH");

use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use vibe_orchestrator::machine_operation::{input_decision as d, lifecycle_view as v};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        lifecycle_action_error::{LifecycleActionError, LifecycleActionErrorCode as Code},
        lifecycle_action_reply::{LifecycleActionReply, Reply},
        operation_envelope as e, session_frame as f,
    },
};

#[path = "machine_session_fixed/cases.rs"]
mod cases;

const FLAGS: [&str; 4] = [
    "--approve-captured-script",
    "--allow-detach",
    "--allow-cancel-pre-dispatch",
    "--allow-cancel-deferred-boundary",
];
struct Fixture {
    dir: tempfile::TempDir,
}
impl Fixture {
    fn new(wait: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for name in ["project", "home/.vibe", "cwd"] {
            fs::create_dir_all(dir.path().join(name)).unwrap();
        }
        for name in ["config.toml", "settings.toml", "registry.toml"] {
            fs::write(dir.path().join("home/.vibe").join(name), "[broken config\n").unwrap();
        }
        let fixture = Self { dir };
        fs::write(fixture.root().join("vibe.toml"), "[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n").unwrap();
        let root = fixture.root().to_str().unwrap().replace('\'', "'\\''");
        let boundary = if wait {
            "touch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); if [ \"$i\" -gt 400 ]; then exit 70; fi; sleep 0.05; done\n"
        } else {
            "touch entered\n"
        };
        fs::write(
            fixture.root().join("run.sh"),
            format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\n{boundary}"),
        )
        .unwrap();
        fixture
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("project")
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", self.dir.path().join("home"))
            .env("USERPROFILE", self.dir.path().join("home"))
            .env("VIBE_SETTINGS", self.dir.path().join("home/.vibe"))
            .env("VIBE_LOG", "invalid=???")
            .env("VIBEVM_HOME", self.dir.path().join("stale-home"))
            .current_dir(self.dir.path().join("cwd"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    fn fixed_command(&self, mode: &str, ttl: Option<&str>, flags: &[&str]) -> Command {
        let mut command = self.command();
        command
            .args(["machine", "session", "--root"])
            .arg(self.root())
            .args(["--fixed-validate", mode]);
        if let Some(ttl) = ttl {
            command.args(["--ttl-ms", ttl]);
        }
        command.args(flags);
        command
    }
    fn client(&self, mode: &str) -> Client {
        Client::new(
            self.fixed_command(mode, (mode == "issue").then_some("60000"), &FLAGS)
                .spawn()
                .unwrap(),
        )
    }
    fn count_one(&self) {
        assert_eq!(fs::read(self.root().join("counter")).unwrap(), b"effect\n");
    }
    fn no_provider(&self) {
        assert!(!self.root().join("counter").exists());
    }
    fn release(&self) {
        fs::write(self.root().join("release"), "go").unwrap();
    }
    fn pointer(&self) -> PathBuf {
        let paths = fs::read_dir(self.root().join(".vibe/cli-host-grants"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        paths[0].clone()
    }
    fn index(&self) -> Value {
        serde_json::from_slice(
            &fs::read(self.root().join(".vibe/machine-submissions.json")).unwrap(),
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.root().join("release"), "cleanup");
    }
}
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !predicate() {
        assert!(Instant::now() < deadline, "bounded provider deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn schemas() -> Vec<f::SchemaOffer> {
    [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MachineLifecycleActionRequest,
        FormatId::MachineLifecycleActionReply,
        FormatId::MachineLifecycleActionError,
        FormatId::MachineInputDecisionMessage,
        FormatId::MachineLifecycleSubmissionView,
    ]
    .into_iter()
    .map(|id| f::SchemaOffer {
        format_id: id.id().into(),
        epoch: id.epoch().try_into().unwrap(),
    })
    .collect()
}
fn offer() -> f::SessionFrame {
    f::SessionFrame {
        schema: 1,
        session_id: None,
        request_id: Some("init".into()),
        body: f::Body::InitializeOffer(Box::new(f::BodyInitializeOffer {
            api_versions: vec!["1".into()],
            position_encodings: vec!["raw-byte-offsets".into()],
            schema_offers: schemas(),
            requested_capabilities: f::Capabilities {
                streaming: true,
                replay: true,
                control: true,
            },
        })),
    }
}
fn bytes(frame: &f::SessionFrame) -> Vec<u8> {
    let json = serde_json::to_vec(frame).unwrap();
    let mut bytes = u32::try_from(json.len()).unwrap().to_be_bytes().to_vec();
    bytes.extend(json);
    bytes
}
fn receive(reader: &mut impl Read) -> Result<Option<f::SessionFrame>, String> {
    let mut prefix = [0; 4];
    if reader.read(&mut prefix[..1]).map_err(|e| e.to_string())? == 0 {
        return Ok(None);
    }
    reader
        .read_exact(&mut prefix[1..])
        .map_err(|e| e.to_string())?;
    let length = u32::from_be_bytes(prefix) as usize;
    if !(1..=8 * 1024 * 1024).contains(&length) {
        return Err("stdout is not a generated frame".into());
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|e| e.to_string())
}
struct Client {
    child: Option<Child>,
    replies: Receiver<Result<f::SessionFrame, String>>,
    reader: Option<std::thread::JoinHandle<()>>,
    session: String,
    context: String,
    target: String,
    next: usize,
}
impl Client {
    fn new(mut child: Child) -> Self {
        let mut stdout = child.stdout.take().unwrap();
        let (tx, replies) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            loop {
                match receive(&mut stdout) {
                    Ok(Some(frame)) => {
                        if tx.send(Ok(frame)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        break;
                    }
                }
            }
        });
        let mut client = Self {
            child: Some(child),
            replies,
            reader: Some(reader),
            session: String::new(),
            context: String::new(),
            target: String::new(),
            next: 0,
        };
        let response = client.send(&offer());
        let f::Body::Initialized(body) = response.body else {
            panic!("fixed initialized")
        };
        assert_eq!(body.selected_schemas, schemas());
        assert!(
            !body.actual_capabilities.streaming
                && !body.actual_capabilities.replay
                && !body.actual_capabilities.control
        );
        assert_eq!(body.contexts.len(), 1);
        assert_eq!(
            body.contexts[0].availability,
            f::ContextAvailability::Available
        );
        assert_eq!(body.contexts[0].target_ids.len(), 1);
        client.session = response.session_id.unwrap();
        client.context = body.contexts[0].context_id.clone();
        client.target = body.contexts[0].target_ids[0].clone();
        client
    }
    fn write(&mut self, frame: &f::SessionFrame) {
        self.child
            .as_mut()
            .unwrap()
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&bytes(frame))
            .unwrap();
    }
    fn send(&mut self, frame: &f::SessionFrame) -> f::SessionFrame {
        self.write(frame);
        let reply = self
            .replies
            .recv_timeout(Duration::from_secs(20))
            .expect("bounded CLI frame response")
            .unwrap();
        assert_eq!(reply.request_id, frame.request_id);
        reply
    }
    fn frame(&mut self, envelope: String) -> f::SessionFrame {
        self.next += 1;
        f::SessionFrame {
            schema: 1,
            session_id: Some(self.session.clone()),
            request_id: Some(format!("r-{}", self.next)),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: self.context.clone(),
                target_id: Some(self.target.clone()),
                envelope_json: envelope,
            })),
        }
    }
    fn raw(&mut self, envelope: Value) -> Publication {
        let frame = self.frame(envelope.to_string());
        Publication::new(self.send(&frame))
    }
    fn call(&mut self, action: Value) -> Publication {
        self.raw(envelope(action))
    }
    fn finish(mut self) {
        let mut child = self.child.take().unwrap();
        drop(child.stdin.take());
        until(|| child.try_wait().unwrap().is_some());
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.reader.take().unwrap().join().unwrap();
        assert!(
            self.replies.try_iter().next().is_none(),
            "no extra stdout frames/prose"
        );
    }
    fn kill(mut self) {
        let mut child = self.child.take().unwrap();
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
        self.reader.take().unwrap().join().unwrap();
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn envelope(action: Value) -> Value {
    json!({"schema":1,"operation_id":vibe_orchestrator::machine_operation::fixed_lifecycle::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_lifecycle::DOMAIN_OWNER,"source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":"machine-lifecycle-action-request","payload_json":json!({"schema":1,"action":action}).to_string()}})
}
struct Publication {
    envelope: e::OperationEnvelope,
    outcome: f::Outcome,
}
impl Publication {
    fn new(frame: f::SessionFrame) -> Self {
        let (envelope, outcome) = match frame.body {
            f::Body::Complete(body) => (body.envelope_json.unwrap(), body.outcome),
            f::Body::Refusal(body) => (body.envelope_json.unwrap(), body.outcome),
            other => panic!("expected action publication: {other:?}"),
        };
        let envelope: e::OperationEnvelope = serde_json::from_str(&envelope).unwrap();
        match &envelope.body {
            e::Body::Result(body) => {
                assert_eq!(
                    serde_json::to_value(&body.effect_state).unwrap(),
                    serde_json::to_value(&outcome.effect_state).unwrap()
                );
                assert_eq!(
                    serde_json::to_value(&body.completeness).unwrap(),
                    serde_json::to_value(&outcome.completeness).unwrap()
                );
                assert_eq!(body.provenance_id, outcome.provenance_id);
            }
            e::Body::Error(body) => {
                assert_eq!(
                    serde_json::to_value(&body.effect_state).unwrap(),
                    serde_json::to_value(&outcome.effect_state).unwrap()
                );
                assert_eq!(
                    serde_json::to_value(&body.completeness).unwrap(),
                    serde_json::to_value(&outcome.completeness).unwrap()
                );
                assert_eq!(body.provenance_id, outcome.provenance_id);
            }
            other => panic!("Result/Error envelope: {other:?}"),
        }
        Self { envelope, outcome }
    }
    fn reply(&self) -> LifecycleActionReply {
        let e::Body::Result(body) = &self.envelope.body else {
            panic!("{:?}", self.envelope)
        };
        assert_eq!(
            body.format_id.as_deref(),
            Some(FormatId::MachineLifecycleActionReply.id())
        );
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
    }
    fn error(&self) -> LifecycleActionError {
        let e::Body::Error(body) = &self.envelope.body else {
            panic!("expected refusal")
        };
        assert_eq!(
            body.format_id.as_deref(),
            Some(FormatId::MachineLifecycleActionError.id())
        );
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
    }
    fn question(&self) -> d::InputDecisionMessage {
        let Reply::Question(q) = self.reply().reply else {
            panic!("typed question")
        };
        serde_json::from_str(&q.payload_json).unwrap()
    }
    fn row(&self) -> v::LifecycleSubmissionView {
        if matches!(self.envelope.body, e::Body::Error(_)) {
            let error = self.error();
            assert_eq!(error.code, Code::InspectionUnavailable);
            return serde_json::from_str(error.view_json.as_deref().expect("typed recovery view"))
                .unwrap();
        }
        let bytes = match self.reply().reply {
            Reply::View(row) => row.payload_json,
            Reply::Submitted(row) => row.payload_json,
            _ => panic!("submission view"),
        };
        serde_json::from_str(&bytes).unwrap()
    }
}
fn assert_same_terminal_truth(
    left: &v::LifecycleSubmissionView,
    right: &v::LifecycleSubmissionView,
) {
    assert_eq!(left.run_id, right.run_id);
    assert_eq!(left.status, right.status);
    assert_eq!(left.operation_id, right.operation_id);
    assert_eq!(left.requested_scope, right.requested_scope);
    assert_eq!(left.bound_scope, right.bound_scope);
    assert_eq!(left.requested_phase, right.requested_phase);
    assert_eq!(left.last_phase, right.last_phase);
    assert_eq!(left.work_outcomes, right.work_outcomes);
    assert_eq!(left.partial, right.partial);
}
fn prepare(control: &str) -> Value {
    json!({"kind":"prepare","phase":"validate","requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":control}})
}
fn answer(q: &d::InputDecisionMessage) -> Value {
    json!({"kind":"answer","message_json":serde_json::to_string(q).unwrap()})
}
fn submit(reference: &str, key: &str) -> Value {
    json!({"kind":"submit","accepted_ref":reference,"key":key})
}
fn inspect(key: &str) -> Value {
    json!({"kind":"inspect","key":key})
}
fn approval(client: &mut Client) -> d::InputDecisionMessage {
    let mut question = client.call(prepare("none")).question();
    assert_ne!(
        question.context_id.as_deref(),
        Some(client.context.as_str())
    );
    assert_ne!(question.target_id.as_deref(), Some(client.target.as_str()));
    assert!(matches!(question.body, d::Body::SelectionQuestion(_)));
    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    question = client.call(answer(&question)).question();
    assert!(matches!(question.body, d::Body::ApprovalQuestion(_)));
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    question
}
fn approve(client: &mut Client) -> String {
    let question = approval(client);
    let Reply::Validated(reply) = client.call(answer(&question)).reply().reply else {
        panic!("exact approval accepted")
    };
    reply.accepted_ref
}
fn terminal(client: &mut Client, key: &str) -> Publication {
    let mut result = None;
    until(|| {
        let publication = client.call(inspect(key));
        if matches!(publication.row().status, v::Status::ResultAvailable(_)) {
            result = Some(publication);
            true
        } else {
            false
        }
    });
    result.unwrap()
}
fn refusal(command: &mut Command, reason: &str, exit: i32) {
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "no frame or prose stdout on startup refusal"
    );
    assert!(
        !output.stderr.is_empty(),
        "startup refusal has a diagnostic"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(reason),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn snapshot(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.insert(path.strip_prefix(root).unwrap().into(), vec![]);
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut tree = std::collections::BTreeMap::new();
    walk(root, root, &mut tree);
    tree
}
