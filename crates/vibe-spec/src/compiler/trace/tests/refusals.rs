//! The three refusals told apart, and the two laws the observer's own
//! translation must obey.

use specmark::verifies;
use vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;

use super::super::*;
use super::support::{
    LyingOutput, Recorder, World, break_anchors, markdown_source, parse_like, pass_name, plan,
};
use crate::SpecAddress;
use crate::compile_artifact;
use crate::compiler::builtin::{BuiltinSchedule, compile_artifact_traced};
use crate::compiler::ir::{DocumentAddress, DocumentIr, SourceFormatId, SourceIr};
use crate::compiler::pass::{IdentityPass, PassSegment, PassSegmentError};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};
use crate::compiler::transform::registry::TransformRegistry;
use crate::compiler::verify::IrVerifier;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_real_pass_failure_records_one_event_and_keeps_the_error_identity() {
    // The real built-in `parse`, refused by a format it does not implement.
    let schedule = BuiltinSchedule::linked_for_test(&plan(), &TransformRegistry::builtins())
        .expect("the empty-plan schedule builds");
    let bad = || {
        SourceIr::reached(
            DocumentAddress::Spec(
                SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap(),
            ),
            SourceFormatId::new("unsupported").unwrap(),
            "body",
        )
    };

    reset_snapshot_encodes();
    let untraced = schedule
        .pipeline_for_test()
        .run_document(bad())
        .unwrap_err();
    assert_eq!(snapshot_encodes(), 0);

    let recorder = Recorder::default();
    let traced = schedule
        .pipeline_for_test()
        .run_document_traced(bad(), Some(&recorder))
        .unwrap_err();
    assert_eq!(traced.to_string(), untraced.to_string());

    let events = recorder.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].pass, "parse");
    assert_eq!(events[0].status, index::PassStatus::PassFailed);
    // The pass body was measured; nothing after it was reached.
    assert_eq!(events[0].timings, (true, false, false));
    assert_eq!(events[0].snapshot, None);
    // The diagnostic is bounded and names the concrete refusal.
    let diagnostic = events[0]
        .diagnostic
        .as_deref()
        .expect("a refusal is diagnosed");
    assert!(diagnostic.starts_with("PassFailed"), "{diagnostic}");
    assert!(diagnostic.contains("unsupported"), "{diagnostic}");
    assert!(diagnostic.len() <= DIAGNOSTIC_CAP_BYTES);
    // Nothing was encoded: a refused pass has no certified output.
    assert_eq!(snapshot_encodes(), 0);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_segment_input_refusal_is_not_fabricated_into_a_pass_event() {
    // A duplicate-anchor DOCUMENT handed to the segment fails verification at
    // the honest engine boundary, before any pass is attempted: that is a
    // compiler error about the input, and inventing a pass event for it would
    // blame a pass that never ran.
    let mut segment = PassSegment::default();
    segment
        .push(IdentityPass::<DocumentIr>::new(pass_name(
            "document-identity",
        )))
        .unwrap();
    let forged = DocumentIr::new(
        markdown_source("root", "# Doc {#root}\n"),
        crate::DocTree::parse("# Doc {#root}\n##dup once\n\n##dup twice\n"),
    );

    reset_snapshot_encodes();
    let recorder = Recorder::default();
    let error = segment
        .run_traced(AnyIr::Document(forged), Some(IrVerifier), Some(&recorder))
        .unwrap_err();
    assert!(matches!(error, PassSegmentError::InputVerification { .. }));
    assert!(recorder.events().is_empty());
    assert_eq!(snapshot_encodes(), 0);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_wrong_runtime_shape_records_verification_failed_without_a_snapshot() {
    let mut segment = PassSegment::default();
    segment.push_erased_for_test(Box::new(LyingOutput)).unwrap();

    reset_snapshot_encodes();
    let recorder = Recorder::default();
    let error = segment
        .run_traced(
            AnyIr::Source(markdown_source("root", "# Doc {#root}\n")),
            None,
            Some(&recorder),
        )
        .unwrap_err();
    assert!(matches!(error, PassSegmentError::WrongOutput { .. }));

    let events = recorder.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, index::PassStatus::VerificationFailed);
    // Both stages are measured; the encode was never attempted.
    assert_eq!(events[0].timings, (true, true, false));
    assert_eq!(events[0].snapshot, None);
    // The event reports what the pass REALLY returned, not what it declared.
    assert_eq!(events[0].input.level, index::IrLevel::Source);
    assert_eq!(events[0].output.level, index::IrLevel::Document);
    assert_eq!(events[0].output.cardinality, index::IrCardinality::Artifact);
    assert!(
        events[0]
            .diagnostic
            .as_deref()
            .is_some_and(|text| text.starts_with("WrongOutput"))
    );
    assert_eq!(snapshot_encodes(), 0);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_semantic_refusal_records_verification_failed_after_the_earlier_pass_was_certified() {
    let mut pipeline = CompilerPipeline::default();
    pipeline.push_document(parse_like()).unwrap();
    pipeline.push_document(break_anchors()).unwrap();
    pipeline.enable_verify_each_for_tests();

    reset_snapshot_encodes();
    let recorder = Recorder::default();
    let error = pipeline
        .run_document_traced(
            markdown_source("root", "# Doc {#root}\nBODY\n"),
            Some(&recorder),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::Segment(PassSegmentError::VerificationFailed { ref pass, .. })
            if pass.as_str() == "break-anchors"
    ));

    let events = recorder.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].pass, "parse-like");
    assert_eq!(events[0].status, index::PassStatus::Ok);
    assert!(events[0].snapshot.is_some());
    assert_eq!(events[1].pass, "break-anchors");
    assert_eq!(events[1].status, index::PassStatus::VerificationFailed);
    assert_eq!(events[1].timings, (true, true, false));
    assert_eq!(events[1].snapshot, None);
    // Exactly one carrier was certified — the refused output never was.
    assert_eq!(snapshot_encodes(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn an_encoder_refusal_is_diagnostic_and_leaves_the_successful_output_alone() {
    let expected = compile_artifact(plan(), &World::two_documents()).unwrap();

    let recorder = Recorder::default();
    let refused = {
        let _seam = RefusedEncoder::install();
        compile_artifact_traced(plan(), &World::two_documents(), &recorder).unwrap()
    };

    // The observer failed; the compile did not.
    assert_eq!(refused.bytes(), expected.bytes());
    let events = recorder.events();
    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .all(|event| event.status == index::PassStatus::SnapshotFailed)
    );
    // All three stages were measured, and no snapshot is claimed.
    assert!(
        events
            .iter()
            .all(|event| event.timings == (true, true, true))
    );
    assert!(events.iter().all(|event| event.snapshot.is_none()));
    assert!(events.iter().all(|event| {
        event
            .diagnostic
            .as_deref()
            .is_some_and(|text| text.len() <= DIAGNOSTIC_CAP_BYTES)
    }));
}

#[test]
fn a_saturated_duration_is_only_marked_at_the_ceiling() {
    let exact = measure(std::time::Duration::from_micros(u64::from(u32::MAX)));
    assert_eq!(exact.micros, u32::MAX);
    assert!(!exact.saturated);

    let over = measure(std::time::Duration::from_micros(u64::from(u32::MAX) + 1));
    assert_eq!(over.micros, u32::MAX);
    assert!(over.saturated);

    let ordinary = measure(std::time::Duration::from_micros(7));
    assert_eq!((ordinary.micros, ordinary.saturated), (7, false));
}

/// The ONE translation this module owns — domain shape to the generated
/// trace-index shape — is total and level-preserving over all six carriers.
#[test]
fn every_domain_carrier_shape_maps_onto_its_generated_counterpart() {
    use crate::compiler::ir::{IrCardinality, IrLevel, IrShape};

    let cases = [
        (
            IrShape::new(IrLevel::Source, IrCardinality::Document),
            index::IrLevel::Source,
            index::IrCardinality::Document,
        ),
        (
            IrShape::new(IrLevel::Document, IrCardinality::Document),
            index::IrLevel::Document,
            index::IrCardinality::Document,
        ),
        (
            IrShape::new(IrLevel::Document, IrCardinality::Artifact),
            index::IrLevel::Document,
            index::IrCardinality::Artifact,
        ),
        (
            IrShape::new(IrLevel::Closure, IrCardinality::Artifact),
            index::IrLevel::Closure,
            index::IrCardinality::Artifact,
        ),
        (
            IrShape::new(IrLevel::Lane, IrCardinality::Artifact),
            index::IrLevel::Lane,
            index::IrCardinality::Artifact,
        ),
        (
            IrShape::new(IrLevel::Emitted, IrCardinality::Artifact),
            index::IrLevel::Emitted,
            index::IrCardinality::Artifact,
        ),
    ];
    for (domain, level, cardinality) in cases {
        assert_eq!(shape(domain), index::PassShape { level, cardinality });
    }
}
