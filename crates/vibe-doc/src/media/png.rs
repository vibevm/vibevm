//! A PNG writer, small enough to read in one sitting and deterministic
//! by construction (PROP-057 `##CARD-PREVIEW-COMPOSED`, PROP-044
//! `##M-CANONICAL-BYTES`).
//!
//! ## Why this is written here rather than taken from a crate
//!
//! The preview is a DERIVED artefact: the same coordinate and the same
//! title must produce the same bytes on every machine and in every build
//! configuration, or a site build's cache stops meaning anything and a
//! `--check` run compares noise. A general compressor cannot promise
//! that — this repository has already been bitten once by exactly that
//! property, when a golden corpus's `.gz` changed backend depending on
//! which crates shared the build graph, and the committed bytes stopped
//! matching the projected ones with nothing in the source having moved.
//!
//! So the encoder is ours and it is fixed: one deflate block, the fixed
//! Huffman table of RFC 1951 §3.2.6 (no tree to choose and therefore no
//! tie to break), and the only matches it emits are runs of one repeated
//! byte. That is not a compromise on size where it matters: a card is
//! flat colour, a vertical gradient and some glyphs, and after the `Sub`
//! filter a flat row is a row of zeros — which is precisely what a
//! distance-1 run encodes in three symbols.
//!
//! What it deliberately is not: a general PNG library. Truecolour, eight
//! bits, no interlace, no ancillary chunks. Anything else is somebody
//! else's picture, and this module never reads one — that is
//! [`super::image`]'s job.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PREVIEW-COMPOSED");

/// The eight bytes every PNG opens with.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Bytes per pixel in the one colour type this writer emits.
const CHANNELS: usize = 3;

/// The `Sub` filter: every byte is stored as its difference from the
/// pixel to its left. One filter for every row, chosen rather than
/// searched, because a search would make the output depend on a
/// heuristic instead of on the image.
const FILTER_SUB: u8 = 1;

/// Encode `rgb` — `width * height * 3` bytes, row-major, no padding — as
/// a PNG.
///
/// ```
/// let red = vec![0xFFu8, 0, 0, 0xFF, 0, 0, 0xFF, 0, 0, 0xFF, 0, 0];
/// let png = vibe_doc::media::png::encode_rgb(2, 2, &red);
/// assert_eq!(&png[1..4], b"PNG");
/// // Deterministic: the same pixels are the same file, always.
/// assert_eq!(png, vibe_doc::media::png::encode_rgb(2, 2, &red));
/// ```
pub fn encode_rgb(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(SIGNATURE);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    // depth 8, colour type 2 (truecolour), deflate, adaptive filtering,
    // no interlace — the whole shape of every file this writer makes.
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);

    let raw = filter_rows(width, height, rgb);
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// One chunk: length, type, payload, CRC over type and payload.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = Crc::new();
    crc.push(kind);
    crc.push(data);
    out.extend_from_slice(&crc.finish().to_be_bytes());
}

/// The scanline stream a PNG decoder sees: each row prefixed by its
/// filter byte, each byte stored as its difference from the pixel three
/// bytes to its left.
fn filter_rows(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let stride = width as usize * CHANNELS;
    let mut out = Vec::with_capacity((stride + 1) * height as usize);
    for y in 0..height as usize {
        out.push(FILTER_SUB);
        let row = &rgb[y * stride..(y + 1) * stride];
        for (x, byte) in row.iter().enumerate() {
            let left = if x >= CHANNELS { row[x - CHANNELS] } else { 0 };
            out.push(byte.wrapping_sub(left));
        }
    }
    out
}

