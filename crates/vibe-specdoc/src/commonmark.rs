//! Ordinary CommonMark structure selected by an explicit source descriptor.
//!
//! This parser never enters the Vibe `SpecDoc`, fact or directive readers.
//! IDs are ordinal within one captured snapshot; headings are not durable
//! fact addresses, and duplicate text is ambiguous until an author chooses
//! a unique export.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DESCRIPTOR-GRAMMAR");

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

mod export;
pub use export::{ExportResolutionError, ResolvedExport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Section {
        level: u8,
        title: String,
        /// Exact byte range of the heading itself; the node span includes its
        /// following section until the next equal-or-higher heading.
        heading_span: Range<usize>,
    },
    Fence {
        info: String,
    },
    Link {
        destination: String,
        label: String,
    },
    Text(String),
    InlineCode(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub span: Range<usize>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadingLookupError {
    Missing(String),
    Ambiguous { title: String, matches: Vec<NodeId> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonMarkDoc {
    source: String,
    nodes: Vec<Node>,
}

impl CommonMarkDoc {
    /// Parse one captured text snapshot. GFM tables are enabled only so their
    /// inline code/text stays structurally quoted; no table or Vibe facts are
    /// inferred from that extension.
    pub fn parse(source: &str) -> Self {
        let mut nodes = vec![Node {
            id: NodeId(0),
            kind: NodeKind::Document,
            span: 0..source.len(),
            parent: None,
            children: Vec::new(),
        }];
        let mut sections: Vec<NodeId> = vec![NodeId(0)];
        let mut containers: Vec<NodeId> = Vec::new();
        let mut heading: Option<NodeId> = None;
        let mut links: Vec<NodeId> = Vec::new();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        for (event, span) in Parser::new_ext(source, options).into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let level = level as u8;
                    while sections.len() > 1 {
                        let previous = *sections.last().unwrap();
                        let NodeKind::Section { level: prior, .. } = &nodes[previous.0].kind else {
                            unreachable!()
                        };
                        if *prior < level {
                            break;
                        }
                        nodes[previous.0].span.end = span.start;
                        sections.pop();
                    }
                    let parent = *sections.last().unwrap();
                    let id = push(
                        &mut nodes,
                        parent,
                        NodeKind::Section {
                            level,
                            title: String::new(),
                            heading_span: span.clone(),
                        },
                        span.start..source.len(),
                    );
                    sections.push(id);
                    containers.push(id);
                    heading = Some(id);
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(id) = heading.take() {
                        if let NodeKind::Section {
                            title,
                            heading_span,
                            ..
                        } = &mut nodes[id.0].kind
                        {
                            *title = normalized(title);
                            heading_span.end = span.end;
                        }
                        let popped = containers.pop();
                        debug_assert_eq!(popped, Some(id));
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let info = match kind {
                        CodeBlockKind::Indented => String::new(),
                        CodeBlockKind::Fenced(info) => info.into_string(),
                    };
                    let parent = containers
                        .last()
                        .copied()
                        .unwrap_or(*sections.last().unwrap());
                    let id = push(&mut nodes, parent, NodeKind::Fence { info }, span.clone());
                    containers.push(id);
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(id) = containers.pop() {
                        nodes[id.0].span.end = span.end;
                    }
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    let parent = containers
                        .last()
                        .copied()
                        .unwrap_or(*sections.last().unwrap());
                    let id = push(
                        &mut nodes,
                        parent,
                        NodeKind::Link {
                            destination: dest_url.into_string(),
                            label: String::new(),
                        },
                        span.clone(),
                    );
                    containers.push(id);
                    links.push(id);
                }
                Event::End(TagEnd::Link) => {
                    if let Some(id) = links.pop() {
                        if let NodeKind::Link { label, .. } = &mut nodes[id.0].kind {
                            *label = normalized(label);
                        }
                        nodes[id.0].span.end = span.end;
                        let popped = containers.pop();
                        debug_assert_eq!(popped, Some(id));
                    }
                }
                Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                    add_text(
                        &mut nodes,
                        &containers,
                        &sections,
                        heading,
                        links.last().copied(),
                        text.as_ref(),
                        span,
                        false,
                    );
                }
                Event::Code(text) => {
                    add_text(
                        &mut nodes,
                        &containers,
                        &sections,
                        heading,
                        links.last().copied(),
                        text.as_ref(),
                        span,
                        true,
                    );
                }
                Event::SoftBreak | Event::HardBreak => {
                    add_text(
                        &mut nodes,
                        &containers,
                        &sections,
                        heading,
                        links.last().copied(),
                        " ",
                        span,
                        false,
                    );
                }
                _ => {}
            }
        }
        for id in sections.into_iter().skip(1) {
            nodes[id.0].span.end = source.len();
        }
        Self {
            source: source.to_owned(),
            nodes,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id.0].children
    }

    /// Exact normalized heading text in this one snapshot. Never chooses the
    /// first duplicate as a durable export.
    pub fn heading(&self, title: &str) -> Result<NodeId, HeadingLookupError> {
        let matches = self
            .nodes
            .iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Section { title: found, .. } if found == title => Some(node.id),
                _ => None,
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(HeadingLookupError::Missing(title.to_owned())),
            [one] => Ok(*one),
            _ => Err(HeadingLookupError::Ambiguous {
                title: title.to_owned(),
                matches,
            }),
        }
    }
}

fn push(nodes: &mut Vec<Node>, parent: NodeId, kind: NodeKind, span: Range<usize>) -> NodeId {
    let id = NodeId(nodes.len());
    nodes.push(Node {
        id,
        kind,
        span,
        parent: Some(parent),
        children: Vec::new(),
    });
    nodes[parent.0].children.push(id);
    id
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn add_text(
    nodes: &mut Vec<Node>,
    containers: &[NodeId],
    sections: &[NodeId],
    heading: Option<NodeId>,
    link: Option<NodeId>,
    text: &str,
    span: Range<usize>,
    code: bool,
) {
    if let Some(id) = heading {
        if let NodeKind::Section { title, .. } = &mut nodes[id.0].kind {
            title.push_str(text);
        }
    }
    if let Some(id) = link {
        if let NodeKind::Link { label, .. } = &mut nodes[id.0].kind {
            label.push_str(text);
        }
    }
    let parent = containers
        .last()
        .copied()
        .unwrap_or(*sections.last().unwrap());
    let kind = if code {
        NodeKind::InlineCode(text.to_owned())
    } else {
        NodeKind::Text(text.to_owned())
    };
    push(nodes, parent, kind, span);
}

#[cfg(test)]
#[path = "commonmark/tests.rs"]
mod tests;
