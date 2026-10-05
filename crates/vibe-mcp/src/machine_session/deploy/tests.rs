//! Fixed Deploy owner truth through the bounded session frame.
use super::*;
use serde_json::json;
use std::{fs, io::Cursor, path::Path};
use vibe_core::manifest::{DeployTarget, ExtensionConfig, ExtensionsControl, MechanismRoutes};
use vibe_extension_registry::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, MechanismRegistry,
    collect_mechanisms,
};
use vibe_lifecycle::process::SelectedFileWitness;
use vibe_lifecycle::{ClientExecutable, ClientExecutables, SubmissionScope};
use vibe_orchestrator::machine_submission::{
    TrustedPolicySource,
    deploy_grant::{DeployGrantAttachment, DeployGrantService},
};
use vibe_safefs::Project;
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{fixed_deploy_action_error as err, fixed_deploy_action_reply as rep},
};
use vibe_wire::{behaviour::artifact_record::validate, generated::artifact_record::ArtifactRecord};

fn registry() -> Arc<MechanismRegistry> {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("deploy-grant"),
                root: ".".into(),
                version: "1.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    Arc::new(collect_mechanisms(&world).unwrap())
}
fn clients() -> ClientExecutables {
    ClientExecutables {
        claude: ClientExecutable::Missing {
            command: "claude".into(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".into(),
        },
    }
}
fn target() -> DeployTarget {
    DeployTarget {
        id: "local-helper".into(),
        artifact: "helper.exe".into(),
        mechanism: "deploy:vibe-bin".parse().unwrap(),
        provider: None,
        when: None,
        depends_on: Some(Vec::new()),
        config: Some(ExtensionConfig::from_table(
            "command = 'local-helper'".parse::<toml::Table>().unwrap(),
        )),
    }
}
fn record(root: &Path, artifact: &Path) {
    let digest = SelectedFileWitness::from_bytes(fs::read(artifact).unwrap()).sha256;
    let record: ArtifactRecord = serde_json::from_value(json!({
        "schema":1,"id":"helper.exe","kind":"executable","shape":"file",
        "path_absolute":artifact.to_string_lossy().replace('\\', "/"),
        "path_relative":{"root":"project","path":"target/debug/helper.exe"},
        "digest":{"algorithm":"sha256","value":digest.trim_start_matches("sha256:")},
        "producer":{"target":"helper","mechanism":"build:cargo",
            "provider":{"key":"org.vibevm/vibe#cargo"}},
        "freshness":{},"created_at":"2026-10-05T12:00:00Z",
        "verification":{"status":"verified","evidence":"fixture artifact"}
    }))
    .unwrap();
    validate(&record).unwrap();
    let home = root.join(".vibe/state/artifacts");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join("helper.exe.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
}
struct Fixture {
    project: tempfile::TempDir,
    settings: tempfile::TempDir,
    home: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let settings = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("vibe.toml"),
            concat!(
                "[project]\nname = 'demo'\nversion = '1.1.0'\n\n",
                "[[artifacts.build]]\nid = 'helper'\nmechanism = 'build:cargo'\n",
                "outputs = [{ id = 'helper.exe', kind = 'executable' }]\n\n",
                "[[deploy.target]]\nid = 'local-helper'\nartifact = 'helper.exe'\n",
                "mechanism = 'deploy:vibe-bin'\ndepends_on = []\n",
                "config = { command = 'local-helper' }\n\n",
                "[deploy.profiles.local]\ntargets = ['local-helper']\n",
            ),
        )
        .unwrap();
        let artifact = project.path().join("target/debug/helper.exe");
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, b"helper executable").unwrap();
        record(project.path(), &artifact);
        Self {
            project,
            settings,
            home,
        }
    }
    fn service(&self) -> DeployGrantService {
        DeployGrantService::from_trusted_host(
            Arc::new(Project::open(self.project.path()).unwrap()),
            Arc::new(Project::open(self.settings.path()).unwrap()),
            Arc::new(Project::open(self.home.path()).unwrap()),
            SubmissionScope {
                context: "cli-deploy-host:e1:trusted".into(),
                target: "instance-a".into(),
                principal: "owner".into(),
            },
            "local".into(),
            target(),
            registry(),
            Arc::new(MechanismRoutes::default()),
            clients(),
            "org.example/demo".into(),
            Some("generation-a".into()),
            "observer-a".into(),
            "nonce-a".into(),
            1,
            Arc::new(|| Ok(Some("generation-a".into()))),
            TrustedPolicySource::from_host_identifier("vibe-cli/fixed-deploy/e1".into()).unwrap(),
        )
        .unwrap()
    }
}

