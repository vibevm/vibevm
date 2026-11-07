//! Inverse annotation placement through the incumbent semantic reader/writer.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-GENERATED-XML-PROJECTION",
    r = 1
);
use crate::md_out::positions::{Bounds, Output, TextSite};
use crate::xml_annotations::{RegionRole, SemanticLayout};
use crate::{Error, Result, SemanticPosition};

/// Whether a generated comment belongs in scalar text or at a Markdown block gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCommentPlacement {
    Inline,
    Block,
}

/// A generated event at a checked byte gap of the plain Markdown projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownComment {
    pub byte_offset: usize,
    pub payload: String,
    pub placement: MarkdownCommentPlacement,
}

/// Canonical Markdown and separately observed actual XML comment events.
/// The caller renders events only after applying its reserved-literal policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedMarkdown {
    plain: String,
    comments: Vec<MarkdownComment>,
}
impl AnnotatedMarkdown {
    /// Consume the projection and its ordered annotation placements.
    pub fn into_parts(self) -> (String, Vec<MarkdownComment>) {
        (self.plain, self.comments)
    }
}

/// Project actual canonical XML comments through the existing semantic reader
/// and Markdown writer. Escaped text and CDATA cannot supply comment events.
/// Ordinary comment layout follows the existing pivot law.
/// ```
/// let xml = "<spec xmlns='https://vibevm.org/spec/1'><p>a<!-- vibe:c1 embed: x -->b</p></spec>";
/// let (plain, comments) = vibe_specdoc::project_annotated_xml_to_markdown(xml)?.into_parts();
/// assert_eq!(plain, "ab\n\n");
/// assert_eq!(comments[0].byte_offset, 1);
/// # Ok::<(), vibe_specdoc::Error>(())
/// ```
pub fn project_annotated_xml_to_markdown(xml: &str) -> Result<AnnotatedMarkdown> {
    let (document, events, layout) = crate::xml_in::parse_observed_mapped(xml)?;
    if events.is_empty() {
        return Ok(AnnotatedMarkdown {
            plain: crate::to_markdown(&document),
            comments: Vec::new(),
        });
    }
    let output = crate::md_out::render(&document, true);
    let nodes = mapped_nodes(&layout, &output)?;
    let mut comments = Vec::with_capacity(events.len());
    for event in events {
        let (byte_offset, placement) = match event.position {
            SemanticPosition::BeforeDocument => (0, MarkdownCommentPlacement::Block),
            SemanticPosition::AfterDocument => (output.text.len(), MarkdownCommentPlacement::Block),
            SemanticPosition::Text { node, offset } => {
                let site = nodes
                    .get(node)
                    .ok_or_else(|| error("unknown semantic text node"))?;
                if site.text.is_some_and(|text| text.fenced) {
                    return Err(error(
                        "an XML comment inside fence text has no metadata-preserving Markdown placement",
                    ));
                }
                (
                    *site.scalars.get(offset).ok_or_else(|| {
                        error("semantic scalar offset is outside the projected text")
                    })?,
                    MarkdownCommentPlacement::Inline,
                )
            }
            SemanticPosition::ChildGap { node, child } => child_gap(&layout, &nodes, node, child)?,
        };
        if !output.text.is_char_boundary(byte_offset) {
            return Err(error("comment position is not a projected UTF-8 boundary"));
        }
        if comments
            .last()
            .is_some_and(|previous: &MarkdownComment| previous.byte_offset > byte_offset)
        {
            return Err(error(
                "semantic comment order is not representable in canonical Markdown",
            ));
        }
        comments.push(MarkdownComment {
            byte_offset,
            payload: event.payload,
            placement,
        });
    }
    Ok(AnnotatedMarkdown {
        plain: output.text,
        comments,
    })
}

struct Node<'a> {
    bounds: Bounds,
    content_start: usize,
    text: Option<&'a TextSite>,
    scalars: Vec<usize>,
}

