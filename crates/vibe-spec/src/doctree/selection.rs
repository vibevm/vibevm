//! Checked address selection leaves the low-level first-match API unchanged.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-UNIQUE-SELECTION"
);

use super::{DocTree, NodeId};

/// A requested path segment has several matches in its actual lookup scope.
///
/// ```
/// use vibe_spec::DocTree;
/// let tree = DocTree::parse("# A {#same}\n\n# B {#same}\n");
/// let error = tree.resolve_unique_path(&["same".into()]).unwrap_err();
/// assert_eq!(error.matches, 2);
/// assert_eq!(error.depth, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "ambiguous anchor `{segment}` at path segment {depth}: {matches} matches (violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-UNIQUE-SELECTION; fix surface: disambiguate the requested source path)"
)]
pub struct AnchorAmbiguity {
    pub segment: String,
    pub depth: usize,
    pub matches: usize,
}

impl DocTree {
    /// Select uniquely: first segment in the flat index's scope, later segments
    /// among the chosen parent's direct children. Absence is Ok(None).
    /// Unrelated duplicates do not invalidate an otherwise unique path.
    ///
    /// ```
    /// use vibe_spec::DocTree;
    /// let tree = DocTree::parse("# Left {#left}\n## Leaf {#leaf}\nL\n# Right {#right}\n## Leaf {#leaf}\nR\n");
    /// assert!(tree.resolve_unique_path(&["leaf".into()]).is_err());
    /// assert!(tree.resolve_unique_path(&["right".into(), "leaf".into()])?.is_some());
    /// assert!(tree.resolve_unique_path(&["missing".into()])?.is_none());
    /// assert_eq!(tree.resolve_unique_path(&[])?, Some(tree.root()));
    /// # Ok::<(), vibe_spec::AnchorAmbiguity>(())
    /// ```
    pub fn resolve_unique_path(&self, path: &[String]) -> Result<Option<NodeId>, AnchorAmbiguity> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(Some(self.root()));
        };
        let matches = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.id.as_deref() == Some(first))
            .map(|(index, _)| NodeId(index));
        let Some(mut current) = unique(matches, first, 0)? else {
            return Ok(None);
        };
        for (depth, segment) in rest.iter().enumerate() {
            let matches = self
                .children(current)
                .iter()
                .copied()
                .filter(|&id| self.node(id).id.as_deref() == Some(segment));
            let Some(next) = unique(matches, segment, depth + 1)? else {
                return Ok(None);
            };
            current = next;
        }
        Ok(Some(current))
    }
}

fn unique(
    mut candidates: impl Iterator<Item = NodeId>,
    segment: &str,
    depth: usize,
) -> Result<Option<NodeId>, AnchorAmbiguity> {
    let Some(first) = candidates.next() else {
        return Ok(None);
    };
    let matches = 1 + candidates.count();
    if matches > 1 {
        return Err(AnchorAmbiguity {
            segment: segment.into(),
            depth,
            matches,
        });
    }
    Ok(Some(first))
}
