//! Actual length-prefixed local exchange and registered MCP adapter fixtures.
use super::*;
use crate::{MemoryTransport, Server};
use serde_json::{Value, json};
use std::{
    fs,
    io::Cursor,
    time::{Duration, Instant},
};
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_orchestrator::{
    machine_operation::{input_decision as d, lifecycle_view as v},
    machine_submission::{FixedLifecycleDecisionHost, HostPolicyFacts},
};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        lifecycle_action_error::{LifecycleActionError, LifecycleActionErrorCode as Code},
        lifecycle_action_reply::{LifecycleActionReply, Reply},
    },
};
mod cases;
mod failure_cases;
fn facts(approve: bool) -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: approve,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
struct Fixture {
    dir: tempfile::TempDir,
    scope: SubmissionScope,
    ns: SubmissionNamespace,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), "[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n").unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"), format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\ntouch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); if [ \"$i\" -gt 400 ]; then exit 70; fi; sleep 0.05; done\n")).unwrap();
        let scope = SubmissionScope {
            context: "host-context".into(),
            target: "host-target".into(),
            principal: "host-principal".into(),
        };
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        let ns = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        drop(lease);
        Self { dir, scope, ns }
    }
    fn host(&self, approve: bool) -> Arc<FixedLifecycleDecisionHost> {
        Arc::new(
            FixedLifecycleDecisionHost::from_host(
                self.dir.path().into(),
                self.ns.clone(),
                self.scope.clone(),
                TargetOs::Linux,
                facts(approve),
            )
            .unwrap(),
        )
    }
    fn context(&self, host: Arc<FixedLifecycleDecisionHost>) -> ServerContext {
        ServerContext::with_store_root(self.dir.path(), self.dir.path().join("unused"))
            .with_fixed_lifecycle_host(host)
            .unwrap()
    }
    fn client(&self, host: Arc<FixedLifecycleDecisionHost>) -> Client {
        Client::new(FixedMachineSessionRunner::new(self.context(host)).unwrap())
    }
    fn release(&self) {
        fs::write(self.dir.path().join("release"), "go").unwrap();
    }
    fn count(&self) {
        assert_eq!(
            fs::read(self.dir.path().join("counter")).unwrap(),
            b"effect\n"
        );
    }
    fn no_provider(&self) {
        assert!(!self.dir.path().join("counter").exists());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.dir.path().join("release"), "cleanup");
    }
}
fn until(mut f: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(20);
    while !f() {
        assert!(Instant::now() < end, "native deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn offer() -> f::SessionFrame {
    let schemas = [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MachineLifecycleActionRequest,
        FormatId::MachineLifecycleActionReply,
        FormatId::MachineLifecycleActionError,
        FormatId::MachineInputDecisionMessage,
        FormatId::MachineLifecycleSubmissionView,
    ]
    .into_iter()
    .map(|id| json!({"format_id":id.id(),"epoch":id.epoch()}))
    .collect::<Vec<_>>();
    serde_json::from_value(
        json!({"schema":1,"session_id":null,"request_id":"init","body":{
        "kind":"initialize_offer","api_versions":["1"],"position_encodings":["raw-byte-offsets"],
        "schema_offers":schemas,
        "requested_capabilities":{"streaming":true,"replay":true,"control":true}}}),
    )
    .unwrap()
}
fn send(runner: &mut FixedMachineSessionRunner, frame: &f::SessionFrame) -> f::SessionFrame {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, frame).unwrap();
    let mut out = Vec::new();
    assert_eq!(
        runner.step(&mut Cursor::new(bytes), &mut out).unwrap(),
        RunnerStep::Replied
    );
    let mut out = Cursor::new(out);
    let response = read_frame(&mut out).unwrap().unwrap();
    assert!(read_frame(&mut out).unwrap().is_none());
    response
}
struct Client {
    runner: FixedMachineSessionRunner,
    session: String,
    context: String,
    target: String,
    next: usize,
}
impl Client {
    fn new(mut runner: FixedMachineSessionRunner) -> Self {
        let response = send(&mut runner, &offer());
        let f::Body::Initialized(b) = response.body else {
            panic!("initialized");
        };
        assert_eq!(b.selected_schemas.len(), 7);
        assert!(
            !b.actual_capabilities.streaming
                && !b.actual_capabilities.replay
                && !b.actual_capabilities.control
        );
        assert_eq!(b.contexts.len(), 1);
        assert_eq!(
            b.contexts[0].availability,
            f::ContextAvailability::Available
        );
        assert_eq!(b.contexts[0].target_ids.len(), 1);
        Self {
            runner,
            session: response.session_id.unwrap(),
            context: b.contexts[0].context_id.clone(),
            target: b.contexts[0].target_ids[0].clone(),
            next: 0,
        }
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
    fn call(&mut self, action: Value) -> Publication {
        self.raw(envelope(action).to_string())
    }
    fn raw(&mut self, envelope: String) -> Publication {
        let frame = self.frame(envelope);
        Publication::new(send(&mut self.runner, &frame))
    }
}
fn envelope(action: Value) -> Value {
    json!({"schema":1,"operation_id":vibe_orchestrator::machine_operation::fixed_lifecycle::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_lifecycle::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"input","format_id":"machine-lifecycle-action-request","payload_json":json!({"schema":1,"action":action}).to_string()}})
}
struct Publication {
    envelope: e::OperationEnvelope,
    outcome: f::Outcome,
    refused: bool,
}
impl Publication {
    fn new(frame: f::SessionFrame) -> Self {
        let (bytes, outcome, refused) = match frame.body {
            f::Body::Complete(b) => (b.envelope_json.unwrap(), b.outcome, false),
            f::Body::Refusal(b) => (b.envelope_json.unwrap(), b.outcome, true),
            _ => panic!("action publication"),
        };
        let envelope: e::OperationEnvelope = serde_json::from_str(&bytes).unwrap();
        assert_eq!(super::outcome(&envelope).unwrap(), outcome);
        Self {
            envelope,
            outcome,
            refused,
        }
    }
    fn reply(&self) -> LifecycleActionReply {
        assert!(!self.refused, "{:?}", self.envelope);
        let e::Body::Result(r) = &self.envelope.body else {
            panic!("result");
        };
        assert_eq!(
            r.format_id.as_deref(),
            Some("machine-lifecycle-action-reply")
        );
        serde_json::from_str(r.payload_json.as_deref().unwrap()).unwrap()
    }
    fn error(&self) -> LifecycleActionError {
        assert!(self.refused);
        let e::Body::Error(r) = &self.envelope.body else {
            panic!("error");
        };
        assert_eq!(
            r.format_id.as_deref(),
            Some("machine-lifecycle-action-error")
        );
        serde_json::from_str(r.payload_json.as_deref().unwrap()).unwrap()
    }
    fn row(&self) -> v::LifecycleSubmissionView {
        let bytes = match self.reply().reply {
            Reply::View(v) => v.payload_json,
            Reply::Submitted(s) => s.payload_json,
            _ => panic!("view"),
        };
        serde_json::from_str(&bytes).unwrap()
    }
    fn question(&self) -> d::InputDecisionMessage {
        let Reply::Question(q) = self.reply().reply else {
            panic!("question");
        };
        serde_json::from_str(&q.payload_json).unwrap()
    }
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
fn approve(client: &mut Client, control: &str) -> String {
    let p = client.call(prepare(control));
    assert_eq!(p.outcome.effect_state, f::OutcomeEffectState::Uncertain);
    let mut q = p.question();
    assert_ne!(q.context_id.as_deref(), Some(client.context.as_str()));
    assert_ne!(q.target_id.as_deref(), Some(client.target.as_str()));
    if matches!(q.body, d::Body::SelectionQuestion(_)) {
        q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        q = client.call(answer(&q)).question();
    }
    let id = q.question_id.clone().unwrap();
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(v) = client.call(answer(&q)).reply().reply else {
        panic!("validated");
    };
    assert_eq!(v.accepted_ref, id);
    assert_eq!(Some(v.plan_id), q.plan_id);
    id
}
fn mcp_call(server: &mut Server<MemoryTransport>, action: Value) -> Value {
    server.transport = MemoryTransport::with_input(json!({"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"lifecycle_operation","arguments":{"schema":1,"action":action}}}).to_string()+"\n");
    server.run().unwrap();
    let response: Value = serde_json::from_str(server.transport.take_output().trim()).unwrap();
    assert!(response["error"].is_null(), "{response}");
    response["result"].clone()
}
fn mcp_approve(server: &mut Server<MemoryTransport>, control: &str) -> String {
    let r = mcp_call(server, prepare(control));
    assert_eq!(r["isError"], false);
    let mut q: d::InputDecisionMessage = serde_json::from_str(
        r["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    if matches!(q.body, d::Body::SelectionQuestion(_)) {
        q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        let r = mcp_call(server, answer(&q));
        assert_eq!(r["isError"], false);
        q = serde_json::from_str(
            r["structuredContent"]["reply"]["payload_json"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    }
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let r = mcp_call(server, answer(&q));
    assert_eq!(r["isError"], false);
    r["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap()
        .into()
}
