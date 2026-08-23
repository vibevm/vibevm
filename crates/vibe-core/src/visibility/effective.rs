specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#effective-set");

use std::collections::{BTreeMap, BTreeSet};

use super::{Diagnostic, NodeId};

/// The rule that admitted the decisive edge of a provenance chain.
///
/// ```
/// use vibe_core::visibility::ProvenanceRule;
///
/// assert_ne!(ProvenanceRule::RootEdge, ProvenanceRule::PublicChain);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceRule {
    RootEdge,
    PublicChain,
    FriendsChain,
}

/// One deterministic witness for membership in the effective set.
///
/// ```
/// use vibe_core::visibility::{Provenance, ProvenanceRule};
///
/// let witness = Provenance {
///     rule: ProvenanceRule::RootEdge,
///     path: vec!["org.x/root".into(), "org.x/api".into()],
///     via_override: None,
/// };
/// assert_eq!(witness.path.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub rule: ProvenanceRule,
    pub path: Vec<NodeId>,
    pub via_override: Option<NodeId>,
}

/// The friend closure, effective set, and warning diagnostics for one root.
///
/// ```
/// use vibe_core::visibility::Analysis;
///
/// let result = Analysis::default();
/// assert!(result.closure.is_empty() && result.effective.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Analysis {
    pub closure: BTreeSet<NodeId>,
    pub effective: BTreeMap<NodeId, Provenance>,
    pub diagnostics: Vec<Diagnostic>,
}
