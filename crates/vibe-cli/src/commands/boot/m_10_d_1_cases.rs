use super::*;
use vibe_wire::generated::{
    boot_debugger::e1::result as r, machine::e1::operation_envelope as env,
};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/inspection_route_fixture.rs"
));

#[test]
fn standard_command_constructor_exposes_exact_retained_history_for_all_families() {
    for family in [
        "commonmark",
        "xml-data",
        "vibe-spec",
        "opaque-file",
        "nested-tree",
    ] {
        let f = InspectionFixture::new(family);
        let before = inspection_files(f.source._home.path());
        let mut output = Vec::new();
        // Drive the same standard constructor and I/O entrypoint as `vibe boot
        // --view --send show`; no injected route replaces command composition.
        run_with_io(
            BootArgs {
                view: true,
                send: Some("show".into()),
                root: f.source.output.root_path().into(),
                artifact: "STATIC.md".into(),
                kind: ArtifactKind::Static,
                registry: None,
                derivative_records: vec!["result/result.json".into()],
            },
            std::io::Cursor::new(Vec::<u8>::new()),
            &mut output,
        )
        .unwrap();
        let frames: Vec<serde_json::Value> = output
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert!(!frames.is_empty());
        for frame in frames {
            let records: Vec<vibe_wire::generated::derivative_result::DerivativeResult> =
                serde_json::from_str(frame["derivative_catalog_json"].as_str().unwrap()).unwrap();
            assert_eq!(records, f.catalog.records());
            assert!(frame["gaps"].as_array().unwrap().iter().any(|g| {
                g.as_str()
                    .unwrap()
                    .contains("compiler occurrence joins are unavailable")
            }));
        }
        // The structured CLI adapter and registered MCP tool consume one exact
        // immutable capture; their host/cursor identities are not normalized away.
        let request = f.inspect();
        let envelope = boot_debugger::input(&request);
        let mut bytes = serde_json::to_vec(&envelope).unwrap();
        bytes.push(b'\n');
        let mut reply = vec![];
        serve(&f.route, std::io::Cursor::new(bytes), &mut reply).unwrap();
        let cli: env::OperationEnvelope =
            serde_json::from_slice(reply.split(|b| *b == b'\n').nth(1).unwrap()).unwrap();
        let expected = f.route.exchange(&serde_json::to_vec(&envelope).unwrap());
        assert_eq!(cli, expected);
        let context = vibe_mcp::ServerContext::with_store_root(
            f.source.output.root_path(),
            f.source._home.path().join("unused"),
        )
        .with_boot_debugger_route(f.route.clone())
        .unwrap();
        let tool = vibe_mcp::default_tools()
            .into_iter()
            .find(|t| t.descriptor().name == "boot_debugger")
            .unwrap();
        assert_eq!(
            *tool
                .run(&serde_json::to_value(envelope).unwrap(), &context)
                .unwrap()
                .structured(),
            serde_json::to_value(&expected).unwrap()
        );
        let env::Body::Result(body) = cli.body else {
            panic!()
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        let result: r::Result =
            serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
        let r::Body::Inspect(view) = result.body else {
            panic!()
        };
        assert_eq!(view.capture_id, f.snapshot);
        let retained = f
            .catalog
            .inspect(&f.catalog.records()[0].result_identity)
            .unwrap();
        for file in &f.source.expected {
            assert_eq!(
                retained
                    .local
                    .iter()
                    .find(|r| r.path == file.path)
                    .unwrap()
                    .raw,
                file.bytes
            );
        }
        assert_eq!(
            f.provider_calls.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert_eq!(inspection_files(f.source._home.path()), before);
        assert!(!f.source._home.path().join("DO_NOT_EXECUTE").exists());
    }
}

#[test]
fn host_record_configuration_refuses_escape_and_corruption() {
    let f = InspectionFixture::new("opaque-file");
    for path in ["../authored.toml", "vibe.toml", "missing.json"] {
        assert!(
            vibe_workspace::source_query::derivatives::DerivativeCatalog::load(
                f.source.output.root_path(),
                &[path.into()]
            )
            .is_err()
        );
    }
}

#[test]
fn standard_command_prepares_real_derivative_then_reads_captured_occurrences() {
    let f = InspectionFixture::new("vibe-spec");
    let draft = vibe_workspace::boot_debugger::scenario::Draft {
        baseline_selections: Default::default(),
        edits: vec![],
    };
    let before = inspection_files(f.source._home.path());
    let mut output = vec![];
    run_with_io(
        BootArgs {
            view: true,
            send: Some(format!(
                "scenario os:linux {};select 0;source 0",
                serde_json::to_string(&draft).unwrap()
            )),
            root: f.source.output.root_path().into(),
            artifact: "STATIC.md".into(),
            kind: ArtifactKind::Static,
            registry: None,
            derivative_records: vec!["result/result.json".into()],
        },
        std::io::Cursor::new(Vec::<u8>::new()),
        &mut output,
    )
    .unwrap();
    let frames: Vec<serde_json::Value> = output
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice(b).unwrap())
        .collect();
    let frame = frames.last().unwrap();
    assert!(frame["diagnostic"].is_null(), "{frame}");
    let sources: Vec<r::Source> = serde_json::from_value(frame["sources"].clone()).unwrap();
    let source = sources
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
    let nodes: Vec<r::Node> = serde_json::from_value(frame["nodes"].clone()).unwrap();
    let occurrences: Vec<_> = nodes.iter().filter(|n| n.source == source.source).collect();
    assert_eq!(occurrences.len(), 2);
    assert_ne!(
        (occurrences[0].contribution, occurrences[0].occurrence),
        (occurrences[1].contribution, occurrences[1].occurrence)
    );
    let observation: r::Result = serde_json::from_value(frame["observation"].clone()).unwrap();
    let r::Body::Chunk(chunk) = observation.body else {
        panic!()
    };
    assert_eq!(
        chunk.resource_id,
        source.authored_resource_id.clone().unwrap()
    );
    let expected: String = f.source.binding.local().files[0]
        .bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(chunk.hex_bytes, expected);
    assert_eq!(inspection_files(f.source._home.path()), before);
}

#[test]
fn standard_spec_command_and_registered_mcp_agree_with_source_api() {
    use clap::Parser;
    let f = InspectionFixture::new("vibe-spec");
    let root = f.source.local.root_path();
    fs::write(
        root.join("retained-result.json"),
        serde_json::to_vec(&f.catalog.records()[0]).unwrap(),
    )
    .unwrap();
    let before = inspection_files(f.source._home.path());
    let cli = crate::cli::Cli::try_parse_from([
        "vibe",
        "spec",
        "read",
        "spec://org.example/local/common/POLICY#RETRY",
        "--path",
        root.to_str().unwrap(),
        "--chunk-bytes",
        "2048",
        "--derivative-record",
        "retained-result.json",
    ])
    .unwrap();
    let crate::cli::Command::Spec(args) = cli.command else {
        panic!()
    };
    let mut output = vec![];
    let mut errors = vec![];
    assert_eq!(
        crate::commands::spec::run_with_io(&args, true, &mut output, &mut errors).unwrap(),
        std::process::ExitCode::SUCCESS
    );
    assert!(errors.is_empty());
    let decoded: serde_json::Value = serde_json::from_slice(&output).unwrap();
    vibe_wire::behaviour::source_query::read_result(&decoded).unwrap();
    let request = serde_json::json!({"schema":1,"address":"spec://org.example/local/common/POLICY#RETRY","mode":"read","format":"md","children":true,"chunk_bytes":2048});
    let api = vibe_workspace::source_query::SourceQueryService::execute_json_with_catalog(
        root,
        &request,
        f.catalog.clone(),
    )
    .into_value();
    assert_eq!(decoded, api);
    let context =
        vibe_mcp::ServerContext::with_store_root(root, f.source._home.path().join("unused"))
            .with_source_derivatives(f.catalog.clone());
    let tool = vibe_mcp::default_tools()
        .into_iter()
        .find(|t| t.descriptor().name == "read_spec")
        .unwrap();
    assert_eq!(*tool.run(&request, &context).unwrap().structured(), api);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            decoded["derivative_result_json"].as_str().unwrap()
        )
        .unwrap(),
        serde_json::to_value(&f.catalog.records()[0]).unwrap()
    );
    assert_eq!(decoded["observation"]["strength"], "observed");
    assert_eq!(inspection_files(f.source._home.path()), before);
}
