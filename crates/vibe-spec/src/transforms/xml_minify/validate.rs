//! XML 1.0 token validation for the byte-preserving minifier.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY");

use quick_xml::XmlVersion;
use quick_xml::encoding::Decoder;
use quick_xml::events::attributes::Attributes;
use quick_xml::events::{BytesDecl, BytesStart};

use super::XmlMinifyError;

pub(super) fn validate_element(
    element: &BytesStart<'_>,
    decoder: Decoder,
    event_offset: usize,
) -> Result<(), XmlMinifyError> {
    let name = element.name();
    validate_xml_name(
        name.as_ref(),
        "element name",
        event_offset.saturating_add(1),
    )?;

    let mut attributes = element.attributes();
    attributes.with_checks(true);
    for result in attributes {
        let attribute = result.map_err(|error| {
            XmlMinifyError::new(
                event_offset,
                format!("element has a malformed or duplicate attribute: {error}"),
            )
        })?;
        validate_xml_name(attribute.key.as_ref(), "attribute name", event_offset)?;
        if attribute.value.as_ref().contains(&b'<') {
            return Err(XmlMinifyError::new(
                event_offset,
                "attribute value contains a literal `<`; write `&lt;` instead",
            ));
        }
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Explicit1_0, decoder)
            .map_err(|error| {
                XmlMinifyError::new(
                    event_offset,
                    format!("attribute value cannot be decoded and unescaped: {error}"),
                )
            })?;
        validate_xml_10_characters(&value, "attribute value", event_offset)?;
    }
    Ok(())
}

pub(super) fn validate_declaration(
    declaration: &BytesDecl<'_>,
    event_offset: usize,
) -> Result<(), XmlMinifyError> {
    let content = std::str::from_utf8(declaration.as_ref()).map_err(|error| {
        XmlMinifyError::new(
            event_offset,
            format!("XML declaration is not UTF-8: {error}"),
        )
    })?;
    if !content.starts_with("xml")
        || content
            .as_bytes()
            .get(3)
            .is_some_and(|byte| !is_xml_whitespace_byte(*byte))
    {
        return Err(XmlMinifyError::new(
            event_offset,
            "XML declaration must begin with `xml` followed by whitespace",
        ));
    }

    let mut attributes = Attributes::new(content, 3);
    attributes.with_checks(true);
    let mut previous_rank = None::<u8>;
    let mut count = 0;
    for result in attributes {
        let attribute = result.map_err(|error| {
            XmlMinifyError::new(
                event_offset,
                format!("XML declaration pseudoattribute is malformed: {error}"),
            )
        })?;
        let key = std::str::from_utf8(attribute.key.as_ref()).map_err(|error| {
            XmlMinifyError::new(
                event_offset,
                format!("XML declaration pseudoattribute name is not UTF-8: {error}"),
            )
        })?;
        let value = std::str::from_utf8(attribute.value.as_ref()).map_err(|error| {
            XmlMinifyError::new(
                event_offset,
                format!("XML declaration pseudoattribute value is not UTF-8: {error}"),
            )
        })?;
        validate_xml_10_characters(value, "XML declaration value", event_offset)?;
        if value.contains(['&', '<']) {
            return Err(XmlMinifyError::new(
                event_offset,
                "XML declaration values cannot contain entity references or `<`",
            ));
        }

        let rank = match key {
            "version" if value == "1.0" => 0,
            "version" => {
                return Err(XmlMinifyError::new(
                    event_offset,
                    format!("XML 1.0 declaration requires `version = \"1.0\"`, found `{value}`"),
                ));
            }
            "encoding" if is_encoding_name(value) => 1,
            "encoding" => {
                return Err(XmlMinifyError::new(
                    event_offset,
                    format!("XML declaration encoding `{value}` is not a valid EncName"),
                ));
            }
            "standalone" if matches!(value, "yes" | "no") => 2,
            "standalone" => {
                return Err(XmlMinifyError::new(
                    event_offset,
                    format!(
                        "XML declaration standalone value must be `yes` or `no`, found `{value}`"
                    ),
                ));
            }
            _ => {
                return Err(XmlMinifyError::new(
                    event_offset,
                    format!("unknown XML declaration pseudoattribute `{key}`"),
                ));
            }
        };
        if count == 0 && rank != 0 {
            return Err(XmlMinifyError::new(
                event_offset,
                "XML declaration must carry `version` as its first pseudoattribute",
            ));
        }
        if previous_rank.is_some_and(|previous| rank <= previous) {
            return Err(XmlMinifyError::new(
                event_offset,
                "XML declaration pseudoattributes must appear once in `version`, `encoding`, `standalone` order",
            ));
        }
        previous_rank = Some(rank);
        count += 1;
    }
    if count == 0 {
        return Err(XmlMinifyError::new(
            event_offset,
            "XML declaration is missing required `version = \"1.0\"`",
        ));
    }
    Ok(())
}

fn validate_xml_name(raw: &[u8], context: &str, byte_offset: usize) -> Result<(), XmlMinifyError> {
    let name = std::str::from_utf8(raw).map_err(|error| {
        XmlMinifyError::new(byte_offset, format!("{context} is not UTF-8: {error}"))
    })?;
    let mut characters = name.char_indices();
    let Some((_, first)) = characters.next() else {
        return Err(XmlMinifyError::new(
            byte_offset,
            format!("{context} is empty"),
        ));
    };
    if !is_xml_name_start(first) {
        return Err(XmlMinifyError::new(
            byte_offset,
            format!(
                "{context} `{name}` starts with illegal XML 1.0 Name character U+{:04X}",
                first as u32
            ),
        ));
    }
    for (index, character) in characters {
        if !is_xml_name_character(character) {
            return Err(XmlMinifyError::new(
                byte_offset.saturating_add(index),
                format!(
                    "{context} `{name}` contains illegal XML 1.0 Name character U+{:04X}",
                    character as u32
                ),
            ));
        }
    }
    Ok(())
}

fn is_xml_name_start(character: char) -> bool {
    matches!(
        character,
        ':' | 'A'..='Z' | '_' | 'a'..='z'
            | '\u{C0}'..='\u{D6}'
            | '\u{D8}'..='\u{F6}'
            | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}'
            | '\u{37F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}'
    )
}

fn is_xml_name_character(character: char) -> bool {
    is_xml_name_start(character)
        || matches!(
            character,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
        )
}

pub(super) fn validate_xml_10_characters(
    text: &str,
    context: &str,
    byte_offset: usize,
) -> Result<(), XmlMinifyError> {
    for (index, character) in text.char_indices() {
        if !is_xml_10_character(character) {
            return Err(XmlMinifyError::new(
                byte_offset.saturating_add(index),
                format!(
                    "{context} contains illegal XML 1.0 character U+{:04X}",
                    character as u32
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn is_xml_10_character(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{A}' | '\u{D}'
            | '\u{20}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'..='\u{10FFFF}'
    )
}

fn is_encoding_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn is_xml_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
