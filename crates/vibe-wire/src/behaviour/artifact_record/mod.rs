//! The artifact record's scalar laws — the hand-written validation
//! cell beside the ONE generated [`ArtifactRecord`] the build/package
//! engine writes per produced artifact (R8A2, the packages-2026-09
//! architecture §4).
//!
//! JTD owns the FORM (the closed kind/shape/root/algorithm/status
//! vocabularies, the optional members, the typed RFC 3339 timestamp);
//! this cell owns what a form cannot say about itself: the epoch
//! constant, the one frozen id grammar, the absolute and relative path
//! spellings, the bare 64-hex digest law, the role-qualified mechanism
//! key, the ExtensionKey provider shape and the free-text safety of
//! every member a reader prints. Every predicate that is not this
//! record's own is REUSED from [`crate::behaviour::scalars`] and the
//! trace-index cell — one grammar, every wire — and nothing here
//! recomputes a digest or re-hashes anything: the record states a fact,
//! and the cell judges its spelling.
//!
//! Every value it reads is untrusted — a record is a file on disk — so
//! no refusal clones the offending scalar: errors carry a bounded
//! [`ScalarPreview`] and the true byte length.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::behaviour::scalars::{
    has_control_bytes, is_lowercase_hex, is_portable_token, is_sha256, provider_key_defect,
    relative_path_defect,
};
use crate::generated::artifact_record::ArtifactRecord;

mod errors;

pub use errors::{AbsolutePathUnsafety, ArtifactRecordError, MechanismDefect};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// The artifact-record epoch this validator speaks — the `schema`
/// member's one legal value today.
pub const RECORD_EPOCH: u32 = 1;

/// The mechanism-key roles that produce an artifact. A deploy mechanism
/// produces no artifact record, and `acquire:` reopens with acquire-
/// role providers (§12's recorded revisit) — carrying either prefix
/// now would license a vocabulary nothing services yet.
const PRODUCING_ROLES: [&str; 2] = ["build", "package"];

/// Validate one artifact record against every scalar law. Pure: the
/// value in, the first broken law out.
pub fn validate(record: &ArtifactRecord) -> Result<(), ArtifactRecordError> {
    if record.schema != RECORD_EPOCH {
        return Err(ArtifactRecordError::SchemaEpoch {
            found: record.schema,
        });
    }
    if !is_portable_token(&record.id) {
        return Err(ArtifactRecordError::IdNotPortableToken {
            id: preview(&record.id),
        });
    }
    absolute_path_gate(&record.path_absolute)?;
    if let Some(defect) = relative_path_defect(&record.path_relative.path) {
        return Err(ArtifactRecordError::UnsafeRelativePath {
            path: preview(&record.path_relative.path),
            defect,
        });
    }
    if !is_lowercase_hex(&record.digest.value, 64) {
        return Err(ArtifactRecordError::DigestValueNotHex {
            value: preview(&record.digest.value),
        });
    }
    producer_gate(record)?;
    freshness_gate(record)?;
    for (field, value) in [
        ("media_type", &record.media_type),
        ("platform", &record.platform),
        ("verification.evidence", &record.verification.evidence),
    ] {
        free_text_gate(field, value.as_deref())?;
    }
    Ok(())
}

/// One bounded preview — the same refusal discipline the trace cells
/// use, applied through their shared type.
fn preview(value: &str) -> ScalarPreview {
    ScalarPreview::of(value)
}

/// The shared free-text predicate: non-blank once trimmed and free of
/// the three bytes that break a log line or a C string. The RULE is
/// shared with the deploy cells; this record keeps its own typed
/// refusal on top of it.
fn is_unsafe_text(value: &str) -> bool {
    value.trim().is_empty() || has_control_bytes(value)
}

