//! Non-panicking structural invariants of the private [`DocTree`] arena.
//!
//! The public accessors (`node`, `children`) panic-index, so a caller outside
//! this cell cannot safely diagnose a malformed arena — a corrupt carrier
//! would abort instead of returning a typed error. This child module reads the
//! arena with checked `.get()` access only and is the one home of the
//! well-formed-tree law the inter-pass verifier delegates to (PROP-054
//! `##INTER-PASS-VERIFIER`: "well-formed trees ... after every pass").
//!
//! The law is everything [`DocTree::parse`] guarantees, not merely "the child
//! pointers happen to line up": the synthetic root's exact shape, one incoming
//! edge per non-root node, spans nested and in document order, heading levels
//! strictly deepening, arena order equal to document order, and an anchor index
//! that is exactly the arena's own first-occurrence view. Malformed arenas are
//! constructible only through the `#[cfg(test)]` constructors at the bottom of
//! this file.

use std::collections::HashMap;

use super::{DocTree, Node, NodeId, NodeKind};

/// Why a document tree's arena violates its structural contract.
///
/// Indices are arena positions; every value is data, never a rendered
/// diagnostic string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DocTreeInvariantError {
    #[error("the node arena is empty: every tree carries its synthetic root")]
    EmptyArena,
    #[error("arena node 0 is not the synthetic root: `{field}` is not its parsed value")]
    RootNotSynthetic { field: &'static str },
    #[error("heading node {index} carries illegal level {level} (root is 0, headings are 1..=6)")]
    IllegalHeadingLevel { index: usize, level: u8 },
    #[error("fact node {index} is not the shape the parser builds: `{field}` is wrong")]
    FactShape { index: usize, field: &'static str },
    #[error("node {index} starts at line {start} but reports heading line {heading_line}")]
    HeadingLineMismatch {
        index: usize,
        heading_line: usize,
        start: usize,
    },
    #[error("node {index} span {start}..{end} leaves the document's {len} source lines")]
    SpanOutOfBounds {
        index: usize,
        start: usize,
        end: usize,
        len: usize,
    },
    #[error("node {parent} names child {child} outside the arena of {len} nodes")]
    ChildOutOfBounds {
        parent: usize,
        child: usize,
        len: usize,
    },
    #[error("node {parent} lists child {child} more than once")]
    DuplicateChild { parent: usize, child: usize },
    #[error(
        "node {index} has {count} incoming child edges: the root must have none and every other node exactly one"
    )]
    IncomingEdges { index: usize, count: usize },
    #[error("node {child} names parent {parent}, but that node does not list it as a child")]
    ParentMismatch { parent: usize, child: usize },
    #[error("node {index} is not reachable from the synthetic root")]
    UnreachableNode { index: usize },
    #[error("node {child} span is not nested under its parent {parent}")]
    SpanNotNested { parent: usize, child: usize },
    #[error("node {parent} lists child {child} before its sibling {previous} in document order")]
    ChildrenOutOfOrder {
        parent: usize,
        previous: usize,
        child: usize,
    },
    #[error("heading {child} is not deeper than the heading {parent} that owns it")]
    HeadingNotDeeper { parent: usize, child: usize },
    #[error("arena node {index} starts before node {previous}: arena order is document order")]
    ArenaOrderBroken { previous: usize, index: usize },
    #[error(
        "anchor {anchor:?} indexes node {actual:?}, but its first arena occurrence is {expected:?}"
    )]
    AnchorIndex {
        anchor: String,
        expected: Option<usize>,
        actual: Option<usize>,
    },
    #[error("the duplicate-anchor record {actual:?} is not the arena's own repeats {expected:?}")]
    DuplicateAnchors {
        expected: Vec<String>,
        actual: Vec<String>,
    },
}

/// Prove the arena is one well-formed tree, in a fixed order: bounds and node
/// shape before any relation, relations before the derived anchor index, so a
/// first failure never depends on hash iteration and nothing is indexed before
/// it has been checked. Never panics on malformed input.
pub(crate) fn verify_structure(tree: &DocTree) -> Result<(), DocTreeInvariantError> {
    let nodes = &tree.nodes;
    if nodes.is_empty() {
        return Err(DocTreeInvariantError::EmptyArena);
    }
    verify_root(&nodes[0], tree.lines.len())?;
    for (index, node) in nodes.iter().enumerate() {
        verify_node_shape(index, node, tree.lines.len())?;
    }
    verify_child_ids(nodes)?;
    verify_parent_edges(nodes)?;
    verify_reachability(nodes)?;
    verify_relations(nodes)?;
    verify_arena_order(nodes)?;
    verify_anchor_index(tree)
}

