use super::*;
include!("m_06_f_2_fixture.rs");

#[test]
fn million_units_keep_repeats_conditions_and_reproducible_qualified_measurements() {
    let root = tempfile::tempdir().unwrap();
    let text = scale_static();
    assert_eq!(text.matches("λ ").count(), 1_048_576);
    std::fs::write(root.path().join("repeated.md"), "λ ".repeat(8192)).unwrap();
    std::fs::write(root.path().join("ordinary.txt"), b"ordinary\0raw\xff").unwrap();
    let first = measurement::capture_texts(root.path(), Some(&text), Some(SCALE_INDEX));
    assert_eq!(
        first,
        measurement::capture_texts(root.path(), Some(&text), Some(SCALE_INDEX))
    );
    let lane = &first.lanes[0];
    assert_eq!(lane.totals.recorded_occurrences, 128);
    assert_eq!(
        lane.totals.unique_raw_bytes,
        measurement::Count::exact(24576)
    );
    assert_eq!(
        lane.totals.repeated_raw_bytes,
        measurement::Count::exact(128 * 24576)
    );
    assert!(matches!(
        first.tokens.count,
        measurement::Count::Unavailable { .. }
    ));
    assert!(first.tokens.tokenizer.is_none());
    let index = dynamic_index::capture_index("conditional".into(), SCALE_INDEX).unwrap();
    assert_eq!(index.index.entries.len(), 2);
    assert_eq!(index.index.entries[1].when.as_deref(), Some("os:linux"));
    assert_eq!(
        index.graph.contributions[0].seed,
        index.graph.contributions[1].seed
    );
    let payload = measurement::measure_reader(text.as_bytes()).unwrap();
    assert_eq!(payload.bytes, text.len() as u64);
    assert_eq!(
        payload.scalars.exact_value(),
        Some(text.chars().count() as u64)
    );
    eprintln!(
        "m06f2 recipe=v1 units=1048576 bytes={} scalars={} unique_raw=24576 repeated_raw={} tokenizer=unavailable",
        payload.bytes,
        text.chars().count(),
        128 * 24576
    );
}

#[test]
fn trace_budget_retention_and_missing_children_do_not_change_boot() {
    use crate::compile_trace::{RunOutcome, ScopeDescriptor, TraceLimits, TraceRun};
    use vibe_spec::{ArtifactInput, ArtifactPlan, ArtifactTarget, SectionSource, SpecAddress};
    use vibe_wire::generated::compiler_trace_index::e1::index::{
        ArtifactTarget as Target, ScopeKind, Timestamp,
    };
    struct NoReads;
    impl SectionSource for NoReads {
        fn section_text(&self, _: &SpecAddress) -> Result<String, String> {
            panic!("simple input must not read a provider")
        }
    }
    let root = tempfile::tempdir().unwrap();
    let plan = || {
        ArtifactPlan::static_lane(
            ArtifactTarget::StaticMarkdown,
            "STATIC.md",
            "spec",
            vec![
                ArtifactInput::simple("org.example/scale", "a.md", "# Scale\nstatus=spec/plan\nλ ")
                    .unwrap(),
            ],
        )
        .unwrap()
    };
    let baseline = vibe_spec::compile_artifact(plan(), &NoReads).unwrap();
    let id = "0123456789abcdef0123456789abcdef";
    let timestamp = || Timestamp::from_timestamp(1000, 0).unwrap();
    let run = TraceRun::open_with_limits(root.path(), id, timestamp(), TraceLimits::for_test(0, 0))
        .unwrap();
    let scope = run
        .declare_scope(&ScopeDescriptor {
            id: "node:.:static-md".into(),
            kind: ScopeKind::Node,
            label: ".".into(),
            artifact: "static-md".into(),
            target: Target::StaticMd,
        })
        .unwrap();
    let traced = vibe_spec::compile_artifact_traced(plan(), &NoReads, &scope).unwrap();
    assert_eq!(traced.bytes(), baseline.bytes());
    scope.complete("fixture").unwrap();
    let all = retained::read_run(root.path(), id).unwrap();
    assert!(!all.snapshots.is_empty());
    assert!(
        all.snapshots
            .iter()
            .all(|s| matches!(s, Err(retained::RetainedUnavailable::Unavailable)))
    );
    run.finish(&RunOutcome::Ok, timestamp());
    drop(scope);
    drop(run);
    let next = TraceRun::open_with_limits(
        root.path(),
        "1123456789abcdef0123456789abcdef",
        timestamp(),
        TraceLimits::for_test(0, 0),
    )
    .unwrap();
    assert!(!root.path().join(".vibe/trace").join(id).exists());
    assert!(matches!(
        retained::read_run(root.path(), id),
        Err(retained::RetainedUnavailable::Missing)
    ));
    assert_eq!(
        vibe_spec::compile_artifact(plan(), &NoReads)
            .unwrap()
            .bytes(),
        baseline.bytes()
    );
    drop(next);
    eprintln!(
        "m06f2 trace budget=0: explicit unavailable; retention removed owned completed run; absent read=Missing, not inferred Evicted; canonical identical"
    );
}

