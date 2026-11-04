//! Named local definition, answer, question and merge failures.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

use super::super::{Cardinality, EvaluationError};
use super::ChoiceGuard;

/// Invalid choice definitions cannot produce a model, even on dormant branches.
///
/// ```
/// use vibe_core::configuration::ChoiceDefinitionError;
/// let e = ChoiceDefinitionError::UnknownMember { group: "g".into(), member: "x".into() };
/// assert!(e.to_string().contains("CHOICE-CORE"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE")]
#[error(
    "violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE: {self:?}; fix surface: supplied choice definition"
)]
pub enum ChoiceDefinitionError {
    InvalidGroupId {
        group: String,
    },
    DuplicateGroup {
        group: String,
    },
    EmptyGroup {
        group: String,
    },
    UnknownMember {
        group: String,
        member: String,
    },
    DuplicateMember {
        group: String,
        member: String,
    },
    MultipleOwners {
        member: String,
        first: String,
        second: String,
    },
    UnknownMemberGuards {
        member: String,
    },
    DuplicateMemberGuards {
        member: String,
    },
    UnknownGuard {
        owner: ChoiceGuard,
        guard: ChoiceGuard,
    },
    DuplicateGuard {
        owner: ChoiceGuard,
        guard: ChoiceGuard,
    },
    UnknownDefault {
        group: String,
        member: String,
    },
    DuplicateDefault {
        group: String,
        member: String,
    },
    DefaultCardinality {
        group: String,
        cardinality: Cardinality,
        members: Vec<String>,
    },
    /// Includes cyclic nodes and their blocked dependents, in stable local order.
    GuardCycle {
        blocked: Vec<ChoiceGuard>,
    },
}

/// Original invalid answers remain available beside all collected errors.
///
/// ```
/// use vibe_core::configuration::AnswerError;
/// let e = AnswerError::UnknownGroup { consumer: "a".into(), group: "old".into() };
/// assert!(e.to_string().contains("old"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE")]
#[error(
    "violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE: {self:?}; fix surface: original consumer answers"
)]
pub enum AnswerError {
    InvalidConsumer {
        consumer: String,
    },
    DuplicateConsumer {
        consumer: String,
    },
    UnknownGroup {
        consumer: String,
        group: String,
    },
    UnknownMember {
        consumer: String,
        group: String,
        member: String,
    },
    DuplicateMember {
        consumer: String,
        group: String,
        member: String,
    },
}

/// An active compatible projection lacks a mandatory selection.
///
/// ```
/// use vibe_core::configuration::{ChoiceQuestion, Cardinality};
/// let q = ChoiceQuestion { group: "renderer".into(), cardinality: Cardinality::ExactlyOne,
///     eligible_members: vec!["text".into()], consumers: vec!["project".into()] };
/// assert_eq!(q.eligible_members, ["text"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceQuestion {
    pub group: String,
    pub cardinality: Cardinality,
    pub eligible_members: Vec<String>,
    pub consumers: Vec<String>,
}

/// Conflicts retain the direct contributing projections or mismatched members.
/// No variant contains an executable partial feature result.
///
/// ```
/// use vibe_core::configuration::ChoiceConflict;
/// let e = ChoiceConflict::UnchosenActivation { members: vec!["unanswered".into()] };
/// assert!(e.to_string().contains("unanswered"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE")]
#[error(
    "violates REQ spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE: {self:?}; fix surface: contributing choice or ordinary feature inputs"
)]
pub enum ChoiceConflict {
    FeatureInput {
        error: EvaluationError,
    },
    ExactDisagreement {
        group: String,
        first_consumer: String,
        first_members: Vec<String>,
        second_consumer: String,
        second_members: Vec<String>,
    },
    AdditiveOutsideExact {
        group: String,
        exact_consumer: String,
        exact_members: Vec<String>,
        additive_consumer: String,
        outside: Vec<String>,
    },
    Cardinality {
        group: String,
        cardinality: Cardinality,
        members: Vec<String>,
        consumers: Vec<String>,
    },
    FeatureClosure {
        error: EvaluationError,
    },
    UnchosenActivation {
        members: Vec<String>,
    },
    ChosenLoss {
        members: Vec<String>,
    },
}
