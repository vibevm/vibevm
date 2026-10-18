use super::*;
use vibe_orchestrator::machine_operation::boot_debugger::input;
use vibe_wire::generated::{
    boot_debugger::e1::result as r, machine::e1::operation_envelope as env,
};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/inspection_route_fixture.rs"
));

#[test]
fn ordinary_lazy_host_carries_explicit_catalogue_through_registered_tools() {
    use vibe_wire::generated::boot_debugger::e1::request as q;
    for family in [
        "commonmark",
        "xml-data",
        "vibe-spec",
        "opaque-file",
        "nested-tree",
    ] {
        let f = InspectionFixture::new(family);
        // Configure the ordinary host only. No prebuilt route is injected.
        let context = ServerContext::with_store_root(
            f.source.output.root_path(),
            f.source._home.path().join("unused"),
        )
        .with_source_derivatives(f.catalog.clone());
        let tools = crate::tools::default_tools();
        let tool = tools
            .iter()
            .find(|t| t.descriptor().name == "boot_debugger")
            .unwrap();
        let decode = |value: &Value| {
            let envelope: env::OperationEnvelope = serde_json::from_value(value.clone()).unwrap();
            let env::Body::Result(body) = envelope.body else {
                panic!()
            };
            assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
            serde_json::from_str::<r::Result>(body.payload_json.as_deref().unwrap()).unwrap()
        };
        let before = inspection_files(f.source._home.path());
        let capture = decode(tool.run(&json!({}), &context).unwrap().structured());
        let route = context.boot_debugger_route().unwrap();
        let request = serde_json::to_value(input(&q::Request {
            schema: 1,
            context_id: route.context_id().into(),
            command: q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: capture.snapshot_id.unwrap(),
            })),
        }))
        .unwrap();
        let actual = tool.run(&request, &context).unwrap();
        let expected = route.exchange(&serde_json::to_vec(&request).unwrap());
        assert_eq!(
            *actual.structured(),
            serde_json::to_value(expected).unwrap()
        );
        let r::Body::Inspect(view) = decode(actual.structured()).body else {
            panic!()
        };
        let records: Vec<vibe_wire::generated::derivative_result::DerivativeResult> =
            serde_json::from_str(view.derivative_catalog_json.as_deref().unwrap()).unwrap();
        assert_eq!(records, f.catalog.records());
        assert_eq!(inspection_files(f.source._home.path()), before);
        fs::remove_dir_all(f.source.base.root_path()).unwrap();
        fs::remove_dir_all(f.source.local.root_path()).unwrap();
        assert_eq!(
            tool.run(&request, &context).unwrap().structured(),
            actual.structured()
        );
        let dispatched = crate::dispatch_one(
            context,
            &json!({
                "jsonrpc":"2.0", "id":10, "method":"tools/call",
                "params":{"name":"boot_debugger", "arguments":request}
            })
            .to_string(),
        )
        .unwrap();
        let dispatched: Value = serde_json::from_str(&dispatched).unwrap();
        assert_eq!(
            dispatched["result"]["structuredContent"],
            *actual.structured()
        );
        assert_eq!(dispatched["result"]["isError"], false);
        assert!(!f.source._home.path().join("unused").exists());
    }
}

