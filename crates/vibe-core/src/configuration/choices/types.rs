//! Input declarations and separately retained/projection/result values.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CHOICE-CORE");

use super::super::{Cardinality, FeatureResolution};
use super::{AnswerError, ChoiceConflict, ChoiceQuestion};
use std::collections::BTreeMap;

/// A positive dependency in the choice DAG, not an ordinary feature implication.
///
/// ```
/// use vibe_core::configuration::ChoiceGuard;
/// assert_ne!(ChoiceGuard::Group("gui".into()), ChoiceGuard::Member("gui".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChoiceGuard {
    Group(String),
    Member(String),
}

/// One consumer-owned group; members are existing local feature IDs.
/// None and an explicitly empty default set remain distinct.
///
/// ```
/// use vibe_core::configuration::{ChoiceGroup, Cardinality};
/// let group = ChoiceGroup { id: "plugins".into(), members: vec!["a".into()],
///     cardinality: Cardinality::ZeroOrMore, defaults: Some(vec![]), guards: vec![] };
/// assert_eq!(group.defaults, Some(vec![]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceGroup {
    pub id: String,
    pub members: Vec<String>,
    pub cardinality: Cardinality,
    pub defaults: Option<Vec<String>>,
    pub guards: Vec<ChoiceGuard>,
}

/// Additional eligibility guards of a member; ownership implies its group guard.
///
/// ```
/// use vibe_core::configuration::{MemberGuards, ChoiceGuard};
/// let guards = MemberGuards { member: "x".into(), guards: vec![ChoiceGuard::Member("on".into())] };
/// assert_eq!(guards.member, "x");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberGuards {
    pub member: String,
    pub guards: Vec<ChoiceGuard>,
}

/// Omission is not the empty explicit answer, regardless of merge strength.
///
/// ```
/// use vibe_core::configuration::Selection;
/// assert_ne!(Selection::Omitted, Selection::Explicit(vec![]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Omitted,
    Explicit(Vec<String>),
}

/// Whether an answered projection contributes members or bounds the shared set.
///
/// ```
/// use vibe_core::configuration::MergeIntent;
/// assert_ne!(MergeIntent::Exact, MergeIntent::Additive);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeIntent {
    Additive,
    Exact,
}

/// Explicit caller policy for omitted answers; this grants no effect authority.
///
/// ```
/// use vibe_core::configuration::ChoicePolicy;
/// let policy = ChoicePolicy::RequireSelection;
/// assert_ne!(policy, ChoicePolicy::UseDeclaredDefaults);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoicePolicy {
    RequireSelection,
    UseDeclaredDefaults,
}

/// Selection state and strength are independent, including omitted/exact.
///
/// ```
/// use vibe_core::configuration::{ChoiceAnswer, Selection, MergeIntent};
/// let answer = ChoiceAnswer { selection: Selection::Omitted, intent: MergeIntent::Exact };
/// assert_eq!(answer.selection, Selection::Omitted);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceAnswer {
    pub selection: Selection,
    pub intent: MergeIntent,
}

/// Consumers already grouped by the caller for one actual instance.
/// The key is diagnostic data, not an identity or permission to create an instance.
///
/// ```
/// use vibe_core::configuration::{ConsumerChoices, ChoicePolicy};
/// let consumer = ConsumerChoices { key: "project/dependency".into(),
///     policy: ChoicePolicy::RequireSelection, answers: Default::default() };
/// assert!(consumer.answers.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerChoices {
    pub key: String,
    pub policy: ChoicePolicy,
    pub answers: BTreeMap<String, ChoiceAnswer>,
}

/// Default use observed for an active group, including its full unfiltered set.
/// This is provisional data even when upstream answers conflict, never a receipt.
///
/// ```
/// use vibe_core::configuration::{DefaultUse, MergeIntent};
/// let used = DefaultUse { consumer: "a".into(), group: "g".into(),
///     declared: vec![], intent: MergeIntent::Exact };
/// assert!(used.declared.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultUse {
    pub consumer: String,
    pub group: String,
    pub declared: Vec<String>,
    pub intent: MergeIntent,
}

/// How a projection obtains a set; Unanswered contributes no exact bound.
///
/// ```
/// use vibe_core::configuration::ProjectionOrigin;
/// assert_ne!(ProjectionOrigin::Unanswered, ProjectionOrigin::Explicit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionOrigin {
    Unanswered,
    Explicit,
    DeclaredDefaults,
}

/// Active and dormant projections of one consumer; original bytes/order stay in
/// ChoiceEvaluation::original. Unknown answers remain in invalid_answers.
///
/// ```
/// use vibe_core::configuration::{ChoiceProjection, MergeIntent, ProjectionOrigin};
/// let p = ChoiceProjection { consumer: "a".into(), intent: MergeIntent::Exact,
///     origin: ProjectionOrigin::Explicit, eligible: vec![], dormant: vec!["x".into()] };
/// assert_eq!(p.dormant, ["x"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceProjection {
    pub consumer: String,
    pub intent: MergeIntent,
    pub origin: ProjectionOrigin,
    pub eligible: Vec<String>,
    pub dormant: Vec<String>,
}

/// A member's eligibility and requested/defaulted contributors before merging.
///
/// ```
/// use vibe_core::configuration::MemberControl;
/// let m = MemberControl { id: "x".into(), eligible: false, requested_by: vec!["a".into()], blocked_by: vec![] };
/// assert!(!m.eligible);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberControl {
    pub id: String,
    pub eligible: bool,
    pub requested_by: Vec<String>,
    /// False guards, including the implicit owning-group dependency.
    pub blocked_by: Vec<ChoiceGuard>,
}

/// Diagnostic control data, not an executable feature result. A compatible
/// selection may still underflow; conflicts and inactive groups have None.
///
/// ```
/// use vibe_core::configuration::{GroupControl, Cardinality};
/// let g = GroupControl { id: "g".into(), active: false,
///     cardinality: Cardinality::ExactlyOne, members: vec![], projections: vec![], selection: None, blocked_by: vec![] };
/// assert!(g.selection.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupControl {
    pub id: String,
    pub active: bool,
    pub cardinality: Cardinality,
    pub members: Vec<MemberControl>,
    pub projections: Vec<ChoiceProjection>,
    pub selection: Option<Vec<String>>,
    /// Declared group guards whose current truth is false.
    pub blocked_by: Vec<ChoiceGuard>,
}

/// Only the resolved variant contains executable feature output.
///
/// ```
/// use vibe_core::configuration::ChoiceOutcome;
/// assert_eq!(ChoiceOutcome::Unresolved, ChoiceOutcome::Unresolved);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoiceOutcome {
    Resolved { features: FeatureResolution },
    Unresolved,
}

/// All original answers and useful diagnostics survive an unresolved result.
/// The containing choice/default evidence is not candidate safety or acceptance.
///
/// ```
/// use vibe_core::configuration::{ChoiceEvaluation, ChoiceOutcome};
/// let e = ChoiceEvaluation { original: vec![], controls: vec![], defaults: vec![],
///     questions: vec![], conflicts: vec![], invalid_answers: vec![], outcome: ChoiceOutcome::Unresolved };
/// assert!(e.controls.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceEvaluation {
    pub original: Vec<ConsumerChoices>,
    pub controls: Vec<GroupControl>,
    pub defaults: Vec<DefaultUse>,
    pub questions: Vec<ChoiceQuestion>,
    pub conflicts: Vec<ChoiceConflict>,
    pub invalid_answers: Vec<AnswerError>,
    pub outcome: ChoiceOutcome,
}
