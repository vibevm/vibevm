specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#model");

use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::{AccessLevel, OverrideEntry, OverrideTarget};

/// A canonical `group/name` identity in a visibility graph.
///
/// ```
/// use vibe_core::visibility::NodeId;
///
/// let node: NodeId = "org.x/api".into();
/// assert_eq!(node, "org.x/api");
/// ```
pub type NodeId = String;

/// One declared dependency edge in the pure visibility input graph.
///
/// ```
/// use vibe_core::visibility::EdgeDecl;
///
/// let edge = EdgeDecl { to: "org.x/api".into(), ..EdgeDecl::default() };
/// assert_eq!(edge.to, "org.x/api");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EdgeDecl {
    pub to: NodeId,
    pub access: Option<AccessLevel>,
    pub friend: Option<bool>,
    pub exclude: Vec<NodeId>,
}

impl EdgeDecl {
    /// The declared access with the public default applied.
    ///
    /// ```
    /// use vibe_core::manifest::AccessLevel;
    /// use vibe_core::visibility::EdgeDecl;
    ///
    /// assert_eq!(EdgeDecl::default().effective_access(), AccessLevel::Public);
    /// ```
    pub fn effective_access(&self) -> AccessLevel {
        self.access.unwrap_or_default()
    }

    /// The friendship flag with the friends-only implication applied.
    ///
    /// ```
    /// use vibe_core::manifest::AccessLevel;
    /// use vibe_core::visibility::EdgeDecl;
    ///
    /// let edge = EdgeDecl { access: Some(AccessLevel::FriendsOnly), ..EdgeDecl::default() };
    /// assert!(edge.effective_friend());
    /// ```
    pub fn effective_friend(&self) -> bool {
        self.friend
            .unwrap_or_else(|| self.effective_access() == AccessLevel::FriendsOnly)
    }
}

/// All declarations owned by one graph node.
///
/// ```
/// use vibe_core::visibility::NodeDecl;
///
/// let node = NodeDecl { friends: vec!["org.x/api".into()], ..NodeDecl::default() };
/// assert!(node.grants().contains("org.x/api"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeDecl {
    pub edges: Vec<EdgeDecl>,
    pub friends: Vec<NodeId>,
    pub unfriend: Vec<NodeId>,
    pub allow_friends: Option<Vec<NodeId>>,
    pub overrides: Vec<(OverrideTarget, OverrideEntry)>,
}

impl NodeDecl {
    /// Direct friend grants: effective edge grants plus the list, minus unfriend.
    ///
    /// ```
    /// use vibe_core::visibility::NodeDecl;
    ///
    /// let node = NodeDecl {
    ///     friends: vec!["org.x/a".into(), "org.x/b".into()],
    ///     unfriend: vec!["org.x/b".into()],
    ///     ..NodeDecl::default()
    /// };
    /// assert_eq!(node.grants().into_iter().collect::<Vec<_>>(), ["org.x/a"]);
    /// ```
    pub fn grants(&self) -> BTreeSet<NodeId> {
        let mut grants: BTreeSet<NodeId> = self
            .edges
            .iter()
            .filter(|edge| edge.effective_friend())
            .map(|edge| edge.to.clone())
            .chain(self.friends.iter().cloned())
            .collect();
        for target in &self.unfriend {
            grants.remove(target);
        }
        grants
    }
}

/// The complete pure input graph for one visibility analysis.
///
/// ```
/// use vibe_core::visibility::{NodeDecl, VisibilityGraph};
///
/// let mut graph = VisibilityGraph::default();
/// graph.nodes.insert("org.x/root".into(), NodeDecl::default());
/// assert_eq!(graph.nodes.len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisibilityGraph {
    pub nodes: BTreeMap<NodeId, NodeDecl>,
}
