use super::*;
use vibe_mcp::{ServerContext, ToolError};
use vibe_orchestrator::document_analysis::{BufferIntent, ByteEncoding};
use vibe_wire::generated::{
    boot_debugger::e1::{error as e, progress as p, request as q, result as r},
    machine::e1::operation_envelope as env,
};

fn cli(route: &BootDebuggerRoute, request: &q::Request) -> env::OperationEnvelope {
    let mut bytes = serde_json::to_vec(&boot_debugger::input(request)).unwrap();
    bytes.push(b'\n');
    let mut output = vec![];
    serve(route, std::io::Cursor::new(bytes), &mut output).unwrap();
    let lines = output
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    serde_json::from_slice(lines[1]).unwrap()
}
fn mcp(ctx: &ServerContext, request: &q::Request) -> env::OperationEnvelope {
    let tools = vibe_mcp::default_tools();
    let tool = tools
        .iter()
        .find(|t| t.descriptor().name == "boot_debugger")
        .expect("registered MCP tool");
    let args = serde_json::to_value(boot_debugger::input(request)).unwrap();
    let value = match tool.run(&args, ctx) {
        Ok(output) => output.structured().clone(),
        Err(ToolError::StructuredPreflight { report, .. }) => report,
        Err(error) => panic!("{error:?}"),
    };
    serde_json::from_value(value).unwrap()
}
fn result(envelope: env::OperationEnvelope) -> r::Result {
    let env::Body::Result(body) = envelope.body else {
        panic!("result envelope")
    };
    assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
    assert_eq!(body.format_id.as_deref(), Some("boot-debugger-result"));
    serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
}