#[test]
fn large_complete_trace_keeps_exact_snapshots_and_explicit_missing_child() {
    use crate::compile_trace::{ScopeDescriptor, TraceRun};
    use vibe_spec::{ArtifactInput, ArtifactPlan, ArtifactTarget, SectionSource, SpecAddress};
    use vibe_wire::generated::compiler_trace_index::e1::index::{
        ArtifactTarget as Target, ScopeKind, Timestamp,
    };
    struct NoReads;
    impl SectionSource for NoReads {
        fn section_text(&self, _: &SpecAddress) -> Result<String, String> {
            panic!("unexpected provider")
        }
    }
    let text = format!("# Scale\n{}", "λ ".repeat(1_048_576));
    let plan = || {
        ArtifactPlan::static_lane(
            ArtifactTarget::StaticMarkdown,
            "STATIC.md",
            "spec",
            vec![ArtifactInput::simple("org.example/scale", "a.md", &text).unwrap()],
        )
        .unwrap()
    };
    let baseline = vibe_spec::compile_artifact(plan(), &NoReads).unwrap();
    let root = tempfile::tempdir().unwrap();
    let id = "2123456789abcdef0123456789abcdef";
    let run = TraceRun::open(root.path(), id, Timestamp::from_timestamp(1000, 0).unwrap()).unwrap();
    let scope = run
        .declare_scope(&ScopeDescriptor {
            id: "node:.:static-md".into(),
            kind: ScopeKind::Node,
            label: ".".into(),
            artifact: "static-md".into(),
            target: Target::StaticMd,
        })
        .unwrap();
    assert_eq!(
        vibe_spec::compile_artifact_traced(plan(), &NoReads, &scope)
            .unwrap()
            .bytes(),
        baseline.bytes()
    );
    let all = retained::read_run(root.path(), id).unwrap();
    assert!(!all.snapshots.is_empty());
    let mut snapshot_bytes = 0;
    for (event, child) in all.index.events.iter().zip(&all.snapshots) {
        let child = child
            .as_ref()
            .expect("complete successful capture at this fixture scale");
        assert_eq!(child.sequence, event.sequence);
        assert_eq!(
            child.bytes,
            std::fs::read(run.run_dir().join(&child.snapshot)).unwrap()
        );
        assert!(child.graph.carrier.is_some());
        snapshot_bytes += child.bytes.len();
    }
    let first = all.snapshots[0].as_ref().unwrap();
    std::fs::remove_file(run.run_dir().join(&first.snapshot)).unwrap();
    let missing = retained::read_run(root.path(), id).unwrap();
    assert!(matches!(
        missing.snapshots[0],
        Err(retained::RetainedUnavailable::Missing)
    ));
    assert_eq!(
        missing.snapshots.iter().filter(|s| s.is_ok()).count(),
        all.snapshots.len() - 1
    );
    assert_eq!(
        vibe_spec::compile_artifact(plan(), &NoReads)
            .unwrap()
            .bytes(),
        baseline.bytes()
    );
    eprintln!(
        "m06f2 complete trace units=1048576 snapshots={} snapshot_bytes={} one removed child explicit Missing; canonical bytes identical",
        all.snapshots.len(),
        snapshot_bytes
    );
}
