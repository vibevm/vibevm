use super::*;
use vibe_wire::generated::{
    boot_debugger::e1::{request as q, result as r},
    machine::e1::operation_envelope as env,
};
use vibe_workspace::boot_debugger::viewer::{Action, Service, Viewer};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/inspection_route_fixture.rs"
));

fn payload(envelope: env::OperationEnvelope) -> r::Result {
    let env::Body::Result(body) = envelope.body else {
        panic!("expected result")
    };
    assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
    serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
}

#[test]
fn current_retained_history_preserves_all_five_derivative_families() {
    super::m_10_d_1_cases::standard_command_constructor_exposes_exact_retained_history_for_all_families();
}

#[test]
fn retained_derivative_all_surfaces_and_every_chunk_and_page_agree() {
    let f = InspectionFixture::new("vibe-spec");
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
    let service = viewer::RouteService(&f.route);
    let same = |command: q::Command| {
        let request = q::Request {
            schema: 1,
            context_id: f.route.context_id().into(),
            command: command.clone(),
        };
        let direct = f.route.request(request.clone()).unwrap();
        let envelope = boot_debugger::input(&request);
        let encoded = serde_json::to_vec(&envelope).unwrap();
        let canonical = f.route.exchange(&encoded);
        assert_eq!(payload(canonical.clone()), direct);
        let mut line = encoded;
        line.push(b'\n');
        let mut output = vec![];
        serve(&f.route, std::io::Cursor::new(line), &mut output).unwrap();
        let cli: env::OperationEnvelope =
            serde_json::from_slice(output.split(|b| *b == b'\n').nth(1).unwrap()).unwrap();
        assert_eq!(cli, canonical);
        assert_eq!(
            *tool
                .run(&serde_json::to_value(envelope).unwrap(), &context)
                .unwrap()
                .structured(),
            serde_json::to_value(canonical).unwrap()
        );
        assert_eq!(service.query(command).unwrap(), direct);
        direct
    };
    let mut view = Viewer::new(viewer::RouteService(&f.route)).unwrap();
    let prepared = view
        .apply(Action::Scenario {
            target: "os:linux".into(),
            draft_json: "{\"baseline_selections\":{\"features\":[],\"no_defaults\":false,\"all_features\":false,\"configuration\":null},\"edits\":[]}".into(),
        })
        .unwrap()
        .unwrap();
    let r::Body::Preview(prepared) = prepared.body else {
        panic!()
    };
    assert_eq!(prepared.status, r::BodyPreviewStatus::Complete);
    let id = view.frame(None, None).snapshot_id;
    let inspected = view.apply(Action::Inspect).unwrap().unwrap();
    assert_eq!(
        inspected,
        same(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: id.clone()
        })))
    );
    let r::Body::Inspect(inspect) = inspected.body else {
        panic!()
    };
    assert!(inspect.compiler_carrier_json.is_some());
    let source = inspect
        .sources
        .iter()
        .find(|s| s.derivative_read.is_some())
        .unwrap();
    let read = source.derivative_read.as_ref().unwrap();
    let record: vibe_wire::generated::derivative_result::DerivativeResult =
        serde_json::from_str(&read.derivative_result_json).unwrap();
    assert_eq!(record, f.catalog.records()[0]);
    assert_ne!(read.raw_resource_id, read.projection_resource_id);
    let repeated: Vec<_> = inspect
        .nodes
        .iter()
        .filter(|n| n.source == source.source)
        .collect();
    assert_eq!(repeated.len(), 2);
    assert_ne!(
        (repeated[0].contribution, repeated[0].occurrence),
        (repeated[1].contribution, repeated[1].occurrence)
    );
    // Remove the entire live provider/source tree. All following reads must use
    // only the retained snapshot, including the compiler carrier and schemas.
    fs::remove_dir_all(f.source.output.root_path().join("vibevm/vibepacks")).unwrap();
    fs::remove_dir_all(f.source.base.root_path()).unwrap();
    fs::remove_dir_all(f.source.local.root_path()).unwrap();
    let before = inspection_files(f.source._home.path());
    assert_eq!(
        view.apply(Action::Cost).unwrap().unwrap(),
        same(q::Command::Cost(Box::new(q::CommandCost {
            snapshot_id: id.clone()
        })))
    );
    assert_eq!(
        view.apply(Action::Compare(1, 1)).unwrap().unwrap(),
        same(q::Command::Compare(Box::new(q::CommandCompare {
            snapshot_id: id.clone(),
            other_snapshot_id: id.clone()
        })))
    );
    let mut nodes = vec![];
    for i in (0..inspect.nodes.len()).step_by(20) {
        view.apply(Action::Select(i)).unwrap();
        nodes.extend(view.frame(None, None).nodes);
    }
    assert_eq!(nodes, inspect.nodes);
    for (i, node) in inspect.nodes.iter().enumerate() {
        view.apply(Action::Select(i)).unwrap();
        assert_eq!(
            view.apply(Action::Detail).unwrap().unwrap(),
            same(q::Command::Explain(Box::new(q::CommandExplain {
                snapshot_id: id.clone(),
                contribution: node.contribution,
                occurrence: node.occurrence
            })))
        );
    }
    for (i, resource) in inspect.resources.iter().enumerate() {
        let mut offset = 0;
        let mut total = 0;
        loop {
            let value = view.apply(Action::Resource(i, offset)).unwrap().unwrap();
            assert_eq!(
                value,
                same(q::Command::Chunk(Box::new(q::CommandChunk {
                    snapshot_id: id.clone(),
                    resource_id: resource.resource_id.clone(),
                    filter: q::CommandChunkFilter::Raw,
                    offset: offset.to_string(),
                    limit: 2048
                })))
            );
            let r::Body::Chunk(chunk) = value.body else {
                panic!()
            };
            assert_eq!(chunk.digest, resource.digest.clone().unwrap());
            assert_eq!(
                chunk.basis == r::BodyChunkBasis::CapturedProjection,
                resource.basis == r::ResourceBasis::CapturedProjection
            );
            total += chunk.hex_bytes.len() / 2;
            if let Some(next) = chunk.next_offset {
                offset = next.parse().unwrap();
            } else {
                assert_eq!(total.to_string(), chunk.total);
                break;
            }
        }
    }
    // Each stateful page traversal uses its own legitimate session. Compare
    // all rows, not opaque session/cursor identities minted for each traversal.
    let mut expected = None;
    for surface in 0..3 {
        let mut rows = vec![];
        let (mut session, mut cursor) = (None, None);
        loop {
            let request = q::Request {
                schema: 1,
                context_id: f.route.context_id().into(),
                command: q::Command::Tree(Box::new(q::CommandTree {
                    snapshot_id: id.clone(),
                    inspection_id: session.clone(),
                    cursor: cursor.clone(),
                    filter: "".into(),
                    limit: 1,
                })),
            };
            let envelope = boot_debugger::input(&request);
            let value = match surface {
                0 => f.route.request(request).unwrap(),
                1 => {
                    let mut bytes = serde_json::to_vec(&envelope).unwrap();
                    bytes.push(b'\n');
                    let mut out = vec![];
                    serve(&f.route, std::io::Cursor::new(bytes), &mut out).unwrap();
                    payload(
                        serde_json::from_slice(out.split(|b| *b == b'\n').nth(1).unwrap()).unwrap(),
                    )
                }
                _ => payload(
                    serde_json::from_value(
                        tool.run(&serde_json::to_value(envelope).unwrap(), &context)
                            .unwrap()
                            .structured()
                            .clone(),
                    )
                    .unwrap(),
                ),
            };
            let r::Body::Tree(tree) = value.body else {
                panic!()
            };
            let page: vibe_wire::generated::machine::e1::bound_page::BoundPage =
                serde_json::from_str(&tree.page_json).unwrap();
            rows.extend(page.rows);
            session = Some(tree.inspection_id);
            cursor = page.next_cursor_id;
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(rows.len(), inspect.nodes.len());
        if let Some(expected) = &expected {
            assert_eq!(&rows, expected);
        } else {
            expected = Some(rows);
        }
    }
    assert!(
        view.apply(Action::Resource(inspect.resources.len(), 0))
            .is_err()
    );
    assert_eq!(
        f.provider_calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(inspection_files(f.source._home.path()), before);
}

#[test]
fn standard_cli_constructor_drives_cost_inspect_why_and_resource() {
    use clap::Parser;
    let f = InspectionFixture::new("vibe-spec");
    let script = "scenario os:linux {\"baseline_selections\":{\"features\":[],\"no_defaults\":false,\"all_features\":false,\"configuration\":null},\"edits\":[]};inspect;cost;select 0;why;source;resource 0;compare 1 1;resource 999";
    let cli = crate::cli::Cli::try_parse_from([
        "vibe",
        "boot",
        "--root",
        f.source.output.root_path().to_str().unwrap(),
        "--view",
        "--send",
        script,
        "--derivative-record",
        "result/result.json",
    ])
    .unwrap();
    let crate::cli::Command::Boot(args) = cli.command else {
        panic!()
    };
    let mut out = vec![];
    run_with_io(args, std::io::Cursor::new([]), &mut out).unwrap();
    let frames: Vec<serde_json::Value> = out
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(frames.len(), 10);
    for frame in &frames[..9] {
        assert!(frame["diagnostic"].is_null(), "{frame}");
    }
    assert!(frames[2]["observation"]["body"]["compiler_carrier_json"].is_string());
    assert!(frames[3]["observation"]["body"]["tokens"]["unavailable"].is_string());
    assert!(
        frames[9]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("resource ordinal")
    );
}

#[test]
fn standard_compiler_capture_keeps_native_format_without_inventing_authored_mapping() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
    // A retained carrier from a native source transform with no inverse map.
    // The backend is intentionally absent: observation must decode the open
    // format identity and retain its text without loading that backend.
    let native = serde_json::json!({
        "shape":"source-document", "ir_schema":1, "level":"source", "cardinality":"document",
        "doc": {"address":{"kind":"static-entry","origin":"native/owner","path":"generated.native"},
        "format":"native.transform.fixture/v1", "text":"opaque transformed payload",
        "subject":{"declared_path":"generated.native","provider":{"kind":"undetermined"}}}
    });
    let bytes = serde_json::to_vec(&native).unwrap();
    // Use the existing strict compiler reader before exposing this fixture.
    vibe_workspace::boot_debugger::protocol::project("native", CapturedKind::Compiler, &bytes)
        .unwrap();
    fs::write(root.path().join("native.json"), &bytes).unwrap();
    let args = BootArgs {
        view: true,
        send: Some("inspect;resource 1;cost;source".into()),
        root: root.path().into(),
        artifact: "native.json".into(),
        kind: ArtifactKind::Compiler,
        registry: None,
        derivative_records: vec![],
    };
    let mut out = vec![];
    run_with_io(args, std::io::Cursor::new([]), &mut out).unwrap();
    let frames: Vec<serde_json::Value> = out
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(frames.len(), 5);
    let view = &frames[1]["observation"]["body"];
    assert!(view["sources"][0]["authored_resource_id"].is_null());
    assert!(view["sources"][0]["derivative_read"].is_null());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(view["compiler_carrier_json"].as_str().unwrap())
            .unwrap(),
        native
    );
    assert!(
        frames[4]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("no selected captured occurrence")
    );
    assert_eq!(fs::read(root.path().join("native.json")).unwrap(), bytes);
}
