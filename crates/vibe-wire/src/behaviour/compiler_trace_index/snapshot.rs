//! `snapshot-portability` — the ONE canonical filename a trace snapshot
//! may carry, CONSTRUCTED and compared, never pattern-matched.
//!
//! An allowlist grammar ("these characters, these escapes") answers the
//! wrong question. It admits `0000-parse-node_x_y-000.json` for an event
//! whose pass was `close`, admits `000-…` for sequence 0, admits `%41`
//! beside `A`, and admits `~deadbeef…` for a digest that was never
//! computed. Every one of those is a filename that no longer round-trips
//! back to the event that wrote it — and reversibility is the whole
//! point of the codec (architecture v0.1 §4.1, R3.4 refresh ruling 3).
//!
//! So this cell builds what the event and its referenced scope REQUIRE
//! and compares. Two forms are admissible, and no third:
//!
//! ```text
//! <seq:04>-<enc(pass)>-<kind>_<enc(label)>_<enc(artifact)>-<ord:03>.json
//! <seq:04>-~<digest16>-<ord:03>.json
//! ```
//!
//! `enc` leaves ONLY `[A-Za-z0-9.]` raw and percent-escapes every other
//! UTF-8 byte as uppercase `%XX` — `%` and `~` included — so `-` and `_`
//! cannot occur inside a component and the layout split is total. Widths
//! are Rust minimum widths: zero-padded to 4 and 3, wider when the value
//! needs it. `digest16` is the first 16 lowercase hex characters of the
//! SHA-256 of the encoded middle `<enc(pass)>-<kind>_<enc(label)>_<enc(artifact)>`,
//! and the validator RECOMPUTES it — a plausible-looking `~` plus hex is
//! not a digest. The short form is admissible whenever the writer chose
//! it, because the pressure that forces it (the absolute run directory
//! against Windows MAX_PATH, F19) is invisible from inside the index.
//!
//! Both forms are one path component of at most [`SNAPSHOT_NAME_CAP`]
//! bytes. Windows-safety falls out rather than being enumerated: the
//! alphabet excludes `<>:"/\|?*` and every control byte, the name starts
//! with a digit so it cannot be a device stem, and the byte before
//! `.json` is a digit so there is no trailing dot or space.
//!
//! Nothing here allocates in proportion to its input. An over-cap
//! filename is refused before anything is built; the full-name builder
//! stops at the cap, so a multi-megabyte label costs a bounded number of
//! iterations and one ≤96-byte string; the digest streams through a
//! fixed buffer.
//!
//! The WRITER reaches the very same codec through [`SnapshotName`]: a
//! durable recorder picks the name it is about to publish from the
//! builder its own validator will later reconstruct, so the producer and
//! the reader cannot drift apart over two copies of a percent encoder,
//! a digest width or a pad.

use sha2::{Digest, Sha256};

use crate::generated::compiler_trace_index::e1::index::ScopeKind;

use super::errors::{ScalarPreview, SnapshotUnsafety};

/// The byte ceiling both canonical forms obey — one path component,
/// short enough to survive a deep run directory.
pub const SNAPSHOT_NAME_CAP: usize = 96;

/// Hex characters of the middle's SHA-256 the short form retains.
pub const SHORT_DIGEST_HEX: usize = 16;

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";
const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

/// The wire spelling of a scope kind — the filename's `kind` field.
pub(super) fn kind_spelling(kind: &ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Node => "node",
        ScopeKind::Unit => "unit",
        ScopeKind::Publish => "publish",
    }
}

/// Everything the filename must spell, gathered from the event and the
/// scope it references.
pub(super) struct SnapshotIdentity<'a> {
    pub(super) sequence: u32,
    pub(super) invocation: u32,
    pub(super) kind: &'static str,
    pub(super) pass: &'a str,
    pub(super) label: &'a str,
    pub(super) artifact: &'a str,
}

/// `None` when the filename is one of the two names this event may
/// write; otherwise the reason, carrying what was expected instead.
pub(super) fn snapshot_unsafety(
    filename: &str,
    identity: &SnapshotIdentity<'_>,
) -> Option<SnapshotUnsafety> {
    // First, and before any name is built: an over-cap filename cannot
    // equal either canonical form, so refusing it here keeps the cost of
    // a hostile document constant.
    if filename.len() > SNAPSHOT_NAME_CAP {
        return Some(SnapshotUnsafety::TooLong {
            bytes: filename.len(),
        });
    }
    let full = identity.full_name();
    if full.as_deref() == Some(filename) {
        return None;
    }
    let short = identity.short_name();
    if filename == short {
        return None;
    }
    Some(SnapshotUnsafety::NotCanonical {
        full: full.as_deref().map(ScalarPreview::of),
        short: ScalarPreview::of(&short),
    })
}

