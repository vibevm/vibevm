use super::*;
use crate::document_analysis::{AnalysisLimits, ResourceDeclaration, ResourceRole};
fn fixture() -> (tempfile::TempDir, BootDebuggerRoute, String) {
    let root = tempfile::tempdir().unwrap();
    let artifact = (0..80)
        .map(|i| format!("[[entry]]\npath='source-{i}.md'\nkind='dynamic'\n"))
        .collect::<String>();
    std::fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
    std::fs::write(root.path().join("INDEX.md"), &artifact).unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "INDEX.md".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 100_000,
            capture_bytes: 200_000,
            open_buffers: 2,
        },
    )
    .unwrap();
    let id = host.resource("INDEX.md").unwrap();
    (
        root,
        BootDebuggerRoute::new(host, id, CapturedKind::Index).unwrap(),
        artifact,
    )
}
fn ask(route: &BootDebuggerRoute, command: q::Command) -> Answer<r::Result> {
    route.request(q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command,
    })
}
fn capture(route: &BootDebuggerRoute) -> String {
    ask(route, q::Command::Capture(Box::new(q::CommandCapture {})))
        .unwrap()
        .snapshot_id
        .unwrap()
}
fn tree(
    snapshot: &str,
    inspection: Option<String>,
    cursor: Option<String>,
    filter: &str,
) -> q::Command {
    q::Command::Tree(Box::new(q::CommandTree {
        snapshot_id: snapshot.into(),
        inspection_id: inspection,
        cursor,
        filter: filter.into(),
        limit: 1,
    }))
}
fn page(result: r::Result) -> (String, page::BoundPage) {
    let r::Body::Tree(body) = result.body else {
        panic!("tree result")
    };
    (
        body.inspection_id,
        serde_json::from_str(&body.page_json).unwrap(),
    )
}

