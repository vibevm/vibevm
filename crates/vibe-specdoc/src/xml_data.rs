//! Optional, byte-preserving inspection of ordinary XML data.
//!
//! This is separate from the closed Vibe XML dialect. An inspection failure
//! never changes or hides the captured bytes, and no entity resolver or file
//! access is involved.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-OBSERVATION-LAW");

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

pub const MAX_XML_DATA_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_XML_DATA_DEPTH: usize = 64;
pub const MAX_XML_DATA_ELEMENTS: usize = 10_000;
pub const MAX_XML_DATA_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDataOutline {
    pub root: String,
    pub elements: usize,
    pub max_depth: usize,
    pub text_bytes: usize,
    pub comments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlDataIssue {
    InvalidEncoding,
    UnsupportedEncoding,
    MalformedXml,
    ForbiddenEntity,
    LimitExceeded(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataInspection {
    /// Opaque content is intentionally not parsed.
    NotRequested,
    Xml(XmlDataOutline),
    Unavailable(XmlDataIssue),
}

/// The exact captured bytes remain available under every inspection result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservedData<'a> {
    raw: &'a [u8],
    pub inspection: DataInspection,
}

impl<'a> PreservedData<'a> {
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }
}

pub fn preserve_opaque(raw: &[u8]) -> PreservedData<'_> {
    PreservedData {
        raw,
        inspection: DataInspection::NotRequested,
    }
}

/// Inspect one already captured byte slice, with no conversion or writes.
/// Only UTF-8 XML is inspected in this first bounded profile; other encodings
/// stay raw with a distinct diagnostic rather than being misdecoded.
pub fn inspect_xml_data(raw: &[u8]) -> PreservedData<'_> {
    let inspection = inspect(raw).map_or_else(DataInspection::Unavailable, DataInspection::Xml);
    PreservedData { raw, inspection }
}

fn inspect(raw: &[u8]) -> Result<XmlDataOutline, XmlDataIssue> {
    if raw.len() > MAX_XML_DATA_BYTES {
        return Err(XmlDataIssue::LimitExceeded("bytes"));
    }
    let source = std::str::from_utf8(raw).map_err(|_| XmlDataIssue::InvalidEncoding)?;
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    if !source.chars().all(xml_1_0_char) {
        return Err(XmlDataIssue::MalformedXml);
    }
    let mut reader = Reader::from_str(source);
    let mut stack = Vec::<Vec<u8>>::new();
    let mut root = None;
    let mut elements = 0usize;
    let mut max_depth = 0usize;
    let mut text_bytes = 0usize;
    let mut comments = 0usize;
    let mut declared = false;
    let mut declaration_allowed = true;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| XmlDataIssue::MalformedXml)?;
        if !matches!(&event, Event::Decl(_) | Event::Eof) {
            declaration_allowed = false;
        }
        match event {
            Event::Decl(decl) => {
                if declared || !declaration_allowed || root.is_some() {
                    return Err(XmlDataIssue::MalformedXml);
                }
                declared = true;
                if let Some(encoding) = decl.encoding() {
                    let encoding = encoding.map_err(|_| XmlDataIssue::MalformedXml)?;
                    if !encoding.as_ref().eq_ignore_ascii_case(b"utf-8") {
                        return Err(XmlDataIssue::UnsupportedEncoding);
                    }
                }
            }
            Event::DocType(_) => return Err(XmlDataIssue::ForbiddenEntity),
            Event::GeneralRef(reference) => {
                if stack.is_empty() || !safe_reference(&reference) {
                    return Err(XmlDataIssue::ForbiddenEntity);
                }
                let spelling: &[u8] = reference.as_ref();
                count_text(&mut text_bytes, spelling.len() + 2)?;
            }
            Event::Start(element) => {
                if stack.is_empty() && root.is_some() {
                    return Err(XmlDataIssue::MalformedXml);
                }
                check_attrs(&element)?;
                let name = element.name().as_ref().to_vec();
                if name.is_empty() {
                    return Err(XmlDataIssue::MalformedXml);
                }
                if root.is_none() {
                    root = Some(
                        String::from_utf8(name.clone())
                            .map_err(|_| XmlDataIssue::InvalidEncoding)?,
                    );
                }
                stack.push(name);
                count_element(&mut elements, &mut max_depth, stack.len())?;
            }
            Event::Empty(element) => {
                if stack.is_empty() && root.is_some() {
                    return Err(XmlDataIssue::MalformedXml);
                }
                check_attrs(&element)?;
                if root.is_none() {
                    root = Some(
                        std::str::from_utf8(element.name().as_ref())
                            .map_err(|_| XmlDataIssue::InvalidEncoding)?
                            .to_owned(),
                    );
                }
                count_element(&mut elements, &mut max_depth, stack.len() + 1)?;
            }
            Event::End(element) => {
                let expected = stack.pop().ok_or(XmlDataIssue::MalformedXml)?;
                if expected != element.name().as_ref() {
                    return Err(XmlDataIssue::MalformedXml);
                }
            }
            Event::Text(text) => {
                let bytes: &[u8] = text.as_ref();
                if stack.is_empty() && !bytes.iter().all(u8::is_ascii_whitespace) {
                    return Err(XmlDataIssue::MalformedXml);
                }
                count_text(&mut text_bytes, bytes.len())?;
            }
            Event::CData(text) => {
                if stack.is_empty() {
                    return Err(XmlDataIssue::MalformedXml);
                }
                let bytes: &[u8] = text.as_ref();
                count_text(&mut text_bytes, bytes.len())?;
            }
            Event::Comment(_) => comments += 1,
            Event::PI(_) => {}
            Event::Eof => break,
        }
    }
    if !stack.is_empty() {
        return Err(XmlDataIssue::MalformedXml);
    }
    let root = root.ok_or(XmlDataIssue::MalformedXml)?;
    Ok(XmlDataOutline {
        root,
        elements,
        max_depth,
        text_bytes,
        comments,
    })
}

fn check_attrs(element: &BytesStart<'_>) -> Result<(), XmlDataIssue> {
    for attribute in element.attributes().with_checks(true) {
        attribute
            .map_err(|_| XmlDataIssue::MalformedXml)?
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|_| XmlDataIssue::ForbiddenEntity)?;
    }
    Ok(())
}

fn safe_reference(reference: &quick_xml::events::BytesRef<'_>) -> bool {
    if let Ok(Some(ch)) = reference.resolve_char_ref() {
        return xml_1_0_char(ch);
    }
    let name: &[u8] = reference.as_ref();
    matches!(name, b"lt" | b"gt" | b"amp" | b"apos" | b"quot")
}

fn xml_1_0_char(ch: char) -> bool {
    matches!(ch,
        '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}'
        | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}

fn count_element(
    elements: &mut usize,
    max_depth: &mut usize,
    depth: usize,
) -> Result<(), XmlDataIssue> {
    *elements += 1;
    *max_depth = (*max_depth).max(depth);
    if depth > MAX_XML_DATA_DEPTH {
        return Err(XmlDataIssue::LimitExceeded("depth"));
    }
    if *elements > MAX_XML_DATA_ELEMENTS {
        return Err(XmlDataIssue::LimitExceeded("elements"));
    }
    Ok(())
}

fn count_text(text_bytes: &mut usize, added: usize) -> Result<(), XmlDataIssue> {
    *text_bytes += added;
    if *text_bytes > MAX_XML_DATA_TEXT_BYTES {
        return Err(XmlDataIssue::LimitExceeded("text"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "xml_data/tests.rs"]
mod tests;
