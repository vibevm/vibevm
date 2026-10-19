use super::*;
use std::cell::{Cell, RefCell};
use vibe_orchestrator::machine_operation::boot_debugger::input;
use vibe_wire::generated::{
    boot_debugger::e1::{request as q, result as r},
    machine::e1::operation_envelope as env,
};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vibe-workspace/src/boot_debugger/m_06_f_2_fixture.rs"
));

struct Client {
    server: RefCell<crate::Server<crate::MemoryTransport>>,
    max_frame: Cell<usize>,
}
impl Client {
    fn new(root: &std::path::Path) -> Self {
        Self::with_context(ServerContext::with_store_root(root, root.join("unused")))
    }
    fn with_context(context: ServerContext) -> Self {
        Self {
            server: RefCell::new(crate::Server::new(
                crate::MemoryTransport::with_input(""),
                context,
            )),
            max_frame: Cell::new(0),
        }
    }
    fn rpc(&self, args: Value) -> Value {
        let mut server = self.server.borrow_mut();
        server.transport = crate::MemoryTransport::with_input(format!(
            "{}\n",
            json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":"boot_debugger", "arguments":args}})
        ));
        server.run().unwrap();
        let text = server.transport.take_output();
        self.max_frame.set(self.max_frame.get().max(text.len()));
        serde_json::from_str(&text).unwrap()
    }
    fn query(&self, context: &str, command: q::Command) -> Result<r::Result, Value> {
        let response = self.rpc(
            serde_json::to_value(input(&q::Request {
                schema: 1,
                context_id: context.into(),
                command,
            }))
            .unwrap(),
        );
        if response["result"]["isError"] != false {
            return Err(response);
        }
        let envelope: env::OperationEnvelope =
            serde_json::from_value(response["result"]["structuredContent"].clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!("{envelope:?}")
        };
        assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
        Ok(serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap())
    }
    fn capture(&self) -> r::Result {
        let response = self.rpc(json!({}));
        assert_eq!(response["result"]["isError"], false, "{response}");
        let envelope: env::OperationEnvelope =
            serde_json::from_value(response["result"]["structuredContent"].clone()).unwrap();
        let env::Body::Result(body) = envelope.body else {
            panic!()
        };
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
    }
}
fn tree(id: &str, inspection_id: Option<String>, cursor: Option<String>) -> q::Command {
    q::Command::Tree(Box::new(q::CommandTree {
        snapshot_id: id.into(),
        inspection_id,
        cursor,
        filter: "".into(),
        limit: 7,
    }))
}
#[test]
fn million_units_registered_server_traverses_chunks_pages_and_cancels_only_inspection() {
    let root = tempfile::tempdir().unwrap();
    let original = scale_static();
    std::fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='scale'\nversion='1.0.0'\n",
    )
    .unwrap();
    std::fs::write(root.path().join("STATIC.md"), &original).unwrap();
    std::fs::write(root.path().join("INDEX.md"), SCALE_INDEX).unwrap();
    std::fs::create_dir_all(root.path().join("vibevm/vibespecs/common")).unwrap();
    std::fs::write(
        root.path().join("vibevm/vibespecs/common/ordinary.xml"),
        b"<data>ordinary bytes, no Vibe facts</data>\r\n",
    )
    .unwrap();
    let client = Client::new(root.path());
    let capture = client.capture();
    let context = capture.context_id;
    let id = capture.snapshot_id.unwrap();
    let inspect = || {
        client
            .query(
                &context,
                q::Command::Inspect(Box::new(q::CommandInspect {
                    snapshot_id: id.clone(),
                })),
            )
            .unwrap()
    };
    let first = inspect();
    let r::Body::Inspect(view) = &first.body else {
        panic!()
    };
    assert_eq!(view.nodes.len(), 128);
    let mut cursor = None;
    let mut inspection = None;
    let mut rows = Vec::new();
    let mut max_page = 0;
    loop {
        let result = client
            .query(&context, tree(&id, inspection.clone(), cursor.clone()))
            .unwrap();
        let r::Body::Tree(body) = result.body else {
            panic!()
        };
        let page: Value = serde_json::from_str(&body.page_json).unwrap();
        inspection = Some(body.inspection_id.clone());
        let page_rows = page["rows"].as_array().unwrap();
        assert!(page_rows.len() <= 7);
        max_page = max_page.max(body.page_json.len());
        rows.extend(page_rows.iter().cloned());
        let previous = cursor;
        cursor = page["next_cursor_id"].as_str().map(str::to_owned);
        if let Some(previous) = previous {
            assert!(
                client
                    .query(&context, tree(&id, inspection.clone(), Some(previous)))
                    .is_err()
            );
        }
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(rows.len(), view.nodes.len());
    // Compare the exact ordered semantic row identities, not only the count.
    for (row, node) in rows.iter().zip(&view.nodes) {
        assert_eq!(
            row["occurrence_id"].as_str().unwrap(),
            format!("{}:{}", node.contribution, node.occurrence)
        );
    }
    assert!(
        view.resources
            .iter()
            .any(|r| r.path.ends_with("ordinary.xml"))
    );
    let mut chunks = 0;
    let mut max_chunk = 0;
    let mut traversed_bytes = 0;
    // Record actual serialized RPC frame size separately from complete Inspect.
    client.max_frame.set(0);
    for _ in 0..2 {
        for resource in &view.resources {
            let expected = std::fs::read(root.path().join(&resource.path)).unwrap();
            let mut joined = Vec::new();
            let mut offset = "0".to_string();
            loop {
                let result = client
                    .query(
                        &context,
                        q::Command::Chunk(Box::new(q::CommandChunk {
                            snapshot_id: id.clone(),
                            resource_id: resource.resource_id.clone(),
                            filter: q::CommandChunkFilter::Raw,
                            offset,
                            limit: 2048,
                        })),
                    )
                    .unwrap();
                let r::Body::Chunk(chunk) = result.body else {
                    panic!()
                };
                let bytes: Vec<u8> = chunk
                    .hex_bytes
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
                    .collect();
                max_chunk = max_chunk.max(bytes.len());
                assert!(bytes.len() <= 2048);
                joined.extend(bytes);
                chunks += 1;
                if let Some(next) = chunk.next_offset {
                    offset = next;
                } else {
                    assert_eq!(chunk.total, expected.len().to_string());
                    break;
                }
            }
            assert_eq!(joined, expected);
            traversed_bytes += joined.len();
        }
    }
    let chunk_frame = client.max_frame.get();
    eprintln!(
        "m06f2 twice complete resource traversal bytes={} resources={}",
        traversed_bytes,
        view.resources.len()
    );
    let cost = || {
        client
            .query(
                &context,
                q::Command::Cost(Box::new(q::CommandCost {
                    snapshot_id: id.clone(),
                })),
            )
            .unwrap()
    };
    let measured = cost();
    assert_eq!(cost(), measured);
    std::fs::write(root.path().join("STATIC.md"), "changed live source").unwrap();
    assert_eq!(inspect(), first);
    assert_eq!(cost(), measured);
    let fresh = client.capture().snapshot_id.unwrap();
    assert_ne!(fresh, id);
    assert!(
        client
            .query(&context, tree(&fresh, inspection.clone(), None))
            .is_err()
    );
    let open = client.query(&context, tree(&id, None, None)).unwrap();
    let r::Body::Tree(open) = open.body else {
        panic!()
    };
    let result = client
        .query(
            &context,
            q::Command::Cancel(Box::new(q::CommandCancel {
                inspection_id: open.inspection_id.clone(),
            })),
        )
        .unwrap();
    let r::Body::Cancel(cancel) = result.body else {
        panic!()
    };
    assert_eq!(cancel.scope, r::BodyCancelScope::InspectionOnly);
    assert_eq!(
        cancel.build_cancelled,
        r::BodyCancelBuildCancelled::NotRequested
    );
    assert!(
        client
            .query(&context, tree(&id, Some(open.inspection_id), None))
            .is_err()
    );
    assert_eq!(inspect(), first);
    let memory = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .map(str::to_owned)
        });
    eprintln!(
        "m06f2 units=1048576 result_bytes={} rows={} chunks={} max_chunk_bytes={} max_chunk_rpc_frame={} max_page_json={} cancel={:?} memory_process_high_water={:?}; no total heap bound asserted",
        original.len(),
        rows.len(),
        chunks,
        max_chunk,
        chunk_frame,
        max_page,
        cancel.status,
        memory
    );
}

