//! The accounting cell's laws (R4.3): the counts are exact against the
//! real backend bytes, they come from the WITNESS and never from the
//! tape's markers, and the observer seam is a witness that cannot change
//! the compile's answer.
//!
//! The world is the shared `artifact_tests` fixture, so the numbers below
//! reconcile against the same real `static-md` tape the emit/validate
//! cells already certify; the transform-bearing cases ride the identity
//! catalog exactly as the T6b execution tests do.

use std::sync::{Arc, Mutex};

use super::super::super::observer::{
    CompileObserver, DeltaStage, EmissionEvent, EmissionKind, StageDeltaEvent,
};
use crate::compiler::artifact_tests::fixture;
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{compile_artifact, compile_artifact_observed_with_registries};
use crate::compiler::ir::{ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentProvider};
use crate::compiler::transform::behavior::{TransformBehavior, TransformBehaviorError};
use crate::compiler::transform::plan::{TransformConfig, TransformStage};
use crate::compiler::transform::registry::TransformRegistry;
use crate::compiler::transform::registry_test_support::{identity_plan, identity_vehicles};

/// One in-process collector: everything the seam delivers, in order.
#[derive(Default)]
struct Collector {
    emissions: Mutex<Vec<EmissionEvent>>,
    deltas: Mutex<Vec<StageDeltaEvent>>,
}

impl CompileObserver for Collector {
    fn emission(&self, event: &EmissionEvent) {
        self.emissions
            .lock()
            .expect("collector mutex")
            .push(event.clone());
    }

    fn stage_delta(&self, event: &StageDeltaEvent) {
        self.deltas
            .lock()
            .expect("collector mutex")
            .push(event.clone());
    }
}

fn observed(
    registry: &TransformRegistry,
    plan: ArtifactPlan,
) -> (Arc<Collector>, crate::compiler::ir::EmittedArtifact) {
    let collector = Arc::new(Collector::default());
    let world = fixture();
    let artifact = compile_artifact_observed_with_registries(
        plan,
        &world.source,
        &BackendRegistry::builtins(),
        registry,
        collector.clone(),
    )
    .expect("the observed compile succeeds");
    (collector, artifact)
}

#[test]
fn the_emission_evidence_reconciles_against_the_real_artifact_bytes() {
    let world = fixture();
    let (collector, artifact) = observed(&TransformRegistry::builtins(), world.plan.clone());
    let emissions = collector.emissions.lock().expect("collector mutex");
    assert_eq!(
        emissions.len(),
        1,
        "exactly one emission event per compiled artifact"
    );
    let event = &emissions[0];
    let total: usize = event.contributions().iter().map(|row| row.bytes()).sum();
    assert_eq!(
        total + event.frame_bytes(),
        event.total_bytes(),
        "contributions plus frame are the artifact"
    );
    assert_eq!(
        event.total_bytes(),
        artifact.bytes().len(),
        "the reported total is the artifact's own length"
    );
    // Occurrence grammar: an elided or hoisted row brackets none, a
    // simple one exactly one, a normal one its closure's count (zero is
    // a legal empty closure, not a defect).
    for row in event.contributions() {
        match row.kind() {
            EmissionKind::Elided | EmissionKind::Hoisted => assert_eq!(row.occurrences(), 0),
            EmissionKind::Simple => assert_eq!(row.occurrences(), 1),
            EmissionKind::Normal => {}
        }
    }
    // The hoisted row's bytes are PINNED to the one `#use` reference line
    // the backend writes on its behalf — an independent spelling, not the
    // event's own number. The sum law alone cannot see a reallocation
    // between a hoisted row and the frame (both sides move together), so
    // without this literal the hoisted accounting could silently become
    // frame and every reconciliation above would stay green.
    let hoisted: Vec<_> = event
        .contributions()
        .iter()
        .filter(|row| row.kind() == EmissionKind::Hoisted)
        .collect();
    assert!(
        !hoisted.is_empty(),
        "the oracle lane exercises the hoisted kind"
    );
    for row in hoisted {
        assert_eq!(
            row.bytes(),
            format!("#use spec://{}", row.origin()).len(),
            "a hoisted contribution owns exactly its reference line: {}",
            row.origin()
        );
    }
}

