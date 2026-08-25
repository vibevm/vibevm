//! Byte-preserving XML whitespace squeeze used as the staged-transform vehicle.
//!
//! The compiled XML lane is a stream, not one XML document: generated comments,
//! repeated declarations, and several top-level `<spec>` roots share one file.
//! This kernel therefore uses quick-xml only as an event recogniser. It removes
//! eligible source spans and copies every other byte from the input; it never
//! sends the artifact through an XML writer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY");

use std::borrow::Cow;
use std::ops::Range;

use quick_xml::events::{BytesRef, Event};
use quick_xml::reader::Reader;

use validate::{
    is_xml_10_character, validate_declaration, validate_element, validate_xml_10_characters,
};

const MINIFY_SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY";

#[path = "xml_minify/validate.rs"]
mod validate;

/// Why the emitted-XML minifier refused an input stream.
///
/// The byte offset is into the original UTF-8 input. The diagnostic is owned,
/// so this public error does not expose quick-xml as part of vibe-spec's API.
///
/// ```
/// use vibe_spec::minify_emitted_xml;
///
/// let error = minify_emitted_xml("plain text").expect_err("not emitted XML");
/// assert_eq!(error.byte_offset(), 0);
/// assert!(error.diagnostic().contains("outside an element"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("xml-minify refused emitted XML at byte {byte_offset}: {diagnostic} (see {MINIFY_SPEC})")]
pub struct XmlMinifyError {
    byte_offset: usize,
    diagnostic: String,
}

impl XmlMinifyError {
    fn new(byte_offset: usize, diagnostic: impl Into<String>) -> Self {
        Self {
            byte_offset,
            diagnostic: diagnostic.into(),
        }
    }

    /// Byte offset of the rejected construct in the original UTF-8 input.
    #[must_use]
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Actionable reason the stream could not be transformed safely.
    #[must_use]
    pub fn diagnostic(&self) -> &str {
        &self.diagnostic
    }
}

