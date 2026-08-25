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

use crate::directives::Directives;

/// An index into a [`DocTree`]'s node arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

/// What kind of IR node this is (PROP-035 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// A heading and the subtree it owns (`#`…`######`, or the synthetic root).
    Heading,
    /// A `##<ID>` fact leaf — the finest grain, a paragraph or list item whose
    /// lead token anchors it. Always childless; its parent is the enclosing
    /// section.
    Fact,
}

/// One node of the document tree: a heading and the subtree it owns, or a
/// `##<ID>` fact leaf. The synthetic root (`NodeId(0)`) is a [`NodeKind::Heading`]
/// with `level = 0`, no `id`, spanning the whole document (its own body is the
/// preamble before the first heading).
#[derive(Debug, Clone)]
pub struct Node {
    /// The heading's `{#anchor}` or the fact's `##<ID>`, if it declared one.
    pub id: Option<String>,
    /// Heading level: `1..=6` for headings, `0` for the synthetic root. A
    /// [`NodeKind::Fact`] leaf is levelless and carries `0` (unused — a fact is
    /// attached directly to its section, never via the level stack).
    pub level: u8,
    /// Whether this node is a heading section or a fact leaf.
    pub kind: NodeKind,
    /// Heading text, with the leading `#`s and trailing `{#anchor}` stripped.
    /// Empty for a fact leaf.
    pub heading: String,
    /// Any text after the `{#anchor}` on the heading line — e.g. a `#source`
    /// merge marker `:add` / `:replace` (PROP-035 §7.3). Empty when absent and
    /// for a fact leaf.
    pub trailing: String,
    /// 0-based source line of the heading (`0` for the root, which has none), or
    /// the first line of a fact leaf's span.
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
    directives: Box<Directives>,
}

impl DocTree {
    /// Parse Markdown source into a document tree. Infallible: malformed
    /// Markdown still yields a tree. A repeated anchor keeps its **first**
    /// occurrence in the index and records the collision (see
    /// [`duplicate_anchors`](Self::duplicate_anchors)).
    pub fn parse(source: &str) -> Self {
        let directives = Directives::parse(source);
        let lines: Vec<String> = source.lines().map(String::from).collect();
        let fenced = fence_mask(&lines);

        let mut nodes = vec![Node {
            id: None,
            level: 0,
            kind: NodeKind::Heading,
            heading: String::new(),
            trailing: String::new(),
            heading_line: 0,
            span: 0..lines.len(),
            parent: None,
            children: Vec::new(),
        }];
        let mut anchors: HashMap<String, NodeId> = HashMap::new();
        let mut duplicate_anchors = Vec::new();
        let mut stack: Vec<NodeId> = vec![NodeId(0)];

        // The open text block (a maximal run of non-blank, non-heading,
        // non-fenced lines), `[block_start, i)`, and the section that encloses
        // it — captured when the block opened, constant for its life because a
        // heading is what would change the section and also ends the block.
        let mut block_start: Option<usize> = None;
        let mut block_section = NodeId(0);

        for (i, line) in lines.iter().enumerate() {
            // A fenced, blank, or heading line all close an open text block.
            if fenced[i] || line.trim().is_empty() {
                if let Some(bs) = block_start.take() {
                    flush_block(
                        &lines,
                        bs,
                        i,
                        block_section,
                        &mut nodes,
                        &mut anchors,
                        &mut duplicate_anchors,
                    );
                }
                continue;
            }
            let Some((level, heading, anchor, trailing)) = parse_heading(line) else {
                // A content line: open a block if none is open, else extend it.
                if block_start.is_none() {
                    block_start = Some(i);
                    block_section = *stack.last().unwrap();
                }
                continue;
            };

            if let Some(bs) = block_start.take() {
                flush_block(
                    &lines,
                    bs,
                    i,
                    block_section,
                    &mut nodes,
                    &mut anchors,
                    &mut duplicate_anchors,
                );
            }

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
                kind: NodeKind::Heading,
                heading,
                trailing,
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
        // A block still open at EOF, and any nodes left open, run to end of doc.
        if let Some(bs) = block_start.take() {
            flush_block(
                &lines,
                bs,
                lines.len(),
                block_section,
                &mut nodes,
                &mut anchors,
                &mut duplicate_anchors,
            );
        }

        DocTree {
            nodes,
            anchors,
            duplicate_anchors,
            lines,
            directives: Box::new(directives),
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

    /// Directives parsed from the same document bytes as this tree.
    ///
    /// Keeping them in the document carrier lets close discover and order the
    /// graph without rescanning raw [`crate::compiler::ir::SourceIr`] text or
    /// creating a second parse side path.
    pub(crate) fn directives(&self) -> &Directives {
        &self.directives
    }

    /// The qualified heirs of a short anchor name (B-011 §6.1 layer 3): every
    /// anchor in this tree whose qualified form ends `--<short>` — i.e. a label
    /// the qualify phase renamed from `short` to `<origin-slug>--short`. Sorted
    /// for determinism (the tree's anchor map is unordered). Empty when no
    /// qualified tail matches, so a caller can tell "definitely absent" from
    /// "renamed — here are the candidates".
    ///
    /// Used by the resolver to answer a missed short anchor with its qualified
    /// heirs rather than emptiness (design §5: fail with candidates, never a
    /// silent pick).
    pub fn qualified_candidates(&self, short: &str) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .anchors
            .keys()
            .map(String::as_str)
            .filter(|a| a.rsplit_once("--").is_some_and(|(_, tail)| tail == short))
            .collect();
        out.sort();
        out
    }

    /// Every anchored node — heading **and** fact leaf — in document order, as
    /// `(id, anchor)`. Skips the root and any heading without an anchor. Fact
    /// ids share the one anchor namespace (PROP-035 §5), so both grains appear;
    /// [`sections`](Self::sections) is the heading-only view.
    pub fn anchored(&self) -> impl Iterator<Item = (NodeId, &str)> {
        self.nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, n)| n.id.as_deref().map(|a| (NodeId(i), a)))
    }

