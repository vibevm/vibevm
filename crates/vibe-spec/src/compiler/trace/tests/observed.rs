//! The accepted path: off means off, the observed schedule IS the declared
//! schedule, and a certified snapshot is the strict `compiler_ir/e1` wire
//! rather than a summary of it.

use specmark::verifies;

use super::super::*;
use super::support::{DefaultingRecorder, Recorder, World, declared_artifact_passes, plan};
use crate::compile_artifact;
use crate::compiler::builtin::{BuiltinSchedule, compile_artifact_traced};
use crate::compiler::wire;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn an_untraced_compile_encodes_nothing_and_keeps_its_bytes_and_errors() {
    reset_snapshot_encodes();
    let untraced = compile_artifact(plan(), &World::two_documents()).unwrap();
    assert_eq!(snapshot_encodes(), 0);

    let untraced_error = compile_artifact(plan(), &World::dangling_use()).unwrap_err();
    assert_eq!(snapshot_encodes(), 0);

    // The observed route is byte- and error-identical to the route that has
    // no observer at all, so installing one cannot perturb a compile.
    let recorder = Recorder::default();
    let traced = compile_artifact_traced(plan(), &World::two_documents(), &recorder).unwrap();
    assert_eq!(traced.bytes(), untraced.bytes());
    assert!(snapshot_encodes() > 0);

    let failures = Recorder::default();
    let traced_error =
        compile_artifact_traced(plan(), &World::dangling_use(), &failures).unwrap_err();
    assert_eq!(traced_error.to_string(), untraced_error.to_string());

    // The same public route records the real refusal: every pass before it is
    // certified, the failing one is `pass-failed` with only its body measured,
    // and the schedule stops there.
    let observed = failures.events();
    let (refused, earlier) = observed
        .split_last()
        .expect("a failing compile still observes the passes it did run");
    assert!(
        earlier
            .iter()
            .all(|event| event.status == index::PassStatus::Ok)
    );
    assert_eq!(refused.status, index::PassStatus::PassFailed);
    assert_eq!(refused.timings, (true, false, false));
    assert_eq!(refused.snapshot, None);
    assert!(refused.diagnostic.is_some());
    // The refusal names a pass the schedule really declares, not an invention.
    let schedule = BuiltinSchedule::emitted_for_test(&plan());
    assert!(declared_artifact_passes(schedule.pipeline_for_test()).contains(&refused.pass));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn the_observed_names_equal_the_declared_schedule_with_one_parse_per_document() {
    let recorder = Recorder::default();
    compile_artifact_traced(plan(), &World::two_documents(), &recorder).unwrap();

    let schedule = BuiltinSchedule::emitted_for_test(&plan());
    let declared = declared_artifact_passes(schedule.pipeline_for_test());
    let (parse, artifact) = declared
        .split_first()
        .expect("the schedule declares passes");
    assert_eq!(parse, "parse");

    // Two addressed documents: `parse` is observed twice, in encounter order,
    // and every whole-artifact pass exactly once, in declared order.
    let observed = recorder.names();
    assert_eq!(observed[0], *parse);
    assert_eq!(observed[1], *parse);
    assert_eq!(&observed[2..], artifact);
    assert!(
        recorder
            .events()
            .iter()
            .all(|event| event.status == index::PassStatus::Ok)
    );

    // Every certified snapshot is distinct: adjacent events carry the real
    // carrier each pass produced, never a copied summary of one of them.
    let snapshots = recorder.snapshots();
    assert_eq!(snapshots.len(), observed.len());
    let unique: std::collections::BTreeSet<&Vec<u8>> = snapshots.iter().collect();
    assert_eq!(unique.len(), snapshots.len());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn every_certified_snapshot_decodes_through_the_strict_reader_at_its_own_shape() {
    let recorder = Recorder::default();
    compile_artifact_traced(plan(), &World::two_documents(), &recorder).unwrap();

    for event in recorder.events() {
        let bytes = event.snapshot.expect("an `ok` event certifies a snapshot");
        let decoded = wire::decode(&bytes).unwrap_or_else(|error| {
            panic!(
                "the `{}` snapshot must survive the strict reader: {error}",
                event.pass
            )
        });
        assert_eq!(
            event.output,
            shape(decoded.shape()),
            "the `{}` event must report the shape its snapshot really carries",
            event.pass
        );
        assert_eq!(event.timings, (true, true, true));
        assert_eq!(event.diagnostic, None);
    }

    // The pretty spelling is the trace spelling, and the two `parse` snapshots
    // are two different documents rather than one document twice.
    let parses = recorder
        .events()
        .into_iter()
        .filter(|event| event.pass == "parse")
        .filter_map(|event| event.snapshot)
        .collect::<Vec<_>>();
    assert_eq!(parses.len(), 2);
    assert_ne!(parses[0], parses[1]);
    assert!(parses[0].starts_with(b"{\n"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_sink_that_stands_down_on_budget_costs_the_compiler_no_encode_at_all() {
    let expected = compile_artifact(plan(), &World::two_documents()).unwrap();

    reset_snapshot_encodes();
    let recorder = Recorder::on_budget();
    let produced = compile_artifact_traced(plan(), &World::two_documents(), &recorder).unwrap();

    // The compiler's answer is the ordinary successful one.
    assert_eq!(produced.bytes(), expected.bytes());
    // The encoder was never called, so the retention budget a writer owns can
    // genuinely stop the cost rather than merely discard the result.
    assert_eq!(snapshot_encodes(), 0);

    // One correctly shaped observation per attempted pass: the epoch's
    // `snapshot-skipped-budget` row — pass and verify measured, NO encode
    // duration, no snapshot, and no diagnostic, because standing down is not
    // a failure.
    let schedule = BuiltinSchedule::emitted_for_test(&plan());
    let declared = declared_artifact_passes(schedule.pipeline_for_test());
    let events = recorder.events();
    assert_eq!(events.len(), declared.len() + 1, "two documents parse");
    for event in &events {
        assert_eq!(event.status, index::PassStatus::SnapshotSkippedBudget);
        assert_eq!(event.timings, (true, true, false));
        assert_eq!(event.snapshot, None);
        assert_eq!(event.diagnostic, None);
        assert!(declared.contains(&event.pass));
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn the_default_pre_encode_decision_still_certifies_every_snapshot() {
    reset_snapshot_encodes();
    let recorder = DefaultingRecorder::default();
    compile_artifact_traced(plan(), &World::two_documents(), &recorder).unwrap();

    let events = recorder.events();
    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .all(|event| event.status == index::PassStatus::Ok)
    );
    assert!(events.iter().all(|event| event.snapshot.is_some()));
    assert!(
        events
            .iter()
            .all(|event| event.timings == (true, true, true))
    );
    assert_eq!(snapshot_encodes(), events.len());
}