/// Remove indentation text from pure element containers in an emitted XML stream.
///
/// A whitespace-only text node is removed exactly when its immediate parent has
/// at least one element child and no non-whitespace character data. XML
/// whitespace means only space, tab, carriage return, and line feed. Comments,
/// CDATA, declarations, processing instructions, element spelling, attributes,
/// quote style, and top-level stream framing are copied byte-for-byte.
///
/// The input may contain several top-level roots and declarations, matching the
/// generated `STATIC.xml` framing. Every individual XML token must still be
/// well formed: DTDs, unknown entities, and comments containing `--` refuse.
/// Names, attributes, declarations, and character data are validated against
/// XML 1.0; each declaration belongs to exactly the next top-level root. A
/// no-op borrows `input`; allocation happens only when at least one byte range
/// is removed.
///
/// ```
/// use std::borrow::Cow;
/// use vibe_spec::minify_emitted_xml;
///
/// assert_eq!(
///     minify_emitted_xml("<root>\n  <a/>\n  <b/>\n</root>").unwrap(),
///     "<root><a/><b/></root>",
/// );
/// assert!(matches!(
///     minify_emitted_xml("<p>prose <b>stays</b> intact</p>").unwrap(),
///     Cow::Borrowed(_),
/// ));
/// ```
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
pub fn minify_emitted_xml(input: &str) -> Result<Cow<'_, str>, XmlMinifyError> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_comments = true;
    reader.config_mut().check_end_names = true;
    reader.config_mut().allow_unmatched_ends = false;
    reader.config_mut().expand_empty_elements = false;

    let mut frames = Vec::<ElementFrame>::new();
    let mut deletions = Vec::<Range<usize>>::new();
    let mut saw_element = false;
    let mut pending_declaration = None::<usize>;

    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(|error| {
            XmlMinifyError::new(
                reader.error_position() as usize,
                format!("XML is not well formed: {error}"),
            )
        })?;
        let end = reader.buffer_position() as usize;

        match event {
            Event::Start(element) => {
                validate_element(&element, reader.decoder(), start)?;
                flush_character_run(frames.last_mut());
                if let Some(parent) = frames.last_mut() {
                    parent.has_element_child = true;
                } else {
                    pending_declaration = None;
                }
                frames.push(ElementFrame::default());
                saw_element = true;
            }
            Event::Empty(element) => {
                validate_element(&element, reader.decoder(), start)?;
                flush_character_run(frames.last_mut());
                if let Some(parent) = frames.last_mut() {
                    parent.has_element_child = true;
                } else {
                    pending_declaration = None;
                }
                saw_element = true;
            }
            Event::End(_) => {
                flush_character_run(frames.last_mut());
                let Some(frame) = frames.pop() else {
                    return Err(XmlMinifyError::new(
                        start,
                        "an end tag has no matching open element",
                    ));
                };
                if frame.has_element_child && !frame.has_non_whitespace_data {
                    deletions.extend(frame.whitespace_runs);
                }
            }
            Event::Text(text) => {
                let decoded = text.decode().map_err(|error| {
                    XmlMinifyError::new(start, format!("text is not decodable UTF-8: {error}"))
                })?;
                validate_xml_10_characters(&decoded, "text", start)?;
                let whitespace = is_xml_whitespace(&decoded);
                record_character_data(frames.last_mut(), start..end, whitespace, start)?;
            }
            Event::GeneralRef(reference) => {
                if frames.is_empty() {
                    return Err(XmlMinifyError::new(
                        start,
                        "character references require an open element, even when they resolve to whitespace",
                    ));
                }
                let whitespace = reference_is_xml_whitespace(&reference, start)?;
                record_character_data(frames.last_mut(), start..end, whitespace, start)?;
            }
            Event::CData(data) => {
                if frames.is_empty() {
                    return Err(XmlMinifyError::new(
                        start,
                        "CDATA sections require an open element, even when they contain only whitespace",
                    ));
                }
                flush_character_run(frames.last_mut());
                let decoded = data.decode().map_err(|error| {
                    XmlMinifyError::new(start, format!("CDATA is not decodable UTF-8: {error}"))
                })?;
                validate_xml_10_characters(&decoded, "CDATA", start)?;
                let whitespace = is_xml_whitespace(&decoded);
                match frames.last_mut() {
                    Some(parent) if !whitespace => parent.has_non_whitespace_data = true,
                    Some(_) => {}
                    None => unreachable!("top-level CDATA returned above"),
                }
            }
            Event::Comment(_) | Event::PI(_) => {
                // Each is a character-data boundary, but never a deletion
                // candidate. Its exact bytes stay in the source slices.
                flush_character_run(frames.last_mut());
            }
            Event::Decl(declaration) => {
                validate_declaration(&declaration, start)?;
                if !frames.is_empty() {
                    return Err(XmlMinifyError::new(
                        start,
                        "an XML declaration is legal only at top level before its root",
                    ));
                }
                if let Some(previous) = pending_declaration {
                    return Err(XmlMinifyError::new(
                        start,
                        format!(
                            "a second XML declaration appears before the first declaration at byte {previous} acquired a root"
                        ),
                    ));
                }
                pending_declaration = Some(start);
            }
            Event::DocType(_) => {
                return Err(XmlMinifyError::new(
                    start,
                    "DTD declarations are unsupported because entity expansion makes parent content classification unsafe",
                ));
            }
            Event::Eof => {
                flush_character_run(frames.last_mut());
                if !frames.is_empty() {
                    return Err(XmlMinifyError::new(
                        input.len(),
                        format!("input ended with {} unclosed element(s)", frames.len()),
                    ));
                }
                if let Some(offset) = pending_declaration {
                    return Err(XmlMinifyError::new(
                        offset,
                        "an XML declaration is orphaned at end of stream without a following root",
                    ));
                }
                break;
            }
        }
    }

    if !saw_element {
        return Err(XmlMinifyError::new(
            0,
            "the emitted XML stream contains no element",
        ));
    }
    render_without_ranges(input, deletions)
}

