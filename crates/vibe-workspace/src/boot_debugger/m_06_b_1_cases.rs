use super::*;

#[test]
fn repeated_sources_nested_why_positions_and_capture_identity() {
    let row = StaticContribution {
        order: 0,
        origin: "host".into(),
        source_path: "a.md".into(),
        bytes: 1,
        lines: 1,
        embeds: vec![
            EmbedSpan {
                address: "shared".into(),
                start_line: 2,
                end_line: 8,
            },
            EmbedSpan {
                address: "shared".into(),
                start_line: 3,
                end_line: 5,
            },
        ],
    };
    let capture = from_static_summary("capture-001/exact".into(), &[row.clone(), row]);
    assert_eq!(capture.capture_id, "capture-001/exact");
    let graph = capture.graph;
    assert_eq!(graph.sources.len(), 2);
    assert_eq!(graph.contributions.len(), 2);
    let row = &graph.contributions[0];
    assert_eq!(row.occurrences.len(), 3);
    assert_eq!(row.occurrences[1].source, row.occurrences[2].source);
    assert_eq!(why_included(row, 2), Some(vec![0, 1, 2]));
    assert_eq!(row.occurrences[2].artifact_lines, Some((3, 5)));
    assert_eq!(row.occurrences[2].state, ReferenceState::Unknown);
    assert_ne!(row.occurrences[2].state, ReferenceState::Unresolved);
}

#[test]
fn retained_absence_and_corruption_never_create_writer_state() {
    let dir = tempfile::tempdir().unwrap();
    let run = "0123456789abcdef0123456789abcdef";
    assert!(matches!(
        retained::read(dir.path(), run, 0),
        Err(retained::RetainedUnavailable::Missing)
    ));
    assert!(!dir.path().join(".vibe").exists());
    let path = dir.path().join(".vibe/trace").join(run);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("index.json"), b"broken").unwrap();
    assert!(matches!(
        retained::read(dir.path(), run, 0),
        Err(retained::RetainedUnavailable::Unreadable(_))
    ));
    assert!(!dir.path().join(".vibe/compile-trace.lock").exists());
    assert_eq!(std::fs::read(path.join("index.json")).unwrap(), b"broken");
}

