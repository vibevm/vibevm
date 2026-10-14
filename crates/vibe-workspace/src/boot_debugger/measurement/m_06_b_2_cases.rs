use super::*;

#[test]
fn streamed_large_utf8_is_exact_and_chunk_independent_without_quota() {
    struct Corpus {
        left: usize,
        position: usize,
        chunk: usize,
    }
    impl Read for Corpus {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = out.len().min(self.left).min(self.chunk);
            let word = "雪🙂a".as_bytes();
            for b in &mut out[..n] {
                *b = word[self.position % word.len()];
                self.position += 1;
            }
            self.left -= n;
            Ok(n)
        }
    }
    let size = 64 * 1024 * 1024;
    let a = measure_reader(Corpus {
        left: size,
        position: 0,
        chunk: 8191,
    })
    .unwrap();
    let b = measure_reader(Corpus {
        left: size,
        position: 0,
        chunk: 127,
    })
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(a.bytes, size as u64);
    assert_eq!(a.scalars, Count::exact(size as u64 / 8 * 3));
}

#[test]
fn repeated_sources_exact_crlf_and_unavailable_token_recipe() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.md"), "雪\r\n").unwrap();
    let text = "<!-- vibe:static org.demo/a — a.md -->\r\n雪\r\n<!-- vibe:static org.demo/a — a.md -->\r\n雪";
    let report = capture_texts(tmp.path(), Some(text), None);
    assert_eq!(report, capture_texts(tmp.path(), Some(text), None));
    let lane = &report.lanes[0];
    assert_eq!(lane.snippets.len(), 2);
    assert_eq!(lane.snippets[0].emitted.as_ref().unwrap().bytes, 5);
    assert_eq!(lane.snippets[1].emitted.as_ref().unwrap().bytes, 3);
    assert_eq!(lane.totals.unique_raw_bytes, Count::exact(5));
    assert_eq!(lane.totals.repeated_raw_bytes, Count::exact(10));
    assert_eq!(lane.totals.inclusive_bytes, Count::exact(8));
    assert_eq!(lane.totals.recorded_occurrences, 2);
    assert!(matches!(report.tokens.count, Count::Unavailable { .. }));
    assert!(report.tokens.tokenizer.is_none());
    assert!(matches!(report.ambient_tail, Count::Unavailable { .. }));
}

#[test]
fn mapping_gaps_preserve_all_children_without_proportions() {
    let mut graph = BootGraph::default();
    let source = graph.source(
        vibe_spec::SourceIdentity::Spec("spec://org.demo/a/x#root".into()),
        None,
    );
    graph.contributions.push(vibe_spec::GraphContribution {
        origin: "org.demo/a".into(),
        path: "a".into(),
        kind: "normal".into(),
        seed: Some(source),
        occurrences: (0..10000)
            .map(|_| vibe_spec::GraphOccurrence {
                source,
                requested: None,
                state: vibe_spec::ReferenceState::Unknown,
                parent: vibe_spec::InclusionParent::Unknown,
                artifact_lines: None,
            })
            .collect(),
    });
    let report = measure_graph("captured", &graph, &BTreeMap::new(), &BTreeMap::new());
    assert_eq!(report.snippets[0].occurrences.len(), 10000);
    assert!(matches!(
        report.totals.inclusive_bytes,
        Count::Unavailable { .. }
    ));
    assert!(matches!(
        report.totals.direct_bytes,
        Count::Unavailable { .. }
    ));
    assert!(
        report.snippets[0]
            .occurrences
            .iter()
            .all(|o| o.parent.is_none() && !o.root && o.emitted.is_none())
    );
}

#[test]
fn dynamic_declarations_repeat_without_claiming_runtime_reads() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a"), "🙂").unwrap();
    let text = "[[entry]]\npath='a'\nkind='dynamic'\nwhen='os:unknown'\n[[entry]]\npath='a'\nkind='dynamic'\n";
    let report = capture_texts(tmp.path(), None, Some(text));
    let lane = &report.lanes[1];
    assert_eq!(lane.snippets.len(), 2);
    assert_eq!(lane.snippets[0].condition.as_deref(), Some("os:unknown"));
    assert_eq!(lane.totals.unique_raw_bytes, Count::exact(4));
    assert_eq!(lane.totals.repeated_raw_bytes, Count::exact(8));
    assert!(matches!(
        lane.totals.runtime_reads,
        Count::Unavailable { .. }
    ));
    assert!(lane.snippets.iter().all(|s| s.emitted.is_none()));
    assert!(!tmp.path().join(".vibe").exists());
}

#[test]
fn invalid_utf8_does_not_invent_scalars_or_empty_payload() {
    let p = measure_reader(&b"a\xff"[..]).unwrap();
    assert_eq!(p.bytes, 2);
    assert!(matches!(p.scalars, Count::Unavailable { .. }));
}

