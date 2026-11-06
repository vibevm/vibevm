//! Opt-in generated-comment observation over the incumbent XML reader.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-GENERATED-PROVENANCE"
);

use crate::doc::SpecDoc;
use crate::xml_in::{Ev, Parser};
use crate::{Error, Result};
mod comments;
pub use comments::{XmlCommentScanError, XmlCommentSpan, xml_comment_spans};

/// A position in the parsed semantic document, independent of XML spelling.
/// Node ordinals follow semantic preorder (containers and text leaves); offsets
/// count decoded Unicode scalars after the incumbent leaf normalization.
/// ```
/// use vibe_specdoc::SemanticPosition;
/// assert_ne!(SemanticPosition::BeforeDocument, SemanticPosition::AfterDocument);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticPosition {
    BeforeDocument,
    ChildGap { node: usize, child: usize },
    Text { node: usize, offset: usize },
    AfterDocument,
}

/// One actual canonical c1 comment, in source order. Equality excludes physical
/// line/byte coordinates so equivalent XML formatting does not move an event.
/// ```
/// use vibe_specdoc::{SemanticComment, SemanticPosition};
/// let event = SemanticComment { payload: "embed: spec://org.demo/p/a#x".into(),
///     position: SemanticPosition::BeforeDocument };
/// assert!(event.payload.starts_with("embed:"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticComment {
    pub payload: String,
    pub position: SemanticPosition,
}

/// Physical location of an actual XML comment, for artifact line attribution.
/// These inclusive one-based lines are deliberately NOT semantic equality data.
/// ```
/// use vibe_specdoc::GeneratedXmlComment;
/// let event = GeneratedXmlComment { payload: "embed: x".into(), start_line: 2, end_line: 2 };
/// assert_eq!(event.start_line, event.end_line);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedXmlComment {
    pub payload: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Parse the ordinary spec dialect AND observe all actual canonical c1 comments.
/// Escaped text and CDATA are never decoded as comments. This observes editable
/// artifact data; it does not authenticate its producer.
/// ```
/// use vibe_specdoc::observe_annotated_xml;
/// let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><p>a<!-- vibe:c1 embed: x -->b</p></spec>";
/// let (doc, events) = observe_annotated_xml(xml)?;
/// assert_eq!(events.len(), 1);
/// assert_eq!(doc.preamble.len(), 1);
/// # Ok::<(), vibe_specdoc::Error>(())
/// ```
pub fn observe_annotated_xml(xml: &str) -> Result<(SpecDoc, Vec<SemanticComment>)> {
    crate::xml_in::parse_observed(xml).map(|(doc, semantic, _)| (doc, semantic))
}

/// Read physical c1 comment locations through that SAME validating XML reader.
/// ```
/// use vibe_specdoc::generated_xml_comments;
/// let xml = "<spec xmlns=\"https://vibevm.org/spec/1\">\n<!-- vibe:c1 embed: x -->\n</spec>";
/// assert_eq!(generated_xml_comments(xml)?[0].start_line, 2);
/// # Ok::<(), vibe_specdoc::Error>(())
/// ```
pub fn generated_xml_comments(xml: &str) -> Result<Vec<GeneratedXmlComment>> {
    crate::xml_in::parse_observed(xml).map(|(_, _, physical)| physical)
}

#[derive(Clone, Copy)]
pub(super) enum RegionKind {
    Container,
    TrimmedText,
    VerbatimText,
}

enum Anchor {
    Gap(usize),
    Text { event: usize, offset: usize },
}
impl Anchor {
    fn event(&self) -> usize {
        match *self {
            Self::Gap(event) | Self::Text { event, .. } => event,
        }
    }
}
struct RawComment {
    location: GeneratedXmlComment,
    anchor: Anchor,
}
struct Region {
    start: usize,
    end: usize,
    kind: RegionKind,
    children: Vec<usize>,
}

#[derive(Default)]
pub(super) struct Observation {
    comments: Vec<RawComment>,
    regions: Vec<Region>,
    stack: Vec<usize>,
}

fn error(line: usize, reason: &str) -> Error {
    Error::at(
        line,
        format!(
            "{reason} (spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-GENERATED-PROVENANCE; fix: regenerate a valid annotated XML artifact)"
        ),
    )
}