#[test]
fn registered_dispatcher_retains_all_families_with_exact_records_and_explicit_join_gaps() {
    for family in [
        "commonmark",
        "xml-data",
        "vibe-spec",
        "opaque-file",
        "nested-tree",
    ] {
        let f = InspectionFixture::new(family);
        let context = ServerContext::with_store_root(
            f.source.output.root_path(),
            f.source._home.path().join("unused"),
        )
        .with_boot_debugger_route(f.route.clone())
        .unwrap();
        let tool = crate::tools::default_tools()
            .into_iter()
            .find(|t| t.descriptor().name == "boot_debugger")
            .unwrap();
        let before = inspection_files(f.source._home.path());
        let request = input(&f.inspect());
        let expected = f.route.exchange(&serde_json::to_vec(&request).unwrap());
        let call = || {
            tool.run(&serde_json::to_value(&request).unwrap(), &context)
                .unwrap()
                .structured()
                .clone()
        };
        let original = serde_json::to_value(&expected).unwrap();
        let dispatched = crate::dispatch_one(
            ServerContext::with_store_root(f.source.output.root_path(),f.source._home.path().join("unused")).with_boot_debugger_route(f.route.clone()).unwrap(),
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"boot_debugger","arguments":request}}).to_string()).unwrap();
        let dispatched: Value = serde_json::from_str(&dispatched).unwrap();
        assert_eq!(dispatched["result"]["structuredContent"], original);
        assert_eq!(dispatched["result"]["isError"], false);
        assert_eq!(call(), original);
        let env::Body::Result(body) = expected.body else {
            panic!()
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        let result: r::Result =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        let r::Body::Inspect(view) = result.body else {
            panic!()
        };
        let records: Vec<vibe_wire::generated::derivative_result::DerivativeResult> =
            serde_json::from_str(view.derivative_catalog_json.as_deref().unwrap()).unwrap();
        assert_eq!(records, f.catalog.records());
        if family == "vibe-spec" {
            let joined: Vec<_> = view
                .sources
                .iter()
                .filter(|s| s.derivative_read.is_some())
                .collect();
            assert_eq!(joined.len(), 1);
            let source = joined[0];
            let occurrences: Vec<_> = view
                .nodes
                .iter()
                .filter(|n| n.source == source.source)
                .collect();
            assert_eq!(occurrences.len(), 2);
            assert_ne!(
                (occurrences[0].contribution, occurrences[0].occurrence),
                (occurrences[1].contribution, occurrences[1].occurrence)
            );
            assert!(source.authored_resource_id.is_some());
        } else {
            assert!(
                view.gaps
                    .iter()
                    .any(|g| g.contains("compiler occurrence joins are unavailable"))
            );
            assert!(
                view.sources
                    .iter()
                    .all(|s| s.authored_resource_id.is_none())
            );
        }
        let shown = f.catalog.inspect(&records[0].result_identity).unwrap();
        for file in &f.source.expected {
            assert_eq!(
                shown
                    .local
                    .iter()
                    .find(|r| r.path == file.path)
                    .unwrap()
                    .raw,
                file.bytes
            );
        }
        assert_eq!(inspection_files(f.source._home.path()), before);
        assert_eq!(
            f.provider_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        // Provider/source removal does not reopen, reconstruct or activate history.
        fs::remove_dir_all(f.source.base.root_path()).unwrap();
        fs::remove_dir_all(f.source.local.root_path()).unwrap();
        let before = inspection_files(f.source._home.path());
        assert_eq!(call(), original);
        assert_eq!(inspection_files(f.source._home.path()), before);
        assert_eq!(
            f.provider_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert!(!f.source._home.path().join("unused").exists());
        assert!(!f.source._home.path().join("DO_NOT_EXECUTE").exists());
    }
}
#[test]
fn registered_source_and_debugger_routes_share_exact_retained_record_without_proof_promotion() {
    let f = InspectionFixture::new("vibe-spec");
    let manifest = format!(
        "[package]\ngroup='org.example'\nname='local'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\n{}",
        toml::to_string(&f.source.binding.local().descriptor).unwrap()
    );
    fs::write(f.source.local.root_path().join("vibe.toml"), manifest).unwrap();
    let context = ServerContext::with_store_root(
        f.source.local.root_path(),
        f.source._home.path().join("unused"),
    )
    .with_source_derivatives(f.catalog.clone());
    let tools = crate::tools::default_tools();
    let tool = tools
        .iter()
        .find(|t| t.descriptor().name == "read_spec")
        .unwrap();
    let request = json!({"schema":1,"address":"spec://org.example/local/common/POLICY#RETRY","mode":"read","format":"md","children":true});
    let expected = vibe_workspace::source_query::SourceQueryService::execute_json_with_catalog(
        f.source.local.root_path(),
        &request,
        f.catalog.clone(),
    )
    .into_value();
    let before = inspection_files(f.source._home.path());
    let actual = tool.run(&request, &context).unwrap().structured().clone();
    let dispatched = crate::dispatch_one(ServerContext::with_store_root(f.source.local.root_path(),f.source._home.path().join("unused")).with_source_derivatives(f.catalog.clone()), &json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_spec","arguments":request}}).to_string()).unwrap();
    let dispatched: Value = serde_json::from_str(&dispatched).unwrap();
    assert_eq!(dispatched["result"]["structuredContent"], actual);
    assert_eq!(inspection_files(f.source._home.path()), before);
    assert_eq!(actual, expected);
    let source = vibe_wire::behaviour::source_query::read_result(&actual).unwrap();
    let record: vibe_wire::generated::derivative_result::DerivativeResult =
        serde_json::from_str(source.derivative_result_json.as_deref().unwrap()).unwrap();
    assert_eq!(record, f.catalog.records()[0]);
    assert_eq!(
        source.observation.strength,
        vibe_wire::generated::source_query::e1::result::ObservationStrength::Observed
    );
    assert!(source.content.contains("Retry at most once"));
    // The same ordinary host that served read_spec constructs the lazy boot
    // route from its own configuration, without prebuilt-route injection.
    let debugger = tools
        .iter()
        .find(|t| t.descriptor().name == "boot_debugger")
        .unwrap();
    let capture = debugger.run(&json!({}), &context).unwrap();
    let decode = |value: &Value| {
        let envelope: env::OperationEnvelope = serde_json::from_value(value.clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        serde_json::from_str::<r::Result>(body.payload_json.as_deref().unwrap()).unwrap()
    };
    let captured = decode(capture.structured());
    use vibe_wire::generated::boot_debugger::e1::request as q;
    let inspect = serde_json::to_value(input(&q::Request {
        schema: 1,
        context_id: captured.context_id,
        command: q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: captured.snapshot_id.unwrap(),
        })),
    }))
    .unwrap();
    let historical = debugger.run(&inspect, &context).unwrap();
    let r::Body::Inspect(view) = decode(historical.structured()).body else {
        panic!()
    };
    let records: Vec<vibe_wire::generated::derivative_result::DerivativeResult> =
        serde_json::from_str(view.derivative_catalog_json.as_deref().unwrap()).unwrap();
    assert_eq!(records, vec![record.clone()]);
    let path = f
        .source
        .local
        .root_path()
        .join(&f.source.binding.local().files[0].path);
    fs::write(path, b"changed").unwrap();
    assert!(tool.run(&request, &context).is_err());
    assert_eq!(
        debugger.run(&inspect, &context).unwrap().structured(),
        historical.structured()
    );
    assert!(f.catalog.inspect(&record.result_identity).is_ok());
}