fn mapped_nodes<'a>(layout: &SemanticLayout, output: &'a Output) -> Result<Vec<Node<'a>>> {
    let (mut texts, mut headings, mut blocks, mut rows, mut statuses) = (
        output.texts.iter(),
        output.headings.iter(),
        output.blocks.iter(),
        output.rows.iter(),
        output.statuses.iter(),
    );
    let mut nodes = Vec::with_capacity(layout.regions.len());
    for region in &layout.regions {
        let mut node = Node {
            bounds: Bounds {
                before: 0,
                after: 0,
            },
            content_start: 0,
            text: None,
            scalars: Vec::new(),
        };
        match region.role {
            RegionRole::Root => node.bounds.after = output.text.len(),
            RegionRole::Heading | RegionRole::Section => {
                let site = headings
                    .next()
                    .ok_or_else(|| error("heading inventories differ"))?;
                node.bounds = Bounds {
                    before: site.before,
                    after: site.after,
                };
                node.content_start = site.after;
                node.scalars = scalar_positions(&site.value, &site.bytes)?;
            }
            RegionRole::Text => {
                let site = texts
                    .next()
                    .ok_or_else(|| error("text inventories differ"))?;
                node.bounds = Bounds {
                    before: site.before,
                    after: site.after,
                };
                node.text = Some(site);
                node.scalars = scalar_positions(&site.value, &site.bytes)?;
            }
            RegionRole::Block => {
                node.bounds = *blocks
                    .next()
                    .ok_or_else(|| error("block inventories differ"))?
            }
            RegionRole::Row => {
                node.bounds = *rows
                    .next()
                    .ok_or_else(|| error("table-row inventories differ"))?
            }
            RegionRole::Status => {
                node.bounds = *statuses
                    .next()
                    .ok_or_else(|| error("status inventories differ"))?
            }
        }
        nodes.push(node);
    }
    if texts.next().is_some()
        || headings.next().is_some()
        || blocks.next().is_some()
        || rows.next().is_some()
        || statuses.next().is_some()
    {
        return Err(error("reader/writer semantic inventories differ"));
    }
    for (index, region) in layout.regions.iter().enumerate().rev() {
        if matches!(region.role, RegionRole::Section)
            && let Some(last) = region.children.last()
        {
            nodes[index].bounds.after = nodes[*last].bounds.after;
        }
    }
    Ok(nodes)
}

fn scalar_positions(value: &str, bytes: &[usize]) -> Result<Vec<usize>> {
    value
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
        .map(|index| {
            bytes
                .get(index)
                .copied()
                .ok_or_else(|| error("writer scalar map is incomplete"))
        })
        .collect()
}

fn child_gap(
    layout: &SemanticLayout,
    nodes: &[Node<'_>],
    node: usize,
    child: usize,
) -> Result<(usize, MarkdownCommentPlacement)> {
    let region = layout
        .regions
        .get(node)
        .ok_or_else(|| error("unknown semantic container"))?;
    let mapped = nodes
        .get(node)
        .ok_or_else(|| error("unmapped semantic container"))?;
    if child > region.children.len() {
        return Err(error("semantic child gap is outside its container"));
    }
    let inline = MarkdownCommentPlacement::Inline;
    let block = MarkdownCommentPlacement::Block;
    match region.role {
        RegionRole::Root | RegionRole::Section => {
            if region.children.is_empty() {
                return Ok((mapped.content_start, block));
            }
            if let Some(next) = region.children.get(child) {
                Ok((nodes[*next].bounds.before, block))
            } else {
                Ok((mapped.bounds.after, block))
            }
        }
        RegionRole::Block => {
            if child == 0 {
                return Ok((mapped.bounds.before, block));
            }
            if child == region.children.len() {
                return Ok((mapped.bounds.after, block));
            }
            let next = region.children[child];
            if matches!(layout.regions[next].role, RegionRole::Row) {
                return Err(error(
                    "a comment between table rows cannot split the canonical Markdown table",
                ));
            }
            Ok((nodes[next].bounds.before, inline))
        }
        RegionRole::Row => {
            if let Some(next) = region.children.get(child) {
                Ok((nodes[*next].bounds.before, inline))
            } else if let Some(last) = region.children.last() {
                Ok((nodes[*last].bounds.after, inline))
            } else {
                Err(error("empty table row has no comment placement"))
            }
        }
        _ => Err(error(
            "container comment has no canonical Markdown placement",
        )),
    }
}

fn error(reason: &str) -> Error {
    Error::at(
        0,
        format!(
            "{reason}; violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-GENERATED-XML-PROJECTION-LAW; fix surface: preserve raw XML when semantic comment placement is unrepresentable"
        ),
    )
}
