//! Pure local feature closure, independent of package kind and effect owners.
//!
//! Construct a validated [`FeatureModel`], then supply an explicit base and
//! additions/disables. App input precedence belongs to its adapter. This kernel
//! is also used by the local consumer-choice layer. Neither layer persists
//! answers, authorizes effects or binds defaults to a candidate receipt.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL");

mod choices;
mod errors;
mod model;
mod resolve;
mod types;

pub use choices::{
    AnswerError, ChoiceAnswer, ChoiceConflict, ChoiceDefinitionError, ChoiceEvaluation,
    ChoiceGroup, ChoiceGuard, ChoiceModel, ChoiceOutcome, ChoicePolicy, ChoiceProjection,
    ChoiceQuestion, ConsumerChoices, DefaultUse, GroupControl, MemberControl, MemberGuards,
    MergeIntent, ProjectionOrigin, Selection,
};
pub use errors::{DefinitionError, DefinitionProblem, EvaluationError, ReferenceKind};
pub use model::FeatureModel;
pub(crate) use model::valid_identifier;
pub use types::{
    Cardinality, ClosureGroup, FeatureBase, FeatureInput, FeatureNode, FeatureResolution,
    Implication, ImplicationReason,
};

#[cfg(test)]
mod tests;
