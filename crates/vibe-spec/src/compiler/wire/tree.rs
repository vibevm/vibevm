//! Document-tree conversion: the node arena, anchor index, directive scan,
//! and the ARENA BOUNDS / FOREST / SPAN BOUNDS / anchor-coherence gates that
//! must hold before any arena index is followed.

use std::collections::HashMap;

use crate::directives::{Directive, DirectiveError, DirectiveKind, Directives, InPlaceUse};
use crate::doctree::{DocTree, Node, NodeId, NodeKind};

use super::super::ir::DocumentIr;
use super::address::{decode_spec_address, encode_spec_address};
use super::bounded::{display, preview};
use super::{
    G_ANCHOR_COHERENCE, G_ARENA_BOUNDS, G_FOREST, G_SPAN_BOUNDS, IrWireError, construction, gate,
    narrow, wire,
};

pub(super) fn decode_document_ir(value: &wire::DocumentIr) -> Result<DocumentIr, IrWireError> {
    let source = super::address::decode_source_doc(&value.source)?;
    let tree = decode_doc_tree(&value.tree)?;
    Ok(DocumentIr::new(source, tree))
}

pub(super) fn decode_doc_tree(value: &wire::DocTree) -> Result<DocTree, IrWireError> {
    check_arena_bounds(value)?;
    check_forest(value)?;
    check_span_bounds(value)?;
    check_anchor_coherence(value)?;
    let mut nodes = Vec::with_capacity(value.nodes.len());
    for node in &value.nodes {
        nodes.push(decode_node(node)?);
    }
    let mut anchors = HashMap::with_capacity(value.anchors.len());
    for (anchor, index) in &value.anchors {
        anchors.insert(anchor.clone(), NodeId::new(narrow("anchor index", *index)?));
    }
    let directives = decode_directives(&value.directives)?;
    DocTree::from_parts(
        nodes,
        anchors,
        value.duplicate_anchors.clone(),
        value.lines.clone(),
        directives,
    )
    .map_err(|source| {
        construction(format!(
            "the document tree law refused it: {}",
            display(source)
        ))
    })
}

/// ARENA BOUNDS: every parent, child, and anchor index is inside the arena
/// before anything indexes it.
pub(super) fn check_arena_bounds(value: &wire::DocTree) -> Result<(), IrWireError> {
    let len = value.nodes.len();
    for (index, node) in value.nodes.iter().enumerate() {
        if let Some(parent) = node.parent
            && parent as usize >= len
        {
            return Err(gate(
                G_ARENA_BOUNDS,
                format!("node {index} names parent {parent} outside the arena of {len}"),
            ));
        }
        for child in &node.children {
            if *child as usize >= len {
                return Err(gate(
                    G_ARENA_BOUNDS,
                    format!("node {index} names child {child} outside the arena of {len}"),
                ));
            }
        }
    }
    for (anchor, index) in &value.anchors {
        if *index as usize >= len {
            return Err(gate(
                G_ARENA_BOUNDS,
                format!(
                    "anchor ({}) names node {index} outside the arena of {len}",
                    preview(anchor)
                ),
            ));
        }
    }
    Ok(())
}

/// FOREST, total and iterative with a step bound, before any descent: the
/// arena is non-empty, node 0 is the synthetic root, every non-root node has
/// exactly one incoming child edge and claims that parent back, and the root
/// reaches every node. A `children` cycle is reported, never followed —
/// `DocTree::facts_under` would loop forever on one.
pub(super) fn check_forest(value: &wire::DocTree) -> Result<(), IrWireError> {
    let nodes = &value.nodes;
    if nodes.is_empty() {
        return Err(gate(
            G_FOREST,
            "the node arena is empty; every tree carries its synthetic root",
        ));
    }
    let root = &nodes[0];
    if root.parent.is_some() || root.level != 0 || root.kind != wire::DocNodeKind::Heading {
        return Err(gate(
            G_FOREST,
            "node 0 is not the synthetic level-0 heading root",
        ));
    }
    let mut incoming = vec![0usize; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        for child in &node.children {
            let child = *child as usize;
            incoming[child] += 1;
            if nodes[child].parent != Some(index as u32) {
                return Err(gate(
                    G_FOREST,
                    format!("child {child} does not claim parent {index} back"),
                ));
            }
        }
    }
    for (index, count) in incoming.into_iter().enumerate() {
        let expected = usize::from(index > 0);
        if count != expected {
            return Err(gate(
                G_FOREST,
                format!("node {index} has {count} incoming child edges, expected {expected}"),
            ));
        }
    }
    let mut seen = vec![false; nodes.len()];
    let mut stack = vec![0usize];
    let mut steps = 0usize;
    while let Some(index) = stack.pop() {
        steps += 1;
        if steps > nodes.len() + 1 {
            return Err(gate(
                G_FOREST,
                "the forest walk did not terminate; the arena carries a cycle",
            ));
        }
        if seen[index] {
            return Err(gate(G_FOREST, format!("child cycle at node {index}")));
        }
        seen[index] = true;
        for child in &nodes[index].children {
            stack.push(*child as usize);
        }
    }
    if let Some(index) = seen.iter().position(|reached| !reached) {
        return Err(gate(
            G_FOREST,
            format!("node {index} is unreachable from the synthetic root"),
        ));
    }
    Ok(())
}

