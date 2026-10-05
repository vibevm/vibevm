//! CLI frame and direct local Cargo route share an issued owner namespace.

use super::*;
use vibe_safefs::Project;

fn route(fixture: &Fixture) -> FixedCargoRoute {
    let service = CargoGrantService::from_trusted_builtin_host(
        Arc::new(Project::open(&fixture.root()).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::current().unwrap(),
    )
    .unwrap();
    FixedCargoRoute::new(Arc::new(service.reattach().unwrap()))
}

fn local(route: &FixedCargoRoute, root: &Path, kind: &str, key: &str) -> Result<Value, String> {
    let input = json!({
        "schema":1,
        "operation_id":vibe_orchestrator::machine_operation::fixed_cargo::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::fixed_cargo::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineFixedCargoActionRequest.id(),
            "payload_json":json!({"schema":1,"action":{"kind":kind,"key":key}}).to_string()}
    });
    let envelope = FixedCargoRoute::exchange_local(Some(route), input.to_string().as_bytes(), root)
        .map_err(|error| error.to_string())?;
    let value = serde_json::to_value(envelope).unwrap();
    Ok(serde_json::from_str(value["body"]["payload_json"].as_str().unwrap()).unwrap())
}

#[test]
fn cli_frame_and_direct_local_route_share_one_cargo_run_then_mcp_inspects() {
    let fixture = Fixture::new();
    Client::new(
        fixture
            .cargo_command("issue", Some("60000"))
            .spawn()
            .unwrap(),
    )
    .finish();
    let pointer = fs::read(fixture.pointer()).unwrap();
    let mut cli = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    let active_route = route(&fixture);
    let root = fixture.root();
    let key = "direct-local-cargo";
    let barrier = Barrier::new(3);
    let (cli_result, local_result) = thread::scope(|scope| {
        let cli_wait = &barrier;
        let local_wait = &barrier;
        let cli_task = scope.spawn(|| {
            cli_wait.wait();
            cli.call(json!({"kind":"submit","key":key}))
        });
        let local_task = scope.spawn(|| {
            local_wait.wait();
            local(&active_route, &root, "submit", key)
        });
        barrier.wait();
        (cli_task.join().unwrap(), local_task.join().unwrap())
    });
    cli.finish();
    let mut cold = Client::new(fixture.cargo_command("reattach", None).spawn().unwrap());
    let inspected = cold.call(json!({"kind":"inspect","key":key}));
    let inspected_envelope = envelope(&inspected);
    let owner: Value =
        serde_json::from_str(inspected_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    assert_eq!(owner["reply"]["kind"], "completed", "{owner}");
    assert!(
        owner["reply"]["warning"]
            .as_str()
            .unwrap()
            .contains("rustup")
    );
    let run = owner["reply"]["run_id"].as_str().unwrap();
    assert_eq!(
        local(&route(&fixture), &root, "submit", key).unwrap(),
        owner
    );
    let mcp = mcp_cargo_submit(mcp_cargo_context(&root), key);
    assert_eq!(mcp["result"]["isError"], false, "{mcp}");
    assert_eq!(mcp["result"]["structuredContent"], owner);
    let replay = cold.call(json!({"kind":"submit","key":key}));
    assert_eq!(envelope(&replay), inspected_envelope);
    cold.finish();

    let cli_envelope = envelope(&cli_result);
    let cli_payload: Value =
        serde_json::from_str(cli_envelope["body"]["payload_json"].as_str().unwrap()).unwrap();
    if cli_envelope["body"]["stage"] == "result" {
        assert_eq!(cli_payload["reply"]["run_id"], run);
    }
    if let Ok(local_result) = local_result {
        if local_result["reply"]["kind"] == "completed" {
            assert_eq!(local_result["reply"]["run_id"], run);
        }
    }
    let results = fs::read_dir(root.join(".vibe/build-operations"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".cargo-result.json"))
        .collect::<Vec<_>>();
    assert_eq!(
        results,
        [root.join(format!(".vibe/build-operations/{run}.cargo-result.json"))]
    );
    assert!(
        root.join(owner["reply"]["output_relative"].as_str().unwrap())
            .is_file()
    );
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer);
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
}