const FORMATS: [FormatId; 5] = [
    FormatId::MachineSessionFrame,
    FormatId::MachineOperationEnvelope,
    FormatId::MachineFixedDeployActionRequest,
    FormatId::MachineFixedDeployActionReply,
    FormatId::MachineFixedDeployActionError,
];
fn connect(
    fixture: &Fixture,
    route: Arc<FixedDeployRoute>,
) -> (DeployMachineSessionRunner, String, String, String) {
    let context = ServerContext::with_store_root(
        fixture.project.path(),
        fixture.project.path().join("unused"),
    )
    .with_fixed_deploy_route(route)
    .unwrap();
    let mut runner = DeployMachineSessionRunner::new(context).unwrap();
    let offer = f::SessionFrame {
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
    };
    let initialized = send(&mut runner, &offer);
    let f::Body::Initialized(body) = initialized.body else {
        panic!("initialization refused")
    };
    assert_eq!(body.selected_schemas.len(), FORMATS.len());
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
    assert_eq!(context.target_ids.len(), 1);
    (
        runner,
        initialized.session_id.unwrap(),
        context.context_id.clone(),
        context.target_ids[0].clone(),
    )
}
fn send(runner: &mut DeployMachineSessionRunner, frame: &f::SessionFrame) -> f::SessionFrame {
    let mut input = Vec::new();
    write_frame(&mut input, frame).unwrap();
    let mut output = Vec::new();
    assert_eq!(
        runner.step(&mut Cursor::new(input), &mut output).unwrap(),
        RunnerStep::Replied
    );
    let mut output = Cursor::new(output);
    let reply = read_frame(&mut output).unwrap().unwrap();
    assert!(read_frame(&mut output).unwrap().is_none());
    reply
}
fn call(
    runner: &mut DeployMachineSessionRunner,
    session: &str,
    context: &str,
    target: &str,
    kind: &str,
    key: &str,
) -> (e::OperationEnvelope, f::Outcome, bool) {
    let payload = json!({"schema":1,"action":{"kind":kind,"key":key}}).to_string();
    let envelope = json!({"schema":1,"operation_id":vibe_orchestrator::machine_operation::fixed_deploy::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_deploy::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineFixedDeployActionRequest.id(),"payload_json":payload}}).to_string();
    let frame = f::SessionFrame {
        schema: 1,
        session_id: Some(session.into()),
        request_id: Some(format!("{kind}-{key}")),
        body: f::Body::Request(Box::new(f::BodyRequest {
            context_id: context.into(),
            target_id: Some(target.into()),
            envelope_json: envelope,
        })),
    };
    let response = send(runner, &frame);
    let (payload, outcome, refused) = match response.body {
        f::Body::Complete(row) => (row.envelope_json.unwrap(), row.outcome, false),
        f::Body::Refusal(row) => (row.envelope_json.unwrap(), row.outcome, true),
        other => panic!("unexpected frame: {other:?}"),
    };
    let envelope: e::OperationEnvelope = serde_json::from_str(&payload).unwrap();
    assert_eq!(outcome, super::outcome(&envelope).unwrap());
    (envelope, outcome, refused)
}
#[test]
fn active_inspect_returns_retained_completed_run() {
    let fixture = Fixture::new();
    let host = fixture.service().issue(60_000).unwrap();
    let completed = host.submit("once").unwrap();
    let vibe_lifecycle::retained_mechanism::DeployInspection::Completed { run_id, .. } = completed
    else {
        panic!("owner did not complete")
    };
    let route = Arc::new(FixedDeployRoute::new(Arc::new(host)));
    let (mut runner, session, context, target) = connect(&fixture, route);
    let (envelope, outcome, refused) =
        call(&mut runner, &session, &context, &target, "inspect", "once");
    assert!(!refused);
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Completed);
    assert_eq!(outcome.completeness, f::OutcomeCompleteness::Complete);
    assert_eq!(outcome.provenance_id.as_deref(), Some(run_id.as_str()));
    let e::Body::Result(row) = envelope.body else {
        panic!("no result")
    };
    let reply: rep::FixedDeployActionReply =
        serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    assert!(matches!(reply.reply, rep::Reply::Completed(view) if view.run_id == run_id));
}

