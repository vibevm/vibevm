//! Local consumer intent, guard eligibility and merge, before feature effects.
//! Original answers survive unresolved/dormant projections. Only a resolved
//! outcome contains feature output; default usage is not a consent receipt.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

mod diagnostics;
mod evaluate;
mod merge;
mod model;
mod types;

pub use diagnostics::{AnswerError, ChoiceConflict, ChoiceDefinitionError, ChoiceQuestion};
pub use model::ChoiceModel;
pub use types::{
    ChoiceAnswer, ChoiceEvaluation, ChoiceGroup, ChoiceGuard, ChoiceOutcome, ChoicePolicy,
    ChoiceProjection, ConsumerChoices, DefaultUse, GroupControl, MemberControl, MemberGuards,
    MergeIntent, ProjectionOrigin, Selection,
};

#[cfg(test)]
mod tests;