/// The zlib container: the two-byte header whose check value is fixed,
/// one deflate block, and Adler-32 of the uncompressed bytes.
fn zlib(raw: &[u8]) -> Vec<u8> {
    // 0x78 0x01: deflate, 32 KiB window, «fastest» level. The pair is
    // divisible by 31, which is the whole of the header's check rule.
    let mut out = vec![0x78, 0x01];
    out.extend_from_slice(&deflate_fixed(raw));
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// One final block under the fixed Huffman table.
///
/// The matcher looks at exactly one thing: how many bytes from here on
/// repeat the byte before them. That is a run, it is what a filtered
/// flat row is made of, and it costs three symbols however long it gets.
/// A general matcher would buy a few more percent and would have to
/// choose between equally good candidates — a choice this module has no
/// way to make identically forever.
fn deflate_fixed(raw: &[u8]) -> Vec<u8> {
    let mut bits = BitWriter::default();
    bits.bit(1); // BFINAL
    bits.lsb(0b01, 2); // BTYPE = fixed Huffman

    let mut i = 0usize;
    while i < raw.len() {
        let run = run_length(raw, i);
        if run >= MIN_MATCH {
            let (code, extra_bits, extra) = length_code(run);
            let (c, n) = literal_code(code);
            bits.code(c, n);
            bits.lsb(extra, extra_bits);
            // Distance 1 is distance code 0, five bits, no extra.
            bits.code(0, 5);
            i += run;
            continue;
        }
        let (c, n) = literal_code(u16::from(raw[i]));
        bits.code(c, n);
        i += 1;
    }
    let (c, n) = literal_code(END_OF_BLOCK);
    bits.code(c, n);
    bits.finish()
}

/// The shortest run deflate can encode at all.
const MIN_MATCH: usize = 3;
/// The longest one symbol pair can carry.
const MAX_MATCH: usize = 258;
/// The literal/length alphabet's end-of-block symbol.
const END_OF_BLOCK: u16 = 256;

/// How many bytes starting at `at` repeat the byte before `at`.
fn run_length(raw: &[u8], at: usize) -> usize {
    if at == 0 {
        return 0;
    }
    let repeated = raw[at - 1];
    let mut run = 0usize;
    while run < MAX_MATCH && raw.get(at + run) == Some(&repeated) {
        run += 1;
    }
    run
}

/// `(symbol, extra-bit count, extra-bit value)` for a match length —
/// RFC 1951 §3.2.5. The table is written out rather than computed: it is
/// a specification, and arithmetic that reproduces it would be a second
/// statement of the same thing.
fn length_code(length: usize) -> (u16, u32, u32) {
    const BASE: [(u16, usize, u32); 29] = [
        (257, 3, 0),
        (258, 4, 0),
        (259, 5, 0),
        (260, 6, 0),
        (261, 7, 0),
        (262, 8, 0),
        (263, 9, 0),
        (264, 10, 0),
        (265, 11, 1),
        (266, 13, 1),
        (267, 15, 1),
        (268, 17, 1),
        (269, 19, 2),
        (270, 23, 2),
        (271, 27, 2),
        (272, 31, 2),
        (273, 35, 3),
        (274, 43, 3),
        (275, 51, 3),
        (276, 59, 3),
        (277, 67, 4),
        (278, 83, 4),
        (279, 99, 4),
        (280, 115, 4),
        (281, 131, 5),
        (282, 163, 5),
        (283, 195, 5),
        (284, 227, 5),
        (285, 258, 0),
    ];
    let mut chosen = BASE[0];
    for row in BASE {
        if row.1 <= length {
            chosen = row;
        }
    }
    (chosen.0, chosen.2, (length - chosen.1) as u32)
}

/// `(code, bit count)` of one literal/length symbol under the fixed
/// table — RFC 1951 §3.2.6.
fn literal_code(symbol: u16) -> (u32, u32) {
    match symbol {
        0..=143 => (0x30 + u32::from(symbol), 8),
        144..=255 => (0x190 + u32::from(symbol) - 144, 9),
        256..=279 => (u32::from(symbol) - 256, 7),
        _ => (0xC0 + u32::from(symbol) - 280, 8),
    }
}

/// Bits into bytes, least significant first — deflate's own order.
#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    acc: u8,
    filled: u32,
}

impl BitWriter {
    fn bit(&mut self, value: u32) {
        self.acc |= ((value & 1) as u8) << self.filled;
        self.filled += 1;
        if self.filled == 8 {
            self.out.push(self.acc);
            self.acc = 0;
            self.filled = 0;
        }
    }

    /// A plain field: its own least significant bit first.
    fn lsb(&mut self, value: u32, count: u32) {
        for shift in 0..count {
            self.bit(value >> shift);
        }
    }

    /// A Huffman code: most significant bit first, which is the one
    /// place deflate reverses itself.
    fn code(&mut self, value: u32, count: u32) {
        for shift in (0..count).rev() {
            self.bit(value >> shift);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.filled > 0 {
            self.out.push(self.acc);
        }
        self.out
    }
}

/// Adler-32 of the uncompressed stream.
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// CRC-32 as PNG spells it, table built on first use.
struct Crc(u32);

impl Crc {
    fn new() -> Crc {
        Crc(0xFFFF_FFFF)
    }

    fn push(&mut self, data: &[u8]) {
        for byte in data {
            let index = ((self.0 ^ u32::from(*byte)) & 0xFF) as usize;
            self.0 = CRC_TABLE[index] ^ (self.0 >> 8);
        }
    }

    fn finish(self) -> u32 {
        self.0 ^ 0xFFFF_FFFF
    }
}

/// The CRC table, computed once at compile time so the writer carries no
/// initialisation order of its own.
const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0usize;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

#[cfg(test)]
mod tests;
