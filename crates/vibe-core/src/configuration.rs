//! Pure local feature closure, independent of package kind and effect owners.
//!
//! Construct a validated [`FeatureModel`], then supply an explicit base and
//! additions/disables. App input precedence belongs to its adapter. This kernel
//! does not implement consumer-owned exact/additive choices or persistence.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL");

mod errors;
mod model;
mod resolve;
mod types;

pub use errors::{DefinitionError, DefinitionProblem, EvaluationError, ReferenceKind};
pub use model::FeatureModel;
pub use types::{
    Cardinality, ClosureGroup, FeatureBase, FeatureInput, FeatureNode, FeatureResolution,
    Implication, ImplicationReason,
};

#[cfg(test)]
mod tests;
