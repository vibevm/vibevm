//! A cold trusted library client joins the CLI and MCP fixed Deploy owner.
use super::*;
use std::sync::Arc;
use vibe_core::manifest::Manifest;
use vibe_lifecycle::{
    process::SelectedFileWitness, ClientExecutable, ClientExecutables, SubmissionScope,
};
use vibe_orchestrator::{
    machine_operation::fixed_deploy::FixedDeployRoute,
    machine_submission::{
        deploy_grant::{DeployGrantAttachment, DeployGrantService},
        TrustedPolicySource,
    },
};
use vibe_safefs::Project;
use vibe_wire::generated::{format_id::FormatId, machine::e1::operation_envelope as e};

fn route(fixture: &Fixture) -> FixedDeployRoute {
    let root = fixture.root();
    let project = Arc::new(Project::open(&root).unwrap());
    let token = project.root_correlation_token().unwrap();
    let manifest = Manifest::read(root.join(Manifest::FILENAME)).unwrap();
    let target = manifest.deploy.as_ref().unwrap().targets[0].clone();
    let loaded = vibe_orchestrator::inspect(&root).unwrap();
    let missing = |command: &str| ClientExecutable::Missing {
        command: command.into(),
    };
    let service = DeployGrantService::from_trusted_host(
        project,
        Arc::new(Project::open(&fixture.dir.path().join("settings")).unwrap()),
        Arc::new(Project::open(&fixture.dir.path().join("home")).unwrap()),
        SubmissionScope {
            context: "vibe-cli-project-local/e1".into(),
            target: format!("physical-root:{token}:deploy-profile:local"),
            principal: "cooperating-project-cli".into(),
        },
        "local".into(),
        target,
        Arc::new(loaded.mechanisms),
        Arc::new(loaded.mechanism_routes),
        ClientExecutables {
            claude: missing("claude"),
            codex: missing("codex"),
            opencode: missing("opencode"),
        },
        manifest.project.as_ref().unwrap().name.clone(),
        None,
        "project-local-no-generation/e1".into(),
        SelectedFileWitness::from_bytes(
            format!("vibevm/project-local-fixed-deploy/e1\0{token}\0local").into_bytes(),
        )
        .sha256,
        1,
        Arc::new(|| Ok(None)),
        TrustedPolicySource::from_host_identifier("vibe-cli/fixed-deploy/e1".into()).unwrap(),
    )
    .unwrap();
    match service.reattach().unwrap() {
        DeployGrantAttachment::Active(host) => FixedDeployRoute::new(Arc::new(host)),
        DeployGrantAttachment::RecoveryOnly(host) => {
            FixedDeployRoute::recovery_only(Arc::new(host))
        }
    }
}

fn local_envelope(fixture: &Fixture, kind: &str, key: &str) -> e::OperationEnvelope {
    let input = json!({
        "schema":1,
        "operation_id":vibe_orchestrator::machine_operation::fixed_deploy::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_deploy::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineFixedDeployActionRequest.id(),
            "payload_json":json!({"schema":1,"action":{"kind":kind,"key":key}}).to_string()}
    });
    FixedDeployRoute::exchange_local(
        Some(&route(fixture)),
        input.to_string().as_bytes(),
        &fixture.root(),
    )
    .unwrap()
}

fn local(fixture: &Fixture, kind: &str, key: &str) -> Value {
    let envelope = local_envelope(fixture, kind, key);
    let e::Body::Result(result) = envelope.body else {
        panic!("expected fixed Deploy result: {envelope:?}")
    };
    serde_json::from_str(result.payload_json.as_deref().unwrap()).unwrap()
}

