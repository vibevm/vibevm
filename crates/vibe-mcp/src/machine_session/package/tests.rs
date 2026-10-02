//! Native package journal fixtures exercised through the actual frame codec.
use super::*;
use serde_json::{Value, json};
use std::{fs, io::Cursor};
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, StaticFileJournalInspection, SubmissionNamespace,
    SubmissionReservations, SubmissionScope, submit_fixed_static_file_package,
};
use vibe_orchestrator::machine_operation::fixed_package::{
    DOMAIN_OWNER, FixedPackageHost, OPERATION_ID,
};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{fixed_package_action_error as err, fixed_package_action_reply as rep},
};

mod evidence;
mod negative;
mod positive;

const FORMATS: [FormatId; 5] = [
    FormatId::MachineSessionFrame,
    FormatId::MachineOperationEnvelope,
    FormatId::MachineFixedPackageActionRequest,
    FormatId::MachineFixedPackageActionReply,
    FormatId::MachineFixedPackageActionError,
];
struct Fixture {
    dir: tempfile::TempDir,
    ns: SubmissionNamespace,
    scope: SubmissionScope,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("source.bin"), b"copy once").unwrap();
        let scope = SubmissionScope {
            context: "host-context".into(),
            target: "launcher".into(),
            principal: "host-principal".into(),
        };
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        let ns = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        Self { dir, ns, scope }
    }
    fn host(&self) -> Arc<FixedPackageHost> {
        Arc::new(
            FixedPackageHost::from_trusted_host(
                Arc::new(vibe_safefs::Project::open(self.dir.path()).unwrap()),
                self.ns.clone(),
                self.scope.clone(),
            )
            .unwrap(),
        )
    }
    fn route(&self) -> Arc<FixedPackageRoute> {
        Arc::new(FixedPackageRoute::new(self.host()))
    }
    fn context(&self, route: Arc<FixedPackageRoute>) -> ServerContext {
        ServerContext::with_store_root(self.dir.path(), self.dir.path().join("unused"))
            .with_fixed_package_route(route)
            .unwrap()
    }
    fn client(&self, route: Arc<FixedPackageRoute>) -> Client {
        Client::new(PackageMachineSessionRunner::new(self.context(route)).unwrap())
    }
    fn output(&self) -> PathBuf {
        self.dir
            .path()
            .join("target/vibe-package/launcher/launcher.bin")
    }
    fn result(&self, run: &str) -> PathBuf {
        self.dir
            .path()
            .join(".vibe/package-operations")
            .join(format!("{run}.static-file-result.json"))
    }
    fn no_copy(&self) {
        assert!(!self.output().exists());
        assert!(!self.dir.path().join(".vibe/package-operations").exists());
        let lease = LifecycleLease::acquire(self.dir.path()).unwrap();
        assert!(
            SubmissionReservations::inspect_existing_key(&lease, &self.ns, &self.scope, "copy")
                .unwrap()
                .is_none()
        );
    }
    fn one_run(&self, run: &str) {
        let lease = LifecycleLease::acquire(self.dir.path()).unwrap();
        let row =
            SubmissionReservations::inspect_existing_key(&lease, &self.ns, &self.scope, "copy")
                .unwrap()
                .unwrap();
        assert_eq!(row.domain_run_id, run);
        assert_eq!(row.journal_admission, JournalAdmission::Ready);
        assert_eq!(
            fs::read_dir(self.dir.path().join(".vibe/package-operations"))
                .unwrap()
                .count(),
            3
        );
    }
}
fn offer() -> f::SessionFrame {
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
                    epoch: 1,
                })
                .collect(),
            requested_capabilities: f::Capabilities {
                streaming: true,
                replay: true,
                control: true,
            },
        })),
    }
}
fn send(runner: &mut PackageMachineSessionRunner, frame: &f::SessionFrame) -> f::SessionFrame {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, frame).unwrap();
    let mut output = Vec::new();
    assert_eq!(
        runner.step(&mut Cursor::new(bytes), &mut output).unwrap(),
        RunnerStep::Replied
    );
    let mut output = Cursor::new(output);
    let response = read_frame(&mut output).unwrap().unwrap();
    assert!(
        read_frame(&mut output).unwrap().is_none(),
        "exactly one reply"
    );
    response
}
struct Client {
    runner: PackageMachineSessionRunner,
    session: String,
    context: String,
    target: String,
    next: usize,
}
impl Client {
    fn new(mut runner: PackageMachineSessionRunner) -> Self {
        let response = send(&mut runner, &offer());
        let f::Body::Initialized(body) = response.body else {
            panic!("initialized")
        };
        assert_eq!(
            body.selected_schemas,
            FORMATS
                .iter()
                .map(|id| f::SchemaOffer {
                    format_id: id.id().into(),
                    epoch: 1
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(
            body.actual_capabilities,
            f::Capabilities {
                streaming: false,
                replay: false,
                control: false
            }
        );
        assert_eq!(body.contexts.len(), 1);
        let context = &body.contexts[0];
        assert_eq!(context.kind, f::ContextKind::Workspace);
        assert_eq!(context.availability, f::ContextAvailability::Available);
        assert_eq!(context.target_ids.len(), 1);
        assert!(context.reason.is_none());
        let json = serde_json::to_string(&body).unwrap();
        assert!(!json.contains("host-context") && !json.contains("host-principal"));
        Self {
            runner,
            session: response.session_id.unwrap(),
            context: context.context_id.clone(),
            target: context.target_ids[0].clone(),
            next: 0,
        }
    }
    fn frame(&mut self, envelope_json: String) -> f::SessionFrame {
        self.next += 1;
        f::SessionFrame {
            schema: 1,
            session_id: Some(self.session.clone()),
            request_id: Some(format!("r-{}", self.next)),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: self.context.clone(),
                target_id: Some(self.target.clone()),
                envelope_json,
            })),
        }
    }
    fn raw(&mut self, envelope: String) -> Publication {
        let frame = self.frame(envelope);
        Publication::new(send(&mut self.runner, &frame))
    }
    fn call(&mut self, action: &str, key: &str) -> Publication {
        self.raw(envelope(action, key).to_string())
    }
}
fn envelope(action: &str, key: &str) -> Value {
    json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"input", "format_id":FormatId::MachineFixedPackageActionRequest.id(),
        "payload_json":json!({"schema":1,"action":{"kind":action,"key":key}}).to_string()}})
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
            _ => panic!("publication"),
        };
        let envelope: e::OperationEnvelope = serde_json::from_str(&bytes).unwrap();
        assert_eq!(super::outcome(&envelope).unwrap(), outcome);
        assert_eq!(envelope.operation_id.as_deref(), Some(OPERATION_ID));
        assert_eq!(envelope.domain_owner.as_deref(), Some(DOMAIN_OWNER));
        assert!(envelope.source_id.is_none() && envelope.evidence_id.is_none());
        Self {
            envelope,
            outcome,
            refused,
        }
    }
    fn reply(&self) -> rep::FixedPackageActionReply {
        assert!(!self.refused);
        let e::Body::Result(body) = &self.envelope.body else {
            panic!("result")
        };
        assert_eq!(
            body.format_id.as_deref(),
            Some(FormatId::MachineFixedPackageActionReply.id())
        );
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
    }
    fn error(&self) -> err::FixedPackageActionError {
        assert!(self.refused);
        let e::Body::Error(body) = &self.envelope.body else {
            panic!("error")
        };
        assert_eq!(
            body.format_id.as_deref(),
            Some(FormatId::MachineFixedPackageActionError.id())
        );
        let error: err::FixedPackageActionError =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        assert_eq!(error.known_run_id, self.outcome.provenance_id);
        error
    }
    fn completed(&self) -> String {
        let rep::Reply::Completed(reply) = self.reply().reply else {
            panic!("completed")
        };
        assert_eq!(self.outcome.effect_state, f::OutcomeEffectState::Completed);
        assert_eq!(self.outcome.completeness, f::OutcomeCompleteness::Complete);
        assert_eq!(
            self.outcome.provenance_id.as_deref(),
            Some(reply.run_id.as_str())
        );
        reply.run_id
    }
}