impl Observation {
    pub(super) fn capture(
        &mut self,
        xml: &str,
        start: usize,
        end: usize,
        events: &[Ev],
    ) -> Result<()> {
        let raw = xml
            .get(start..end)
            .ok_or_else(|| error(0, "XML comment bounds are invalid"))?;
        let start_line = crate::xml_support::pos_of(xml, start).0;
        let Some(payload) = crate::decode_generated_xml_comment(raw)
            .map_err(|e| error(start_line, &format!("invalid generated XML comment: {e}")))?
        else {
            return Ok(());
        };
        let anchor = match events.last() {
            Some(Ev::Text(text) | Ev::CData(text)) => Anchor::Text {
                event: events.len() - 1,
                offset: text.len(),
            },
            _ => Anchor::Gap(events.len()),
        };
        self.comments.push(RawComment {
            location: GeneratedXmlComment {
                payload,
                start_line,
                end_line: crate::xml_support::pos_of(xml, end.saturating_sub(1)).0,
            },
            anchor,
        });
        Ok(())
    }

    fn begin(&mut self, start: usize, kind: RegionKind) -> usize {
        let id = self.regions.len();
        if let Some(parent) = self.stack.last() {
            self.regions[*parent].children.push(id);
        }
        self.regions.push(Region {
            start,
            end: start,
            kind,
            children: vec![],
        });
        self.stack.push(id);
        id
    }

    pub(super) fn finish(
        self,
        events: &[Ev],
    ) -> Result<(Vec<SemanticComment>, Vec<GeneratedXmlComment>)> {
        if !self.stack.is_empty() {
            return Err(error(0, "semantic XML regions were not closed"));
        }
        let mut semantic = Vec::new();
        let mut physical = Vec::new();
        for comment in &self.comments {
            let event = comment.anchor.event();
            let inside = self
                .regions
                .iter()
                .enumerate()
                .rev()
                .find(|(_, r)| r.start < event && event < r.end);
            let position = if let Some((node, region)) = inside {
                match region.kind {
                    RegionKind::Container => SemanticPosition::ChildGap {
                        node,
                        child: region
                            .children
                            .iter()
                            .filter(|id| self.regions[**id].end <= event)
                            .count(),
                    },
                    RegionKind::TrimmedText | RegionKind::VerbatimText => SemanticPosition::Text {
                        node,
                        offset: text_offset(events, region, &comment.anchor)?,
                    },
                }
            } else if self.regions.first().is_some_and(|r| event <= r.start) {
                SemanticPosition::BeforeDocument
            } else {
                SemanticPosition::AfterDocument
            };
            semantic.push(SemanticComment {
                payload: comment.location.payload.clone(),
                position,
            });
            physical.push(comment.location.clone());
        }
        Ok((semantic, physical))
    }
}

fn text_offset(events: &[Ev], region: &Region, anchor: &Anchor) -> Result<usize> {
    let mut text = String::new();
    let mut prefix = 0;
    for (index, event) in events
        .iter()
        .enumerate()
        .take(region.end)
        .skip(region.start + 1)
    {
        let (Ev::Text(value) | Ev::CData(value)) = event else {
            continue;
        };
        if index < anchor.event() {
            prefix += value.len();
        }
        if let Anchor::Text { event, offset } = *anchor
            && index == event
        {
            if value.get(..offset).is_none() {
                return Err(error(0, "semantic text offset is not a UTF-8 boundary"));
            }
            prefix += offset;
        }
        text.push_str(value);
    }
    let (start, length) = match region.kind {
        RegionKind::TrimmedText => (text.len() - text.trim_start().len(), text.trim().len()),
        _ => (0, text.len()),
    };
    let offset = prefix.saturating_sub(start).min(length);
    let semantic = text
        .get(start..start + offset)
        .ok_or_else(|| error(0, "normalized semantic text range is invalid"))?;
    Ok(semantic.chars().count())
}

impl Parser<'_> {
    pub(super) fn begin_semantic(&mut self, kind: RegionKind) -> Option<usize> {
        self.observation
            .as_mut()
            .map(|o| o.begin(self.i.saturating_sub(1), kind))
    }
    pub(super) fn begin_semantic_text(&mut self) -> Option<usize> {
        if self.observation.as_ref().is_some_and(|o| {
            o.stack.last().is_some_and(|id| {
                matches!(
                    o.regions[*id].kind,
                    RegionKind::TrimmedText | RegionKind::VerbatimText
                )
            })
        }) {
            return None;
        }
        self.begin_semantic(RegionKind::TrimmedText)
    }
    pub(super) fn end_semantic(&mut self, node: Option<usize>) -> Result<()> {
        if let (Some(observation), Some(node)) = (&mut self.observation, node) {
            if observation.stack.pop() != Some(node) {
                return Err(error(0, "semantic XML nesting mismatch"));
            }
            observation.regions[node].end = self.i;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