/// The name a WRITER is about to give one certified snapshot — the
/// public half of this cell, built from exactly what the event and its
/// scope already say.
///
/// It exists so a durable recorder never carries its own copy of the
/// percent encoder, the digest width or the zero pads: the writer asks
/// this builder, and the validator later rebuilds the same two forms
/// from the index and compares. `kind` is the GENERATED scope-kind
/// value, so a caller cannot spell a kind the epoch does not have.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotName<'a> {
    /// Dense global run sequence — the filename's decimal prefix.
    pub sequence: u32,
    /// Dense `(scope, pass)` invocation ordinal.
    pub invocation: u32,
    /// Which kind of compilation the owning scope names.
    pub kind: &'a ScopeKind,
    /// The exact pass name, unencoded.
    pub pass: &'a str,
    /// The owning scope's exact unencoded label.
    pub label: &'a str,
    /// The owning scope's artifact id.
    pub artifact: &'a str,
}

impl SnapshotName<'_> {
    fn identity(&self) -> SnapshotIdentity<'_> {
        SnapshotIdentity {
            sequence: self.sequence,
            invocation: self.invocation,
            kind: kind_spelling(self.kind),
            pass: self.pass,
            label: self.label,
            artifact: self.artifact,
        }
    }

    /// The full canonical spelling, or `None` when it would pass
    /// [`SNAPSHOT_NAME_CAP`] — in which case only [`short`](Self::short)
    /// is admissible.
    #[must_use]
    pub fn full(&self) -> Option<String> {
        self.identity().full_name()
    }

    /// The short canonical spelling. Bounded by construction, so it
    /// always exists — but a caller with its own ceiling still has to
    /// check that it fits, which is what [`within`](Self::within) does.
    #[must_use]
    pub fn short(&self) -> String {
        self.identity().short_name()
    }

    /// The name to publish under a caller's own byte ceiling: the full
    /// form when it fits, otherwise the short one, and `None` when even
    /// the short form does not fit.
    ///
    /// The caller's ceiling is the pressure the index cannot see — the
    /// absolute run directory against Windows `MAX_PATH` — and it is
    /// always taken together with the epoch's own cap, never above it.
    /// `None` is a real answer: a run directory that cannot afford 31
    /// bytes of filename must refuse rather than publish a truncated
    /// name no validator would reconstruct.
    #[must_use]
    pub fn within(&self, cap: usize) -> Option<String> {
        let cap = cap.min(SNAPSHOT_NAME_CAP);
        let identity = self.identity();
        if let Some(full) = identity.full_name_within(cap) {
            return Some(full);
        }
        let short = identity.short_name();
        (short.len() <= cap).then_some(short)
    }
}

impl SnapshotIdentity<'_> {
    /// The full canonical name, or `None` when it would pass the epoch's
    /// cap — in which case the short form is the only admissible
    /// spelling.
    fn full_name(&self) -> Option<String> {
        self.full_name_within(SNAPSHOT_NAME_CAP)
    }

    /// The same construction under an arbitrary ceiling at or below the
    /// epoch's own, so a writer under path pressure and the validator
    /// run one builder rather than two.
    fn full_name_within(&self, cap: usize) -> Option<String> {
        let mut name = Capped::new(cap.min(SNAPSHOT_NAME_CAP));
        write_number(&mut name, self.sequence, 4);
        name.ascii(b'-');
        write_encoded(&mut name, self.pass);
        name.ascii(b'-');
        write_raw(&mut name, self.kind);
        name.ascii(b'_');
        write_encoded(&mut name, self.label);
        name.ascii(b'_');
        write_encoded(&mut name, self.artifact);
        name.ascii(b'-');
        write_number(&mut name, self.invocation, 3);
        write_raw(&mut name, ".json");
        name.finish()
    }

    /// The short canonical name. Bounded by construction — two decimal
    /// numbers, sixteen hex characters and eight literal bytes — so it
    /// always exists.
    fn short_name(&self) -> String {
        let digest = self.middle_digest();
        let mut name = String::with_capacity(48);
        write_number(&mut name, self.sequence, 4);
        write_raw(&mut name, "-~");
        write_raw(&mut name, &digest);
        name.ascii(b'-');
        write_number(&mut name, self.invocation, 3);
        write_raw(&mut name, ".json");
        name
    }

    /// The first [`SHORT_DIGEST_HEX`] lowercase hex characters of the
    /// SHA-256 of the canonical encoded middle. Streamed: the hasher
    /// never sees a materialised copy of the middle.
    fn middle_digest(&self) -> String {
        let mut sink = DigestSink::new();
        write_encoded(&mut sink, self.pass);
        sink.ascii(b'-');
        write_raw(&mut sink, self.kind);
        sink.ascii(b'_');
        write_encoded(&mut sink, self.label);
        sink.ascii(b'_');
        write_encoded(&mut sink, self.artifact);
        sink.finish()
    }
}