#[test]
fn retained_exact_snapshot_reads_while_writer_is_held_without_provider_calls() {
    use crate::compile_trace::{ScopeDescriptor, TraceRun};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vibe_spec::{ArtifactInput, ArtifactPlan, ArtifactTarget, SectionSource, SpecAddress};
    use vibe_wire::generated::compiler_trace_index::e1::index::{
        ArtifactTarget as WireTarget, CompilerTraceIndex, ScopeKind, Timestamp,
    };
    struct World(AtomicUsize);
    impl SectionSource for World {
        fn section_text(&self, _: &SpecAddress) -> Result<String, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok("# Root {#root}\nbody\n".into())
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let id = "0123456789abcdef0123456789abcdef";
    let run = TraceRun::open(dir.path(), id, Timestamp::from_timestamp(1000, 0).unwrap()).unwrap();
    let scope = run
        .declare_scope(&ScopeDescriptor {
            id: "node:.:static-md".into(),
            kind: ScopeKind::Node,
            label: ".".into(),
            artifact: "static-md".into(),
            target: WireTarget::StaticMd,
        })
        .unwrap();
    let input = || {
        ArtifactInput::normal(
            "org.demo/alpha",
            "boot/a.md",
            SpecAddress::parse("spec://org.demo/alpha/boot/a#root").unwrap(),
        )
        .unwrap()
    };
    let plan = ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "boot/STATIC.md",
        "spec",
        vec![input(), input()],
    )
    .unwrap();
    let world = World(AtomicUsize::new(0));
    vibe_spec::compile_artifact_traced(plan, &world, &scope).unwrap();
    let index_bytes = std::fs::read(run.run_dir().join("index.json")).unwrap();
    let index: CompilerTraceIndex = serde_json::from_slice(&index_bytes).unwrap();
    let event = index.events.iter().find(|e| e.pass == "close").unwrap();
    let calls = world.0.load(Ordering::SeqCst);
    let all = retained::read_run(dir.path(), id).unwrap();
    assert_eq!(all.index, index);
    assert_eq!(all.snapshots.len(), index.events.len());
    for (event, snapshot) in index.events.iter().zip(&all.snapshots) {
        let snapshot = snapshot.as_ref().unwrap();
        assert_eq!(snapshot.sequence, event.sequence);
        assert!(snapshot.graph.carrier.is_some());
        if event.pass == "assemble" {
            assert_eq!(snapshot.graph.contributions.len(), 2);
            assert_eq!(snapshot.graph.contributions[0].occurrences.len(), 1);
            assert_eq!(
                snapshot.graph.contributions[0].occurrences[0].source,
                snapshot.graph.contributions[1].occurrences[0].source
            );
            assert_eq!(why_included(&snapshot.graph.contributions[0], 0), None);
        }
        if event.pass.starts_with("emit:") {
            assert_eq!(snapshot.graph.contributions.len(), 2);
            assert!(snapshot.graph.contributions[0].occurrences.is_empty());
            assert!(
                snapshot
                    .graph
                    .gaps
                    .iter()
                    .any(|g| g.contains("chunk_digest"))
            );
        }
    }
    assert!(index.events.iter().any(|e| e.pass == "assemble"));
    assert!(index.events.iter().any(|e| e.pass.starts_with("emit:")));
    let captured = retained::read(dir.path(), id, event.sequence).unwrap();
    assert_eq!(captured.index.run_id, id);
    assert_eq!(captured.scope, event.scope);
    assert_eq!(captured.sequence, event.sequence);
    assert_eq!(captured.invocation, event.invocation);
    assert_eq!(Some(&captured.snapshot), event.snapshot.as_ref());
    assert_eq!(captured.graph.contributions.len(), 2);
    assert_eq!(
        captured.graph.contributions[0].occurrences[0].source,
        captured.graph.contributions[1].occurrences[0].source
    );
    assert_eq!(world.0.load(Ordering::SeqCst), calls);
    assert_eq!(
        std::fs::read(run.run_dir().join("index.json")).unwrap(),
        index_bytes
    );
    std::fs::remove_file(run.run_dir().join(&captured.snapshot)).unwrap();
    assert!(matches!(
        retained::read(dir.path(), id, event.sequence),
        Err(retained::RetainedUnavailable::Missing)
    ));
    let missing_child = retained::read_run(dir.path(), id).unwrap();
    assert!(matches!(
        missing_child.snapshots[event.sequence as usize],
        Err(retained::RetainedUnavailable::Missing)
    ));
    assert_eq!(
        missing_child.snapshots.iter().filter(|s| s.is_ok()).count(),
        index.events.len() - 1
    );
    assert!(matches!(
        retained::read(dir.path(), id, u32::MAX),
        Err(retained::RetainedUnavailable::Unavailable)
    ));
}

#[test]
fn dynamic_index_preserves_repeats_conditions_and_unknown_kinds() {
    let capture = dynamic_index::capture_index(
        "index/exact".into(),
        r#"
static = "STATIC.md"
[[entry]]
path = "same.md"
kind = "dynamic"
when = "os:linux"
[[entry]]
path = "same.md"
kind = "future-kind"
when = "unknown:predicate"
"#,
    )
    .unwrap();
    assert_eq!(capture.capture_id, "index/exact");
    assert_eq!(capture.index.static_pointer.as_deref(), Some("STATIC.md"));
    assert_eq!(
        capture.index.entries[1].when.as_deref(),
        Some("unknown:predicate")
    );
    assert_eq!(capture.graph.contributions.len(), 2);
    assert_eq!(capture.graph.contributions[1].kind, "future-kind");
    assert_eq!(
        capture.graph.contributions[0].seed,
        capture.graph.contributions[1].seed
    );
    assert_eq!(
        capture.graph.contributions[1].occurrences[0].state,
        ReferenceState::Unknown
    );
    assert!(dynamic_index::capture_index("invalid".into(), "[[entry]]").is_err());
}
