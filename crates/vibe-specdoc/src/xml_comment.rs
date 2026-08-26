//! Canonical wire codec for comments generated into XML artifacts.
//!
//! XML comments may contain neither an internal `--` nor a terminal hyphen.
//! Generated payloads are not a closed vocabulary (paths and qualified names
//! are data), so XML lanes carry them through one reversible, versioned wire:
//! `<!-- vibe:c1 <encoded payload> -->`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#STATIC-FOLLOWS-THE-TARGET");

use std::fmt;

const COMMENT_OPEN: &str = "<!--";
const COMMENT_CLOSE: &str = "-->";
const RESERVED_PREFIX: &str = " vibe:c";
const C1_PREFIX: &str = " vibe:c1 ";

/// A precise refusal raised while decoding a generated XML comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlCommentCodecError {
    /// The input is not exactly one complete XML comment.
    IncompleteComment,
    /// The reserved `vibe:c` namespace does not carry a decimal version.
    MalformedVersion { value: String },
    /// The reserved namespace was found outside the exact c1 wrapper spacing.
    MalformedFraming,
    /// A future or obsolete version cannot be interpreted as c1.
    UnsupportedVersion { version: String },
    /// A percent escape is truncated or contains a non-hex digit.
    MalformedEscape { offset: usize, value: String },
    /// Hex letters in c1 escapes are uppercase by definition.
    LowercaseEscape { offset: usize, value: String },
    /// Percent bytes did not decode to one valid UTF-8 string.
    InvalidUtf8 { offset: usize },
    /// The payload decodes, but uses a second spelling for the same value.
    NonCanonical { offset: usize, canonical: String },
}

impl fmt::Display for XmlCommentCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteComment => f.write_str(
                "generated XML comment input is not one complete `<!-- ... -->` comment; \
                 fix: pass exactly one comment with no leading or trailing bytes",
            ),
            Self::MalformedVersion { value } => write!(
                f,
                "generated XML comment has malformed reserved version `{value}`; \
                 fix: regenerate it as `<!-- vibe:c1 <payload> -->`"
            ),
            Self::MalformedFraming => f.write_str(
                "generated XML comment uses malformed reserved `vibe:c` framing; \
                 fix: regenerate it as exactly `<!-- vibe:c1 <payload> -->`",
            ),
            Self::UnsupportedVersion { version } => write!(
                f,
                "generated XML comment uses unsupported `vibe:c{version}`; \
                 fix: regenerate the artifact with a binary that writes and reads c1"
            ),
            Self::MalformedEscape { offset, value } => write!(
                f,
                "generated XML comment has malformed (truncated or non-hex) percent escape `{value}` at byte {offset}; \
                 fix: use one uppercase `%HH` byte or regenerate the artifact"
            ),
            Self::LowercaseEscape { offset, value } => write!(
                f,
                "generated XML comment percent escape `{value}` at byte {offset} is not uppercase; \
                 fix: use uppercase hex or regenerate the artifact"
            ),
            Self::InvalidUtf8 { offset } => write!(
                f,
                "generated XML comment percent bytes are not valid UTF-8 at byte {offset}; \
                 fix: regenerate the artifact from its logical Unicode payload"
            ),
            Self::NonCanonical { offset, canonical } => write!(
                f,
                "generated XML comment is not canonical at byte {offset}; \
                 fix: regenerate it with encoded payload `{canonical}`"
            ),
        }
    }
}

impl std::error::Error for XmlCommentCodecError {}

/// Encode one logical generated-comment payload using canonical c1 spelling.
///
/// Literal `%` is always escaped. A hyphen is escaped exactly when appending
/// it literally would create `--`, or when it is the final scalar. XML 1.0
/// illegal scalars are escaped bytewise as uppercase UTF-8 `%HH`; every other
/// scalar, including `&`, `<`, `>`, and non-ASCII Unicode, stays readable.
pub fn encode_generated_xml_comment(payload: &str) -> String {
    let mut encoded = String::with_capacity(payload.len());
    let mut chars = payload.char_indices().peekable();
    let mut last_emitted_was_hyphen = false;

    while let Some((_, character)) = chars.next() {
        let terminal = chars.peek().is_none();
        match character {
            '%' => {
                encoded.push_str("%25");
                last_emitted_was_hyphen = false;
            }
            '-' if terminal || last_emitted_was_hyphen => {
                encoded.push_str("%2D");
                last_emitted_was_hyphen = false;
            }
            '-' => {
                encoded.push('-');
                last_emitted_was_hyphen = true;
            }
            character if !is_xml_10_character(character) => {
                let mut utf8 = [0_u8; 4];
                for byte in character.encode_utf8(&mut utf8).as_bytes() {
                    push_percent_byte(&mut encoded, *byte);
                }
                last_emitted_was_hyphen = false;
            }
            character => {
                encoded.push(character);
                last_emitted_was_hyphen = false;
            }
        }
    }
    encoded
}

