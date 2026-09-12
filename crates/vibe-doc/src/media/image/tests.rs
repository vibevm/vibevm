//! Unit tests for [`super`] — the header reader, exercised on bytes
//! built here rather than on fixture files, so every case is exactly the
//! header under test and nothing else.

use super::*;

/// A PNG that is nothing but its signature and IHDR — everything this
/// reader looks at.
pub(crate) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = PNG_MAGIC.to_vec();
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes
}

/// A JPEG carrying one APP0 segment before its start-of-frame, so the
/// segment walk is actually walked and not merely entered.
pub(crate) fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF];
    // APP0: marker, length 16, payload.
    bytes.pop();
    bytes.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    bytes.extend_from_slice(b"JFIF\0");
    bytes.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
    // SOF0: marker, length 17, precision, height, width, components.
    bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.push(3);
    bytes.extend_from_slice(&[1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    bytes
}

/// An extended-container WebP — the `VP8X` shape, which states the
/// canvas outright (minus one, little-endian, 24 bits each).
pub(crate) fn webp_vp8x(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(b"VP8X");
    bytes.extend_from_slice(&10u32.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    let w = width - 1;
    let h = height - 1;
    bytes.extend_from_slice(&w.to_le_bytes()[..3]);
    bytes.extend_from_slice(&h.to_le_bytes()[..3]);
    bytes
}

#[test]
fn each_permitted_format_is_read_from_its_own_header() {
    let cases: [(Vec<u8>, Format, (u32, u32)); 3] = [
        (png(512, 512), Format::Png, (512, 512)),
        (jpeg(1500, 500), Format::Jpeg, (1500, 500)),
        (webp_vp8x(1200, 630), Format::WebP, (1200, 630)),
    ];
    for (bytes, format, dimensions) in cases {
        let probe = Probe::of(&bytes);
        assert_eq!(probe.format, format);
        assert_eq!(probe.dimensions, Some(dimensions), "{}", format.as_str());
    }
}

/// The lossy and lossless WebP shapes carry the canvas in two other
/// places entirely; both are read, so a legal WebP is never reported as
/// unmeasurable merely for being the ordinary encoder's output.
#[test]
fn the_lossy_and_lossless_webp_shapes_are_read_too() {
    let mut lossy = b"RIFF".to_vec();
    lossy.extend_from_slice(&0u32.to_le_bytes());
    lossy.extend_from_slice(b"WEBP");
    lossy.extend_from_slice(b"VP8 ");
    lossy.extend_from_slice(&0u32.to_le_bytes());
    lossy.extend_from_slice(&[0x00, 0x00, 0x00]); // frame tag
    lossy.extend_from_slice(&[0x9D, 0x01, 0x2A]); // start code
    lossy.extend_from_slice(&640u16.to_le_bytes());
    lossy.extend_from_slice(&480u16.to_le_bytes());
    assert_eq!(Probe::of(&lossy).dimensions, Some((640, 480)));

    let mut lossless = b"RIFF".to_vec();
    lossless.extend_from_slice(&0u32.to_le_bytes());
    lossless.extend_from_slice(b"WEBP");
    lossless.extend_from_slice(b"VP8L");
    lossless.extend_from_slice(&0u32.to_le_bytes());
    lossless.push(0x2F);
    // 14 bits of (width - 1), then 14 bits of (height - 1).
    let packed: u32 = (256 - 1) | ((256 - 1) << 14);
    lossless.extend_from_slice(&packed.to_le_bytes());
    assert_eq!(Probe::of(&lossless).dimensions, Some((256, 256)));
}

/// SVG has no magic number, so it is recognised by its text — with or
/// without an XML prologue, with or without a BOM. The recognition is
/// what makes the refusal specific instead of «unrecognised format».
#[test]
fn svg_is_recognised_by_its_text_in_every_shape_it_arrives_in() {
    let shapes: [&[u8]; 3] = [
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
        b"<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\">\n<SVG/>",
        b"\xEF\xBB\xBF<svg/>",
    ];
    for bytes in shapes {
        assert_eq!(Probe::of(bytes).format, Format::Svg);
    }
}

#[test]
fn anything_else_is_unrecognised_and_measures_nothing() {
    for bytes in [&b"GIF89a"[..], &b""[..], &[0xFF, 0xD9][..]] {
        let probe = Probe::of(bytes);
        assert_eq!(probe.format, Format::Unknown);
        assert_eq!(probe.dimensions, None);
    }
}

/// A truncated or hostile header ends the walk; it never panics and
/// never loops. The dimensions come back absent, which the cell reports
/// as an unverified shape rather than a defect.
#[test]
fn a_truncated_header_answers_none_instead_of_panicking() {
    let full = png(64, 64);
    for cut in 8..full.len() {
        let probe = Probe::of(&full[..cut]);
        assert_eq!(probe.format, Format::Png);
        if cut < 24 {
            assert_eq!(probe.dimensions, None, "cut at {cut}");
        }
    }

    let full = jpeg(64, 64);
    for cut in 3..full.len() {
        let probe = Probe::of(&full[..cut]);
        assert_eq!(probe.format, Format::Jpeg);
    }

    // A segment claiming an impossible length, and a run of fill bytes:
    // both end the walk rather than running off or spinning.
    let hostile = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00];
    assert_eq!(Probe::of(&hostile).dimensions, None);
    let fill = [0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0xFF];
    assert_eq!(Probe::of(&fill).dimensions, None);
}