#[test]
fn m_06_c_1_cases_independent_clients_and_immutable_capture_cost_compare() {
    let (root, route, _) = fixture();
    let id = capture(&route);
    let request = q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command: q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: id.clone(),
        })),
    };
    let wire = serde_json::to_vec(&input(&request)).unwrap();
    let first = route.exchange(&wire);
    std::fs::write(root.path().join("INDEX.md"), "").unwrap();
    assert_eq!(first, route.exchange(&wire));
    let new = capture(&route);
    let compared = ask(
        &route,
        q::Command::Compare(Box::new(q::CommandCompare {
            snapshot_id: id.clone(),
            other_snapshot_id: new,
        })),
    )
    .unwrap();
    let r::Body::Compare(diff) = compared.body else {
        panic!()
    };
    assert_eq!(diff.changed, ["INDEX.md"]);
    let cost = ask(
        &route,
        q::Command::Cost(Box::new(q::CommandCost {
            snapshot_id: id.clone(),
        })),
    )
    .unwrap();
    let r::Body::Cost(cost) = cost.body else {
        panic!()
    };
    assert!(cost.raw_bytes.exact.unwrap().parse::<u64>().unwrap() > 0);
    assert!(cost.tokens.exact.is_none() && cost.tokens.unavailable.is_some());
    assert!(cost.provider_billing.exact.is_none());
    let explain = ask(
        &route,
        q::Command::Explain(Box::new(q::CommandExplain {
            snapshot_id: id,
            contribution: 0,
            occurrence: 0,
        })),
    )
    .unwrap();
    let r::Body::Explain(explain) = explain.body else {
        panic!()
    };
    assert_eq!(explain.chain, [0]);
    assert!(!root.path().join(".vibe").exists());
}
#[test]
fn m_06_c_1_cases_scope_epoch_filters_cursors_and_cancellation_boundary() {
    let (_root, route, _) = fixture();
    let id = capture(&route);
    let (inspection, first) = page(ask(&route, tree(&id, None, None, "")).unwrap());
    let cursor = first.next_cursor_id.unwrap();
    assert_eq!(
        ask(
            &route,
            tree(&id, Some(inspection.clone()), Some(cursor.clone()), "other")
        )
        .unwrap_err()
        .code,
        e::ErrorCode::MixedFilter
    );
    let foreign = capture(&route);
    assert_eq!(
        ask(
            &route,
            q::Command::Chunk(Box::new(q::CommandChunk {
                snapshot_id: foreign.clone(),
                resource_id: format!("{id}:resource-1"),
                filter: q::CommandChunkFilter::Raw,
                offset: "0".into(),
                limit: 13,
            }))
        )
        .unwrap_err()
        .code,
        e::ErrorCode::UnknownResource
    );

    assert_eq!(
        ask(
            &route,
            tree(&foreign, Some(inspection.clone()), Some(cursor.clone()), "")
        )
        .unwrap_err()
        .code,
        e::ErrorCode::MixedFilter
    );
    assert!(
        route
            .request(q::Request {
                schema: 2,
                context_id: route.context_id().into(),
                command: tree(&id, None, None, "")
            })
            .is_err()
    );
    assert!(
        route
            .request(q::Request {
                schema: 1,
                context_id: "foreign".into(),
                command: tree(&id, None, None, "")
            })
            .is_err()
    );
    ask(
        &route,
        tree(&id, Some(inspection.clone()), Some(cursor.clone()), ""),
    )
    .unwrap();
    assert_eq!(
        ask(
            &route,
            tree(&id, Some(inspection.clone()), Some(cursor), "")
        )
        .unwrap_err()
        .code,
        e::ErrorCode::StaleCursor
    );
    let cancelled = ask(
        &route,
        q::Command::Cancel(Box::new(q::CommandCancel {
            inspection_id: inspection.clone(),
        })),
    )
    .unwrap();
    let r::Body::Cancel(cancelled) = cancelled.body else {
        panic!()
    };
    assert_eq!(cancelled.scope, r::BodyCancelScope::InspectionOnly);
    assert_eq!(
        cancelled.build_cancelled,
        r::BodyCancelBuildCancelled::NotRequested
    );
    assert_eq!(
        ask(&route, tree(&id, Some(inspection), None, ""))
            .unwrap_err()
            .code,
        e::ErrorCode::Cancelled
    );
    assert!(
        ask(
            &route,
            q::Command::Inspect(Box::new(q::CommandInspect { snapshot_id: id }))
        )
        .is_ok()
    );
}
#[test]
fn m_06_c_1_cases_complete_page_and_chunk_reassembly_and_explicit_replay_gap() {
    let (_root, route, original) = fixture();
    let id = capture(&route);
    let mut inspection = None;
    let mut cursor = None;
    let mut rows = Vec::new();
    loop {
        let (i, p) = page(ask(&route, tree(&id, inspection.clone(), cursor, "")).unwrap());
        inspection = Some(i);
        rows.extend(p.rows);
        cursor = p.next_cursor_id;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(rows.len(), 80);
    let replay = ask(
        &route,
        q::Command::Replay(Box::new(q::CommandReplay {
            inspection_id: inspection.clone().unwrap(),
            after_sequence: "0".into(),
            limit: 64,
        })),
    )
    .unwrap();
    let r::Body::Replay(replay) = replay.body else {
        panic!()
    };
    let message: stream::OperationStreamMessage =
        serde_json::from_str(&replay.stream_json).unwrap();
    stream_message::validate(&message, route.binding(inspection.as_deref().unwrap())).unwrap();
    let stream::Body::ReplayReply(replay) = message.body else {
        panic!()
    };
    assert!(replay.gap.is_some());
    assert_eq!(replay.events.len(), 64);
    let request = q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command: q::Command::Replay(Box::new(q::CommandReplay {
            inspection_id: inspection.clone().unwrap(),
            after_sequence: "0".into(),
            limit: 64,
        })),
    };
    let published = route.exchange(&serde_json::to_vec(&input(&request)).unwrap());
    let env::Body::Event(event) = published.body else {
        panic!("generated progress envelope")
    };
    assert_eq!(event.effect_state, env::BodyEventEffectState::NoEffects);
    let mut progress: vibe_wire::generated::boot_debugger::e1::progress::Progress =
        serde_json::from_str(event.payload_json.as_deref().unwrap()).unwrap();
    route.validate_progress(&progress).unwrap();
    progress.snapshot_id = "foreign".into();
    assert!(route.validate_progress(&progress).is_err());
    let mut offset = "0".to_string();
    let mut joined = Vec::new();
    loop {
        let response = ask(
            &route,
            q::Command::Chunk(Box::new(q::CommandChunk {
                snapshot_id: id.clone(),
                resource_id: format!("{id}:resource-1"),
                filter: q::CommandChunkFilter::Raw,
                offset,
                limit: 13,
            })),
        )
        .unwrap();
        let r::Body::Chunk(chunk) = response.body else {
            panic!()
        };
        for pair in chunk.hex_bytes.as_bytes().chunks_exact(2) {
            joined.push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
        }
        if let Some(next) = chunk.next_offset {
            offset = next;
        } else {
            break;
        }
    }
    assert_eq!(joined, original.as_bytes());
}

