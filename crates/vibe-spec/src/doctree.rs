//! The document IR — a hierarchical tree of a spec document (PROP-035 §5).
//!
//! [`DocTree`] is the common intermediate representation the router resolves
//! against. Today it has one frontend — Markdown (ATX headings) — parsed into a
//! tree where a heading of level *L* owns every following heading of level
//! *> L* until the next heading of level *≤ L*. A future XML frontend will
//! build the same [`Node`] tree from elements; everything above the parser
//! (addressing, the router, granularity rules) is written against the tree, not
//! the Markdown.
//!
//! A node's span covers its heading line through the whole of its subtree — so
//! extracting a node's text (`#embed`) or its top-level ancestor's text
//! (`#use`, PROP-035 §5) is a slice of the source lines.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::Range;

/// An index into a [`DocTree`]'s node arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

/// One node of the document tree: a heading and the subtree it owns. The
/// synthetic root (`NodeId(0)`) has `level = 0`, no `id`, and spans the whole
/// document (its own body is the preamble before the first heading).
#[derive(Debug, Clone)]
pub struct Node {
    /// The heading's `{#anchor}`, if it declared one.
    pub id: Option<String>,
    /// Heading level: `1..=6` for headings, `0` for the synthetic root.
    pub level: u8,
    /// Heading text, with the leading `#`s and trailing `{#anchor}` stripped.
    pub heading: String,
    /// 0-based source line of the heading (`0` for the root, which has none).
    pub heading_line: usize,
    /// Source lines `[start, end)` this node covers, subtree included.
    pub span: Range<usize>,
    /// Parent node (`None` only for the root).
    pub parent: Option<NodeId>,
    /// Child nodes, in document order.
    pub children: Vec<NodeId>,
}

/// A parsed document tree plus an anchor index.
#[derive(Debug, Clone)]
pub struct DocTree {
    nodes: Vec<Node>,
    anchors: HashMap<String, NodeId>,
    duplicate_anchors: Vec<String>,
    lines: Vec<String>,
}

impl DocTree {
    /// Parse Markdown source into a document tree. Infallible: malformed
    /// Markdown still yields a tree. A repeated anchor keeps its **first**
    /// occurrence in the index and records the collision (see
    /// [`duplicate_anchors`](Self::duplicate_anchors)).
    pub fn parse(source: &str) -> Self {
        let lines: Vec<String> = source.lines().map(String::from).collect();
        let fenced = fence_mask(&lines);

        let mut nodes = vec![Node {
            id: None,
            level: 0,
            heading: String::new(),
            heading_line: 0,
            span: 0..lines.len(),
            parent: None,
            children: Vec::new(),
        }];
        let mut anchors: HashMap<String, NodeId> = HashMap::new();
        let mut duplicate_anchors = Vec::new();
        let mut stack: Vec<NodeId> = vec![NodeId(0)];

        for (i, line) in lines.iter().enumerate() {
            if fenced[i] {
                continue;
            }
            let Some((level, heading, anchor)) = parse_heading(line) else {
                continue;
            };

            // Close every open node the new heading is a sibling of or an
            // ancestor break from: level >= this one ends here.
            while stack.len() > 1 {
                let top = *stack.last().unwrap();
                if nodes[top.0].level >= level {
                    nodes[top.0].span.end = i;
                    stack.pop();
                } else {
                    break;
                }
            }

            let parent = *stack.last().unwrap();
            let id = NodeId(nodes.len());
            nodes.push(Node {
                id: anchor.clone(),
                level,
                heading,
                heading_line: i,
                span: i..lines.len(),
                parent: Some(parent),
                children: Vec::new(),
            });
            nodes[parent.0].children.push(id);

            if let Some(a) = anchor {
                match anchors.entry(a) {
                    Entry::Vacant(slot) => {
                        slot.insert(id);
                    }
                    Entry::Occupied(slot) => duplicate_anchors.push(slot.key().clone()),
                }
            }
            stack.push(id);
        }
        // Nodes left open run to end of document (their provisional span end).

        DocTree {
            nodes,
            anchors,
            duplicate_anchors,
            lines,
        }
    }

