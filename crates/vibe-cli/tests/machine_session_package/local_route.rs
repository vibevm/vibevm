//! CLI frame, direct local route and MCP process share one package grant.

use super::*;
use vibe_core::manifest::TargetOs;
use vibe_orchestrator::{
    machine_operation::fixed_package::FixedPackageRoute,
    machine_submission::package_grant::PackageGrantService,
};
use vibe_safefs::Project;

fn route(fixture: &Fixture) -> FixedPackageRoute {
    let service = PackageGrantService::from_trusted_host(
        Arc::new(Project::open(&fixture.root()).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::Linux,
    )
    .unwrap();
    FixedPackageRoute::new(Arc::new(service.reattach().unwrap()))
}

fn local(
    route: &FixedPackageRoute,
    root: &std::path::Path,
    kind: &str,
    key: &str,
) -> Result<Value, String> {
    let input = json!({
        "schema":1,
        "operation_id":vibe_orchestrator::machine_operation::fixed_package::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_package::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineFixedPackageActionRequest.id(),
            "payload_json":json!({"schema":1,"action":{"kind":kind,"key":key}}).to_string()}
    });
    let envelope =
        FixedPackageRoute::exchange_local(Some(route), input.to_string().as_bytes(), root)
            .map_err(|error| error.to_string())?;
    let value = serde_json::to_value(envelope).unwrap();
    Ok(serde_json::from_str(value["body"]["payload_json"].as_str().unwrap()).unwrap())
}

#[test]
fn cli_frame_and_local_route_race_then_mcp_cold_inspect_one_package_key() {
    let fixture = Fixture::new();
    Client::new(
        fixture
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    let pointer = fs::read(fixture.pointer()).unwrap();
    let mut machine = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
    let local_route = route(&fixture);
    let root = fixture.root();
    let key = "local-cli-race";
    let barrier = Barrier::new(3);
    let (machine_result, local_result) = thread::scope(|scope| {
        let machine_wait = &barrier;
        let local_wait = &barrier;
        let machine = scope.spawn(|| {
            machine_wait.wait();
            machine.call(json!({"kind":"submit","key":key}))
        });
        let local = scope.spawn(|| {
            local_wait.wait();
            local(&local_route, &root, "submit", key)
        });
        barrier.wait();
        (machine.join().unwrap(), local.join().unwrap())
    });
    machine.finish();
    let mut cold = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
    let inspected = cold.call(json!({"kind":"inspect","key":key}));
    let cold_envelope = envelope(&inspected);
    let cold_reply: Value =
        serde_json::from_str(cold_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(cold_reply["reply"]["kind"], "completed", "{cold_reply}");
    let run = cold_reply["reply"]["run_id"].as_str().unwrap();
    let local_cold = local(&route(&fixture), &root, "submit", key).unwrap();
    assert_eq!(local_cold, cold_reply);
    let mcp_cold = mcp_package_call(&fixture, "inspect", key);
    assert_eq!(mcp_cold, cold_reply);
    let mcp_replay = mcp_package_call(&fixture, "submit", key);
    assert_eq!(mcp_replay, cold_reply);
    let machine_replay = cold.call(json!({"kind":"submit","key":key}));
    let replay_envelope = envelope(&machine_replay);
    let replay: Value =
        serde_json::from_str(replay_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(replay, cold_reply);
    cold.finish();

    let machine_value = envelope(&machine_result);
    let machine_payload: Value =
        serde_json::from_str(machine_value["body"]["payload_json"].as_str().unwrap()).unwrap();
    if machine_value["body"]["stage"] == "result" {
        assert_eq!(machine_payload["reply"]["run_id"], run);
    }
    if let Ok(local_result) = &local_result {
        if local_result["reply"]["kind"] == "completed" {
            assert_eq!(local_result["reply"]["run_id"], run);
        }
    }
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
    assert_eq!(
        fs::read(root.join("target/vibe-package/launcher/launcher.bin")).unwrap(),
        b"launcher bytes\n"
    );
    let index: Value =
        serde_json::from_slice(&fs::read(root.join(".vibe/machine-submissions.json")).unwrap())
            .unwrap();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[key]["domain_run_id"], run);
    assert_eq!(
        fs::read_dir(root.join(".vibe/package-operations"))
            .unwrap()
            .count(),
        3
    );
}

#[test]
fn damaged_package_output_keeps_run_and_effect_truth_across_three_surfaces() {
    let fixture = Fixture::new();
    let key = "damaged-three-surfaces";
    let mut issue = Client::new(
        fixture
            .package_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    );
    let submitted = issue.call(json!({"kind":"submit","key":key}));
    let submitted_envelope = envelope(&submitted);
    let submitted_reply: Value =
        serde_json::from_str(submitted_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    let run = submitted_reply["reply"]["run_id"].as_str().unwrap();
    issue.finish();
    fs::remove_file(
        fixture
            .root()
            .join("target/vibe-package/launcher/launcher.bin"),
    )
    .unwrap();

    let local_view = local(&route(&fixture), &fixture.root(), "inspect", key).unwrap();
    assert_eq!(local_view["reply"]["kind"], "uncertain", "{local_view}");
    assert_eq!(local_view["reply"]["run_id"], run);
    assert_eq!(
        local_view["reply"]["partial"]["output_file"]["kind"],
        "absent"
    );
    assert_eq!(
        local_view["reply"]["partial"]["artifact_record"]["kind"],
        "present"
    );
    let mcp = mcp_package_response(mcp_package_process(&fixture), "inspect", key);
    assert_eq!(mcp["result"]["isError"], false, "{mcp}");
    assert_eq!(mcp["result"]["structuredContent"], local_view);

    let mut cli = Client::new(fixture.package_command("reattach", None).spawn().unwrap());
    let inspected = cli.call(json!({"kind":"inspect","key":key}));
    let inspected_envelope = envelope(&inspected);
    let cli_view: Value =
        serde_json::from_str(inspected_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(cli_view, local_view);
    assert_ne!(
        outcome(&inspected).effect_state,
        f::OutcomeEffectState::NoEffects
    );
    let replay = cli.call(json!({"kind":"submit","key":key}));
    assert_eq!(envelope(&replay), inspected_envelope);
    cli.finish();
    assert!(
        !fixture
            .root()
            .join("target/vibe-package/launcher/launcher.bin")
            .exists()
    );
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
    assert_eq!(keys[key]["domain_run_id"], run);
}