#[test]
fn derivative_update_keeps_old_capture_and_refuses_mixed_snapshot_paging() {
    use vibe_wire::generated::boot_debugger::e1::request as q;
    let f = InspectionFixture::new("vibe-spec");
    let context = ServerContext::with_store_root(
        f.source.output.root_path(),
        f.source._home.path().join("unused"),
    )
    .with_boot_debugger_route(f.route.clone())
    .unwrap();
    let tool = crate::tools::default_tools()
        .into_iter()
        .find(|t| t.descriptor().name == "boot_debugger")
        .unwrap();
    let run = |command| {
        let request = input(&q::Request {
            schema: 1,
            context_id: f.route.context_id().into(),
            command,
        });
        tool.run(&serde_json::to_value(request).unwrap(), &context)
    };
    let decode = |output: super::ToolOutput| {
        let envelope: env::OperationEnvelope =
            serde_json::from_value(output.structured().clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        serde_json::from_str::<r::Result>(body.payload_json.as_deref().unwrap()).unwrap()
    };
    let tree = |snapshot: String, inspection: Option<String>, cursor: Option<String>| {
        q::Command::Tree(Box::new(q::CommandTree {
            snapshot_id: snapshot,
            filter: String::new(),
            limit: 1,
            inspection_id: inspection,
            cursor,
        }))
    };
    let first = decode(run(tree(f.snapshot.clone(), None, None)).unwrap());
    let r::Body::Tree(first) = first.body else {
        panic!()
    };
    let page: serde_json::Value = serde_json::from_str(&first.page_json).unwrap();
    let cursor = page["next_cursor_id"].as_str().unwrap().to_owned();
    // Intentional update is outside the subsequent read interval. Equal local
    // bytes with a changed base must still produce a distinct result identity.
    let mut base = f.source.binding.base().clone();
    base.files[0].bytes.extend_from_slice(b"\n");
    fs::write(
        f.source.base.root_path().join(&base.files[0].path),
        &base.files[0].bytes,
    )
    .unwrap();
    let binding = DerivationBinding::new(
        base.clone(),
        f.source.binding.local().clone(),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let materialized = vibe_workspace::derivatives::MaterializedDerivative::derive(
        &toml::to_string(&binding).unwrap(),
        &f.source.base,
        &f.source.local,
        &f.source.output,
        "result-updated",
    )
    .unwrap();
    let updated = vibe_workspace::source_query::derivatives::DerivativeCatalog::new(vec![
        materialized.result().record().clone(),
    ])
    .unwrap();
    assert_ne!(
        updated.records()[0].result_identity,
        f.catalog.records()[0].result_identity
    );
    f.route.configure_derivatives(updated.clone()).unwrap();
    let capture = decode(run(q::Command::Capture(Box::new(q::CommandCapture {}))).unwrap());
    let produced = decode(
        run(q::Command::Scenario(Box::new(q::CommandScenario {
            snapshot_id: capture.snapshot_id.unwrap(),
            target: "os:linux".into(),
            draft_json: serde_json::to_string(&vibe_workspace::boot_debugger::scenario::Draft {
                baseline_selections: Default::default(),
                edits: vec![],
            })
            .unwrap(),
        })))
        .unwrap(),
    );
    let r::Body::Preview(produced) = produced.body else {
        panic!()
    };
    assert_eq!(
        produced.status,
        r::BodyPreviewStatus::Complete,
        "{:?}",
        produced.missing
    );
    let next = produced.produced_snapshot_id.unwrap();
    assert_ne!(next, f.snapshot);
    assert!(
        run(tree(
            next.clone(),
            Some(first.inspection_id.clone()),
            Some(cursor.clone())
        ))
        .is_err()
    );
    assert!(
        run(tree(
            f.snapshot.clone(),
            Some(first.inspection_id),
            Some(cursor)
        ))
        .is_ok()
    );
    for (id, expected) in [
        (f.snapshot.clone(), f.catalog.records()),
        (next, updated.records()),
    ] {
        let value = decode(
            run(q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: id,
            })))
            .unwrap(),
        );
        let r::Body::Inspect(view) = value.body else {
            panic!()
        };
        let records: Vec<vibe_wire::generated::derivative_result::DerivativeResult> =
            serde_json::from_str(view.derivative_catalog_json.as_deref().unwrap()).unwrap();
        assert_eq!(records, expected);
    }
}