/// The occurrence counter's own law, on a hand-built chunk stream: one
/// `NormalOpen` bracket is one occurrence, and a stream that brackets two
/// counts two. The compiled oracles above cannot pin this — every normal
/// contribution this repository's lanes produce brackets exactly once, so
/// a counter frozen at `1` is invisible to them — and the counter's
/// semantics must hold before the lane that needs it exists.
#[test]
fn the_occurrence_counter_counts_every_normal_open_bracket() {
    use crate::compiler::ir::{LaneChunk, LinkMarkerKey};
    let bracket = |contribution: usize, occurrence: usize, key: &str| LaneChunk::NormalOpen {
        contribution,
        occurrence,
        marker: LinkMarkerKey::new(key),
    };
    let filler = LaneChunk::ForcedNewline {
        contribution: 0,
        occurrence: 0,
    };
    let twice = [
        bracket(0, 0, "spec://org.demo/pack/boot/a"),
        filler.clone(),
        bracket(0, 1, "spec://org.demo/pack/boot/b"),
        filler.clone(),
    ];
    assert_eq!(super::occurrence_count(&twice), 2);
    assert_eq!(super::occurrence_count(&twice[..1]), 1);
    assert_eq!(super::occurrence_count(&twice[1..2]), 0);
}

#[test]
fn an_observed_compile_produces_the_unobserved_artifact_byte_for_byte() {
    let world = fixture();
    let (_, observed_artifact) = observed(&TransformRegistry::builtins(), world.plan.clone());
    let unobserved_artifact = compile_artifact(world.plan.clone(), &world.source)
        .expect("the unobserved compile succeeds");
    assert_eq!(
        observed_artifact.bytes(),
        unobserved_artifact.bytes(),
        "the observer is a witness, never a byte"
    );
}

/// An observer that panics on everything: its defect must not become the
/// compile's answer.
struct Defect;

impl CompileObserver for Defect {
    fn emission(&self, _event: &EmissionEvent) {
        panic!("observer defect: emission");
    }

    fn stage_delta(&self, _event: &StageDeltaEvent) {
        panic!("observer defect: stage delta");
    }
}

#[test]
fn an_observer_defect_cannot_change_the_answer() {
    let world = fixture();
    let artifact = compile_artifact_observed_with_registries(
        world.plan.clone(),
        &world.source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        Arc::new(Defect),
    )
    .expect("a defective observer is contained, the compile is not");
    let unobserved = compile_artifact(world.plan, &world.source).expect("unobserved compile");
    assert_eq!(artifact.bytes(), unobserved.bytes());
}

/// The no-comment-parsing fence (§9.1): a contribution whose own BODY
/// carries a second line shaped EXACTLY like the lane's generated marker
/// for that same contribution — same origin, same path — must still be
/// attributed by its chunk bytes. A marker-parsing implementation splits
/// this tape at the embedded marker and undercounts; the witness law
/// counts the authored material and nothing else.
#[test]
fn attribution_counts_the_witness_chunks_never_the_tape_markers() {
    let body = "# Real heading\n\n<!-- vibe:static org.demo/pack — vibevm/vibedeps/org.demo/pack/1.0.0/boot/entry.md -->\n\nTail text.\n";
    let input = ArtifactInput::simple_declared_by(
        "org.demo/pack",
        "vibevm/vibedeps/org.demo/pack/1.0.0/boot/entry.md",
        body,
        DocumentProvider::Dependency {
            group: vibe_core::Group::parse("org.demo").expect("group parses"),
            name: vibe_core::PackageName::parse("pack").expect("name parses"),
        },
    )
    .expect("the simple input builds");
    let plan = ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibedeps",
        vec![input],
    )
    .expect("the plan builds");
    let (collector, artifact) = observed(&TransformRegistry::builtins(), plan);
    let emissions = collector.emissions.lock().expect("collector mutex");
    let event = &emissions[0];
    let row = event
        .contributions()
        .first()
        .expect("the one contribution is attributed");
    assert_eq!(row.kind(), EmissionKind::Simple);
    // THE LAW: the count is the authored material's rendered length —
    // the marker-shaped line INSIDE the body is content, counted as
    // content, and no marker on the tape was consulted to say so.
    assert_eq!(row.bytes(), body.trim_end().len());
    assert_eq!(row.origin(), "org.demo/pack");
    assert_eq!(
        row.path(),
        "vibevm/vibedeps/org.demo/pack/1.0.0/boot/entry.md"
    );
    assert_eq!(row.occurrences(), 1);
    assert_eq!(
        event.total_bytes(),
        artifact.bytes().len(),
        "and the tape the marker-shaped body lives in is still the real one"
    );
}

