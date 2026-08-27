//! Cross-thread budget authority and the pre-encode lifecycle gate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};

use vibe_spec::{CompileTraceSink, PassTraceEvent, SnapshotDecision, compile_artifact_traced};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    IrCardinality, IrLevel, PassShape, PassStatus,
};

use super::super::{RunOutcome, TraceLimits, TraceScope};
use super::support::{
    RUN_A, World, at, entries, node_scope, open, plan, project, read_index, roomy,
};

/// Hold the first two pre-encode answers until both compiler threads have
/// received them. This deterministically creates the race a sequential test
/// cannot: both outputs were authorized while `spent == 0`, before either
/// event could publish and cross the one-byte soft ceiling.
struct FirstDecisionBarrier {
    inner: TraceScope,
    barrier: Arc<Barrier>,
    first: AtomicBool,
}

impl CompileTraceSink for FirstDecisionBarrier {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.inner.record(event);
    }

    fn before_snapshot(&self, pass: &str, output: &PassShape) -> SnapshotDecision {
        let decision = self.inner.before_snapshot(pass, output);
        if self.first.swap(false, Ordering::SeqCst) {
            self.barrier.wait();
        }
        decision
    }
}

#[test]
fn concurrent_scopes_publish_only_one_soft_ceiling_crossing() {
    let root = project();
    let run = open(root.path(), RUN_A, TraceLimits::for_test(1, 9));
    let left = run.declare_scope(&node_scope("node:left", "left")).unwrap();
    let right = run
        .declare_scope(&node_scope("node:right", "right"))
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));

    let thread = |scope: TraceScope, barrier: Arc<Barrier>| {
        std::thread::spawn(move || {
            let sink = FirstDecisionBarrier {
                inner: scope,
                barrier,
                first: AtomicBool::new(true),
            };
            compile_artifact_traced(plan(), &World::two_documents(), &sink)
                .expect("both ordinary compiles stay green");
        })
    };
    let a = thread(left.clone(), Arc::clone(&barrier));
    let b = thread(right.clone(), barrier);
    a.join().expect("left compiler thread");
    b.join().expect("right compiler thread");

    left.complete("left-fp").unwrap();
    right.complete("right-fp").unwrap();
    let summary = run.finish(&RunOutcome::Ok, at(2_000));
    let index = read_index(run.run_dir());

    let named = index
        .events
        .iter()
        .filter(|event| event.snapshot.is_some())
        .count();
    assert_eq!(named, 1, "exactly one event owns the soft crossing");
    let raced = index
        .events
        .iter()
        .filter(|event| {
            event.status == PassStatus::SnapshotFailed
                && event
                    .diagnostic
                    .as_deref()
                    .is_some_and(|text| text.contains("encoded concurrently"))
        })
        .count();
    assert_eq!(
        raced, 1,
        "the already-encoded loser is truthful but publishes nothing",
    );
    assert!(summary.budget_exhausted);
    let payloads: Vec<String> = entries(run.run_dir())
        .into_iter()
        .filter(|name| name != "index.json")
        .collect();
    assert_eq!(payloads.len(), 1, "one physical payload: {payloads:?}");
    assert_eq!(
        summary.snapshot_bytes,
        std::fs::metadata(run.run_dir().join(&payloads[0]))
            .unwrap()
            .len(),
    );
}

#[test]
fn a_closed_or_finalised_scope_stands_down_at_the_pre_encode_seam() {
    let shape = PassShape {
        cardinality: IrCardinality::Document,
        level: IrLevel::Source,
    };
    for terminal in ["compiled", "failed", "skipped"] {
        let root = project();
        let run = open(root.path(), RUN_A, roomy());
        let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
        assert_eq!(
            scope.before_snapshot("parse", &shape),
            SnapshotDecision::Encode,
            "an open scope may encode",
        );
        match terminal {
            "compiled" => scope.complete("fp").unwrap(),
            "failed" => scope.fail("failed").unwrap(),
            _ => scope.skip("fresh").unwrap(),
        }
        assert_eq!(
            scope.before_snapshot("parse", &shape),
            SnapshotDecision::SkipBudget,
            "{terminal}: the compiler's established SkipBudget branch invokes no encoder",
        );
    }

    let root = project();
    let run = open(root.path(), RUN_A, roomy());
    let scope = run.declare_scope(&node_scope("node:.", ".")).unwrap();
    scope.complete("fp").unwrap();
    assert!(run.finish(&RunOutcome::Ok, at(2_000)).finalised);
    assert_eq!(
        scope.before_snapshot("parse", &shape),
        SnapshotDecision::SkipBudget,
        "a finalised run never invokes the encoder through a retained sink",
    );
}
