//! Mechanical offsets on exact retained UTF-8 text; no resource authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDITOR-POSITIONS");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorEncoding {
    Utf8,
    Utf16,
    Utf32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorPosition {
    pub line: usize,
    pub character: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionError {
    InvalidUtf8,
    OutOfRange,
    SplitCharacter,
    SplitNewline,
    NoEncodingIntersection,
}
pub fn negotiate_encoding(offered: &[EditorEncoding]) -> Result<EditorEncoding, PositionError> {
    offered
        .first()
        .copied()
        .ok_or(PositionError::NoEncodingIntersection)
}
fn units(c: char, encoding: EditorEncoding) -> usize {
    match encoding {
        EditorEncoding::Utf8 => c.len_utf8(),
        EditorEncoding::Utf16 => c.len_utf16(),
        EditorEncoding::Utf32 => 1,
    }
}
fn lines(text: &str) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, c) in text.char_indices() {
        if c == '\n' {
            let end = if index > start && text.as_bytes()[index - 1] == b'\r' {
                index - 1
            } else {
                index
            };
            result.push((start, end));
            start = index + 1;
        } else if c == '\r' && text.as_bytes().get(index + 1) != Some(&b'\n') {
            result.push((start, index));
            start = index + 1;
        }
    }
    result.push((start, text.len()));
    result
}
pub fn byte_to_position(
    raw: &[u8],
    offset: usize,
    encoding: EditorEncoding,
) -> Result<EditorPosition, PositionError> {
    let text = std::str::from_utf8(raw).map_err(|_| PositionError::InvalidUtf8)?;
    if offset > text.len() {
        return Err(PositionError::OutOfRange);
    }
    if !text.is_char_boundary(offset) {
        return Err(PositionError::SplitCharacter);
    }
    for (line, (start, end)) in lines(text).into_iter().enumerate() {
        if offset >= start && offset <= end {
            return Ok(EditorPosition {
                line,
                character: text[start..offset]
                    .chars()
                    .map(|c| units(c, encoding))
                    .sum(),
            });
        }
    }
    Err(PositionError::SplitNewline)
}
pub fn position_to_byte(
    raw: &[u8],
    position: EditorPosition,
    encoding: EditorEncoding,
) -> Result<usize, PositionError> {
    let text = std::str::from_utf8(raw).map_err(|_| PositionError::InvalidUtf8)?;
    let (start, end) = *lines(text)
        .get(position.line)
        .ok_or(PositionError::OutOfRange)?;
    let mut column = 0;
    for (index, c) in text[start..end].char_indices() {
        if column == position.character {
            return Ok(start + index);
        }
        column += units(c, encoding);
        if column > position.character {
            return Err(PositionError::SplitCharacter);
        }
    }
    if column == position.character {
        Ok(end)
    } else {
        Err(PositionError::OutOfRange)
    }
}
#[cfg(test)]
mod m_16_d_2_cases {
    use super::*;
    #[test]
    fn unicode_crlf_roundtrip() {
        let raw = "a😀e\u{301}\r\nЖ\rZ\n".as_bytes();
        for (encoding, columns) in [
            (EditorEncoding::Utf8, [0, 1, 5, 6, 8]),
            (EditorEncoding::Utf16, [0, 1, 3, 4, 5]),
            (EditorEncoding::Utf32, [0, 1, 2, 3, 4]),
        ] {
            for (byte, column) in [0, 1, 5, 6, 8].into_iter().zip(columns) {
                let pos = EditorPosition {
                    line: 0,
                    character: column,
                };
                assert_eq!(byte_to_position(raw, byte, encoding), Ok(pos));
                assert_eq!(position_to_byte(raw, pos, encoding), Ok(byte));
            }
            for (byte, line) in [(10, 1), (13, 2), (15, 3)] {
                let pos = EditorPosition { line, character: 0 };
                assert_eq!(byte_to_position(raw, byte, encoding), Ok(pos));
                assert_eq!(position_to_byte(raw, pos, encoding), Ok(byte));
            }
        }
    }
    #[test]
    fn invalid_boundaries_and_negotiation() {
        let raw = "😀\r\n".as_bytes();
        assert_eq!(
            byte_to_position(raw, 1, EditorEncoding::Utf8),
            Err(PositionError::SplitCharacter)
        );
        assert_eq!(
            byte_to_position(raw, 5, EditorEncoding::Utf8),
            Err(PositionError::SplitNewline)
        );
        assert_eq!(
            position_to_byte(
                raw,
                EditorPosition {
                    line: 0,
                    character: 1
                },
                EditorEncoding::Utf16
            ),
            Err(PositionError::SplitCharacter)
        );
        assert_eq!(
            byte_to_position(b"\xff", 0, EditorEncoding::Utf32),
            Err(PositionError::InvalidUtf8)
        );
        assert_eq!(
            negotiate_encoding(&[]),
            Err(PositionError::NoEncodingIntersection)
        );
        assert_eq!(
            negotiate_encoding(&[EditorEncoding::Utf16, EditorEncoding::Utf8]),
            Ok(EditorEncoding::Utf16)
        );
    }
}