/// The synthetic root is exact, field by field — a tree whose root drifted is
/// not the tree `parse` builds, however consistent its pointers look.
fn verify_root(root: &Node, line_count: usize) -> Result<(), DocTreeInvariantError> {
    let field = if root.kind != NodeKind::Heading {
        "kind"
    } else if root.level != 0 {
        "level"
    } else if root.id.is_some() {
        "id"
    } else if !root.heading.is_empty() {
        "heading"
    } else if !root.trailing.is_empty() {
        "trailing"
    } else if root.heading_line != 0 {
        "heading_line"
    } else if root.parent.is_some() {
        "parent"
    } else if root.span != (0..line_count) {
        "span"
    } else {
        return Ok(());
    };
    Err(DocTreeInvariantError::RootNotSynthetic { field })
}

/// The per-node shape law: heading levels, fact leaf shape, the heading line
/// inside its own span, and span bounds.
fn verify_node_shape(
    index: usize,
    node: &Node,
    line_count: usize,
) -> Result<(), DocTreeInvariantError> {
    match node.kind {
        NodeKind::Heading => {
            let legal = index == 0 && node.level == 0 || index > 0 && (1..=6).contains(&node.level);
            if !legal {
                return Err(DocTreeInvariantError::IllegalHeadingLevel {
                    index,
                    level: node.level,
                });
            }
        }
        NodeKind::Fact => {
            // `flush_block` mints every fact leaf from one `##<ID>` segment: an
            // id it always carries, a childless levelless body, and none of the
            // heading fields. Each is checked by name, so a red states which
            // one drifted rather than "some fact is wrong".
            let field = if index == 0 {
                Some("position")
            } else if node.id.is_none() {
                Some("id")
            } else if !node.heading.is_empty() {
                Some("heading")
            } else if !node.trailing.is_empty() {
                Some("trailing")
            } else if node.level != 0 {
                Some("level")
            } else if !node.children.is_empty() {
                Some("children")
            } else {
                None
            };
            if let Some(field) = field {
                return Err(DocTreeInvariantError::FactShape { index, field });
            }
        }
    }
    if node.span.start > node.span.end || node.span.end > line_count {
        return Err(DocTreeInvariantError::SpanOutOfBounds {
            index,
            start: node.span.start,
            end: node.span.end,
            len: line_count,
        });
    }
    if index > 0 && node.heading_line != node.span.start {
        return Err(DocTreeInvariantError::HeadingLineMismatch {
            index,
            heading_line: node.heading_line,
            start: node.span.start,
        });
    }
    Ok(())
}

/// Every child id is in bounds and listed at most once by its parent. Runs
/// before any relation so no later pass can index an invalid position.
fn verify_child_ids(nodes: &[Node]) -> Result<(), DocTreeInvariantError> {
    for (index, node) in nodes.iter().enumerate() {
        let mut seen: Vec<usize> = Vec::with_capacity(node.children.len());
        for &child in &node.children {
            if child.0 >= nodes.len() {
                return Err(DocTreeInvariantError::ChildOutOfBounds {
                    parent: index,
                    child: child.0,
                    len: nodes.len(),
                });
            }
            if seen.contains(&child.0) {
                return Err(DocTreeInvariantError::DuplicateChild {
                    parent: index,
                    child: child.0,
                });
            }
            seen.push(child.0);
        }
    }
    Ok(())
}

