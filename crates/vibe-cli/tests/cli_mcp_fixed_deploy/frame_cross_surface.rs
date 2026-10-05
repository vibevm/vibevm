use super::*;
use std::io::{Read, Write};
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{operation_envelope as e, session_frame as f},
};

const FORMATS: [FormatId; 5] = [
    FormatId::MachineSessionFrame,
    FormatId::MachineOperationEnvelope,
    FormatId::MachineFixedDeployActionRequest,
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
    let mut size = [0; 4];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut size)
        .unwrap();
    let len = u32::from_be_bytes(size) as usize;
    assert!((1..=8 * 1024 * 1024).contains(&len));
    let mut reply = vec![0; len];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut reply)
        .unwrap();
    let reply: f::SessionFrame = serde_json::from_slice(&reply).unwrap();
    assert_eq!(reply.request_id, frame.request_id);
    reply
}

fn frame_action(
    child: &mut std::process::Child,
    session: &str,
    context: &str,
    target: &str,
    kind: &str,
) -> (Value, f::Outcome) {
    let envelope = json!({
        "schema":1,
        "operation_id":vibe_orchestrator::machine_operation::fixed_deploy::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_deploy::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineFixedDeployActionRequest.id(),
            "payload_json":json!({"schema":1,"action":{"kind":kind,"key":"old-key"}}).to_string()}
    });
    let reply = exchange(
        child,
        f::SessionFrame {
            schema: 1,
            session_id: Some(session.into()),
            request_id: Some(kind.into()),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: context.into(),
                target_id: Some(target.into()),
                envelope_json: envelope.to_string(),
            })),
        },
    );
    let f::Body::Complete(body) = reply.body else {
        panic!("fixed Deploy frame: {reply:?}")
    };
    let published: e::OperationEnvelope =
        serde_json::from_str(body.envelope_json.as_deref().unwrap()).unwrap();
    let e::Body::Result(result) = published.body else {
        panic!("fixed Deploy result: {published:?}")
    };
    assert_eq!(
        result.format_id.as_deref(),
        Some(FormatId::MachineFixedDeployActionReply.id())
    );
    (
        serde_json::from_str(result.payload_json.as_deref().unwrap()).unwrap(),
        body.outcome,
    )
}

#[test]
fn cli_frame_real_deploy_replays_through_cold_mcp() {
    let fixture = Fixture::new();
    fixture.seed_artifact();
    let mut child = fixture
        .command()
        .args(["machine", "session", "--root"])
        .arg(fixture.root())
        .args([
            "--fixed-deploy",
            "issue",
            "--deploy-profile",
            "local",
            "--ttl-ms",
            "60000",
        ])
        .spawn()
        .unwrap();
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
        panic!("initialization: {initialized:?}")
    };
    assert_eq!(body.selected_schemas.len(), FORMATS.len());
    let context = &body.contexts[0];
    let session = initialized.session_id.as_deref().unwrap();
    let (reply, outcome) = frame_action(
        &mut child,
        session,
        &context.context_id,
        &context.target_ids[0],
        "submit",
    );
    assert_eq!(reply["reply"]["kind"], "completed", "{reply}");
    assert_eq!(
        serde_json::to_value(outcome.effect_state).unwrap(),
        "completed"
    );
    let run = reply["reply"]["run_id"].as_str().unwrap();
    let launcher = fs::read(fixture.dir.path().join("settings/bin/local-helper")).unwrap();
    assert!(launcher.starts_with(b"#!/bin/sh\n# vibevm-launcher"));
    let pointer = fs::read_dir(fixture.root().join(".vibe/cli-deploy-host-grants"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pointer_before = fs::read(&pointer).unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());

    let cold = mcp_reply(&fixture.mcp("reattach", false, &request("submit")));
    assert_eq!(cold["result"]["isError"], false, "{cold}");
    assert_eq!(cold["result"]["structuredContent"]["reply"], reply["reply"]);
    assert_eq!(cold["result"]["structuredContent"]["reply"]["run_id"], run);
    assert_eq!(
        fs::read_dir(fixture.root().join(".vibe/cli-deploy-host-grants"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(fs::read(&pointer).unwrap(), pointer_before);
    assert_eq!(
        fs::read(fixture.dir.path().join("settings/bin/local-helper")).unwrap(),
        launcher
    );
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    let keys = index["namespaces"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["old-key"]["domain_run_id"], run);
}
