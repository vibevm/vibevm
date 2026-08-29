//! The one compiler digest framing primitive (PROP-054
//! `#TRANSFORM-PLAN-IDENTITY`): domain-separated SHA-256 over length-framed
//! little-endian fields.
//!
//! Every canonical compiler digest — the lane/chunks/emitted-bytes digests in
//! `emit`, the R4.1 transform config/implementation/plan digests in
//! `transform` — is written through this cell so no digest site re-derives
//! framing on its own. The framing is frozen: the domain is the first
//! length-framed field; discriminants are single bytes; numbers are u32/u64
//! little-endian, with signed components framed as their two's-complement
//! bits; every byte field is `u64_le(len) || bytes`.

use sha2::{Digest, Sha256};

pub(crate) struct StableDigest(Sha256);

impl StableDigest {
    /// Start a digest whose first length-framed field is the domain.
    pub(crate) fn new(domain: &[u8]) -> Self {
        let mut value = Self(Sha256::new());
        value.field(domain);
        value
    }

    /// One discriminant, variant, or optional-presence byte.
    pub(crate) fn byte(&mut self, value: u8) {
        self.0.update([value]);
    }

    /// A `u32` little-endian.
    pub(crate) fn u32(&mut self, value: u32) {
        self.0.update(value.to_le_bytes());
    }

    /// A `u64` little-endian — also the write width for every count, every
    /// length frame, and every signed 64-bit component (two's-complement
    /// bits), so framing never depends on the host pointer width.
    pub(crate) fn u64(&mut self, value: u64) {
        self.0.update(value.to_le_bytes());
    }

    /// A count or index as `u64` little-endian.
    pub(crate) fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    /// A byte field: `u64_le(len) || bytes`.
    pub(crate) fn field(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.0.update(value);
    }

    /// Finalize to the raw 32-byte SHA-256 digest.
    pub(crate) fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}
