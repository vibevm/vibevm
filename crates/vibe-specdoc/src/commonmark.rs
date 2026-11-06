//! Ordinary CommonMark outline over exact supplied UTF-8 bytes.
//!
//! This is data grammar, independent of the normative Vibe Markdown frontend.
//! Quote/list/item containers bound local sections; they are transparent to
//! heading-path selection. A raw selected slice need not be standalone Markdown.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#ORDINARY-COMMONMARK-PARSER");
use std::ops::Range;
mod parse;
mod selection;
pub use selection::{HeadingSelectionError, SectionSelection};

/// A document-local position, never a durable identity or ownership token.
/// A position from another document can be in bounds here; lookup cannot prove
/// common origin. Hosts must pair external references with their source witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);
impl NodeId {
    /// Construct a positional query; use the document's checked `node` lookup.
    pub fn from_index(index: usize) -> Self {
        Self(index)
    }
    pub fn index(self) -> usize {
        self.0
    }
}
/// Container boundaries relevant to section extent; not a complete HTML AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Quote,
    List,
    Item,
}
/// Ordinary syntax only. There is deliberately no fact, directive or trust node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Container(ContainerKind),
    Section {
        level: u8,
        title: String,
        heading_span: Range<usize>,
    },
    /// Fenced or indented code; an indented block has empty info.
    Fence {
        info: String,
    },
    Link {
        destination: String,
        label: String,
    },
    Image {
        destination: String,
        label: String,
    },
    Text(String),
    InlineCode(String),
}
/// Immutable through the document API. Spans index original UTF-8 source bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub span: Range<usize>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}
/// An owned snapshot of one ordinary document and its bounded outline.
///
/// ```
/// use vibe_specdoc::commonmark::{CommonMarkDoc, NodeId};
/// let source = "# Guide\r\n## Café `code`\r\n@fact:literal\r\n";
/// let doc = CommonMarkDoc::parse(source);
/// assert_eq!(doc.source(), source);
/// let section = doc.resolve_headings(&["Guide".into(), "Café code".into()]).unwrap();
/// assert_eq!(section.raw(), "## Café `code`\r\n@fact:literal\r\n");
/// assert!(std::ptr::eq(section.document(), &doc));
/// assert_eq!(doc.node(section.node().id), Some(section.node()));
/// assert!(doc.node(NodeId::from_index(usize::MAX)).is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonMarkDoc {
    source: String,
    nodes: Vec<Node>,
}
impl CommonMarkDoc {
    /// Parse standard CommonMark plus tables only. No HTML rendering, URL
    /// callbacks, filesystem discovery or native Vibe interpretation occurs.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-045#ORDINARY-COMMONMARK-PARSER",
        r = 1
    )]
    pub fn parse(source: &str) -> Self {
        parse::parse(source)
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }
    /// Bounds checking only, not cross-document identity validation.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }
    pub fn children(&self, id: NodeId) -> Option<&[NodeId]> {
        self.node(id).map(|node| node.children.as_slice())
    }
}
fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
#[cfg(test)]
mod tests;
