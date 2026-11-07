//! Event/attribute/reference checks delegated to quick-xml; no resource resolver.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION");
use super::*;
use quick_xml::{
    XmlVersion,
    events::{BytesDecl, BytesStart, Event},
    reader::Reader,
};
type Checked<T> = Result<T, XmlDataIssue>;
fn budget(value: usize, limit: Option<usize>, budget: InspectionBudget) -> Checked<()> {
    if let Some(limit) = limit
        && value > limit
    {
        return Err(XmlDataIssue::LimitExceeded { budget, limit });
    }
    Ok(())
}
fn add(value: &mut usize, amount: usize) -> Checked<()> {
    *value = value
        .checked_add(amount)
        .ok_or(XmlDataIssue::CounterOverflow)?;
    Ok(())
}
fn xml_character(ch: char) -> bool {
    matches!(ch, '\u{9}'|'\u{a}'|'\u{d}'|'\u{20}'..='\u{d7ff}'|'\u{e000}'..='\u{fffd}'|'\u{10000}'..='\u{10ffff}')
}
fn attributes(element: &BytesStart<'_>) -> Checked<()> {
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|_| XmlDataIssue::MalformedEvents)?;
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| XmlDataIssue::ForbiddenReference)?;
        if !value.chars().all(xml_character) {
            return Err(XmlDataIssue::ForbiddenReference);
        }
    }
    Ok(())
}
fn declaration(decl: &BytesDecl<'_>) -> Checked<()> {
    match decl
        .xml_version()
        .map_err(|_| XmlDataIssue::InvalidDeclaration)?
    {
        XmlVersion::Explicit1_0 => {}
        _ => return Err(XmlDataIssue::UnsupportedXmlVersion),
    }
    // Reuse the owning attribute grammar to detect duplicates/unknown/order.
    // BytesDecl's convenience accessors alone intentionally return the first duplicate.
    let content = std::str::from_utf8(decl.as_ref()).map_err(|_| XmlDataIssue::InvalidUtf8)?;
    let element = BytesStart::from_content(content, 3); // upstream Decl content starts with xml
    let mut last = 0;
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|_| XmlDataIssue::InvalidDeclaration)?;
        let rank = match attribute.key.as_ref() {
            b"version" if attribute.value.as_ref() == b"1.0" => 1,
            b"encoding" => {
                if !attribute.value.as_ref().eq_ignore_ascii_case(b"utf-8") {
                    return Err(XmlDataIssue::UnsupportedEncoding);
                }
                2
            }
            b"standalone" if matches!(attribute.value.as_ref(), b"yes" | b"no") => 3,
            _ => return Err(XmlDataIssue::InvalidDeclaration),
        };
        if rank <= last {
            return Err(XmlDataIssue::InvalidDeclaration);
        }
        last = rank;
    }
    Ok(())
}
pub(super) fn inspect(raw: &[u8], limits: InspectionLimits) -> Checked<XmlOutline> {
    budget(
        raw.len(),
        limits.max_input_bytes,
        InspectionBudget::InputBytes,
    )?;
    let source = std::str::from_utf8(raw).map_err(|_| XmlDataIssue::InvalidUtf8)?;
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    if !source.chars().all(xml_character) {
        return Err(XmlDataIssue::MalformedEvents);
    }
    let mut reader = Reader::from_str(source);
    reader.config_mut().enable_all_checks(true);
    let mut root = None;
    let mut depth = 0usize;
    let mut outline = XmlOutline {
        root: String::new(),
        elements: 0,
        max_depth: 0,
        text_bytes: 0,
        comments: 0,
        processing_instructions: 0,
    };
    let mut events = 0usize;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| XmlDataIssue::MalformedEvents)?;
        if matches!(event, Event::Eof) {
            break;
        }
        add(&mut events, 1)?;
        budget(events, limits.max_events, InspectionBudget::Events)?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Decl(decl) => {
                if events != 1 {
                    return Err(XmlDataIssue::InvalidDeclaration);
                }
                declaration(&decl)?;
            }
            Event::DocType(_) => return Err(XmlDataIssue::ForbiddenDtd),
            Event::PI(_) => add(&mut outline.processing_instructions, 1)?,
            Event::Start(element) | Event::Empty(element) => {
                if depth == 0 && root.is_some() {
                    return Err(XmlDataIssue::MalformedEvents);
                }
                attributes(&element)?;
                if element.name().as_ref().is_empty() {
                    return Err(XmlDataIssue::MalformedEvents);
                }
                if root.is_none() {
                    root = Some(
                        std::str::from_utf8(element.name().as_ref())
                            .map_err(|_| XmlDataIssue::InvalidUtf8)?
                            .to_owned(),
                    );
                }
                let next_depth = depth.checked_add(1).ok_or(XmlDataIssue::CounterOverflow)?;
                budget(next_depth, limits.max_depth, InspectionBudget::Depth)?;
                add(&mut outline.elements, 1)?;
                budget(
                    outline.elements,
                    limits.max_elements,
                    InspectionBudget::Elements,
                )?;
                outline.max_depth = outline.max_depth.max(next_depth);
                if !empty {
                    depth = next_depth;
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(XmlDataIssue::MalformedEvents)?;
            }
            Event::Text(text) => {
                let bytes: &[u8] = text.as_ref();
                if depth == 0
                    && !bytes
                        .iter()
                        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                {
                    return Err(XmlDataIssue::MalformedEvents);
                }
                add(&mut outline.text_bytes, bytes.len())?;
            }
            Event::CData(text) => {
                if depth == 0 {
                    return Err(XmlDataIssue::MalformedEvents);
                }
                add(&mut outline.text_bytes, text.len())?;
            }
            Event::GeneralRef(reference) => {
                if depth == 0 {
                    return Err(XmlDataIssue::ForbiddenReference);
                }
                let character = reference
                    .resolve_char_ref()
                    .map_err(|_| XmlDataIssue::ForbiddenReference)?;
                let name: &[u8] = reference.as_ref();
                if !match character {
                    Some(ch) => xml_character(ch),
                    None => matches!(name, b"amp" | b"lt" | b"gt" | b"apos" | b"quot"),
                } {
                    return Err(XmlDataIssue::ForbiddenReference);
                }
                add(&mut outline.text_bytes, reference.len())?;
                add(&mut outline.text_bytes, 2)?;
            }
            Event::Comment(_) => add(&mut outline.comments, 1)?,
            Event::Eof => {}
        }
        budget(
            outline.text_bytes,
            limits.max_text_bytes,
            InspectionBudget::TextBytes,
        )?;
    }
    if depth != 0 {
        return Err(XmlDataIssue::MalformedEvents);
    }
    outline.root = root.ok_or(XmlDataIssue::MalformedEvents)?;
    Ok(outline)
}
