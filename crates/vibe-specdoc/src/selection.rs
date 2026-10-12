//! Exact IR selection shared by captured queries and compiler embeds.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SHARED-SPECDOC-SELECTION");

use crate::doc::{Block, BlockNode, Section, SpecDoc, Unit, Vocabulary};

pub const SELECTION_RECIPE: &str = "specdoc-selection/1";
pub const MAX_SELECTION_NODES: usize = 16_384;
pub const MAX_SELECTION_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedNodeKind {
    Document,
    Section,
    Fact,
    DocumentationBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    InvalidPath(String),
    Unknown {
        requested: String,
        candidates: Vec<String>,
    },
    Ambiguous {
        requested: String,
        candidates: Vec<String>,
    },
    LimitExceeded(&'static str),
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(path) => write!(f, "invalid node path `{path}`"),
            Self::Unknown {
                requested,
                candidates,
            } => write!(f, "unknown node `{requested}`; candidates: {candidates:?}"),
            Self::Ambiguous {
                requested,
                candidates,
            } => write!(
                f,
                "ambiguous node `{requested}`; candidates: {candidates:?}"
            ),
            Self::LimitExceeded(boundary) => write!(f, "node selection limit exceeded: {boundary}"),
        }
    }
}
impl std::error::Error for SelectionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSelection {
    pub kind: SelectedNodeKind,
    pub path: Vec<String>,
    pub heading_chain: Vec<String>,
    pub ir: SpecDoc,
    pub selected_bytes: usize,
    pub document_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineNode {
    pub kind: SelectedNodeKind,
    pub path: Vec<String>,
    pub heading_chain: Vec<String>,
    pub bytes_with_children: usize,
    pub bytes_without_children: usize,
}

enum Value<'a> {
    Document(&'a SpecDoc),
    Section(&'a Section),
    Blocks(Vec<BlockNode>),
}

struct Entry<'a> {
    kind: SelectedNodeKind,
    path: Vec<String>,
    heading_chain: Vec<String>,
    value: Value<'a>,
}

impl Entry<'_> {
    fn fragment(&self, children: bool) -> SpecDoc {
        match &self.value {
            Value::Document(doc) => {
                let mut doc = (*doc).clone();
                if !children {
                    doc.sections.clear();
                }
                doc
            }
            Value::Section(section) => {
                let mut section = (*section).clone();
                if !children {
                    section.sections.clear();
                }
                SpecDoc {
                    sections: vec![section],
                    ..SpecDoc::default()
                }
            }
            Value::Blocks(blocks) => SpecDoc {
                preamble: blocks.clone(),
                ..SpecDoc::default()
            },
        }
    }
}

/// Select exactly one authored IR node. Dotted segments must be contiguous
/// anchored ancestry; a single anchor may start anywhere in the document.
/// Unanchored prose never gains a synthetic fact or a durable address.
pub fn select_node(
    doc: &SpecDoc,
    anchor: Option<&str>,
    children: bool,
    vocabulary: Vocabulary,
) -> Result<NodeSelection, SelectionError> {
    let entries = index(doc, vocabulary)?;
    let selected = match anchor {
        None => &entries[0],
        Some(requested) => {
            let parts: Vec<_> = requested.split('.').collect();
            let literal_exists = entries
                .iter()
                .any(|entry| entry.path.last().is_some_and(|id| id == requested));
            let path_valid = parts.iter().all(|part| valid_segment(part));
            if !path_valid && !literal_exists {
                return Err(SelectionError::InvalidPath(requested.to_owned()));
            }
            let matches: Vec<_> = entries
                .iter()
                .filter(|entry| {
                    entry.path.last().is_some_and(|id| id == requested)
                        || (path_valid
                            && entry.path.len() >= parts.len()
                            && entry.path[entry.path.len() - parts.len()..]
                                .iter()
                                .map(String::as_str)
                                .eq(parts.iter().copied()))
                })
                .collect();
            match matches.as_slice() {
                [one] => *one,
                [] => {
                    return Err(SelectionError::Unknown {
                        requested: requested.to_owned(),
                        candidates: candidates(entries.iter()),
                    });
                }
                _ => {
                    return Err(SelectionError::Ambiguous {
                        requested: requested.to_owned(),
                        candidates: candidates(matches.into_iter()),
                    });
                }
            }
        }
    };
    Ok(selection_result(doc, selected, children))
}

/// Literal byte-exact anchor lookup over the same index. This preserves
/// historical canonical hash meaning even when an authored heading ID itself
/// contains a dot or uses the frontend's wider heading grammar.
pub fn select_exact_anchor(
    doc: &SpecDoc,
    anchor: &str,
    children: bool,
    vocabulary: Vocabulary,
) -> Result<NodeSelection, SelectionError> {
    let entries = index(doc, vocabulary)?;
    let matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry.path.last().is_some_and(|id| id == anchor))
        .collect();
    match matches.as_slice() {
        [one] => Ok(selection_result(doc, one, children)),
        [] => Err(SelectionError::Unknown {
            requested: anchor.to_owned(),
            candidates: candidates(entries.iter()),
        }),
        _ => Err(SelectionError::Ambiguous {
            requested: anchor.to_owned(),
            candidates: candidates(matches.into_iter()),
        }),
    }
}

fn selection_result(doc: &SpecDoc, selected: &Entry<'_>, children: bool) -> NodeSelection {
    let ir = selected.fragment(children);
    NodeSelection {
        kind: selected.kind,
        path: selected.path.clone(),
        heading_chain: selected.heading_chain.clone(),
        selected_bytes: crate::to_markdown(&ir).len(),
        document_bytes: crate::to_markdown(doc).len(),
        ir,
    }
}