#[test]
fn m_06_c_1_cases_same_client_overlay_presence_and_retained_basis() {
    use crate::document_analysis::{BufferIntent, ByteEncoding};
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
    std::fs::write(root.path().join("STATIC.md"), "old disk").unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        ["vibe.toml", "STATIC.md", "created.md", "moved.md"]
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
    let created = host.resource("created.md").unwrap();
    let moved = host.resource("moved.md").unwrap();
    let client = host.client().unwrap();
    let route = BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap();
    let freeze = || {
        let c = client.freeze().unwrap();
        let id = c.id().to_owned();
        assert_eq!(format!("boot-context:{}", c.context()), route.context_id());
        assert_eq!(route.retain_capture(c).unwrap(), id);
        id
    };
    let before = freeze();
    client
        .open(
            1,
            created,
            1,
            ByteEncoding::Utf8,
            Arc::from(&b"created"[..]),
            BufferIntent::Create,
        )
        .unwrap();
    let added = freeze();
    let compare = |a: &str, b: &str| {
        let answer = ask(
            &route,
            q::Command::Compare(Box::new(q::CommandCompare {
                snapshot_id: a.into(),
                other_snapshot_id: b.into(),
            })),
        )
        .unwrap();
        let r::Body::Compare(diff) = answer.body else {
            panic!()
        };
        diff
    };
    let d = compare(&before, &added);
    assert_eq!(d.added, ["created.md"]);
    assert!(d.changed.is_empty() && d.removed.is_empty());
    let text = "<!-- vibe:static org.test/pkg — input.md -->\n<!-- embed: spec://org.test/pkg/outer -->\n<!-- embed: spec://org.test/pkg/inner -->\nretained overlay\n<!-- /embed: spec://org.test/pkg/inner -->\n<!-- /embed: spec://org.test/pkg/outer -->\n";
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
    let edited = freeze();
    let d = compare(&added, &edited);
    assert_eq!(d.changed, ["STATIC.md"]);
    assert!(d.added.is_empty() && d.removed.is_empty());
    let inspect = || {
        ask(
            &route,
            q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: edited.clone(),
            })),
        )
        .unwrap()
    };
    let stable = inspect();
    let r::Body::Inspect(view) = &stable.body else {
        panic!()
    };
    assert_eq!(view.capture_id, edited);
    assert_eq!(view.resources[1].buffer_version.as_deref(), Some("1"));
    assert_eq!(view.resources[1].basis, r::ResourceBasis::OriginalBytes);
    assert!(view.nodes[0].artifact_lines.is_none());
    assert_eq!(view.sources[0].source, 0);
    assert_eq!(
        view.sources[0].artifact_resource_id,
        view.resources[1].resource_id
    );
    for node in &view.nodes {
        let source = &view.sources[node.source as usize];
        assert_eq!(source.snapshot_id, edited);
        assert!(source.authored_resource_id.is_none());
        if let Some(lines) = &node.artifact_lines {
            assert_eq!(lines.snapshot_id, edited);
            assert_eq!(lines.resource_id, view.resources[1].resource_id);
            assert_eq!(Some(&lines.digest), view.resources[1].digest.as_ref());
            assert_eq!(
                lines.basis,
                r::ArtifactLinesBasis::CapturedEmittedArtifactOneBasedInclusiveLines
            );
            let r::SourceIdentity::Spec(identity) = &source.identity else {
                panic!()
            };
            assert_eq!(Some(&identity.address), node.requested.as_ref());
            if identity.address.ends_with("/inner") {
                assert_eq!((&*lines.start_line, &*lines.end_line), ("3", "5"));
                assert_eq!(node.parent, Some(2));
            } else {
                assert_eq!((&*lines.start_line, &*lines.end_line), ("2", "6"));
            }
        }
    }
    client
        .change(
            1,
            artifact,
            1,
            2,
            ByteEncoding::Utf8,
            Arc::from(&b"later overlay"[..]),
            BufferIntent::Replace,
        )
        .unwrap();
    std::fs::write(root.path().join("STATIC.md"), "later disk").unwrap();
    assert_eq!(stable, inspect());
    let chunk = ask(
        &route,
        q::Command::Chunk(Box::new(q::CommandChunk {
            snapshot_id: edited.clone(),
            resource_id: view.resources[1].resource_id.clone(),
            filter: q::CommandChunkFilter::Raw,
            offset: "0".into(),
            limit: 2048,
        })),
    )
    .unwrap();
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
    client
        .change(
            1,
            artifact,
            2,
            3,
            ByteEncoding::Utf8,
            Arc::from(&b""[..]),
            BufferIntent::Delete,
        )
        .unwrap();
    let deleted = freeze();
    let d = compare(&edited, &deleted);
    assert_eq!(d.removed, ["STATIC.md"]);
    assert!(d.changed.is_empty() && d.added.is_empty());
    client
        .change(
            1,
            artifact,
            3,
            4,
            ByteEncoding::Utf8,
            Arc::from(text.as_bytes()),
            BufferIntent::Move { target: moved },
        )
        .unwrap();
    let relocated = freeze();
    let d = compare(&edited, &relocated);
    assert_eq!(d.removed, ["STATIC.md"]);
    assert_eq!(d.added, ["moved.md"]);
    assert!(d.changed.is_empty());
    assert_eq!(stable, inspect());
    assert!(!root.path().join("created.md").exists());
    assert!(!root.path().join("moved.md").exists());
    assert_eq!(
        std::fs::read_to_string(root.path().join("STATIC.md")).unwrap(),
        "later disk"
    );
    // Read failure is not an absent resource and cannot publish a new snapshot.
    std::fs::create_dir(root.path().join("created.md")).unwrap();
    assert!(client.freeze().is_err());
    assert_eq!(stable, inspect());
}