/// One ASCII byte at a time — the only thing the three sinks (the capped
/// full name, the plain short name, the hasher) have in common, and what
/// lets one encoder feed all of them.
trait PushAscii {
    fn ascii(&mut self, byte: u8);
    /// Whether further bytes would be discarded, so an encoder walking a
    /// huge value can stop instead of running to its end.
    fn is_full(&self) -> bool {
        false
    }
}

impl PushAscii for String {
    fn ascii(&mut self, byte: u8) {
        self.push(byte as char);
    }
}

fn write_raw(out: &mut impl PushAscii, value: &str) {
    for byte in value.bytes() {
        out.ascii(byte);
    }
}

/// `enc`: `[A-Za-z0-9.]` raw, every other UTF-8 byte uppercase `%XX`.
fn write_encoded(out: &mut impl PushAscii, value: &str) {
    for byte in value.bytes() {
        if out.is_full() {
            return;
        }
        if byte.is_ascii_alphanumeric() || byte == b'.' {
            out.ascii(byte);
        } else {
            out.ascii(b'%');
            out.ascii(HEX_UPPER[usize::from(byte >> 4)]);
            out.ascii(HEX_UPPER[usize::from(byte & 0x0f)]);
        }
    }
}

/// A decimal number zero-padded to a MINIMUM width — a value that needs
/// more digits uses them all, exactly as Rust's `{:04}` does.
fn write_number(out: &mut impl PushAscii, value: u32, width: usize) {
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut rest = value;
    loop {
        // `rest % 10` is in 0..=9, so this indexes the decimal prefix of
        // the hex table — no narrowing cast, no fallible conversion.
        digits[count] = HEX_LOWER[(rest % 10) as usize];
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    for _ in count..width {
        out.ascii(b'0');
    }
    for index in (0..count).rev() {
        out.ascii(digits[index]);
    }
}

/// A name builder that stops at a cap instead of growing: the whole
/// reason a multi-megabyte pass name costs a bounded refusal.
struct Capped {
    text: String,
    cap: usize,
    overflowed: bool,
}

impl Capped {
    fn new(cap: usize) -> Self {
        Capped {
            text: String::with_capacity(cap),
            cap,
            overflowed: false,
        }
    }

    /// The built name, or `None` if it wanted more than the cap.
    fn finish(self) -> Option<String> {
        (!self.overflowed).then_some(self.text)
    }
}

impl PushAscii for Capped {
    fn ascii(&mut self, byte: u8) {
        if self.overflowed {
            return;
        }
        if self.text.len() == self.cap {
            self.overflowed = true;
            return;
        }
        self.text.push(byte as char);
    }

    fn is_full(&self) -> bool {
        self.overflowed
    }
}

/// A fixed-buffer streaming sink for the middle's SHA-256.
struct DigestSink {
    hasher: Sha256,
    buffer: [u8; 64],
    filled: usize,
}

impl DigestSink {
    fn new() -> Self {
        DigestSink {
            hasher: Sha256::new(),
            buffer: [0; 64],
            filled: 0,
        }
    }

    fn finish(mut self) -> String {
        self.hasher.update(&self.buffer[..self.filled]);
        let digest = self.hasher.finalize();
        let mut hex = String::with_capacity(SHORT_DIGEST_HEX);
        for byte in digest.iter().take(SHORT_DIGEST_HEX.div_ceil(2)) {
            hex.push(char::from(HEX_LOWER[usize::from(byte >> 4)]));
            hex.push(char::from(HEX_LOWER[usize::from(byte & 0x0f)]));
        }
        hex.truncate(SHORT_DIGEST_HEX);
        hex
    }
}

impl PushAscii for DigestSink {
    fn ascii(&mut self, byte: u8) {
        if self.filled == self.buffer.len() {
            self.hasher.update(self.buffer);
            self.filled = 0;
        }
        self.buffer[self.filled] = byte;
        self.filled += 1;
    }
}
