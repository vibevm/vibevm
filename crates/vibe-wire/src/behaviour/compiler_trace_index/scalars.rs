//! `scalar-gates` — the law that reads a single identity string and
//! decides whether it is one the epoch admits.
//!
//! It is an ALLOWLIST in both halves: an identity is non-blank once
//! trimmed and free of the three bytes that break a log line or a C
//! string, and a custom artifact target is held to the exact charset the
//! compiler's own `BackendId` enforces. The snapshot filename law used
//! to live here too; it now lives in `snapshot.rs`, because a filename
//! is not validated by an alphabet at all — it is CONSTRUCTED from the
//! event and compared.

use super::errors::{ScalarPreview, TraceIndexError};

/// The epoch-1 spelling of `project.display`. The trace never leaks an
/// absolute developer home path, and the outer node/unit labels live in
/// the scopes — so the root project's display is this one character.
pub(super) const ROOT_DISPLAY: &str = ".";

/// `scalar-gates` on one free-text identity member: non-blank once
/// trimmed (a scope id of three spaces is not an identity), and free of
/// the three bytes that break a log line or a C string — CR, LF, NUL.
pub(super) fn scalar_gate(field: &'static str, value: &str) -> Result<(), TraceIndexError> {
    let unsafe_control = value
        .bytes()
        .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0'));
    if value.trim().is_empty() || unsafe_control {
        return Err(TraceIndexError::UnsafeScalar {
            field,
            value: ScalarPreview::of(value),
        });
    }
    Ok(())
}

/// Exactly `len` lowercase hex characters.
///
/// `pub(crate)`: the command-report trace member pins its `run_id` with
/// the same 32-hex law this index pins its own — one predicate, both
/// readers.
pub(crate) fn is_lowercase_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The backend id charset `[a-z0-9][a-z0-9._-]{0,63}` — the same law
/// `vibe-spec`'s `BackendId` enforces on the compiler side. A custom
/// artifact target is an open-vocabulary string arriving from a plugin,
/// so the index holds it to the identity the compiler would have
/// accepted; the two built-in STATIC targets satisfy it by construction.
pub(super) fn is_backend_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=64).contains(&bytes.len())
        && is_id_byte(bytes[0])
        && bytes
            .iter()
            .skip(1)
            .all(|byte| is_id_byte(*byte) || b"._-".contains(byte))
}

fn is_id_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}
