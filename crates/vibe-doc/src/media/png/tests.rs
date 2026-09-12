//! The writer is checked by READING what it wrote.
//!
//! An encoder tested against its own expectations tests nothing: it
//! agrees with itself by construction. So the tests here carry a
//! decoder — small, because the writer's output is small in shape: one
//! fixed-Huffman block whose only matches sit at distance one. A stream
//! this decoder cannot follow is a stream no decoder can, which is the
//! failure worth catching.

use super::*;

/// Inflate one fixed-Huffman block. Panics on anything the writer is not
/// supposed to emit — that IS the assertion.
fn inflate_fixed(stream: &[u8]) -> Vec<u8> {
    let mut reader = Bits {
        data: stream,
        at: 0,
    };
    assert_eq!(reader.lsb(1), 1, "the writer emits one final block");
    assert_eq!(reader.lsb(2), 0b01, "the writer emits fixed Huffman");
    let mut out: Vec<u8> = Vec::new();
    loop {
        let symbol = reader.symbol();
        match symbol {
            256 => return out,
            0..=255 => out.push(symbol as u8),
            _ => {
                let (base, extra) = LENGTHS[(symbol - 257) as usize];
                let length = base + reader.lsb(extra) as usize;
                let distance_code = reader.code(5);
                assert_eq!(distance_code, 0, "the writer only emits distance 1");
                for _ in 0..length {
                    let byte = out[out.len() - 1];
                    out.push(byte);
                }
            }
        }
    }
}

/// `(base length, extra bits)` per length symbol, 257 first.
const LENGTHS: [(usize, u32); 29] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),
    (258, 0),
];

struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn next(&mut self) -> u32 {
        let bit = (self.data[self.at / 8] >> (self.at % 8)) & 1;
        self.at += 1;
        u32::from(bit)
    }

    /// A plain field: least significant bit first.
    fn lsb(&mut self, count: u32) -> u32 {
        let mut value = 0u32;
        for shift in 0..count {
            value |= self.next() << shift;
        }
        value
    }

    /// A Huffman code: most significant bit first.
    fn code(&mut self, count: u32) -> u32 {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | self.next();
        }
        value
    }

    fn symbol(&mut self) -> u16 {
        let seven = self.code(7);
        if seven <= 0x17 {
            return (256 + seven) as u16;
        }
        let eight = (seven << 1) | self.next();
        if (0x30..=0xBF).contains(&eight) {
            return (eight - 0x30) as u16;
        }
        if (0xC0..=0xC7).contains(&eight) {
            return (280 + eight - 0xC0) as u16;
        }
        let nine = (eight << 1) | self.next();
        assert!((0x190..=0x1FF).contains(&nine), "not a fixed-table code");
        (144 + nine - 0x190) as u16
    }
}

/// Undo the writer's framing: signature, chunks, zlib header, filters.
fn decode(png: &[u8], width: u32, height: u32) -> Vec<u8> {
    assert_eq!(&png[..8], SIGNATURE);
    let mut at = 8usize;
    let mut idat: Vec<u8> = Vec::new();
    let mut saw_end = false;
    while at < png.len() {
        let length = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let kind = &png[at + 4..at + 8];
        let data = &png[at + 8..at + 8 + length];
        // Every chunk's CRC is checked, so a framing slip is caught here
        // rather than by whatever opens the file next.
        let mut crc = Crc::new();
        crc.push(kind);
        crc.push(data);
        let stated = u32::from_be_bytes(png[at + 8 + length..at + 12 + length].try_into().unwrap());
        assert_eq!(
            crc.finish(),
            stated,
            "chunk `{}` CRC",
            String::from_utf8_lossy(kind)
        );
        if kind == b"IDAT" {
            idat.extend_from_slice(data);
        }
        if kind == b"IEND" {
            saw_end = true;
        }
        at += 12 + length;
    }
    assert!(saw_end, "the file ends with IEND");

    assert_eq!(&idat[..2], &[0x78, 0x01], "zlib header");
    let body = &idat[2..idat.len() - 4];
    let raw = inflate_fixed(body);
    let stated = u32::from_be_bytes(idat[idat.len() - 4..].try_into().unwrap());
    assert_eq!(adler32(&raw), stated, "adler32 of the scanlines");

    let stride = width as usize * CHANNELS;
    let mut pixels = Vec::with_capacity(stride * height as usize);
    for y in 0..height as usize {
        let row = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        assert_eq!(row[0], FILTER_SUB);
        let start = pixels.len();
        for (x, byte) in row[1..].iter().enumerate() {
            let left = if x >= CHANNELS {
                pixels[start + x - CHANNELS]
            } else {
                0
            };
            pixels.push(byte.wrapping_add(left));
        }
    }
    pixels
}

fn gradient(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        let shade = (y * 255 / height.max(1)) as u8;
        for _ in 0..width {
            out.extend_from_slice(&[shade, shade / 2, 255 - shade]);
        }
    }
    out
}

/// The one law that matters: what comes out is what went in.
#[test]
fn a_written_image_reads_back_pixel_for_pixel() {
    for (w, h) in [(1u32, 1u32), (2, 3), (17, 5), (64, 64)] {
        let pixels = gradient(w, h);
        let png = encode_rgb(w, h, &pixels);
        assert_eq!(decode(&png, w, h), pixels, "{w}×{h}");
    }
}

/// Runs are what a filtered flat row is made of, and encoding them is
/// the only reason this writer compresses at all. A large flat image
/// must come back exactly and must not be stored byte for byte.
#[test]
fn a_flat_image_round_trips_and_is_compressed() {
    let (w, h) = (200u32, 200u32);
    let pixels = vec![0x3Cu8; (w * h * 3) as usize];
    let png = encode_rgb(w, h, &pixels);
    assert_eq!(decode(&png, w, h), pixels);
    assert!(
        (png.len() as u32) < w * h,
        "a flat card compresses well past its pixel count, got {} bytes",
        png.len()
    );
}

/// The property the site build depends on: one image, one file, forever.
/// A writer that chose between equally good encodings would break this
/// silently and only on somebody else's machine.
#[test]
fn the_same_pixels_are_the_same_bytes() {
    let pixels = gradient(40, 40);
    assert_eq!(encode_rgb(40, 40, &pixels), encode_rgb(40, 40, &pixels));
}

/// The header a reader trusts before it decodes anything: this project's
/// own probe must recognise what this writer produced, including the
/// canvas size. The two halves of the media pipeline agree or neither is
/// worth anything.
#[test]
fn this_projects_own_probe_recognises_what_this_writer_emits() {
    let png = encode_rgb(120, 63, &gradient(120, 63));
    let probe = crate::media::image::Probe::of(&png);
    assert_eq!(probe.format, crate::media::image::Format::Png);
    assert_eq!(probe.dimensions, Some((120, 63)));
}