/// Pure outline over the same nodes selected above. Byte sizes describe the
/// named canonical Markdown representation, never native raw source spans.
pub fn outline(doc: &SpecDoc, vocabulary: Vocabulary) -> Result<Vec<OutlineNode>, SelectionError> {
    index(doc, vocabulary)?
        .iter()
        .map(|entry| {
            Ok(OutlineNode {
                kind: entry.kind,
                path: entry.path.clone(),
                heading_chain: entry.heading_chain.clone(),
                bytes_with_children: crate::to_markdown(&entry.fragment(true)).len(),
                bytes_without_children: crate::to_markdown(&entry.fragment(false)).len(),
            })
        })
        .collect()
}

fn candidates<'a, 'd: 'a>(entries: impl Iterator<Item = &'a Entry<'d>>) -> Vec<String> {
    let mut paths: Vec<_> = entries
        .filter(|entry| !entry.path.is_empty())
        .map(|entry| {
            if entry.path.iter().any(|part| part.contains('.')) {
                format!("{:?}", entry.path)
            } else {
                entry.path.join(".")
            }
        })
        .collect();
    paths.sort();
    paths
}

fn valid_segment(part: &str) -> bool {
    !part.is_empty() && part.trim() == part && !part.contains(['}', '\r', '\n'])
}

fn index(doc: &SpecDoc, vocabulary: Vocabulary) -> Result<Vec<Entry<'_>>, SelectionError> {
    let path = doc
        .title
        .as_ref()
        .and_then(|title| title.id.clone())
        .into_iter()
        .collect::<Vec<_>>();
    let chain = doc
        .title
        .as_ref()
        .map(|title| title.text.clone())
        .into_iter()
        .collect::<Vec<_>>();
    let mut entries = vec![Entry {
        kind: SelectedNodeKind::Document,
        path: path.clone(),
        heading_chain: chain.clone(),
        value: Value::Document(doc),
    }];
    blocks(&doc.preamble, &path, &chain, vocabulary, &mut entries)?;
    for child in &doc.sections {
        section(child, &path, &chain, vocabulary, 1, &mut entries)?;
    }
    Ok(entries)
}

fn push<'a>(entries: &mut Vec<Entry<'a>>, entry: Entry<'a>) -> Result<(), SelectionError> {
    if entries.len() >= MAX_SELECTION_NODES {
        return Err(SelectionError::LimitExceeded("nodes"));
    }
    entries.push(entry);
    Ok(())
}

fn section<'a>(
    value: &'a Section,
    parent_path: &[String],
    parent_chain: &[String],
    vocabulary: Vocabulary,
    depth: usize,
    entries: &mut Vec<Entry<'a>>,
) -> Result<(), SelectionError> {
    if depth > MAX_SELECTION_DEPTH {
        return Err(SelectionError::LimitExceeded("depth"));
    }
    let mut path = Vec::new();
    if let Some(id) = &value.id {
        path.extend_from_slice(parent_path);
        path.push(id.clone());
    }
    let mut chain = parent_chain.to_vec();
    chain.push(value.title.clone());
    push(
        entries,
        Entry {
            kind: SelectedNodeKind::Section,
            path: path.clone(),
            heading_chain: chain.clone(),
            value: Value::Section(value),
        },
    )?;
    blocks(&value.blocks, &path, &chain, vocabulary, entries)?;
    for child in &value.sections {
        section(child, &path, &chain, vocabulary, depth + 1, entries)?;
    }
    Ok(())
}

fn blocks<'a>(
    values: &'a [BlockNode],
    path: &[String],
    chain: &[String],
    vocabulary: Vocabulary,
    entries: &mut Vec<Entry<'a>>,
) -> Result<(), SelectionError> {
    for (index, node) in values.iter().enumerate() {
        let fence = values.get(index + 1);
        let mut add = |unit: &Unit, carrier: BlockNode| -> Result<(), SelectionError> {
            let Some(id) = unit.fact.as_ref().and_then(|fact| fact.id.as_ref()) else {
                return Ok(());
            };
            let mut body = vec![carrier];
            if let Some(next) = fence.filter(
                |next| matches!(&next.block, Block::Fence { fact: Some(bound), .. } if bound == id),
            ) {
                body.push(next.clone());
            }
            let mut node_path = path.to_vec();
            node_path.push(id.clone());
            push(
                entries,
                Entry {
                    kind: SelectedNodeKind::Fact,
                    path: node_path,
                    heading_chain: chain.to_vec(),
                    value: Value::Blocks(body),
                },
            )
        };
        match &node.block {
            Block::Paragraph(unit) | Block::Quote(unit) => add(unit, node.clone())?,
            Block::List { ordered, items } => {
                for unit in items {
                    add(
                        unit,
                        BlockNode {
                            when: node.when,
                            block: Block::List {
                                ordered: *ordered,
                                items: vec![unit.clone()],
                            },
                        },
                    )?;
                }
            }
            Block::Table { rows } => {
                for unit in rows.iter().flatten() {
                    add(
                        unit,
                        BlockNode {
                            when: node.when,
                            block: Block::Table {
                                rows: vec![vec![unit.clone()]],
                            },
                        },
                    )?;
                }
            }
            Block::Note { body, .. } => add(body, node.clone())?,
            Block::Figure { caption, .. } => add(caption, node.clone())?,
            Block::Example { id, .. } | Block::Prompt { id, .. }
                if vocabulary == Vocabulary::Doc =>
            {
                let mut node_path = path.to_vec();
                node_path.push(id.clone());
                push(
                    entries,
                    Entry {
                        kind: SelectedNodeKind::DocumentationBlock,
                        path: node_path,
                        heading_chain: chain.to_vec(),
                        value: Value::Blocks(vec![node.clone()]),
                    },
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod m_01_d_1_cases;