#[test]
fn damaged_deploy_result_has_same_partial_truth_in_local_cli_and_mcp() {
    let fixture = Fixture::new();
    fixture.seed_artifact();
    let first = mcp_reply(&fixture.mcp("issue", true, &request_key("submit", "partial-key")));
    assert_eq!(first["result"]["isError"], false, "{first}");
    let run = first["result"]["structuredContent"]["reply"]["run_id"]
        .as_str()
        .unwrap();
    let launcher_path = fixture.dir.path().join("settings/bin/local-helper");
    let launcher_before = fs::read(&launcher_path).unwrap();
    let pointer = fs::read_dir(fixture.root().join(".vibe/cli-deploy-host-grants"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pointer_before = fs::read(&pointer).unwrap();
    let result = fixture.root().join(format!(
        ".vibe/deploy-operations/{run}.bound-vibe-bin-result.json"
    ));
    assert!(result.exists());
    fs::remove_file(&result).unwrap();

    let local_view = local(&fixture, "inspect", "partial-key");
    assert_eq!(local_view["reply"]["kind"], "uncertain", "{local_view}");
    assert_eq!(local_view["reply"]["run_id"], run);
    for (field, expected) in [
        ("intent", "absent"),
        ("receipt", "present"),
        ("sidecar", "present"),
    ] {
        assert_eq!(local_view["reply"]["partial"][field]["kind"], expected);
    }
    assert_eq!(local(&fixture, "submit", "partial-key"), local_view);
    let mcp = mcp_reply(&fixture.mcp("reattach", false, &request_key("submit", "partial-key")));
    assert_eq!(mcp["result"]["isError"], false, "{mcp}");
    assert_eq!(mcp["result"]["structuredContent"], local_view);

    let (mut child, session, context, target) =
        frame_cross_surface::connect_mode(&fixture, "reattach");
    let (cli, outcome) = frame_cross_surface::raw_frame_action(
        &mut child,
        &session,
        &context,
        &target,
        "submit",
        "partial-key",
    );
    let e::Body::Result(row) = cli.body else {
        panic!("fixed Deploy partial replay: {cli:?}")
    };
    let cli_view: Value = serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(cli_view, local_view);
    assert_eq!(
        serde_json::to_value(outcome.effect_state).unwrap(),
        "uncertain"
    );
    drop(child.stdin.take());
    assert!(child.wait_with_output().unwrap().status.success());

    assert!(!result.exists());
    assert_eq!(fs::read(&pointer).unwrap(), pointer_before);
    assert_eq!(fs::read(&launcher_path).unwrap(), launcher_before);
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["partial-key"]["domain_run_id"], run);
}

#[test]
fn cli_mcp_and_local_api_replay_one_fixed_deploy_result() {
    let fixture = Fixture::new();
    fixture.seed_artifact();
    let issued = fixture.machine("issue", true);
    assert!(
        issued.status.success(),
        "{}",
        String::from_utf8_lossy(&issued.stderr)
    );

    let first = local(&fixture, "submit", "one-key");
    assert_eq!(first["reply"]["kind"], "completed", "{first}");
    let run = first["reply"]["run_id"].as_str().unwrap();
    let launcher = fs::read(fixture.dir.path().join("settings/bin/local-helper")).unwrap();
    let pointer = fs::read_dir(fixture.root().join(".vibe/cli-deploy-host-grants"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pointer_before = fs::read(&pointer).unwrap();

    let mcp = mcp_reply(&fixture.mcp("reattach", false, &request_key("submit", "one-key")));
    assert_eq!(mcp["result"]["isError"], false, "{mcp}");
    assert_eq!(mcp["result"]["structuredContent"], first);
    let (mut child, session, context, target) =
        frame_cross_surface::connect_mode(&fixture, "reattach");
    let (cli, outcome) = frame_cross_surface::raw_frame_action(
        &mut child, &session, &context, &target, "submit", "one-key",
    );
    let e::Body::Result(result) = cli.body else {
        panic!("fixed Deploy CLI replay: {cli:?}")
    };
    let cli: Value = serde_json::from_str(result.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(cli, first);
    assert_eq!(
        serde_json::to_value(outcome.effect_state).unwrap(),
        "completed"
    );
    drop(child.stdin.take());
    assert!(child.wait_with_output().unwrap().status.success());
    assert_eq!(local(&fixture, "inspect", "one-key"), first);

    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["one-key"]["domain_run_id"], run);
    assert_eq!(fs::read(&pointer).unwrap(), pointer_before);
    assert_eq!(
        fs::read(fixture.dir.path().join("settings/bin/local-helper")).unwrap(),
        launcher
    );
}