#[test]
fn ordinary_lazy_producer_compiles_derivative_and_retains_exact_source_navigation() {
    use vibe_wire::generated::boot_debugger::e1::request as q;
    let f = InspectionFixture::new("vibe-spec");
    let context = ServerContext::with_store_root(
        f.source.output.root_path(),
        f.source._home.path().join("unused"),
    )
    .with_source_derivatives(f.catalog.clone());
    let route = context.boot_debugger_route().unwrap();
    let tool = crate::tools::default_tools()
        .into_iter()
        .find(|t| t.descriptor().name == "boot_debugger")
        .unwrap();
    let run = |command| {
        let request = input(&q::Request {
            schema: 1,
            context_id: route.context_id().into(),
            command,
        });
        let output = tool
            .run(&serde_json::to_value(&request).unwrap(), &context)
            .unwrap();

        let envelope: env::OperationEnvelope =
            serde_json::from_value(output.structured().clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        serde_json::from_str::<r::Result>(body.payload_json.as_deref().unwrap()).unwrap()
    };
    let captured = run(q::Command::Capture(Box::new(q::CommandCapture {})));
    let produced = run(q::Command::Scenario(Box::new(q::CommandScenario {
        snapshot_id: captured.snapshot_id.unwrap(),
        target: "os:linux".into(),
        draft_json: serde_json::to_string(&vibe_workspace::boot_debugger::scenario::Draft {
            baseline_selections: Default::default(),
            edits: vec![],
        })
        .unwrap(),
    })));
    let r::Body::Preview(preview) = produced.body else {
        panic!()
    };
    assert_eq!(
        preview.status,
        r::BodyPreviewStatus::Complete,
        "{:?}",
        preview.missing
    );
    let snapshot = preview.produced_snapshot_id.unwrap();
    let inspect = || {
        run(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: snapshot.clone(),
        })))
    };
    let original = inspect();
    let r::Body::Inspect(view) = &original.body else {
        panic!()
    };
    let source = view
        .sources
        .iter()
        .find(|s| s.derivative_read.is_some())
        .unwrap();
    let witness: serde_json::Value =
        serde_json::to_value(source.derivative_read.as_ref().unwrap()).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            witness["derivative_result_json"].as_str().unwrap()
        )
        .unwrap(),
        serde_json::to_value(&f.catalog.records()[0]).unwrap()
    );
    let occurrences: Vec<_> = view
        .nodes
        .iter()
        .filter(|n| n.source == source.source)
        .collect();
    assert_eq!(occurrences.len(), 2);
    assert_ne!(
        (occurrences[0].contribution, occurrences[0].occurrence),
        (occurrences[1].contribution, occurrences[1].occurrence)
    );
    let service =
        vibe_workspace::source_query::SourceQueryService::prepare_host(f.source.local.root_path())
            .unwrap()
            .with_derivative_catalog(f.catalog.clone());
    let request = serde_json::from_value(serde_json::json!({"schema":1,"address":"spec://org.example/local/common/POLICY#RETRY","mode":"read","format":"md","children":true,"chunk_bytes":2048})).unwrap();
    let queried = service.query(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            queried.derivative_result_json.as_deref().unwrap()
        )
        .unwrap(),
        serde_json::from_str::<serde_json::Value>(
            witness["derivative_result_json"].as_str().unwrap()
        )
        .unwrap()
    );
    let before = inspection_files(f.source._home.path());
    for (key, expected, basis) in [
        (
            "raw_resource_id",
            f.source.binding.local().files[0].bytes.clone(),
            r::BodyChunkBasis::OriginalBytes,
        ),
        (
            "projection_resource_id",
            queried.content.as_bytes().to_vec(),
            r::BodyChunkBasis::CapturedProjection,
        ),
    ] {
        let chunk = run(q::Command::Chunk(Box::new(q::CommandChunk {
            snapshot_id: snapshot.clone(),
            resource_id: witness[key].as_str().unwrap().into(),
            filter: q::CommandChunkFilter::Raw,
            offset: "0".into(),
            limit: 2048,
        })));
        let r::Body::Chunk(chunk) = chunk.body else {
            panic!()
        };
        assert_eq!(chunk.basis, basis);
        let bytes: Vec<_> = chunk
            .hex_bytes
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(bytes, expected);
    }
    assert_eq!(inspection_files(f.source._home.path()), before);
    // Providers and authored files disappear after explicit preparation. The
    // immutable inspector has only retained bytes and never re-enters capture.
    fs::remove_dir_all(f.source.output.root_path().join("vibevm/vibepacks")).unwrap();
    fs::remove_dir_all(f.source.local.root_path()).unwrap();
    fs::remove_dir_all(f.source.base.root_path()).unwrap();
    assert_eq!(inspect(), original);
}