    /// The anchored **heading** sections, in document order, as `(id, anchor)`.
    /// Fact leaves are excluded — this is the section-grain view the `#source`
    /// merge (PROP-035 §7.3) iterates, where a fact rides inside its section's
    /// span rather than as its own merge unit.
    pub fn sections(&self) -> impl Iterator<Item = (NodeId, &str)> {
        self.nodes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, n)| n.kind == NodeKind::Heading)
            .filter_map(|(i, n)| n.id.as_deref().map(|a| (NodeId(i), a)))
    }

    /// The fact leaves inside a node's subtree (the node itself and every
    /// descendant), in document order, as `(NodeId, id)`. Used by the `:add`
    /// merge to find which contract facts a source section redeclares
    /// (PROP-035 §7.3, per-fact override).
    pub fn facts_under(&self, root: NodeId) -> Vec<(NodeId, &str)> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let n = &self.nodes[id.0];
            if n.kind == NodeKind::Fact
                && let Some(a) = n.id.as_deref()
            {
                out.push((id, a));
            }
            // Push children reversed so the pop order is document order.
            stack.extend(n.children.iter().rev().copied());
        }
        out.sort_by_key(|(id, _)| self.nodes[id.0].span.start);
        out
    }

    /// A node's text (its span, joined) with the given fact leaves' line spans
    /// removed — the per-fact override of PROP-035 §7.3: a contract section's
    /// text minus the facts the source redeclares. `drop` leaves outside the
    /// section's span are ignored.
    pub fn text_without(&self, node: NodeId, drop: &[NodeId]) -> String {
        let dropped: std::collections::HashSet<usize> = drop
            .iter()
            .flat_map(|&f| self.nodes[f.0].span.clone())
            .collect();
        let span = self.nodes[node.0].span.clone();
        span.filter(|i| !dropped.contains(i))
            .map(|i| self.lines[i].as_str())
            .collect::<Vec<_>>()
            .join("\n")
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

/// Segment a closed text block `[start, end)` into `##<ID>` fact leaves, push
/// each as a childless [`NodeKind::Fact`] node parented at `section`, and register
/// its id in the shared anchor namespace (a repeat records a duplicate, exactly
/// as a heading collision does — PROP-035 §5). Fact NodeIds are minted here so
/// they interleave with heading nodes in document order.
#[allow(clippy::too_many_arguments)]
fn flush_block(
    lines: &[String],
    start: usize,
    end: usize,
    section: NodeId,
    nodes: &mut Vec<Node>,
    anchors: &mut HashMap<String, NodeId>,
    duplicate_anchors: &mut Vec<String>,
) {
    for seg in crate::facts::segment_block(lines, start, end) {
        let id = NodeId(nodes.len());
        nodes.push(Node {
            id: Some(seg.id.clone()),
            level: 0,
            kind: NodeKind::Fact,
            heading: String::new(),
            trailing: String::new(),
            heading_line: start + seg.start,
            span: (start + seg.start)..(start + seg.end),
            parent: Some(section),
            children: Vec::new(),
        });
        nodes[section.0].children.push(id);
        match anchors.entry(seg.id) {
            Entry::Vacant(slot) => {
                slot.insert(id);
            }
            Entry::Occupied(slot) => duplicate_anchors.push(slot.key().clone()),
        }
    }
}

/// A precomputed mask marking lines inside fenced code blocks (```` ``` ```` or
/// `~~~`), including the fence lines themselves. Headings on masked lines are
/// not tree nodes — a `#` in a code sample is not a section. Shared with the
/// directive scanner, which ignores directives in fenced code the same way.
pub(crate) fn fence_mask(lines: &[String]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    let mut fence: Option<(char, usize)> = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        match fence {
            Some((ch, open)) => {
                mask[i] = true;
                if closes_fence(trimmed, ch, open) {
                    fence = None;
                }
            }
            None => {
                if let Some(open) = fence_run(trimmed) {
                    fence = Some(open);
                    mask[i] = true;
                }
            }
        }
    }
    mask
}

