//! Structured validation and evaluation failures; IDs are data, never a protocol.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL");

use super::{Cardinality, ImplicationReason};

/// The declaration field that owns a local reference.
///
/// ```
/// use vibe_core::configuration::ReferenceKind;
/// assert_ne!(ReferenceKind::Parent, ReferenceKind::Requires);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    Parent,
    Requires,
    Member,
    Fallback,
}

/// The exact rejected definition, independently matchable from its diagnostic.
///
/// ```
/// use vibe_core::configuration::DefinitionProblem;
/// let problem = DefinitionProblem::DuplicateFeature { id: "core".into() };
/// assert!(matches!(problem, DefinitionProblem::DuplicateFeature { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionProblem {
    InvalidIdentifier {
        id: String,
    },
    DuplicateFeature {
        id: String,
    },
    DuplicateGroup {
        id: String,
    },
    EmptyGroup {
        id: String,
    },
    UnknownReference {
        owner: String,
        id: String,
        kind: ReferenceKind,
    },
    DuplicateReference {
        owner: String,
        id: String,
        kind: ReferenceKind,
    },
    ParentCycle {
        features: Vec<String>,
    },
    ForeignFallback {
        group: String,
        feature: String,
    },
    FallbackCardinality {
        group: String,
        cardinality: Cardinality,
        count: usize,
    },
}

/// Invalid supplied definitions never produce a validated model.
///
/// ```
/// use vibe_core::configuration::{DefinitionError, DefinitionProblem};
/// let error = DefinitionError { problem: DefinitionProblem::InvalidIdentifier { id: "".into() } };
/// assert!(error.to_string().contains("PROP-069#FEATURE-KERNEL"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL")]
#[error(
    "violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL: {problem:?}; fix surface: supplied feature model"
)]
pub struct DefinitionError {
    pub problem: DefinitionProblem,
}

/// An invalid request or conflicting closure never exposes a partial result.
///
/// ```
/// use vibe_core::configuration::EvaluationError;
/// let error = EvaluationError::UnknownFeature { id: "absent".into() };
/// assert!(error.to_string().contains("PROP-069#FEATURE-KERNEL"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL")]
#[error(
    "violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL: {self:?}; fix surface: supplied feature selection"
)]
pub enum EvaluationError {
    ExplicitReplacement {
        group: String,
        selected: String,
        member: String,
        descendant: String,
    },
    UnknownFeature {
        id: String,
    },
    EnabledAndDisabled {
        id: String,
    },
    DisabledAncestor {
        feature: String,
        descendant: String,
    },
    ExplicitCardinality {
        group: String,
        cardinality: Cardinality,
        members: Vec<String>,
    },
    Cardinality {
        group: String,
        cardinality: Cardinality,
        members: Vec<String>,
    },
    DisabledImplied {
        feature: String,
        reason: ImplicationReason,
    },
}