mod preview {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/next/derivatives/inspection_route_fixture.rs"
    ));
    #[test]
    fn registered_server_previews_dependencies_and_refuses_changed_catalog_selectors() {
        let f = InspectionFixture::new("vibe-spec");
        let client = Client::with_context(
            ServerContext::with_store_root(
                f.source.output.root_path(),
                f.source._home.path().join("unused"),
            )
            .with_source_derivatives(f.catalog.clone()),
        );
        let captured = client.capture();
        let context = captured.context_id;
        let scenario = || {
            q::Command::Scenario(Box::new(q::CommandScenario { snapshot_id: captured.snapshot_id.clone().unwrap(), target: "os:linux".into(), draft_json: "{\"baseline_selections\":{\"features\":[],\"no_defaults\":false,\"all_features\":false,\"configuration\":null},\"edits\":[]}".into() }))
        };
        let prepared = client.query(&context, scenario()).unwrap();
        let r::Body::Preview(p) = prepared.body else {
            panic!()
        };
        assert_eq!(p.status, r::BodyPreviewStatus::Complete);
        let comparison: Value =
            serde_json::from_str(p.comparison_json.as_deref().unwrap()).unwrap();
        let costs = comparison["candidate"]["package_costs"].as_array().unwrap();
        let app_cost = costs
            .iter()
            .find(|row| row["coordinate"] == "org.example/app")
            .unwrap();
        assert!(
            app_cost["inclusive_members"]
                .as_array()
                .unwrap()
                .contains(&json!("org.example/local"))
        );
        assert_eq!(
            comparison["baseline"]["static_raw"],
            comparison["candidate"]["static_raw"]
        );
        let command = |catalog: String| {
            q::Command::Preview(Box::new(q::CommandPreview {
                scenario_id: p.scenario_id.clone(),
                expected_baseline: p.baseline_world.clone(),
                expected_candidate: p.candidate_world.clone(),
                expected_catalog: catalog,
                loading_overrides: Default::default(),
            }))
        };
        let before = client
            .query(&context, command(p.candidate_catalog.clone()))
            .unwrap();
        let app = f
            .source
            .output
            .root_path()
            .join("vibevm/vibepacks/org.example/app/v1.0.0/vibe.toml");
        let mut text = fs::read_to_string(&app).unwrap();
        text.push_str("\n# changed captured catalog input\n");
        fs::write(app, text).unwrap();
        let source = f
            .source
            .output
            .root_path()
            .join("vibevm/vibepacks/org.example/app/v1.0.0/vibevm/vibespecs/boot/entry.md");
        let mut changed = fs::read_to_string(&source).unwrap();
        changed.push_str("\nnew captured source\n");
        fs::write(source, changed).unwrap();
        assert_eq!(
            client
                .query(&context, command(p.candidate_catalog.clone()))
                .unwrap(),
            before
        );
        let stale = client
            .query(&context, command("changed catalog".into()))
            .unwrap();
        let r::Body::Preview(stale) = stale.body else {
            panic!()
        };
        assert_eq!(stale.status, r::BodyPreviewStatus::Stale);
        assert!(stale.artifact.is_none());
        let fresh = client.query(&context, scenario()).unwrap();
        let r::Body::Preview(fresh) = fresh.body else {
            panic!()
        };
        assert_ne!(fresh.candidate_catalog, p.candidate_catalog);
        assert_ne!(fresh.candidate_world, p.candidate_world);
        let stale = client
            .query(&context, command(fresh.candidate_catalog))
            .unwrap();
        let r::Body::Preview(stale) = stale.body else {
            panic!()
        };
        assert_eq!(stale.status, r::BodyPreviewStatus::Stale);
        assert!(stale.artifact.is_none());
        eprintln!(
            "m06f2 registered MCP preview app->local dependency retained; changed live catalog preserves old preview; new catalog selector refuses old scenario"
        );
    }
}