/// SPAN BOUNDS before slicing: spans stay inside the line vector, and every
/// non-root node of a non-empty document has an in-range heading line.
pub(super) fn check_span_bounds(value: &wire::DocTree) -> Result<(), IrWireError> {
    let lines = value.lines.len();
    for (index, node) in value.nodes.iter().enumerate() {
        let (start, end) = (node.span.start, node.span.end);
        if start > end || end as usize > lines {
            return Err(gate(
                G_SPAN_BOUNDS,
                format!("node {index} span {start}..{end} leaves the document's {lines} lines"),
            ));
        }
        if index > 0 && (lines == 0 || node.heading_line as usize >= lines) {
            return Err(gate(
                G_SPAN_BOUNDS,
                format!(
                    "node {index} heading line {} is outside the document's {lines} lines",
                    node.heading_line
                ),
            ));
        }
    }
    Ok(())
}

/// Anchor coherence: `anchors[a]` names a node whose `id` is `a`, and every
/// recorded duplicate is an anchor the arena really repeats. The exact derived
/// first-occurrence spelling is `DocTree::from_parts`'s own law.
pub(super) fn check_anchor_coherence(value: &wire::DocTree) -> Result<(), IrWireError> {
    for (anchor, index) in &value.anchors {
        if value.nodes[*index as usize].id.as_deref() != Some(anchor.as_str()) {
            return Err(gate(
                G_ANCHOR_COHERENCE,
                format!(
                    "anchor ({}) names a node that does not carry that id",
                    preview(anchor)
                ),
            ));
        }
    }
    let repeats: HashMap<&str, usize> = {
        let mut counts = HashMap::new();
        for node in &value.nodes {
            if let Some(id) = node.id.as_deref() {
                *counts.entry(id).or_default() += 1;
            }
        }
        counts
    };
    for duplicate in &value.duplicate_anchors {
        if repeats.get(duplicate.as_str()).copied().unwrap_or(0) < 2 {
            return Err(gate(
                G_ANCHOR_COHERENCE,
                format!(
                    "duplicate-anchor record ({}) is not an anchor that repeats",
                    preview(duplicate)
                ),
            ));
        }
    }
    Ok(())
}

fn decode_node(value: &wire::DocNode) -> Result<Node, IrWireError> {
    Ok(Node {
        id: value.id.clone(),
        level: value.level,
        kind: match value.kind {
            wire::DocNodeKind::Heading => NodeKind::Heading,
            wire::DocNodeKind::Fact => NodeKind::Fact,
        },
        heading: value.heading.clone(),
        trailing: value.trailing.clone(),
        heading_line: narrow("heading line", value.heading_line)?,
        span: narrow("span start", value.span.start)?..narrow("span end", value.span.end)?,
        parent: value
            .parent
            .map(|parent| narrow("parent index", parent))
            .transpose()?
            .map(NodeId::new),
        children: value
            .children
            .iter()
            .map(|child| narrow("child index", *child))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(NodeId::new)
            .collect(),
    })
}

