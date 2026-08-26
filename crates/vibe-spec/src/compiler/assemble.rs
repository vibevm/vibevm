//! The named whole-artifact Closure -> Lane lowering.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use super::ir::{ClosureIr, LaneIr, LinkState};
use super::link::{LinkPassError, validate_linked};
use super::pass::{Pass, PassName};

mod project;
use project::{LaneProjectionError, project_lane};
mod transition;
pub(crate) use transition::{LaneTransitionError, validate_assembled_transition};
mod validate;
pub(crate) use validate::{LaneShape, LaneValidationError, validate_lane, validate_shape};

pub(crate) const ASSEMBLE_PASS_NAME: &str = "assemble";

pub(crate) struct AssemblePass {
    name: PassName,
}

impl AssemblePass {
    pub(crate) fn new() -> Self {
        Self {
            name: PassName::new(ASSEMBLE_PASS_NAME)
                .expect("the static built-in assemble pass name is non-blank"),
        }
    }
}

impl Pass for AssemblePass {
    type Input = ClosureIr;
    type Output = LaneIr;
    type Error = AssemblePassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: ClosureIr) -> Result<LaneIr, AssemblePassError> {
        #[cfg(test)]
        ASSEMBLE_INVOCATIONS.with(|count| count.set(count.get() + 1));
        assemble_closure(input)
    }
}

#[cfg(test)]
std::thread_local! {
    static ASSEMBLE_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_assemble_invocations() {
    ASSEMBLE_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn assemble_invocations() -> usize {
    ASSEMBLE_INVOCATIONS.with(std::cell::Cell::get)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AssemblePassError {
    #[error("assemble requires valid linked state: {0}")]
    InvalidLink(#[source] Box<LinkPassError>),
    #[error("assemble cannot project the linked artifact: {0}")]
    Projection(#[source] LaneProjectionError),
    #[error("assemble produced an invalid lane: {0}")]
    InvalidLane(#[source] LaneValidationError),
    #[error("assemble transition is invalid: {0}")]
    InvalidTransition(#[source] LaneTransitionError),
}

fn assemble_closure(input: ClosureIr) -> Result<LaneIr, AssemblePassError> {
    validate_linked(&input).map_err(|error| AssemblePassError::InvalidLink(Box::new(error)))?;
    let LinkState::Linked(link) = &input.link else {
        unreachable!("linked validator accepted only Linked")
    };
    let lane = project_lane(&input, link).map_err(AssemblePassError::Projection)?;
    validate_lane(&lane).map_err(AssemblePassError::InvalidLane)?;
    validate_assembled_transition(&input, &lane).map_err(AssemblePassError::InvalidTransition)?;
    Ok(lane)
}

#[cfg(test)]
#[path = "assemble/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "assemble/fence_boundary_tests.rs"]
mod fence_boundary_tests;

#[cfg(test)]
#[path = "assemble/manager_tests.rs"]
mod manager_tests;

#[cfg(test)]
#[path = "assemble/transition_tests.rs"]
mod transition_tests;
