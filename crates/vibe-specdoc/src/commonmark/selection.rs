//! Exact heading-path selection; export names and source witnesses belong to hosts.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#ORDINARY-COMMONMARK-PARSER");
use super::*;
/// Typed selector failure; the document/raw source remains available on every error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadingSelectionError {
    MalformedSelector,
    Missing { index: usize, title: String },
    Ambiguous { index: usize, title: String },
    InvalidSpan,
}
impl std::fmt::Display for HeadingSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-045#ORDINARY-COMMONMARK-PARSER: ordinary heading selection {self:?}; fix surface: exact normalized heading path and captured document"
        )
    }
}
impl std::error::Error for HeadingSelectionError {}
/// Borrowed document, node and exact raw slice. This is not a durable export ID.
#[derive(Debug, Clone, Copy)]
pub struct SectionSelection<'a> {
    document: &'a CommonMarkDoc,
    node: &'a Node,
    raw: &'a str,
}
impl<'a> SectionSelection<'a> {
    pub fn document(&self) -> &'a CommonMarkDoc {
        self.document
    }
    pub fn node(&self) -> &'a Node {
        self.node
    }
    pub fn span(&self) -> Range<usize> {
        self.node.span.clone()
    }
    pub fn raw(&self) -> &'a str {
        self.raw
    }
}
impl CommonMarkDoc {
    /// Resolve exact root-to-section normalized visible titles. Quote/list/item
    /// wrappers are transparent; matching never descends through another section.
    /// Duplicate siblings refuse, including siblings in separate containers.
    ///
    /// ```
    /// use vibe_specdoc::commonmark::{CommonMarkDoc, HeadingSelectionError};
    /// let doc = CommonMarkDoc::parse("# A\n## Same\nfirst\n# B\n## Same\nsecond\n");
    /// assert_eq!(doc.resolve_headings(&["B".into(), "Same".into()]).unwrap().raw(), "## Same\nsecond\n");
    /// assert_eq!(doc.resolve_headings(&[]).unwrap_err(), HeadingSelectionError::MalformedSelector);
    /// ```
    pub fn resolve_headings(
        &self,
        headings: &[String],
    ) -> Result<SectionSelection<'_>, HeadingSelectionError> {
        if headings.is_empty()
            || headings.iter().any(|title| {
                title.is_empty()
                    || title.chars().any(char::is_control)
                    || normalized(title) != *title
            })
        {
            return Err(HeadingSelectionError::MalformedSelector);
        }
        let mut parent = self.root();
        for (index, title) in headings.iter().enumerate() {
            let matches = self.matching_children(parent, title);
            parent = match matches.as_slice() {
                [one] => *one,
                [] => {
                    return Err(HeadingSelectionError::Missing {
                        index,
                        title: title.clone(),
                    });
                }
                _ => {
                    return Err(HeadingSelectionError::Ambiguous {
                        index,
                        title: title.clone(),
                    });
                }
            };
        }
        let node = self
            .node(parent)
            .ok_or(HeadingSelectionError::InvalidSpan)?;
        let raw = self
            .source
            .get(node.span.clone())
            .ok_or(HeadingSelectionError::InvalidSpan)?;
        Ok(SectionSelection {
            document: self,
            node,
            raw,
        })
    }
    fn matching_children(&self, parent: NodeId, title: &str) -> Vec<NodeId> {
        let mut pending = self
            .children(parent)
            .unwrap_or(&[])
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>();
        let mut found = Vec::new();
        while let Some(id) = pending.pop() {
            if let Some(node) = self.node(id) {
                match &node.kind {
                    NodeKind::Section { title: actual, .. } if actual == title => found.push(id),
                    NodeKind::Container(_) => pending.extend(node.children.iter().rev().copied()),
                    _ => {}
                }
            }
        }
        found
    }
}