    /// The synthetic root node.
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Borrow a node.
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    /// A node's children, in document order.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id.0].children
    }

    /// The node carrying a given flat anchor, if any (first occurrence on a
    /// collision).
    pub fn find_by_anchor(&self, anchor: &str) -> Option<NodeId> {
        self.anchors.get(anchor).copied()
    }

    /// Resolve a tree-path anchor (`SpecAddress::anchor`, e.g. `a.b.c` →
    /// `["a", "b", "c"]`) to a node. The first segment is matched flat — a
    /// label unique anywhere in the document, as anchors are today; each
    /// further segment descends into the children of the current match. An
    /// empty path denotes the whole document (the root).
    pub fn resolve_path(&self, path: &[String]) -> Option<NodeId> {
        let Some((first, rest)) = path.split_first() else {
            return Some(self.root());
        };
        let mut current = self.find_by_anchor(first)?;
        for seg in rest {
            current = self
                .children(current)
                .iter()
                .copied()
                .find(|&c| self.node(c).id.as_deref() == Some(seg.as_str()))?;
        }
        Some(current)
    }

    /// Anchors that appeared more than once (each extra occurrence listed). An
    /// empty slice means every anchor is unique.
    pub fn duplicate_anchors(&self) -> &[String] {
        &self.duplicate_anchors
    }

    /// The source text a node covers (heading line through its whole subtree),
    /// rejoined with `\n`.
    pub fn text(&self, id: NodeId) -> String {
        self.lines[self.nodes[id.0].span.clone()].join("\n")
    }

    /// Total node count, including the root.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Always false — the root is always present. Provided for lint parity with
    /// [`len`](Self::len).
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// A precomputed mask marking lines inside fenced code blocks (```` ``` ```` or
/// `~~~`), including the fence lines themselves. Headings on masked lines are
/// not tree nodes — a `#` in a code sample is not a section. Shared with the
/// directive scanner, which ignores directives in fenced code the same way.
pub(crate) fn fence_mask(lines: &[String]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    let mut fence: Option<&'static str> = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        match fence {
            Some(marker) => {
                mask[i] = true;
                if trimmed.starts_with(marker) {
                    fence = None;
                }
            }
            None => {
                if trimmed.starts_with("```") {
                    fence = Some("```");
                    mask[i] = true;
                } else if trimmed.starts_with("~~~") {
                    fence = Some("~~~");
                    mask[i] = true;
                }
            }
        }
    }
    mask
}

/// Parse an ATX heading line into `(level, heading_text, anchor)`. Requires a
/// space after the `#`s (so `#nospace` is not a heading), matching the vendored
/// engine's rule.
fn parse_heading(line: &str) -> Option<(u8, String, Option<String>)> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    let (heading, anchor) = split_anchor(rest.trim());
    Some((hashes as u8, heading, anchor))
}

