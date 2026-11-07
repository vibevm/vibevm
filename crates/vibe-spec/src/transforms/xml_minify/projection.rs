//! Generated-stream projection, using the existing event and annotation owners.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-GENERATED-XML-PROJECTION-LAW"
);

use super::{XmlMinifyError, scan_stream};
use quick_xml::events::Event;
use std::ops::Range;

/// A refusal of the qualified generated-XML projection profile.
/// Offsets locate the original artifact, not the projected Markdown.
/// ```
/// let error = vibe_spec::project_emitted_xml_to_markdown("<foreign/>").unwrap_err();
/// assert_eq!(error.byte_offset(), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "generated XML projection refused at byte {byte_offset}: {diagnostic}; violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-GENERATED-XML-PROJECTION-LAW; fix surface: provide a supported generated Spec stream and retain raw bytes for unsupported content"
)]
pub struct EmittedXmlProjectionError {
    byte_offset: usize,
    diagnostic: String,
}

impl EmittedXmlProjectionError {
    fn new(byte_offset: usize, diagnostic: impl Into<String>) -> Self {
        Self {
            byte_offset,
            diagnostic: diagnostic.into(),
        }
    }
    /// Byte offset in the supplied artifact where projection refused.
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }
    /// The stream, vocabulary or unsupported-construct refusal.
    pub fn diagnostic(&self) -> &str {
        &self.diagnostic
    }
}

impl From<XmlMinifyError> for EmittedXmlProjectionError {
    fn from(error: XmlMinifyError) -> Self {
        Self::new(error.byte_offset(), error.diagnostic())
    }
}

enum Part {
    Root(Range<usize>),
    Declaration(Range<usize>),
}
impl Part {
    fn range(&self) -> &Range<usize> {
        match self {
            Self::Root(range) | Self::Declaration(range) => range,
        }
    }
}

#[derive(Default)]
pub(super) struct StreamCapture {
    parts: Vec<Part>,
    comments: Vec<(Range<usize>, usize)>,
    open_root: Option<usize>,
    unsupported: Option<(usize, &'static str)>,
    pending_hoist: Option<usize>,
}

impl StreamCapture {
    /// Observe positions from the validated scanner; this does not tokenize or
    /// validate a second grammar. Invalid scans discard the entire capture.
    pub(super) fn observe(
        &mut self,
        input: &str,
        event: &Event<'_>,
        range: Range<usize>,
        depth: usize,
    ) -> Result<(), XmlMinifyError> {
        if depth == 0 && self.pending_hoist.is_some() && !matches!(event, Event::Text(_)) {
            return Err(XmlMinifyError::new(
                range.start,
                "hoisted marker is missing its adjacent #use directive",
            ));
        }
        match event {
            Event::Start(_) if depth == 0 => self.open_root = Some(range.start),
            Event::Empty(_) if depth == 0 => self.parts.push(Part::Root(range)),
            Event::End(_) if depth == 1 => {
                if let Some(start) = self.open_root.take() {
                    self.parts.push(Part::Root(start..range.end));
                }
            }
            Event::Decl(declaration) => {
                if declaration.encoding().is_some_and(|value| {
                    !value.is_ok_and(|encoding| encoding.eq_ignore_ascii_case(b"UTF-8"))
                }) {
                    self.unsupported
                        .get_or_insert((range.start, "projection requires UTF-8 declarations"));
                }
                self.parts.push(Part::Declaration(range));
            }
            Event::Comment(_) => {
                let raw = &input[range.clone()];
                super::validate::validate_xml_10_characters(raw, "comment", range.start)?;
                vibe_specdoc::decode_generated_xml_comment(raw).map_err(|error| {
                    XmlMinifyError::new(
                        range.start,
                        format!("malformed generated comment: {error}"),
                    )
                })?;
                if depth == 0
                    && crate::compiler::emit::framing::hoisted_origin_in_comment(raw).is_some()
                {
                    self.pending_hoist = Some(range.start);
                }
                self.comments.push((range, depth));
            }
            Event::PI(_) => {
                self.unsupported.get_or_insert((
                    range.start,
                    "processing instructions have no supported canonical projection",
                ));
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn accept_hoisted_text(&mut self, text: &str) -> bool {
        if self.pending_hoist.is_some()
            && crate::compiler::emit::framing::read_hoisted_directive(text).is_some()
        {
            self.pending_hoist = None;
            true
        } else {
            false
        }
    }
    pub(super) fn has_outer_framing(&self) -> bool {
        self.comments.iter().any(|(_, depth)| *depth == 0)
    }
}

/// Project the generated multi-document Spec stream to annotated Markdown.
/// Outer framing and hoisted directives survive verbatim; XML declarations are
/// representation metadata. Inner actual c1 events retain mapped positions.
/// This neither minifies nor admits a source. Callers retain XML-projected kind
/// and use `vibe_specdoc::PROJECTION_NOTICE` for projected diagnostic positions.
/// ```
/// let xml = "<?xml version='1.0'?><spec xmlns='https://vibevm.org/spec/1'><title id='a'>A</title></spec><!-- boundary --><?xml version='1.0'?><spec xmlns='https://vibevm.org/spec/1'><title id='b'>B</title></spec>";
/// let md = vibe_spec::project_emitted_xml_to_markdown(xml)?;
/// assert!(md.contains("# A {#a}"));
/// assert!(md.contains("<!-- boundary -->"));
/// assert!(md.contains("# B {#b}"));
/// # Ok::<(), vibe_spec::EmittedXmlProjectionError>(())
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-GENERATED-XML-PROJECTION-LAW"
)]
pub fn project_emitted_xml_to_markdown(input: &str) -> Result<String, EmittedXmlProjectionError> {
    let mut capture = StreamCapture::default();
    // Only the validator is reused: its minifier deletion ranges are not applied.
    scan_stream(input, Some(&mut capture))?;
    if let Some((offset, reason)) = capture.unsupported {
        return Err(EmittedXmlProjectionError::new(offset, reason));
    }
    capture.parts.sort_by_key(|part| part.range().start);
    let mut output = String::new();
    let mut cursor = 0;
    for part in capture.parts {
        let range = part.range();
        output.push_str(&input[cursor..range.start]);
        if let Part::Root(range) = &part {
            let projected = vibe_specdoc::project_annotated_xml_to_markdown(&input[range.clone()])
                .map_err(|error| EmittedXmlProjectionError::new(range.start, error.to_string()))?;
            let (plain, comments) = projected.into_parts();
            let mut offsets = comments
                .iter()
                .map(|event| event.byte_offset)
                .collect::<Vec<_>>();
            let literal =
                crate::embed::occurrences::protect_projection_literals(&plain, &mut offsets);
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            let mut previous = 0;
            for (event, offset) in comments.iter().zip(offsets) {
                output.push_str(&literal[previous..offset]);
                let block = matches!(
                    event.placement,
                    vibe_specdoc::MarkdownCommentPlacement::Block
                );
                if block && !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&vibe_specdoc::frame_generated_xml_comment(&event.payload));
                if block {
                    output.push('\n');
                }
                previous = offset;
            }
            output.push_str(&literal[previous..]);
        }
        cursor = range.end;
    }
    output.push_str(&input[cursor..]);
    Ok(output)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