/// Exactly one incoming edge per non-root node, none for the root, and the
/// child's own parent pointer naming that same edge. Counting incoming edges is
/// what rules out a node adopted by two parents, or by none.
fn verify_parent_edges(nodes: &[Node]) -> Result<(), DocTreeInvariantError> {
    let mut incoming = vec![0usize; nodes.len()];
    for node in nodes {
        for &child in &node.children {
            incoming[child.0] += 1;
        }
    }
    for (index, count) in incoming.into_iter().enumerate() {
        let expected = usize::from(index > 0);
        if count != expected {
            return Err(DocTreeInvariantError::IncomingEdges { index, count });
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        for &child in &node.children {
            if nodes[child.0].parent != Some(NodeId(index)) {
                return Err(DocTreeInvariantError::ParentMismatch {
                    parent: index,
                    child: child.0,
                });
            }
        }
    }
    Ok(())
}

/// One incoming edge each still admits a detached cycle, so the root must
/// actually reach every node.
fn verify_reachability(nodes: &[Node]) -> Result<(), DocTreeInvariantError> {
    let mut reached = vec![false; nodes.len()];
    let mut stack = vec![NodeId(0)];
    reached[0] = true;
    while let Some(current) = stack.pop() {
        for &child in &nodes[current.0].children {
            if !reached[child.0] {
                reached[child.0] = true;
                stack.push(child);
            }
        }
    }
    match reached.iter().position(|seen| !seen) {
        Some(index) => Err(DocTreeInvariantError::UnreachableNode { index }),
        None => Ok(()),
    }
}

/// Parent/child relations: spans nested, siblings in document order and
/// disjoint, and a heading child strictly deeper than the heading owning it.
fn verify_relations(nodes: &[Node]) -> Result<(), DocTreeInvariantError> {
    for (index, node) in nodes.iter().enumerate() {
        let mut previous: Option<usize> = None;
        for &child in &node.children {
            let child_node = &nodes[child.0];
            if child_node.span.start < node.span.start || child_node.span.end > node.span.end {
                return Err(DocTreeInvariantError::SpanNotNested {
                    parent: index,
                    child: child.0,
                });
            }
            if let Some(previous) = previous
                && nodes[previous].span.end > child_node.span.start
            {
                return Err(DocTreeInvariantError::ChildrenOutOfOrder {
                    parent: index,
                    previous,
                    child: child.0,
                });
            }
            if node.kind == NodeKind::Heading
                && child_node.kind == NodeKind::Heading
                && child_node.level <= node.level
            {
                return Err(DocTreeInvariantError::HeadingNotDeeper {
                    parent: index,
                    child: child.0,
                });
            }
            previous = Some(child.0);
        }
    }
    Ok(())
}

/// Arena order is document order: `parse` mints every node when it reads it, so
/// positions 1.. start strictly later in the source, one node per line. The
/// anchor index and `anchored()` both read the arena in this order, so the
/// index law below is only meaningful once this holds.
fn verify_arena_order(nodes: &[Node]) -> Result<(), DocTreeInvariantError> {
    for index in 2..nodes.len() {
        if nodes[index].span.start <= nodes[index - 1].span.start {
            return Err(DocTreeInvariantError::ArenaOrderBroken {
                previous: index - 1,
                index,
            });
        }
    }
    Ok(())
}

/// The anchor index is a *derived* view, not an independent fact: it must be
/// exactly "first arena occurrence wins", and `duplicate_anchors` exactly the
/// repeats after it, in arena order. A tree whose index disagrees would let
/// `find_by_anchor` and `anchored()` name different nodes.
fn verify_anchor_index(tree: &DocTree) -> Result<(), DocTreeInvariantError> {
    let mut expected_index: HashMap<&str, usize> = HashMap::new();
    let mut expected_duplicates: Vec<String> = Vec::new();
    for (index, node) in tree.nodes.iter().enumerate().skip(1) {
        let Some(anchor) = node.id.as_deref() else {
            continue;
        };
        if expected_index.contains_key(anchor) {
            expected_duplicates.push(anchor.to_string());
        } else {
            expected_index.insert(anchor, index);
        }
    }

    // Arena order, not map order: the first failure must not depend on hash
    // iteration.
    for (index, node) in tree.nodes.iter().enumerate().skip(1) {
        let Some(anchor) = node.id.as_deref() else {
            continue;
        };
        if expected_index.get(anchor) != Some(&index) {
            continue;
        }
        let actual = tree.anchors.get(anchor).map(|node| node.0);
        if actual != Some(index) {
            return Err(DocTreeInvariantError::AnchorIndex {
                anchor: anchor.to_string(),
                expected: Some(index),
                actual,
            });
        }
    }
    let mut extra: Vec<(usize, &String)> = tree
        .anchors
        .iter()
        .filter(|(anchor, _)| !expected_index.contains_key(anchor.as_str()))
        .map(|(anchor, node)| (node.0, anchor))
        .collect();
    extra.sort_unstable();
    if let Some((node, anchor)) = extra.first() {
        return Err(DocTreeInvariantError::AnchorIndex {
            anchor: (*anchor).clone(),
            expected: None,
            actual: Some(*node),
        });
    }
    if tree.duplicate_anchors != expected_duplicates {
        return Err(DocTreeInvariantError::DuplicateAnchors {
            expected: expected_duplicates,
            actual: tree.duplicate_anchors.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
impl DocTree {
    /// Build a tree from a hand-mutated arena, bypassing every parse law.
    ///
    /// The anchor index is derived from the arena exactly as `parse` would
    /// derive it, so a structural red states one fact and is never masked by an
    /// incidental index mismatch. Use [`Self::corrupt_index_for_test`] to
    /// corrupt the index itself.
    pub(crate) fn corrupt_for_test(nodes: Vec<Node>, line_count: usize) -> Self {
        let mut anchors = std::collections::HashMap::new();
        let mut duplicate_anchors = Vec::new();
        for (index, node) in nodes.iter().enumerate().skip(1) {
            let Some(anchor) = node.id.clone() else {
                continue;
            };
            match anchors.entry(anchor) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(NodeId(index));
                }
                std::collections::hash_map::Entry::Occupied(slot) => {
                    duplicate_anchors.push(slot.key().clone());
                }
            }
        }
        Self::corrupt_index_for_test(nodes, line_count, anchors, duplicate_anchors)
    }

    /// Build a tree with a hand-written anchor index, so the derived-view law
    /// can be falsified on its own.
    pub(crate) fn corrupt_index_for_test(
        nodes: Vec<Node>,
        line_count: usize,
        anchors: std::collections::HashMap<String, NodeId>,
        duplicate_anchors: Vec<String>,
    ) -> Self {
        Self {
            nodes,
            anchors,
            duplicate_anchors,
            lines: vec![String::new(); line_count],
            directives: Box::new(crate::directives::Directives::parse("")),
        }
    }
}

#[cfg(test)]
mod tests;