/// Split a trailing `{#anchor}` off heading text.
fn split_anchor(text: &str) -> (String, Option<String>) {
    if let Some(pos) = text.rfind("{#")
        && text.ends_with('}')
    {
        let anchor = &text[pos + 2..text.len() - 1];
        let heading = text[..pos].trim_end().to_string();
        return (heading, Some(anchor.to_string()));
    }
    (text.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "\
preamble line
# Title {#root}
intro under title
## First {#first}
first body
### Deep {#deep}
deep body
## Second {#second}
second body
";

    #[test]
    fn builds_hierarchy() {
        let t = DocTree::parse(DOC);
        let root = t.root();
        // One top-level heading (Title) under the synthetic root.
        let top = t.children(root);
        assert_eq!(top.len(), 1);
        let title = top[0];
        assert_eq!(t.node(title).id.as_deref(), Some("root"));
        assert_eq!(t.node(title).level, 1);

        // Title owns First and Second (both h2).
        let under_title = t.children(title);
        assert_eq!(under_title.len(), 2);
        assert_eq!(t.node(under_title[0]).id.as_deref(), Some("first"));
        assert_eq!(t.node(under_title[1]).id.as_deref(), Some("second"));

        // First owns Deep (h3); Second owns nothing.
        assert_eq!(t.children(under_title[0]).len(), 1);
        assert_eq!(
            t.node(t.children(under_title[0])[0]).id.as_deref(),
            Some("deep")
        );
        assert!(t.children(under_title[1]).is_empty());
    }

    #[test]
    fn find_by_anchor_and_heading_text() {
        let t = DocTree::parse(DOC);
        let deep = t.find_by_anchor("deep").unwrap();
        assert_eq!(t.node(deep).heading, "Deep");
        assert_eq!(t.node(deep).level, 3);
        assert!(t.find_by_anchor("missing").is_none());
    }

    #[test]
    fn span_covers_subtree_and_stops_at_sibling() {
        let t = DocTree::parse(DOC);
        // `First` spans its own body plus `Deep`, and stops at `Second`.
        let first = t.find_by_anchor("first").unwrap();
        let text = t.text(first);
        assert!(text.contains("first body"));
        assert!(text.contains("### Deep"));
        assert!(text.contains("deep body"));
        assert!(!text.contains("Second"));
    }

    #[test]
    fn root_spans_whole_document_including_preamble() {
        let t = DocTree::parse(DOC);
        let text = t.text(t.root());
        assert!(text.contains("preamble line"));
        assert!(text.contains("second body"));
    }

    #[test]
    fn headings_in_fences_are_not_nodes() {
        let src = "\
# Real {#real}
```
# Fake heading in code
```
after
";
        let t = DocTree::parse(src);
        assert!(t.find_by_anchor("real").is_some());
        // The fenced `#` produced no node: Real has no children.
        let real = t.find_by_anchor("real").unwrap();
        assert!(t.children(real).is_empty());
        assert_eq!(t.len(), 2); // root + Real
    }

    #[test]
    fn duplicate_anchor_keeps_first_and_reports() {
        let src = "\
# One {#dup}
a
# Two {#dup}
b
";
        let t = DocTree::parse(src);
        let first = t.find_by_anchor("dup").unwrap();
        assert_eq!(t.node(first).heading, "One");
        assert_eq!(t.duplicate_anchors(), &["dup".to_string()]);
    }

    #[test]
    fn heading_without_anchor_has_none() {
        let t = DocTree::parse("# Plain heading\nbody\n");
        let top = t.children(t.root())[0];
        assert_eq!(t.node(top).id, None);
        assert_eq!(t.node(top).heading, "Plain heading");
    }

    #[test]
    fn hash_without_space_is_not_a_heading() {
        let t = DocTree::parse("#notaheading\ntext\n");
        assert_eq!(t.len(), 1); // root only
    }

    #[test]
    fn resolve_flat_and_tree_path() {
        let t = DocTree::parse(DOC);
        // A single segment matches flat.
        assert_eq!(t.resolve_path(&["first".into()]), t.find_by_anchor("first"));
        // A tree path descends: `deep` is a child of `first`.
        let deep = t.resolve_path(&["first".into(), "deep".into()]).unwrap();
        assert_eq!(t.node(deep).heading, "Deep");
        // An empty path is the whole document.
        assert_eq!(t.resolve_path(&[]), Some(t.root()));
        // A wrong descent fails: `second` is a sibling of `first`, not a child.
        assert!(t.resolve_path(&["first".into(), "second".into()]).is_none());
        // A missing first segment fails.
        assert!(t.resolve_path(&["nope".into()]).is_none());
    }
}