fn decode_directives(value: &wire::Directives) -> Result<Directives, IrWireError> {
    let mut directives = Vec::with_capacity(value.directives.len());
    for directive in &value.directives {
        directives.push(Directive {
            kind: match directive.kind {
                wire::DirectiveKind::Embed => DirectiveKind::Embed,
                wire::DirectiveKind::Use => DirectiveKind::Use,
                wire::DirectiveKind::Source => DirectiveKind::Source,
            },
            options: directive.options.clone(),
            address: decode_spec_address(&directive.address)?,
            line: narrow("directive line", directive.line)?,
        });
    }
    let mut in_place_uses = Vec::with_capacity(value.in_place_uses.len());
    for entry in &value.in_place_uses {
        in_place_uses.push(InPlaceUse {
            address: decode_spec_address(&entry.address)?,
            line: narrow("in-place use line", entry.line)?,
        });
    }
    let mut errors = Vec::with_capacity(value.errors.len());
    for error in &value.errors {
        errors.push(DirectiveError {
            line: narrow("directive error line", error.line)?,
            message: error.message.clone(),
        });
    }
    let mut aliases = std::collections::BTreeMap::new();
    for (name, address) in &value.aliases {
        aliases.insert(name.clone(), decode_spec_address(address)?);
    }
    Ok(Directives {
        directives,
        in_place_uses,
        errors,
        aliases,
    })
}

pub(super) fn encode_document_ir(value: &DocumentIr) -> Result<wire::DocumentIr, IrWireError> {
    Ok(wire::DocumentIr {
        source: super::address::encode_source_doc(value.source()),
        tree: encode_doc_tree(value.tree())?,
    })
}

pub(super) fn encode_doc_tree(value: &DocTree) -> Result<wire::DocTree, IrWireError> {
    let (nodes, anchors, duplicates, lines, directives) = value.parts();
    let mut wire_nodes = Vec::with_capacity(nodes.len());
    for node in nodes {
        wire_nodes.push(encode_node(node)?);
    }
    let mut wire_anchors = std::collections::BTreeMap::new();
    for (anchor, index) in anchors {
        wire_anchors.insert(anchor.clone(), super::widen("anchor index", index.index())?);
    }
    Ok(wire::DocTree {
        nodes: wire_nodes,
        anchors: wire_anchors,
        duplicate_anchors: duplicates.to_vec(),
        lines: lines.to_vec(),
        directives: encode_directives(directives)?,
    })
}

fn encode_node(value: &Node) -> Result<wire::DocNode, IrWireError> {
    Ok(wire::DocNode {
        id: value.id.clone(),
        level: value.level,
        kind: match value.kind {
            NodeKind::Heading => wire::DocNodeKind::Heading,
            NodeKind::Fact => wire::DocNodeKind::Fact,
        },
        heading: value.heading.clone(),
        trailing: value.trailing.clone(),
        heading_line: super::widen("heading line", value.heading_line)?,
        span: wire::Span {
            start: super::widen("span start", value.span.start)?,
            end: super::widen("span end", value.span.end)?,
        },
        parent: value
            .parent
            .map(|parent| super::widen("parent index", parent.index()))
            .transpose()?,
        children: value
            .children
            .iter()
            .map(|child| super::widen("child index", child.index()))
            .collect::<Result<_, _>>()?,
    })
}

fn encode_directives(value: &Directives) -> Result<wire::Directives, IrWireError> {
    let mut directives = Vec::with_capacity(value.directives.len());
    for directive in &value.directives {
        directives.push(wire::Directive {
            kind: match directive.kind {
                DirectiveKind::Embed => wire::DirectiveKind::Embed,
                DirectiveKind::Use => wire::DirectiveKind::Use,
                DirectiveKind::Source => wire::DirectiveKind::Source,
            },
            options: directive.options.clone(),
            address: encode_spec_address(&directive.address),
            line: super::widen("directive line", directive.line)?,
        });
    }
    let mut in_place_uses = Vec::with_capacity(value.in_place_uses.len());
    for entry in &value.in_place_uses {
        in_place_uses.push(wire::InPlaceUse {
            address: encode_spec_address(&entry.address),
            line: super::widen("in-place use line", entry.line)?,
        });
    }
    let mut errors = Vec::with_capacity(value.errors.len());
    for error in &value.errors {
        errors.push(wire::DirectiveError {
            line: super::widen("directive error line", error.line)?,
            message: error.message.clone(),
        });
    }
    let mut aliases = std::collections::BTreeMap::new();
    for (name, address) in &value.aliases {
        aliases.insert(name.clone(), encode_spec_address(address));
    }
    Ok(wire::Directives {
        directives,
        in_place_uses,
        errors,
        aliases,
    })
}