/// The fence run a line opens with — its character and how many of it.
///
/// Third site of one defect: a fence delimiter matched by *prefix* instead
/// of by *run*. `progress-core`'s block scanner and the batch-review tool
/// carried it too and were fixed together (campaign finding F-102); this
/// crate is a separate consumer of the same Markdown, so it carries its own
/// copy of the rule rather than a dependency edge.
fn fence_run(trimmed: &str) -> Option<(char, usize)> {
    let ch = trimmed.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|&c| c == ch).count();
    (len >= 3).then_some((ch, len))
}

/// Whether `trimmed` closes a fence of character `ch` opened at run length
/// `open`: same character, a run at least as long, **and nothing else on the
/// line**.
///
/// That last clause is a second bug this function had on its own: it closed
/// on any line merely starting with the delimiter, so an info-string line
/// like `` ```rust `` inside a block ended it early and the code after it
/// was scanned as prose. Demonstrated before it was fixed — the quoted
/// heading below such a line became a real node in the tree.
fn closes_fence(trimmed: &str, ch: char, open: usize) -> bool {
    fence_run(trimmed).is_some_and(|(c, n)| c == ch && n >= open)
        && trimmed.trim_end().chars().all(|c| c == ch)
}

/// Parse an ATX heading line into `(level, heading_text, anchor, trailing)`.
/// Requires a space after the `#`s (so `#nospace` is not a heading), matching
/// the vendored engine's rule.
fn parse_heading(line: &str) -> Option<(u8, String, Option<String>, String)> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    let (heading, anchor, trailing) = split_anchor(rest.trim());
    Some((hashes as u8, heading, anchor, trailing))
}

/// Split a `{#anchor}` out of heading text, returning the text before it, the
/// anchor, and any trailing text after the closing `}` (e.g. a `:add` /
/// `:replace` merge marker). The anchor need not sit at the end of the line.
fn split_anchor(text: &str) -> (String, Option<String>, String) {
    if let Some(open) = text.find("{#")
        && let Some(close_rel) = text[open + 2..].find('}')
    {
        let close = open + 2 + close_rel;
        let anchor = text[open + 2..close].to_string();
        let heading = text[..open].trim_end().to_string();
        let trailing = text[close + 1..].trim().to_string();
        return (heading, Some(anchor), trailing);
    }
    (text.to_string(), None, String::new())
}

#[cfg(test)]
mod tests;
