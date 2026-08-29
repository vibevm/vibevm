//! The T6c manager-side lane admission gate (R4-TRANSFORM-PLAN-ABI §6.2
//! item 3): the immutable pre-transform witness, the intrinsic lane contract
//! and the transition check that together decide whether a changed `LaneIr`
//! may be accepted.
//!
//! Two properties of this cell are load-bearing and easy to lose.
//!
//! **The witness is taken from the pass INPUT.** Evidence derived after the
//! behavior ran always agrees with itself, so [`witness`] is called before
//! [`super::behavior::TransformBehavior::run_lane`] and never after.
//!
//! **The gate is unconditional.** It is the MANAGER's admission decision, not
//! a semantic opinion routed through the optional inter-pass verifier hook:
//! `CompilerPipeline::enable_verify_each_for_tests` is `#[cfg(test)]`-gated,
//! so a lane check wired through it would leave production unguarded. R6.4
//! separately makes the general verifier mandatory; that is not this cell's
//! business, and this cell must keep refusing without it.
//!
//! The gate is a pure decision: it borrows the value, never repairs it, and
//! owns no IR store of its own. The witness is comparison evidence for one
//! invocation — never a second IR store, never a wire field, never a repair.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use crate::compiler::assemble::{LaneValidationError, validate_lane};
use crate::compiler::ir::LaneIr;
use crate::compiler::verify::{LaneWitness, TransitionError, lane_witness, verify_lane_transition};

/// Why one lane behavior's output is not admissible.
///
/// Both sources ride along as their exact types: the intrinsic contract keeps
/// its [`LaneValidationError`], and the provenance refusal keeps the
/// [`TransitionError`] that names the moved field with expected/actual. The
/// wrapper above adds entry identity; nothing here knows about plan order,
/// keys or stages.
#[derive(Debug, thiserror::Error)]
pub(crate) enum LaneAdmissionError {
    #[error("the returned lane violates its intrinsic contract: {0}")]
    Intrinsic(#[source] Box<LaneValidationError>),
    #[error("{0}")]
    Transition(#[source] Box<TransitionError>),
}

/// Derive the immutable pre-transform evidence from the behavior's INPUT.
pub(crate) fn witness(input: &LaneIr) -> LaneWitness {
    #[cfg(test)]
    WITNESS_DERIVATIONS.with(|count| count.set(count.get() + 1));
    lane_witness(input)
}

/// Admit one lane behavior's output, or refuse it.
///
/// Intrinsic contract first, transition second: a value that is not a
/// well-formed lane at all cannot be meaningfully compared against its
/// predecessor, and the intrinsic verdict is exactly the one every other lane
/// consumer (assemble, emit, the assemble transition) already runs — so a
/// transform output is held to the same contract the assembler is.
pub(crate) fn admit(before: &LaneWitness, output: &LaneIr) -> Result<(), LaneAdmissionError> {
    #[cfg(test)]
    INTRINSIC_CHECKS.with(|count| count.set(count.get() + 1));
    validate_lane(output).map_err(|source| LaneAdmissionError::Intrinsic(Box::new(source)))?;
    #[cfg(test)]
    TRANSITION_CHECKS.with(|count| count.set(count.get() + 1));
    verify_lane_transition(before, output)
        .map_err(|source| LaneAdmissionError::Transition(Box::new(source)))
}

// The gate's own instrumentation: a pass counter proves a pass ran, not that
// the check inside it ran, so the three counters sit on the three checks
// themselves. Thread-local because the suite runs tests in parallel.
#[cfg(test)]
std::thread_local! {
    static WITNESS_DERIVATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INTRINSIC_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRANSITION_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_lane_admission_counts() {
    WITNESS_DERIVATIONS.with(|count| count.set(0));
    INTRINSIC_CHECKS.with(|count| count.set(0));
    TRANSITION_CHECKS.with(|count| count.set(0));
}

/// The three gate counts `(witness, intrinsic, transition)`.
#[cfg(test)]
pub(crate) fn lane_admission_counts() -> (usize, usize, usize) {
    (
        WITNESS_DERIVATIONS.with(std::cell::Cell::get),
        INTRINSIC_CHECKS.with(std::cell::Cell::get),
        TRANSITION_CHECKS.with(std::cell::Cell::get),
    )
}
