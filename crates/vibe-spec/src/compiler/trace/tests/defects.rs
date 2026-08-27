//! A defective observer is still only an observer.
//!
//! A sink is arbitrary downstream code, so it may PANIC instead of answering.
//! These reds pin the load-bearing contract: the unwind never escapes as the
//! compile's answer, the artifact bytes and the compiler's own error string are
//! exactly the unobserved route's, and no panic payload is turned into a
//! diagnostic. Rust's own panic hook still fires — that noise on stderr is
//! expected here and is deliberately NOT suppressed (no global hook is
//! touched).

use std::path::PathBuf;

use specmark::verifies;

use super::super::*;
use super::support::{OBSERVER_PANIC, PanicIn, PanickingSink, Recorder, World, plan};
use crate::compile_artifact;
use crate::compiler::builtin::compile_artifact_traced;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_panic_in_before_snapshot_is_an_observer_defect_not_a_compile_outcome() {
    let expected = compile_artifact(plan(), &World::two_documents()).unwrap();

    let sink = PanickingSink::at(PanicIn::BeforeSnapshot);
    let produced = compile_artifact_traced(plan(), &World::two_documents(), &sink).unwrap();

    // The artifact is byte-identical to the unobserved route: the unwind did
    // not escape and did not replace the compiler's answer.
    assert_eq!(produced.bytes(), expected.bytes());
    // Every accepted output still asked, so the schedule ran to the end rather
    // than stopping at the first blow-up.
    assert!(sink.calls() > 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_panic_in_before_snapshot_falls_back_to_the_default_encode_decision() {
    // A sink that panics has not decided anything, so the compiler takes the
    // trait's own default — encode — exactly as if it had never overridden it.
    reset_snapshot_encodes();
    let sink = PanickingSink::at(PanicIn::BeforeSnapshot);
    compile_artifact_traced(plan(), &World::two_documents(), &sink).unwrap();
    assert_eq!(snapshot_encodes(), sink.calls());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_panic_in_record_is_contained_on_a_successful_compile() {
    let expected = compile_artifact(plan(), &World::two_documents()).unwrap();

    let sink = PanickingSink::at(PanicIn::Record);
    let produced = compile_artifact_traced(plan(), &World::two_documents(), &sink).unwrap();

    assert_eq!(produced.bytes(), expected.bytes());
    // Once per attempted pass, not once in total: containment is per crossing.
    let observed = Recorder::default();
    compile_artifact_traced(plan(), &World::two_documents(), &observed).unwrap();
    assert_eq!(sink.calls(), observed.events().len());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_panic_in_record_cannot_replace_a_real_pass_error() {
    let expected = compile_artifact(plan(), &World::dangling_use()).unwrap_err();

    let sink = PanickingSink::at(PanicIn::Record);
    let observed = compile_artifact_traced(plan(), &World::dangling_use(), &sink).unwrap_err();

    // The compiler's ORIGINAL refusal survives verbatim: the observer's panic
    // neither replaced it nor leaked its payload into it.
    assert_eq!(observed.to_string(), expected.to_string());
    assert!(!observed.to_string().contains(OBSERVER_PANIC));
    assert!(sink.calls() > 0);
}

/// Source guard for the off-path allocation profile.
///
/// The untraced route must MOVE the descriptor's pass name into the refusal it
/// returns, exactly as it did before the observer existed; only a live sink
/// pays for a label. A behavioural assertion would need a production counter
/// just to watch an allocation, which the atom must not grow — so the shape is
/// pinned at the source, the way the wire conversion's own guards are.
#[test]
fn the_untraced_refusal_path_moves_the_pass_name_instead_of_cloning_it() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/compiler/pass.rs");
    let text = std::fs::read_to_string(&path).unwrap();

    assert!(
        !text.contains("descriptor.name.clone()"),
        "the refusal branches must move the descriptor's name, not clone it"
    );
    assert_eq!(
        text.matches("pass: descriptor.name,").count(),
        3,
        "registration's `DuplicateName` plus the run loop's `WrongOutput` and \
         `VerificationFailed` all take the name by move"
    );
    // The only label allocation is guarded by the sink option.
    assert!(
        text.contains("trace.map(|_| (input.shape(), descriptor.name.as_str().to_string()))"),
        "the event label is captured only under a live sink"
    );
}