/// The absolute runtime placement: forward-slashed, absolute
/// (`/…` or `X:/…`) and control-free — the same law the trace report
/// holds its run path to, because both name a directory a reader will
/// open on this machine.
fn absolute_path_gate(path: &str) -> Result<(), ArtifactRecordError> {
    let bytes = path.as_bytes();
    if bytes.contains(&b'\\') {
        return Err(ArtifactRecordError::UnsafeAbsolutePath {
            path: preview(path),
            reason: AbsolutePathUnsafety::Backslash,
        });
    }
    if has_control_bytes(path) {
        return Err(ArtifactRecordError::UnsafeAbsolutePath {
            path: preview(path),
            reason: AbsolutePathUnsafety::ControlByte,
        });
    }
    let absolute = path.starts_with('/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'/');
    if !absolute {
        return Err(ArtifactRecordError::UnsafeAbsolutePath {
            path: preview(path),
            reason: AbsolutePathUnsafety::NotAbsolute,
        });
    }
    Ok(())
}

/// The producer triple: a portable-token target, a role-qualified mechanism
/// key whose prefix is one of the producing families, and a provider
/// key in the ExtensionKey shape with its optional identity members.
fn producer_gate(record: &ArtifactRecord) -> Result<(), ArtifactRecordError> {
    let producer = &record.producer;
    if !is_portable_token(&producer.target) {
        return Err(ArtifactRecordError::ProducerTargetNotPortableToken {
            target: preview(&producer.target),
        });
    }
    if let Some(reason) = mechanism_defect(&producer.mechanism) {
        return Err(ArtifactRecordError::BadMechanismKey {
            mechanism: preview(&producer.mechanism),
            reason,
        });
    }
    if let Some(defect) = provider_key_defect(&producer.provider.key) {
        return Err(ArtifactRecordError::BadProviderKey {
            key: preview(&producer.provider.key),
            defect,
        });
    }
    free_text_gate(
        "producer.provider.version",
        producer.provider.version.as_deref(),
    )?;
    if let Some(hash) = &producer.provider.content_hash
        && !is_sha256(hash)
    {
        return Err(ArtifactRecordError::BadContentHash {
            content_hash: preview(hash),
        });
    }
    Ok(())
}

/// The mechanism key: `<role>:<tail>` where the role is one of the
/// producing families and the tail obeys the portable-token grammar —
/// exactly the spelling §12 froze for `[[artifacts.build]]` /
/// `[[artifacts.package]]` rows.
fn mechanism_defect(mechanism: &str) -> Option<MechanismDefect> {
    let Some((role, tail)) = mechanism.split_once(':') else {
        return Some(MechanismDefect::MissingRolePrefix);
    };
    if !PRODUCING_ROLES.contains(&role) {
        return Some(MechanismDefect::UnknownRole);
    }
    if !is_portable_token(tail) {
        return Some(MechanismDefect::BadTail);
    }
    None
}

/// The three freshness digests: each optional, each — when present —
/// exactly 64 lowercase hex.
fn freshness_gate(record: &ArtifactRecord) -> Result<(), ArtifactRecordError> {
    let freshness = &record.freshness;
    for (member, value) in [
        ("freshness.inputs", &freshness.inputs),
        ("freshness.config", &freshness.config),
        ("freshness.toolchain", &freshness.toolchain),
    ] {
        if let Some(digest) = value
            && !is_lowercase_hex(digest, 64)
        {
            return Err(ArtifactRecordError::BadFreshnessDigest {
                member,
                value: preview(digest),
            });
        }
    }
    Ok(())
}

/// A free-text member: non-blank once trimmed and free of the three
/// bytes that break a log line or a C string. Absent is absent — the
/// gate rules only on a present value.
fn free_text_gate(field: &'static str, value: Option<&str>) -> Result<(), ArtifactRecordError> {
    if let Some(text) = value
        && is_unsafe_text(text)
    {
        return Err(ArtifactRecordError::UnsafeScalar {
            field,
            value: preview(text),
        });
    }
    Ok(())
}
