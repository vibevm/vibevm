//! Wire form of an integer wider than 32 bits: a **canonical decimal
//! string** — ASCII digits only, no sign, no leading zeros except the
//! bare `0`, value within `u64` (the owner's standing rule, 2026-08-20,
//! PROP-044 §4.2b `##M-WIDE-INTEGERS-AS-STRINGS`; JTD has no 64-bit
//! integer type, so the schema says `string` and the *value* carries
//! the number).
//!
//! Lifted out of `repomd.rs` the day the second such field appeared
//! (the verify report's `expected_size`/`actual_size`) — exactly the
//! promotion the first application predicted. Both directions are
//! loud: a non-string JSON value, a non-numeric or non-canonical
//! string (`"007"`, `"+42"`, `"-1"`, `""`, `"abc"`) is a refusal,
//! never a coercion or a quiet `0` (PROP-044 law 1). The Rust type
//! stays `u64`, so arithmetic use is unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout");

use serde::{Deserialize, Deserializer, Serializer};

pub(crate) fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let raw = String::deserialize(deserializer)?;
    parse_canonical(&raw).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "`{raw}` is not a canonical decimal u64 (ASCII digits only, no sign, \
             no leading zeros except the bare `0`, value within u64)"
        ))
    })
}

/// The canonical form accepted on the wire — exactly what
/// `u64::from_str` accepts for digits, tightened to reject leading
/// zeros and the empty string so one number has one spelling.
fn parse_canonical(raw: &str) -> Option<u64> {
    // The explicit digit check closes `from_str`'s one leniency: it
    // accepts a leading `+`, which would give one number a second
    // spelling.
    if raw.is_empty()
        || (raw.len() > 1 && raw.starts_with('0'))
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse().ok()
}
