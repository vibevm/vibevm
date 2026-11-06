//! Annotation-aware projection through the existing Markdown adapter and XML
//! writer. Comments are supplied as typed events, never recognized in prose.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-GENERATED-PROVENANCE"
);

use crate::{Error, Result, doc::SpecDoc};
use quick_xml::events::Event;

/// A generated boundary at a half-open Markdown line gap. The caller owns the
/// meaning and ordering of coincident events; the pivot owns their placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceComment {
    pub line_gap: usize,
    pub payload: String,
}

/// One prepared semantic document and independently recorded generated tape.
/// Inserting comments adds no semantic whitespace, including inside paragraphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedXml {
    document: SpecDoc,
    plain: String,
    comments: Vec<(usize, String)>,
}

impl AnnotatedXml {
    /// Ordinary pivot semantics, excluding all generated annotation metadata.
    pub fn document(&self) -> &SpecDoc {
        &self.document
    }

    /// Render canonical c1 comments at the checked semantic offsets.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut cursor = 0;
        for (offset, payload) in &self.comments {
            out.push_str(&self.plain[cursor..*offset]);
            out.push_str(&crate::frame_generated_xml_comment(payload));
            cursor = *offset;
        }
        out.push_str(&self.plain[cursor..]);
        out
    }

    /// Observe actual XML tokens and compare both semantics and the generated
    /// tape. Escaped text and CDATA cannot supply comments to this observer.
    pub fn observe(&self, xml: &str) -> Result<()> {
        let (actual_document, actual_comments) = crate::observe_annotated_xml(xml)?;
        if actual_document != self.document {
            return Err(Error::at(0, "annotated XML changed document semantics"));
        }
        let (expected_document, expected_comments) = crate::observe_annotated_xml(&self.render())?;
        if expected_document != self.document || actual_comments != expected_comments {
            return Err(Error::at(
                0,
                "generated XML semantic boundary placement differs from its prepared witness",
            ));
        }
        Ok(())
    }
}

/// Project explicit generated boundaries through the same parser and scalar
/// extraction used by `from_markdown`. No authored control spelling is scanned.
pub fn from_markdown_annotated(text: &str, events: &[SourceComment]) -> Result<AnnotatedXml> {
    let (document, source) = crate::md_in::from_markdown_mapped(text)?;
    let (plain, texts, headings) = crate::xml_out::render(&document, true);
    if source.texts.len() != texts.len() || source.headings.len() != headings.len() {
        return Err(Error::at(
            0,
            "Markdown/XML semantic position inventory differs",
        ));
    }
    for (input, output) in source.texts.iter().zip(&texts) {
        if input.value != output.value || input.bytes.len() != input.value.len() + 1 {
            return Err(Error::at(
                0,
                "Markdown/XML scalar position inventory differs",
            ));
        }
    }
    let mut gaps = vec![0];
    for line in text.split_inclusive('\n') {
        gaps.push(gaps.last().copied().unwrap_or(0) + line.len());
    }
    let mut comments = Vec::with_capacity(events.len());
    for event in events {
        let at = *gaps
            .get(event.line_gap)
            .ok_or_else(|| Error::at(0, "generated boundary is outside its source body"))?;
        let mut candidates = Vec::new();
        let mut interior = None;
        for (input, output) in source.texts.iter().zip(&texts) {
            candidates.push((input.start, output.before));
            candidates.push((input.end, output.after));
            if input.start < at && at < input.end {
                let offset =
                    input.bytes[..input.value.len()].partition_point(|source| *source < at);
                if !input.value.is_char_boundary(offset) {
                    return Err(Error::at(
                        0,
                        "generated boundary splits a semantic UTF-8 scalar",
                    ));
                }
                interior =
                    Some(output.text + crate::xml_out_emit::esc_text(&input.value[..offset]).len());
            }
        }
        for (input, output) in source.headings.iter().zip(&headings) {
            candidates.push((input.start, output.before));
            candidates.push((input.end, output.after));
        }
        candidates.sort_by_key(|(source, _)| *source);
        let offset = match interior {
            Some(offset) => offset,
            None => candidates
                .iter()
                .find(|(source, _)| *source >= at)
                .or_else(|| candidates.last())
                .map(|(_, target)| *target)
                .unwrap_or(root_content_start(&plain)?),
        };
        if comments
            .last()
            .is_some_and(|(previous, _)| *previous > offset)
        {
            return Err(Error::at(
                0,
                "generated boundary order cannot survive semantic projection",
            ));
        }
        comments.push((offset, event.payload.clone()));
    }
    let projected = AnnotatedXml {
        document,
        plain,
        comments,
    };
    projected.observe(&projected.render())?;
    Ok(projected)
}

fn root_content_start(xml: &str) -> Result<usize> {
    let mut reader = quick_xml::Reader::from_str(xml);
    loop {
        match reader
            .read_event()
            .map_err(|e| Error::at(0, e.to_string()))?
        {
            Event::Start(_) => return Ok(reader.buffer_position() as usize),
            Event::Eof => return Err(Error::at(0, "prepared XML has no document root")),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "annotated/tests.rs"]
mod tests;
