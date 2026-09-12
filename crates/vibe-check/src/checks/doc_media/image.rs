//! What a file's first bytes say it is, and how large its canvas is —
//! a header reader, not a decoder.
//!
//! The cell needs exactly two facts about an image and no more: which of
//! the three permitted formats it is (or that it is SVG, or neither),
//! and its pixel dimensions. Both live in the first few dozen bytes of
//! every format here, so nothing is decoded, no pixel is touched, and a
//! malformed tail can never make the check panic or hang. A header this
//! reader cannot follow yields `None` dimensions, which the cell reports
//! as an unverified shape rather than a defect — the honest answer when
//! the bytes are a picture and only the measurement failed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#card");

/// The formats the card distinguishes: the three it permits, the one it
/// names in its refusal, and everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Format {
    Png,
    Jpeg,
    WebP,
    /// Refused by name because the refusal has a reason of its own —
    /// SVG can carry script (D-20).
    Svg,
    Unknown,
}

impl Format {
    /// The word a finding spells.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Format::Png => "PNG",
            Format::Jpeg => "JPEG",
            Format::WebP => "WebP",
            Format::Svg => "SVG",
            Format::Unknown => "unrecognised",
        }
    }
}

/// What the header reader learned about one file.
pub(super) struct Probe {
    pub(super) format: Format,
    /// `(width, height)` in pixels, or `None` when the format is known
    /// but its header could not be followed.
    pub(super) dimensions: Option<(u32, u32)>,
}

impl Probe {
    /// Read `bytes`' first bytes. Never fails: an unreadable shape is
    /// absence, never an error, and an unknown format is a verdict.
    pub(super) fn of(bytes: &[u8]) -> Probe {
        if bytes.starts_with(PNG_MAGIC) {
            return Probe {
                format: Format::Png,
                dimensions: png_dimensions(bytes),
            };
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Probe {
                format: Format::Jpeg,
                dimensions: jpeg_dimensions(bytes),
            };
        }
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Probe {
                format: Format::WebP,
                dimensions: webp_dimensions(bytes),
            };
        }
        Probe {
            format: if looks_like_svg(bytes) {
                Format::Svg
            } else {
                Format::Unknown
            },
            dimensions: None,
        }
    }
}

/// The eight bytes every PNG opens with.
const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// SVG has no magic number — it is XML — so it is recognised the only
/// way it can be: the first non-blank text names it. Both an `<svg`
/// root and an XML prologue followed by one count, and a UTF-8 BOM is
/// stepped over first.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    // A prologue plus a DOCTYPE is comfortably inside this window, and
    // the window keeps the scan bounded whatever the file's size.
    let head = &body[..body.len().min(1024)];
    let Ok(text) = std::str::from_utf8(head) else {
        return false;
    };
    let lowered = text.to_ascii_lowercase();
    lowered.contains("<svg")
        || (lowered.trim_start().starts_with("<?xml") && lowered.contains("svg"))
}

/// A big-endian `u32` at `offset`, or `None` past the end.
fn be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// A big-endian `u16` at `offset`, or `None` past the end.
fn be_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

/// PNG: the IHDR chunk is mandatory and first, so width and height sit
/// at fixed offsets 16 and 20.
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    Some((be_u32(bytes, 16)?, be_u32(bytes, 20)?))
}

/// JPEG: walk the marker segments to the first start-of-frame, whose
/// payload carries height then width. Every step is bounds-checked and
/// the walk only ever moves forward, so a truncated or hostile file ends
/// the scan instead of looping.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut index = 2usize;
    loop {
        // Fill bytes (0xFF) may pad before a marker.
        while bytes.get(index) == Some(&0xFF) && bytes.get(index + 1) == Some(&0xFF) {
            index += 1;
        }
        if bytes.get(index)? != &0xFF {
            return None;
        }
        let marker = *bytes.get(index + 1)?;
        // Standalone markers carry no length payload.
        if (0xD0..=0xD9).contains(&marker) || marker == 0x01 {
            index += 2;
            continue;
        }
        let length = usize::from(be_u16(bytes, index + 2)?);
        if length < 2 {
            return None;
        }
        // SOF0…SOF15, minus the four that are not frame headers.
        let is_frame = (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_frame {
            let height = be_u16(bytes, index + 5)?;
            let width = be_u16(bytes, index + 7)?;
            return Some((u32::from(width), u32::from(height)));
        }
        index = index.checked_add(2)?.checked_add(length)?;
    }
}

/// WebP: three container shapes, one answer. `VP8X` states the canvas
/// outright; `VP8 ` (lossy) hides it in the key-frame header behind a
/// start code; `VP8L` (lossless) packs 14-bit width and height into
/// bitfields after its own signature byte.
fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let chunk = bytes.get(12..16)?;
    match chunk {
        b"VP8X" => {
            // Canvas size is stored minus one, little-endian, 24 bits each.
            let w = bytes.get(24..27)?;
            let h = bytes.get(27..30)?;
            let width = u32::from(w[0]) | u32::from(w[1]) << 8 | u32::from(w[2]) << 16;
            let height = u32::from(h[0]) | u32::from(h[1]) << 8 | u32::from(h[2]) << 16;
            Some((width + 1, height + 1))
        }
        b"VP8 " => {
            // 20..23 is the frame tag; the 3-byte start code follows.
            if bytes.get(23..26)? != [0x9D, 0x01, 0x2A] {
                return None;
            }
            let w = bytes.get(26..28)?;
            let h = bytes.get(28..30)?;
            // 14 bits of size, 2 bits of scale.
            let width = (u32::from(w[0]) | u32::from(w[1]) << 8) & 0x3FFF;
            let height = (u32::from(h[0]) | u32::from(h[1]) << 8) & 0x3FFF;
            Some((width, height))
        }
        b"VP8L" => {
            if bytes.get(20)? != &0x2F {
                return None;
            }
            let b = bytes.get(21..25)?;
            let bits = u32::from(b[0])
                | u32::from(b[1]) << 8
                | u32::from(b[2]) << 16
                | u32::from(b[3]) << 24;
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "image/tests.rs"]
pub(super) mod tests;