#[test]
fn nested_recorded_ranges_prove_direct_bytes_without_guessing_missing_maps() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = "<!-- embed: child -->\r\n雪\r\n<!-- /embed: child -->\r\n";
    let text = format!("<!-- vibe:static host — missing.md -->\r\nparent\r\n{nested}tail");
    let report = capture_texts(tmp.path(), Some(&text), None);
    let row = &report.lanes[0].snippets[0];
    assert_eq!(
        row.direct_bytes,
        Count::exact("parent\r\ntail".len() as u64)
    );
    assert_eq!(
        row.occurrences[1].emitted.as_ref().unwrap().bytes,
        nested.len() as u64
    );
    assert!(row.raw.is_none());
    assert!(matches!(
        report.lanes[0].totals.unique_raw_bytes,
        Count::Unavailable { .. }
    ));
    assert!(matches!(
        report.lanes[1].totals.unique_raw_bytes,
        Count::Unavailable { .. }
    ));
}

#[test]
fn large_captured_graph_keeps_repeated_emitted_payload_and_one_source_read() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "雪abc\n".repeat(300_000);
    std::fs::write(tmp.path().join("large.md"), &body).unwrap();
    let mut text = String::new();
    for _ in 0..8 {
        text.push_str("<!-- vibe:static host — large.md -->\n");
        text.push_str(&body);
    }
    let report = capture_texts(tmp.path(), Some(&text), None);
    let lane = &report.lanes[0];
    assert_eq!(lane.snippets.len(), 8);
    assert_eq!(lane.sources.len(), 1);
    assert_eq!(
        lane.totals.unique_emitted_bytes,
        Count::exact(body.len() as u64)
    );
    assert_eq!(
        lane.totals.repeated_emitted_bytes,
        Count::exact(body.len() as u64 * 8)
    );
    assert_eq!(lane.totals.direct_bytes, lane.totals.repeated_emitted_bytes);
    assert_eq!(report.inspected_source_reads["large.md"], Count::exact(1));
    assert!(matches!(
        lane.totals.runtime_reads,
        Count::Unavailable { .. }
    ));
}

#[test]
fn retained_carrier_measures_bytes_without_replaying_provider_or_emitter() {
    use crate::compile_trace::{ScopeDescriptor, TraceRun};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vibe_spec::{ArtifactInput, ArtifactPlan, ArtifactTarget, SectionSource, SpecAddress};
    use vibe_wire::generated::compiler_trace_index::e1::index::{
        ArtifactTarget as WireTarget, ScopeKind, Timestamp,
    };
    struct World(AtomicUsize);
    impl SectionSource for World {
        fn section_text(&self, _: &SpecAddress) -> Result<String, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok("# Root {#root}\n雪🙂\n".into())
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let id = "0123456789abcdef0123456789abcdef";
    // Fixture preparation only. Measurement below receives no world or writer.
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
    let before = std::fs::read(run.run_dir().join("index.json")).unwrap();
    let calls = world.0.load(Ordering::SeqCst);
    let captured = crate::boot_debugger::retained::read_run(dir.path(), id).unwrap();
    let mut emitted = 0;
    let mut linked = 0;
    let mut raw = 0;
    for capture in captured.snapshots {
        let capture = capture.unwrap();
        let report = measure_retained(&capture);
        assert!(report.capture_id.contains(&capture.snapshot));
        assert_eq!(report.snapshot.bytes, capture.bytes.len() as u64);
        if matches!(
            capture.graph.carrier,
            Some(vibe_wire::generated::compiler_ir::e1::ir::Ir::EmittedArtifact(_))
        ) {
            assert!(report.emitted_artifact.is_some(), "{:?}", report.gaps);
        }
        if let Some(payload) = report.emitted_artifact {
            assert!(payload.bytes > 0);
            assert!(matches!(payload.scalars, Count::Exact { .. }));
            emitted += 1;
        }
        if !report.linked_occurrences.is_empty() {
            assert_eq!(report.linked_occurrences.len(), 2);
            let Count::Exact { value: unique } = report.unique_linked_body_bytes else {
                panic!("exact unique bodies")
            };
            let Count::Exact { value: repeated } = report.repeated_linked_body_bytes else {
                panic!("exact repeated bodies")
            };
            // Compiler transforms may change the two bodies of one source.
            // Logical source sharing never proves byte-payload equality.
            assert!(repeated >= unique && unique > 0);
            assert_eq!(
                repeated,
                report
                    .linked_occurrences
                    .iter()
                    .map(|o| o.body.bytes)
                    .sum::<u64>()
            );
            linked += 1;
        }
        raw += report
            .graph
            .sources
            .iter()
            .filter(|s| s.raw.is_some())
            .count();
    }
    assert!(
        emitted > 0 && linked > 0 && raw > 0,
        "emitted={emitted} linked={linked} raw={raw}"
    );
    assert_eq!(world.0.load(Ordering::SeqCst), calls);
    assert_eq!(
        std::fs::read(run.run_dir().join("index.json")).unwrap(),
        before
    );
}
