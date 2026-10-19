use super::trace::{CompileTraceSink, PassTraceEvent, RefusedEncoder, SnapshotDecision};
use crate::{ArtifactInput, ArtifactPlan, ArtifactTarget, SectionSource, SpecAddress};
use std::sync::Mutex;
use vibe_wire::generated::compiler_trace_index::e1::index::{PassShape, PassStatus};
struct NoReads;
impl SectionSource for NoReads {
    fn section_text(&self, _: &SpecAddress) -> Result<String, String> {
        panic!("unexpected provider")
    }
}
struct Sink {
    skip: bool,
    panic: bool,
    events: Mutex<Vec<(PassStatus, bool)>>,
}
impl CompileTraceSink for Sink {
    fn before_snapshot(&self, _: &str, _: &PassShape) -> SnapshotDecision {
        if self.skip {
            SnapshotDecision::SkipBudget
        } else {
            SnapshotDecision::Encode
        }
    }
    fn record(&self, e: &PassTraceEvent<'_>) {
        if self.panic {
            panic!("deliberate observer failure");
        }
        self.events
            .lock()
            .unwrap()
            .push((e.status().clone(), e.snapshot().is_some()));
    }
}
fn scale_plan() -> ArtifactPlan {
    let text = format!("# Scale\nstatus=spec/plan\n{}", "λ ".repeat(262144));
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "STATIC.md",
        "spec",
        (0..4)
            .map(|_| ArtifactInput::simple("org.example/scale", "repeated.md", &text).unwrap())
            .collect(),
    )
    .unwrap()
}

#[test]
fn million_units_canonical_bytes_survive_budget_and_encoder_defects() {
    let baseline = crate::compile_artifact(scale_plan(), &NoReads).unwrap();
    let output = std::str::from_utf8(baseline.bytes()).unwrap();
    assert_eq!(output.matches('λ').count(), 1_048_576);
    assert_eq!(output.matches("status=spec/plan").count(), 4);
    for (skip, refuse) in [(true, false), (false, true)] {
        let sink = Sink {
            skip,
            panic: false,
            events: Mutex::new(Vec::new()),
        };
        let seam = refuse.then(RefusedEncoder::install);
        let actual = crate::compile_artifact_traced(scale_plan(), &NoReads, &sink).unwrap();
        drop(seam);
        assert_eq!(actual.bytes(), baseline.bytes());
        let events = sink.events.lock().unwrap();
        assert!(!events.is_empty());
        let status = if skip {
            PassStatus::SnapshotSkippedBudget
        } else {
            PassStatus::SnapshotFailed
        };
        assert!(events.iter().all(|(s, present)| *s == status && !present));
    }
    assert_eq!(
        crate::compile_artifact(scale_plan(), &NoReads)
            .unwrap()
            .bytes(),
        baseline.bytes()
    );
    eprintln!(
        "m06f2 compiler units=1048576 bytes={} status_fields=4 canonical=identical budget/encoder complete_capture=false",
        baseline.bytes().len()
    );
}

#[test]
fn million_units_canonical_bytes_survive_panicking_sink() {
    let baseline = crate::compile_artifact(scale_plan(), &NoReads).unwrap();
    let sink = Sink {
        skip: true,
        panic: true,
        events: Mutex::new(Vec::new()),
    };
    let observed = crate::compile_artifact_traced(scale_plan(), &NoReads, &sink).unwrap();
    assert_eq!(observed.bytes(), baseline.bytes());
    assert!(sink.events.lock().unwrap().is_empty());
}