#[test]
fn real_compiler_capture_is_passive_with_a_rejecting_provider_after_preparation() {
    use vibe_wire::generated::boot_debugger::e1::request as q;
    let InspectionFixture {
        source,
        route,
        snapshot,
        provider_calls,
        catalog: _,
    } = InspectionFixture::new("vibe-spec");
    let route = std::sync::Arc::new(
        std::sync::Arc::try_unwrap(route)
            .ok()
            .unwrap()
            .with_scenario_producer(std::sync::Arc::new(RejectingInspectionProducer(
                provider_calls.clone(),
            ))),
    );
    let context = ServerContext::with_store_root(
        source.output.root_path(),
        source._home.path().join("unused"),
    )
    .with_boot_debugger_route(route.clone())
    .unwrap();
    let tool = crate::tools::default_tools()
        .into_iter()
        .find(|t| t.descriptor().name == "boot_debugger")
        .unwrap();
    let run = |command| {
        tool.run(
            &serde_json::to_value(input(&q::Request {
                schema: 1,
                context_id: route.context_id().into(),
                command,
            }))
            .unwrap(),
            &context,
        )
        .unwrap()
        .structured()
        .clone()
    };
    let before = inspection_files(source._home.path());
    let original = run(q::Command::Inspect(Box::new(q::CommandInspect {
        snapshot_id: snapshot.clone(),
    })));
    run(q::Command::Tree(Box::new(q::CommandTree {
        snapshot_id: snapshot.clone(),
        filter: String::new(),
        limit: 1,
        inspection_id: None,
        cursor: None,
    })));
    run(q::Command::Cost(Box::new(q::CommandCost {
        snapshot_id: snapshot.clone(),
    })));
    assert_eq!(provider_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(inspection_files(source._home.path()), before);
    fs::remove_dir_all(source.output.root_path().join("vibevm/vibepacks")).unwrap();
    assert_eq!(
        run(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: snapshot
        }))),
        original
    );
    assert_eq!(provider_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
