//! The position cell (TCG-PROTOCOL-RUST §2): outer protocol positions
//! (1-based line, 0-based character) ↔ LSP positions (0-based line;
//! utf-8 byte offsets when the server granted utf-8 positionEncoding,
//! else UTF-16 code units converted through the line's text).

specmark::scope!(
    "spec://org.vibevm.ai-native/rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops"
);

use specmark::spec;

/// The encoding the server granted at initialize (ORACLE-RUST §2).
///
/// ```
/// use rust_ai_native_tcg_bridge::position::PositionEncoding;
/// assert_eq!(PositionEncoding::from_wire(Some("utf-8")), PositionEncoding::Utf8);
/// assert_eq!(PositionEncoding::from_wire(Some("utf-16")), PositionEncoding::Utf16);
/// assert_eq!(PositionEncoding::from_wire(None), PositionEncoding::Utf16);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionEncoding {
    Utf8,
    /// The LSP default; conversion goes through the line's text.
    Utf16,
}

impl PositionEncoding {
    pub fn from_wire(granted: Option<&str>) -> Self {
        match granted {
            Some("utf-8") => Self::Utf8,
            _ => Self::Utf16,
        }
    }
}

/// An outer-protocol position: 1-based line, 0-based character
/// (character = Unicode scalar count on the line, the tcg convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OuterPosition {
    pub line: u32,
    pub character: u32,
}

/// Convert an outer position to an LSP position against the line's
/// text. Characters beyond the line's end clamp to its end (the
/// forgiving read a completion caller wants at `foo.|`).
///
/// ```
/// use rust_ai_native_tcg_bridge::position::{
///     OuterPosition, PositionEncoding, to_lsp,
/// };
/// // "пример" — every char is 2 utf-8 bytes, 1 utf-16 unit.
/// let lsp = to_lsp(
///     OuterPosition { line: 3, character: 2 },
///     "пример",
///     PositionEncoding::Utf8,
/// );
/// assert_eq!((lsp.0, lsp.1), (2, 4));
/// let lsp16 = to_lsp(
///     OuterPosition { line: 3, character: 2 },
///     "пример",
///     PositionEncoding::Utf16,
/// );
/// assert_eq!((lsp16.0, lsp16.1), (2, 2));
/// ```
#[spec(
    implements = "spec://org.vibevm.ai-native/rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops"
)]
pub fn to_lsp(pos: OuterPosition, line_text: &str, encoding: PositionEncoding) -> (u32, u32) {
    let line = pos.line.saturating_sub(1);
    let take = pos.character as usize;
    let mut column: u32 = 0;
    for (i, ch) in line_text.chars().enumerate() {
        if i >= take {
            break;
        }
        column += match encoding {
            PositionEncoding::Utf8 => ch.len_utf8() as u32,
            PositionEncoding::Utf16 => ch.len_utf16() as u32,
        };
    }
    (line, column)
}

/// Convert an LSP position back to the outer convention against the
/// line's text (offsets snap to the nearest scalar boundary at or
/// before them — a defensive read of server output).
///
/// ```
/// use rust_ai_native_tcg_bridge::position::{
///     PositionEncoding, from_lsp,
/// };
/// let outer = from_lsp(2, 4, "пример", PositionEncoding::Utf8);
/// assert_eq!((outer.line, outer.character), (3, 2));
/// let outer16 = from_lsp(2, 2, "пример", PositionEncoding::Utf16);
/// assert_eq!((outer16.line, outer16.character), (3, 2));
/// ```
#[spec(
    implements = "spec://org.vibevm.ai-native/rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops"
)]
pub fn from_lsp(
    line: u32,
    column: u32,
    line_text: &str,
    encoding: PositionEncoding,
) -> OuterPosition {
    let mut used: u32 = 0;
    let mut character: u32 = 0;
    for ch in line_text.chars() {
        let width = match encoding {
            PositionEncoding::Utf8 => ch.len_utf8() as u32,
            PositionEncoding::Utf16 => ch.len_utf16() as u32,
        };
        if used + width > column {
            break;
        }
        used += width;
        character += 1;
        if used == column {
            break;
        }
    }
    OuterPosition {
        line: line + 1,
        character,
    }
}

#[cfg(test)]
mod tests {
    use super::{OuterPosition, PositionEncoding, from_lsp, to_lsp};

    #[test]
    fn ascii_is_identity_modulo_line_base() {
        let lsp = to_lsp(
            OuterPosition {
                line: 1,
                character: 5,
            },
            "let x = gre",
            PositionEncoding::Utf8,
        );
        assert_eq!(lsp, (0, 5));
        let back = from_lsp(0, 5, "let x = gre", PositionEncoding::Utf8);
        assert_eq!(
            back,
            OuterPosition {
                line: 1,
                character: 5
            }
        );
    }

    #[test]
    fn cyrillic_diverges_per_encoding_and_roundtrips() {
        // "привет мир" — 6 + 1 + 3 chars; utf-8 widths 2,2,2,2,2,2,1,2,2,2.
        let text = "привет мир";
        for enc in [PositionEncoding::Utf8, PositionEncoding::Utf16] {
            for character in [0u32, 3, 6, 7, 10] {
                let (l, c) = to_lsp(OuterPosition { line: 9, character }, text, enc);
                let back = from_lsp(l, c, text, enc);
                assert_eq!(back.character, character, "{enc:?} char {character}");
                assert_eq!(back.line, 9);
            }
        }
    }

    #[test]
    fn past_end_clamps_instead_of_panicking() {
        let (l, c) = to_lsp(
            OuterPosition {
                line: 2,
                character: 99,
            },
            "ab",
            PositionEncoding::Utf16,
        );
        assert_eq!((l, c), (1, 2));
    }

    #[test]
    fn surrogate_pair_widths_count_in_utf16() {
        // '𝄞' is 4 utf-8 bytes and 2 utf-16 units.
        let text = "𝄞x";
        let (_, c8) = to_lsp(
            OuterPosition {
                line: 1,
                character: 1,
            },
            text,
            PositionEncoding::Utf8,
        );
        let (_, c16) = to_lsp(
            OuterPosition {
                line: 1,
                character: 1,
            },
            text,
            PositionEncoding::Utf16,
        );
        assert_eq!(c8, 4);
        assert_eq!(c16, 2);
        assert_eq!(from_lsp(0, 2, text, PositionEncoding::Utf16).character, 1);
    }
}
