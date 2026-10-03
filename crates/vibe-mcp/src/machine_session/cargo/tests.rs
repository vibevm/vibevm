//! Fixed Cargo frames use one host-granted route and the incumbent build journal.
use super::*;
use serde_json::{Value, json};
use std::{fs, io::Cursor};
use vibe_core::manifest::TargetOs;
use vibe_orchestrator::{
    machine_operation::fixed_cargo::{DOMAIN_OWNER, OPERATION_ID},
    machine_submission::cargo_grant::CargoGrantService,
};
use vibe_wire::generated::format_id::FormatId;

const FORMATS: [FormatId; 5] = [
    FormatId::MachineSessionFrame,
    FormatId::MachineOperationEnvelope,
    FormatId::MachineFixedCargoActionRequest,
    FormatId::MachineFixedCargoActionReply,
    FormatId::MachineFixedCargoActionError,
];
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n").unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        b"fn main() {\n    println!(\"r8\");\n}\n",
    )
    .unwrap();
    root
}
fn route(root: &std::path::Path) -> Arc<FixedCargoRoute> {
    let grant = CargoGrantService::from_trusted_builtin_host(
        Arc::new(vibe_safefs::Project::open(root).unwrap()),
        "shared-principal".into(),
        TargetOs::current().unwrap(),
    )
    .unwrap();
    Arc::new(FixedCargoRoute::new(Arc::new(grant.issue(60_000).unwrap())))
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
fn send(runner: &mut CargoMachineSessionRunner, frame: &f::SessionFrame) -> f::SessionFrame {
    let mut input = Vec::new();
    write_frame(&mut input, frame).unwrap();
    let mut output = Vec::new();
    assert_eq!(
        runner.step(&mut Cursor::new(input), &mut output).unwrap(),
        RunnerStep::Replied
    );
    read_frame(&mut Cursor::new(output)).unwrap().unwrap()
}
fn envelope(kind: &str, key: &str) -> String {
    json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"input",
        "format_id":FormatId::MachineFixedCargoActionRequest.id(),
        "payload_json":json!({"schema":1,"action":{"kind":kind,"key":key}}).to_string()}})
    .to_string()
}
struct Client {
    runner: CargoMachineSessionRunner,
    session: String,
    context: String,
    target: String,
    next: usize,
}
impl Client {
    fn new(mut runner: CargoMachineSessionRunner) -> Self {
        let reply = send(&mut runner, &offer());
        let f::Body::Initialized(body) = reply.body else {
            panic!("fixed Cargo initialized")
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
        assert!(
            !serde_json::to_string(context)
                .unwrap()
                .contains("shared-principal")
        );
        Self {
            runner,
            session: reply.session_id.unwrap(),
            context: context.context_id.clone(),
            target: context.target_ids[0].clone(),
            next: 0,
        }
    }
    fn call(&mut self, kind: &str, key: &str) -> f::SessionFrame {
        self.next += 1;
        let request = f::SessionFrame {
            schema: 1,
            session_id: Some(self.session.clone()),
            request_id: Some(format!("r-{}", self.next)),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: self.context.clone(),
                target_id: Some(self.target.clone()),
                envelope_json: envelope(kind, key),
            })),
        };
        send(&mut self.runner, &request)
    }
}
fn payload(frame: f::SessionFrame) -> (f::Outcome, Value) {
    let f::Body::Complete(body) = frame.body else {
        panic!("owner result")
    };
    let envelope: e::OperationEnvelope =
        serde_json::from_str(body.envelope_json.as_deref().unwrap()).unwrap();
    let e::Body::Result(row) = envelope.body else {
        panic!("typed Cargo reply")
    };
    (
        body.outcome,
        serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap(),
    )
}

#[test]
fn ungranted_and_foreign_outer_context_refuse_before_cargo() {
    let root = fixture();
    assert!(
        CargoMachineSessionRunner::new(ServerContext::with_store_root(
            root.path(),
            root.path().join("unused")
        ))
        .is_err()
    );
    let route = route(root.path());
    let context = ServerContext::with_store_root(root.path(), root.path().join("unused"))
        .with_fixed_cargo_route(route)
        .unwrap();
    let mut client = Client::new(CargoMachineSessionRunner::new(context).unwrap());
    let bad = f::SessionFrame {
        schema: 1,
        session_id: Some(client.session.clone()),
        request_id: Some("bad".into()),
        body: f::Body::Request(Box::new(f::BodyRequest {
            context_id: "foreign".into(),
            target_id: Some(client.target.clone()),
            envelope_json: envelope("submit", "once"),
        })),
    };
    assert!(matches!(
        send(&mut client.runner, &bad).body,
        f::Body::Refusal(_)
    ));
    assert!(!root.path().join("target").exists());
}

#[test]
fn completed_warning_and_damaged_owner_result_keep_typed_uncertainty() {
    let root = fixture();
    let route = route(root.path());
    let context = ServerContext::with_store_root(root.path(), root.path().join("unused"))
        .with_fixed_cargo_route(route)
        .unwrap();
    let mut client = Client::new(CargoMachineSessionRunner::new(context).unwrap());
    let (first_outcome, first) = payload(client.call("submit", "once"));
    assert_eq!(first_outcome.effect_state, f::OutcomeEffectState::Completed);
    assert_eq!(first["reply"]["kind"], "completed");
    assert!(
        first["reply"]["warning"]
            .as_str()
            .unwrap()
            .contains("rustup")
    );
    let run = first["reply"]["run_id"].as_str().unwrap();
    let retained = root
        .path()
        .join(format!(".vibe/build-operations/{run}.cargo-result.json"));
    assert!(retained.exists());
    let (_, replay) = payload(client.call("submit", "once"));
    assert_eq!(replay, first);
    fs::remove_file(retained).unwrap();
    let (outcome, uncertain) = payload(client.call("inspect", "once"));
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Uncertain);
    assert_eq!(uncertain["reply"]["kind"], "uncertain");
    let (_, retry) = payload(client.call("submit", "once"));
    assert_eq!(retry, uncertain);
}
