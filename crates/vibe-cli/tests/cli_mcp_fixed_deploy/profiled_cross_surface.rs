use super::*;
use std::io::{Read, Write};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{operation_envelope as e, session_frame as f},
};

const FORMATS: [FormatId; 6] = [
    FormatId::MachineSessionFrame,
    FormatId::MachineOperationEnvelope,
    FormatId::MachineFixedDeployActionRequest,
    FormatId::MachineProfiledDeployActionRequest,
    FormatId::MachineFixedDeployActionReply,
    FormatId::MachineFixedDeployActionError,
];

fn exchange(child: &mut std::process::Child, frame: f::SessionFrame) -> f::SessionFrame {
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
    let size = u32::from_be_bytes(length) as usize;
    assert!((1..=8 * 1024 * 1024).contains(&size));
    let mut reply = vec![0; size];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut reply)
        .unwrap();
    serde_json::from_slice(&reply).unwrap()
}

fn connect(fixture: &Fixture) -> (std::process::Child, String, String, String) {
    let mut command = fixture.command();
    command
        .args(["machine", "session", "--root"])
        .arg(fixture.root())
        .args([
            "--fixed-deploy",
            "issue",
            "--deploy-profile",
            "local",
            "--deploy-allow-profile",
            "alt",
            "--ttl-ms",
            "60000",
        ]);
    let mut child = command.spawn().unwrap();
    let initialized = exchange(
        &mut child,
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
    let f::Body::Initialized(body) = initialized.body else {
        panic!("profiled initialization refused: {initialized:?}")
    };
    assert_eq!(body.selected_schemas.len(), FORMATS.len());
    let context = &body.contexts[0];
    (
        child,
        initialized.session_id.unwrap(),
        context.context_id.clone(),
        context.target_ids[0].clone(),
    )
}

fn submit_profiled(
    child: &mut std::process::Child,
    session: &str,
    context: &str,
    target: &str,
    key: &str,
    profile: &str,
) -> (e::OperationEnvelope, f::Outcome) {
    let envelope = json!({"schema":1,
        "operation_id":vibe_orchestrator::machine_operation::fixed_deploy::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_deploy::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineProfiledDeployActionRequest.id(),
            "payload_json":json!({"schema":1,"action":{"kind":"submit","key":key,
                "deploy_profile":profile}}).to_string()}});
    let response = exchange(
        child,
        f::SessionFrame {
            schema: 1,
            session_id: Some(session.into()),
            request_id: Some("profiled-submit".into()),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: context.into(),
                target_id: Some(target.into()),
                envelope_json: envelope.to_string(),
            })),
        },
    );
    let f::Body::Complete(body) = response.body else {
        panic!("profiled frame refused: {response:?}")
    };
    (
        serde_json::from_str(body.envelope_json.as_deref().unwrap()).unwrap(),
        body.outcome,
    )
}

#[test]
fn cli_frame_and_mcp_share_one_profiled_deploy_owner() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root().join("vibe.toml"),
        manifest("\n[deploy.profiles.alt]\ntargets=['local-helper']\n"),
    )
    .unwrap();
    fixture.seed_artifact();
    let (mut child, session, context, target) = connect(&fixture);
    let (published, outcome) = submit_profiled(
        &mut child,
        &session,
        &context,
        &target,
        "profiled-once",
        "alt",
    );
    let e::Body::Result(row) = published.body else {
        panic!("profiled frame did not publish result: {published:?}")
    };
    let frame_reply: Value = serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    let run = frame_reply["reply"]["run_id"].as_str().unwrap();
    let receipt: Value =
        serde_json::from_str(frame_reply["reply"]["receipt_json"].as_str().unwrap()).unwrap();
    assert_eq!(receipt["profile"], "alt");
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Completed);
    drop(child.stdin.take());
    assert!(child.wait_with_output().unwrap().status.success());

    let cold = mcp_reply(&fixture.mcp_allowed(
        "reattach",
        false,
        &profiled_request("profiled-once", "alt"),
    ));
    assert_eq!(cold["result"]["isError"], false, "{cold}");
    assert_eq!(cold["result"]["structuredContent"], frame_reply);
    assert_eq!(cold["result"]["structuredContent"]["reply"]["run_id"], run);
    let wrong_host_set = fixture.mcp("reattach", false, &request_key("inspect", "profiled-once"));
    assert!(!wrong_host_set.status.success());
    let changed =
        mcp_reply(&fixture.mcp_allowed("reattach", false, &request_key("submit", "profiled-once")));
    assert_eq!(changed["result"]["isError"], true, "{changed}");
    let foreign = mcp_reply(&fixture.mcp_allowed(
        "reattach",
        false,
        &profiled_request("foreign-key", "foreign"),
    ));
    assert_eq!(foreign["result"]["isError"], true, "{foreign}");
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
    assert_eq!(keys["profiled-once"]["domain_run_id"], run);

    let mut drifted = fs::read_to_string(fixture.root().join("vibe.toml")).unwrap();
    drifted.push_str("\n# changed after profiled effect\n");
    fs::write(fixture.root().join("vibe.toml"), drifted).unwrap();
    let cold_inspect = mcp_reply(&fixture.mcp_allowed(
        "reattach",
        false,
        &request_key("inspect", "profiled-once"),
    ));
    assert_eq!(cold_inspect["result"]["isError"], false, "{cold_inspect}");
    assert_eq!(cold_inspect["result"]["structuredContent"], frame_reply);
    let cold_submit = mcp_reply(&fixture.mcp_allowed(
        "reattach",
        false,
        &profiled_request("profiled-once", "alt"),
    ));
    assert_eq!(cold_submit["result"]["isError"], true, "{cold_submit}");
    assert_eq!(
        cold_submit["result"]["structuredContent"]["known_run_id"],
        run
    );
}