#[derive(Debug, Default)]
struct ElementFrame {
    has_element_child: bool,
    has_non_whitespace_data: bool,
    whitespace_runs: Vec<Range<usize>>,
    pending_run: Option<CharacterRun>,
}

#[derive(Debug)]
struct CharacterRun {
    range: Range<usize>,
    whitespace: bool,
}

fn record_character_data(
    frame: Option<&mut ElementFrame>,
    range: Range<usize>,
    whitespace: bool,
    offset: usize,
) -> Result<(), XmlMinifyError> {
    let Some(frame) = frame else {
        if !whitespace {
            return Err(XmlMinifyError::new(
                offset,
                "non-whitespace character data appears outside an element",
            ));
        }
        return Ok(());
    };

    match frame.pending_run.as_mut() {
        Some(pending) if pending.range.end == range.start => {
            pending.range.end = range.end;
            pending.whitespace &= whitespace;
        }
        Some(_) => {
            flush_character_run(Some(frame));
            frame.pending_run = Some(CharacterRun { range, whitespace });
        }
        None => frame.pending_run = Some(CharacterRun { range, whitespace }),
    }
    Ok(())
}

fn flush_character_run(frame: Option<&mut ElementFrame>) {
    let Some(frame) = frame else {
        return;
    };
    let Some(run) = frame.pending_run.take() else {
        return;
    };
    if run.whitespace {
        frame.whitespace_runs.push(run.range);
    } else {
        frame.has_non_whitespace_data = true;
    }
}

fn is_xml_whitespace(text: &str) -> bool {
    text.as_bytes()
        .iter()
        .all(|byte| is_xml_whitespace_byte(*byte))
}

fn is_xml_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn reference_is_xml_whitespace(
    reference: &BytesRef<'_>,
    offset: usize,
) -> Result<bool, XmlMinifyError> {
    match reference.resolve_char_ref() {
        Ok(Some(character)) => {
            if !is_xml_10_character(character) {
                return Err(XmlMinifyError::new(
                    offset,
                    format!(
                        "numeric reference resolves to illegal XML 1.0 character U+{:04X}",
                        character as u32
                    ),
                ));
            }
            Ok(matches!(character, ' ' | '\t' | '\r' | '\n'))
        }
        Ok(None) => {
            let name = reference.decode().map_err(|error| {
                XmlMinifyError::new(
                    offset,
                    format!("entity reference name is not decodable UTF-8: {error}"),
                )
            })?;
            if matches!(name.as_ref(), "amp" | "lt" | "gt" | "apos" | "quot") {
                Ok(false)
            } else {
                Err(XmlMinifyError::new(
                    offset,
                    format!(
                        "unknown entity `&{name};` cannot be classified safely; use a numeric reference or one of XML's five predefined entities"
                    ),
                ))
            }
        }
        Err(error) => Err(XmlMinifyError::new(
            offset,
            format!("invalid character reference: {error}"),
        )),
    }
}

fn render_without_ranges(
    input: &str,
    mut ranges: Vec<Range<usize>>,
) -> Result<Cow<'_, str>, XmlMinifyError> {
    if ranges.is_empty() {
        return Ok(Cow::Borrowed(input));
    }
    ranges.sort_unstable_by_key(|range| (range.start, range.end));

    let mut cursor = 0;
    let mut removed = 0;
    for range in &ranges {
        if range.start < cursor
            || range.start > range.end
            || range.end > input.len()
            || !input.is_char_boundary(range.start)
            || !input.is_char_boundary(range.end)
        {
            return Err(XmlMinifyError::new(
                range.start.min(input.len()),
                "internal event spans overlap or do not align to UTF-8 boundaries",
            ));
        }
        cursor = range.end;
        removed += range.end - range.start;
    }

    let mut output = String::with_capacity(input.len() - removed);
    cursor = 0;
    for range in ranges {
        output.push_str(&input[cursor..range.start]);
        cursor = range.end;
    }
    output.push_str(&input[cursor..]);
    Ok(Cow::Owned(output))
}

#[cfg(test)]
#[path = "xml_minify/tests.rs"]
mod tests;