/// Decode exactly one generated c1 XML comment.
///
/// A complete non-c1 comment is legacy/authored input and returns `Ok(None)`.
/// Anything in the reserved `vibe:c` namespace is strict: decode performs one
/// percent pass, validates UTF-8, then requires exact canonical re-encoding.
pub fn decode_generated_xml_comment(comment: &str) -> Result<Option<String>, XmlCommentCodecError> {
    let Some(interior) = comment
        .strip_prefix(COMMENT_OPEN)
        .and_then(|value| value.strip_suffix(COMMENT_CLOSE))
    else {
        return Err(XmlCommentCodecError::IncompleteComment);
    };
    if interior.contains(COMMENT_OPEN) || interior.contains(COMMENT_CLOSE) {
        return Err(XmlCommentCodecError::IncompleteComment);
    }
    let Some(reserved) = interior.strip_prefix(RESERVED_PREFIX) else {
        return if interior.trim_start().starts_with("vibe:c") {
            Err(XmlCommentCodecError::MalformedFraming)
        } else {
            Ok(None)
        };
    };

    let Some(version_end) = reserved.find(' ') else {
        return Err(XmlCommentCodecError::MalformedVersion {
            value: reserved.to_string(),
        });
    };
    let version = &reserved[..version_end];
    if version.is_empty() || !version.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(XmlCommentCodecError::MalformedVersion {
            value: version.to_string(),
        });
    }
    if version != "1" {
        return Err(XmlCommentCodecError::UnsupportedVersion {
            version: version.to_string(),
        });
    }
    let Some(encoded) = interior
        .strip_prefix(C1_PREFIX)
        .and_then(|value| value.strip_suffix(' '))
    else {
        return Err(XmlCommentCodecError::MalformedFraming);
    };

    let payload_offset = COMMENT_OPEN.len() + C1_PREFIX.len();
    let decoded = decode_percent_once(encoded, payload_offset)?;
    let canonical = encode_generated_xml_comment(&decoded);
    if canonical != encoded {
        let relative = first_difference(encoded.as_bytes(), canonical.as_bytes());
        return Err(XmlCommentCodecError::NonCanonical {
            offset: payload_offset + relative,
            canonical,
        });
    }
    Ok(Some(decoded))
}

fn decode_percent_once(
    encoded: &str,
    payload_offset: usize,
) -> Result<String, XmlCommentCodecError> {
    let input = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(input.len());
    let mut source_offsets = Vec::with_capacity(input.len());
    let mut cursor = 0;

    while cursor < input.len() {
        if input[cursor] != b'%' {
            let character = encoded[cursor..]
                .chars()
                .next()
                .expect("cursor is inside a non-empty valid Rust string");
            let end = cursor + character.len_utf8();
            decoded.extend_from_slice(&input[cursor..end]);
            source_offsets.extend(std::iter::repeat_n(payload_offset + cursor, end - cursor));
            cursor = end;
            continue;
        }

        let end = (cursor + 3).min(input.len());
        let spelling = String::from_utf8_lossy(&input[cursor..end]).into_owned();
        if cursor + 2 >= input.len() {
            return Err(XmlCommentCodecError::MalformedEscape {
                offset: payload_offset + cursor,
                value: spelling,
            });
        }
        let high = input[cursor + 1];
        let low = input[cursor + 2];
        if matches!(high, b'a'..=b'f') || matches!(low, b'a'..=b'f') {
            return Err(XmlCommentCodecError::LowercaseEscape {
                offset: payload_offset + cursor,
                value: spelling,
            });
        }
        let Some(high) = hex_value(high) else {
            return Err(XmlCommentCodecError::MalformedEscape {
                offset: payload_offset + cursor,
                value: spelling,
            });
        };
        let Some(low) = hex_value(low) else {
            return Err(XmlCommentCodecError::MalformedEscape {
                offset: payload_offset + cursor,
                value: spelling,
            });
        };
        decoded.push((high << 4) | low);
        source_offsets.push(payload_offset + cursor);
        cursor += 3;
    }

    String::from_utf8(decoded).map_err(|error| {
        let decoded_offset = error.utf8_error().valid_up_to();
        XmlCommentCodecError::InvalidUtf8 {
            offset: source_offsets
                .get(decoded_offset)
                .copied()
                .unwrap_or(payload_offset + encoded.len()),
        }
    })
}

fn push_percent_byte(output: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push('%');
    output.push(HEX[(byte >> 4) as usize] as char);
    output.push(HEX[(byte & 0x0f) as usize] as char);
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn first_difference(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| left.len().min(right.len()))
}

fn is_xml_10_character(character: char) -> bool {
    matches!(
        character as u32,
        0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}