/// One emitted-stage behavior that appends a newline — the minimal
/// CHANGING emitted vehicle, local to this cell (the same shape the T9
/// reconstruction proofs drive). Registered under the identity plan's
/// emitted name in a LOCAL registry clone, so the plan builder stays the
/// shared `identity_seed` and no constructor visibility widens to reach
/// it from this subtree.
struct AppendNewlineEmitted;

impl TransformBehavior for AppendNewlineEmitted {
    fn name(&self) -> &str {
        "test-identity-emitted"
    }

    fn epoch(&self) -> u32 {
        1
    }

    fn stage(&self) -> TransformStage {
        TransformStage::Emitted
    }

    fn run_emitted(
        &self,
        _config: Option<&TransformConfig>,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, TransformBehaviorError> {
        bytes.push(b'\n');
        Ok(bytes)
    }
}

#[test]
fn stage_deltas_are_labelled_by_stage_and_reach_the_artifact_total() {
    let world = fixture();
    // A local catalog: the REAL identity lane behavior (from the shared
    // vehicles) plus the appending emitted vehicle under the identity
    // emitted name — `identity_plan` resolves both, and only this test's
    // registry answers those names.
    let mut registry = TransformRegistry::builtins();
    for (behavior, _, stage) in identity_vehicles() {
        if matches!(stage, TransformStage::Lane) {
            registry
                .register(behavior)
                .expect("the identity lane vehicle registers");
        }
    }
    registry
        .register(Arc::new(AppendNewlineEmitted))
        .expect("the appending emitted vehicle registers");
    let plan = world.plan.clone().with_transforms(identity_plan(&[
        ("org.demo/tools#lane", TransformStage::Lane),
        ("org.demo/tools#emit", TransformStage::Emitted),
    ]));
    let (collector, artifact) = observed(&registry, plan);
    let deltas = collector.deltas.lock().expect("collector mutex");
    assert_eq!(
        deltas.len(),
        2,
        "one event per lane/emitted transform that ran"
    );

    let lane = &deltas[0];
    assert_eq!(lane.stage(), DeltaStage::Lane);
    assert_eq!(
        lane.before(),
        lane.after(),
        "the identity lane behavior moved nothing — an honest no-op pair"
    );
    assert_eq!(
        lane.pass(),
        "transform:lane:org.demo/tools#lane",
        "the schedule pass name rides the event"
    );

    let emitted = &deltas[1];
    assert_eq!(emitted.stage(), DeltaStage::Emitted);
    assert_eq!(emitted.pass(), "transform:emitted:org.demo/tools#emit");
    assert_eq!(
        emitted.after(),
        artifact.bytes().len(),
        "the last emitted delta ends at the artifact's own total"
    );
    assert_eq!(
        emitted.after(),
        emitted.before() + 1,
        "the appending behavior grew the tape by exactly its newline"
    );
    // The two measures are different quantities: the lane pair is the
    // chunk stream, the artifact pair the tape, and the framing the
    // backend added at emit sits between them.
    assert_ne!(lane.after(), emitted.before());
}