#[test]
fn active_submit_uses_owner_and_inspect_replays_its_run() {
    let fixture = Fixture::new();
    let host = fixture.service().issue(60_000).unwrap();
    let route = Arc::new(FixedDeployRoute::new(Arc::new(host)));
    let (mut runner, session, context, target) = connect(&fixture, route);
    let (submitted, effect, refused) =
        call(&mut runner, &session, &context, &target, "submit", "once");
    assert!(!refused);
    assert_eq!(effect.effect_state, f::OutcomeEffectState::Completed);
    assert_eq!(effect.completeness, f::OutcomeCompleteness::Complete);
    let run_id = effect.provenance_id.unwrap();
    let e::Body::Result(row) = submitted.body else {
        panic!("no owner result")
    };
    let reply: rep::FixedDeployActionReply =
        serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    assert!(matches!(reply.reply, rep::Reply::Completed(view) if view.run_id == run_id));

    let (inspected, replay, refused) =
        call(&mut runner, &session, &context, &target, "inspect", "once");
    assert!(!refused);
    assert!(matches!(inspected.body, e::Body::Result(_)));
    assert_eq!(replay.effect_state, f::OutcomeEffectState::Completed);
    assert_eq!(replay.provenance_id.as_deref(), Some(run_id.as_str()));
}

#[test]
fn recovery_only_submit_refuses_and_preserves_landed_run() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let host = service.issue(60_000).unwrap();
    let vibe_lifecycle::retained_mechanism::DeployInspection::Completed { run_id, .. } =
        host.submit("once").unwrap()
    else {
        panic!("owner did not complete")
    };
    let manifest = fixture.project.path().join("vibe.toml");
    let mut source = fs::read_to_string(&manifest).unwrap();
    source.push_str("# drift after retained effect\n");
    fs::write(manifest, source).unwrap();
    let DeployGrantAttachment::RecoveryOnly(host) = fixture.service().reattach().unwrap() else {
        panic!("expected recovery only")
    };
    let route = Arc::new(FixedDeployRoute::recovery_only(Arc::new(host)));
    let (mut runner, session, context, target) = connect(&fixture, route);
    let (inspected, truth, refused) =
        call(&mut runner, &session, &context, &target, "inspect", "once");
    assert!(!refused);
    assert_eq!(truth.provenance_id.as_deref(), Some(run_id.as_str()));
    assert!(matches!(inspected.body, e::Body::Result(_)));
    let (envelope, outcome, refused) =
        call(&mut runner, &session, &context, &target, "submit", "once");
    assert!(refused);
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Completed);
    assert_eq!(outcome.provenance_id.as_deref(), Some(run_id.as_str()));
    let e::Body::Error(row) = envelope.body else {
        panic!("no typed refusal")
    };
    let error: err::FixedDeployActionError =
        serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(error.known_run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(
        error.effect_observation,
        err::FixedDeployActionErrorEffectObservation::Landed
    );
    let (fresh, fresh_outcome, refused) =
        call(&mut runner, &session, &context, &target, "submit", "fresh");
    assert!(refused);
    assert!(matches!(fresh.body, e::Body::Error(_)));
    assert_eq!(fresh_outcome.effect_state, f::OutcomeEffectState::NoEffects);
}
