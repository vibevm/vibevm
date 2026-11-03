//! Supplied definitions and explicit input/output values for the local kernel.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-KERNEL");

/// Cardinality of an active closure group.
///
/// ```
/// use vibe_core::configuration::Cardinality;
/// let optional = Cardinality::ZeroOrOne;
/// assert_ne!(optional, Cardinality::ExactlyOne);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
    ExactlyOne,
}

impl Cardinality {
    pub(super) fn accepts(self, count: usize) -> bool {
        match self {
            Self::ZeroOrMore => true,
            Self::OneOrMore => count > 0,
            Self::ZeroOrOne => count <= 1,
            Self::ExactlyOne => count == 1,
        }
    }

    pub(super) fn at_most_one(self) -> bool {
        matches!(self, Self::ZeroOrOne | Self::ExactlyOne)
    }
}

/// One declared feature; identifiers are opaque local, nonblank tokens.
///
/// ```
/// use vibe_core::configuration::FeatureNode;
/// let feature = FeatureNode {
///     id: "_detail".into(), default: false, parent: None,
///     required: false, requires: vec![],
/// };
/// assert!(!feature.default);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureNode {
    pub id: String,
    pub default: bool,
    pub parent: Option<String>,
    pub required: bool,
    pub requires: Vec<String>,
}

/// A constraint over the computed feature set, in declared evaluation order.
/// Constraints may overlap; they do not give a member one owning choice group.
/// Active when any member has no parent or a member's parent is selected.
/// Fallback applies only when this group is active and has no selected member.
///
/// ```
/// use vibe_core::configuration::{Cardinality, ClosureGroup};
/// let group = ClosureGroup {
///     id: "renderer".into(), members: vec!["text".into(), "gui".into()],
///     cardinality: Cardinality::ExactlyOne, fallback: Some(vec!["text".into()]),
/// };
/// assert_eq!(group.members.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureGroup {
    pub id: String,
    pub members: Vec<String>,
    pub cardinality: Cardinality,
    pub fallback: Option<Vec<String>>,
}

/// The caller chooses base policy explicitly; no default implementation selects it.
///
/// ```
/// use vibe_core::configuration::FeatureBase;
/// let empty = FeatureBase::Selected(vec![]);
/// assert_ne!(empty, FeatureBase::Defaults);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureBase {
    Defaults,
    Selected(Vec<String>),
}

/// Explicit additions replace competing members of an at-most-one group.
/// Base members alone do not have that precedence. Repeated input IDs are sets.
///
/// ```
/// use vibe_core::configuration::{FeatureBase, FeatureInput};
/// let input = FeatureInput {
///     base: FeatureBase::Defaults, additions: vec!["gui".into()], disabled: vec![],
/// };
/// assert_eq!(input.additions, ["gui"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureInput {
    pub base: FeatureBase,
    pub additions: Vec<String>,
    pub disabled: Vec<String>,
}

/// The first rule that added a feature during deterministic closure.
///
/// ```
/// use vibe_core::configuration::ImplicationReason;
/// let reason = ImplicationReason::Required { parent: None };
/// assert_eq!(reason, ImplicationReason::Required { parent: None });
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplicationReason {
    Parent { child: String },
    Requires { feature: String },
    Required { parent: Option<String> },
    GroupDefault { group: String },
}

/// One implied selection; the result orders these by feature declaration.
///
/// ```
/// use vibe_core::configuration::{Implication, ImplicationReason};
/// let implied = Implication {
///     feature: "core".into(), reason: ImplicationReason::Required { parent: None },
/// };
/// assert_eq!(implied.feature, "core");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Implication {
    pub feature: String,
    pub reason: ImplicationReason,
}

/// Selected features and explanations in declaration order, never a partial failure.
///
/// ```
/// use vibe_core::configuration::FeatureResolution;
/// let empty = FeatureResolution { selected: vec![], implied: vec![] };
/// assert!(empty.selected.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureResolution {
    pub selected: Vec<String>,
    pub implied: Vec<Implication>,
}