#[test]
fn m_06_c_1_cases_structured_cli_mcp_shared_overlay_positions_progress_and_cancel() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
    std::fs::write(root.path().join("STATIC.md"), "disk").unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        ["vibe.toml", "STATIC.md", "new.md", "moved.md"]
            .into_iter()
            .enumerate()
            .map(|(i, path)| ResourceDeclaration {
                path: path.into(),
                role: if i == 0 {
                    ResourceRole::Manifest
                } else {
                    ResourceRole::Opaque
                },
            })
            .collect(),
        AnalysisLimits {
            resource_bytes: 4096,
            capture_bytes: 16384,
            open_buffers: 4,
        },
    )
    .unwrap();
    let artifact = host.resource("STATIC.md").unwrap();
    let new = host.resource("new.md").unwrap();
    let moved = host.resource("moved.md").unwrap();
    let client = host.client().unwrap();
    let route = Arc::new(BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap());
    let contexts = [
        ServerContext::with_store_root(root.path(), root.path().join("unused"))
            .with_boot_debugger_route(route.clone())
            .unwrap(),
        ServerContext::with_store_root(root.path(), root.path().join("unused"))
            .with_boot_debugger_route(route.clone())
            .unwrap(),
    ];
    let request = |command| q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command,
    };
    let same = |command| {
        let req = request(command);
        let c = cli(&route, &req);
        assert_eq!(
            c,
            route.exchange(&serde_json::to_vec(&boot_debugger::input(&req)).unwrap())
        );
        for ctx in &contexts {
            assert_eq!(c, mcp(ctx, &req));
        }
        c
    };
    let before = route.retain_capture(client.freeze().unwrap()).unwrap();
    let text = "<!-- vibe:static org.test/pkg — authored.md -->\n<!-- embed: spec://org.test/pkg/outer -->\n<!-- embed: spec://org.test/pkg/inner -->\noverlay\n<!-- /embed: spec://org.test/pkg/inner -->\n<!-- /embed: spec://org.test/pkg/outer -->\n";
    client
        .open(
            1,
            artifact,
            1,
            ByteEncoding::Utf8,
            Arc::from(text.as_bytes()),
            BufferIntent::Replace,
        )
        .unwrap();
    client
        .open(
            1,
            new,
            1,
            ByteEncoding::Utf8,
            Arc::from(&b"new"[..]),
            BufferIntent::Create,
        )
        .unwrap();
    let frozen = client.freeze().unwrap();
    let id = frozen.id().to_owned();
    let digest = frozen.resource(artifact).unwrap().content_digest().unwrap();
    assert_eq!(
        format!("boot-context:{}", frozen.context()),
        route.context_id()
    );
    route.retain_capture(frozen).unwrap();
    let inspect = || {
        same(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: id.clone(),
        })))
    };
    let stable = inspect();
    let view = result(stable.clone());
    let r::Body::Inspect(view) = view.body else {
        panic!()
    };
    assert_eq!(view.capture_id, id);
    assert_eq!(view.resources[1].digest.as_ref(), Some(&digest));
    assert_eq!(view.resources[1].buffer_version.as_deref(), Some("1"));
    assert!(view.nodes[0].artifact_lines.is_none());
    assert!(view.compiler_carrier_json.is_none());
    let inner = view
        .nodes
        .iter()
        .find(|n| n.requested.as_deref() == Some("spec://org.test/pkg/inner"))
        .unwrap();
    let lines = inner.artifact_lines.as_ref().unwrap();
    assert_eq!((&*lines.start_line, &*lines.end_line), ("3", "5"));
    assert_eq!(lines.digest, digest);
    assert_eq!(lines.snapshot_id, id);
    assert_eq!(lines.resource_id, view.resources[1].resource_id);
    let source = &view.sources[inner.source as usize];
    assert_eq!(source.artifact_resource_id, lines.resource_id);
    assert!(source.authored_resource_id.is_none());
    assert!(
        matches!(&source.identity,r::SourceIdentity::Spec(s) if s.address=="spec://org.test/pkg/inner")
    );
    let compare = |a: &str, b: &str| {
        let result = result(same(q::Command::Compare(Box::new(q::CommandCompare {
            snapshot_id: a.into(),
            other_snapshot_id: b.into(),
        }))));
        let r::Body::Compare(v) = result.body else {
            panic!()
        };
        v
    };
    let diff = compare(&before, &id);
    assert_eq!(diff.added, ["new.md"]);
    assert_eq!(diff.changed, ["STATIC.md"]);
    assert!(diff.removed.is_empty());
    let cost = result(same(q::Command::Cost(Box::new(q::CommandCost {
        snapshot_id: id.clone(),
    }))));
    let r::Body::Cost(cost) = cost.body else {
        panic!()
    };
    assert_eq!(cost.raw_bytes.exact, Some(text.len().to_string()));
    assert!(cost.tokens.unavailable.is_some());
    let explain = result(same(q::Command::Explain(Box::new(q::CommandExplain {
        snapshot_id: id.clone(),
        contribution: inner.contribution,
        occurrence: inner.occurrence,
    }))));
    let r::Body::Explain(explain) = explain.body else {
        panic!()
    };
    assert_eq!(explain.chain, [0, 2, 1]);
    client
        .change(
            1,
            artifact,
            1,
            2,
            ByteEncoding::Utf8,
            Arc::from(&b""[..]),
            BufferIntent::Delete,
        )
        .unwrap();
    let deleted = route.retain_capture(client.freeze().unwrap()).unwrap();
    let diff = compare(&id, &deleted);
    assert_eq!(diff.removed, ["STATIC.md"]);
    assert!(diff.added.is_empty() && diff.changed.is_empty());
    client
        .change(
            1,
            artifact,
            2,
            3,
            ByteEncoding::Utf8,
            Arc::from(text.as_bytes()),
            BufferIntent::Move { target: moved },
        )
        .unwrap();
    let relocated = route.retain_capture(client.freeze().unwrap()).unwrap();
    let diff = compare(&id, &relocated);
    assert_eq!(diff.removed, ["STATIC.md"]);
    assert_eq!(diff.added, ["moved.md"]);
    assert!(diff.changed.is_empty());
    std::fs::write(root.path().join("STATIC.md"), "later disk").unwrap();
    assert_eq!(stable, inspect());
    let chunk = result(same(q::Command::Chunk(Box::new(q::CommandChunk {
        snapshot_id: id.clone(),
        resource_id: view.resources[1].resource_id.clone(),
        filter: q::CommandChunkFilter::Raw,
        offset: "0".into(),
        limit: 2048,
    }))));
    let r::Body::Chunk(chunk) = chunk.body else {
        panic!()
    };
    assert_eq!(
        chunk.hex_bytes,
        text.as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let tree = result(cli(
        &route,
        &request(q::Command::Tree(Box::new(q::CommandTree {
            snapshot_id: id.clone(),
            inspection_id: None,
            cursor: None,
            filter: "".into(),
            limit: 1,
        }))),
    ));
    let r::Body::Tree(tree) = tree.body else {
        panic!()
    };
    let progress = same(q::Command::Replay(Box::new(q::CommandReplay {
        inspection_id: tree.inspection_id.clone(),
        after_sequence: "0".into(),
        limit: 64,
    })));
    let env::Body::Event(event) = progress.body else {
        panic!()
    };
    let progress: p::Progress =
        serde_json::from_str(event.payload_json.as_deref().unwrap()).unwrap();
    route.validate_progress(&progress).unwrap();
    assert_eq!(progress.snapshot_id, id);
    let cancelled = result(cli(
        &route,
        &request(q::Command::Cancel(Box::new(q::CommandCancel {
            inspection_id: tree.inspection_id.clone(),
        }))),
    ));
    let r::Body::Cancel(cancelled) = cancelled.body else {
        panic!()
    };
    assert_eq!(cancelled.scope, r::BodyCancelScope::InspectionOnly);
    assert_eq!(
        cancelled.build_cancelled,
        r::BodyCancelBuildCancelled::NotRequested
    );
    let refused = same(q::Command::Tree(Box::new(q::CommandTree {
        snapshot_id: id.clone(),
        inspection_id: Some(tree.inspection_id),
        cursor: None,
        filter: "".into(),
        limit: 1,
    })));
    let env::Body::Error(error) = refused.body else {
        panic!()
    };
    let error: e::Error = serde_json::from_str(error.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(error.code, e::ErrorCode::Cancelled);
    assert_eq!(stable, inspect());
    assert!(!root.path().join(".vibe").exists());
    assert!(!root.path().join("new.md").exists());
    assert!(!root.path().join("moved.md").exists());
}
