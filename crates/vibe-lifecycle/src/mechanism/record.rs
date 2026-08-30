//! Artifact records — the ENGINE's, never the provider's.
//!
//! §3.2 is unambiguous: "The engine, not the provider, owns ordering,
//! scratch paths, artifact identities, state persistence, locks,
//! narration, timing and redaction. A provider may add typed evidence but
//! cannot mint an unscoped output path or silently invent a second
//! lifecycle." So a provider reports *what it produced*; this cell decides
//! what that becomes on disk and where.
//!
//! Two laws hold the write together:
//!
//! - the record is validated through the A2 behaviour cell BEFORE any byte
//!   reaches the filesystem. A record that does not validate is a bug in
//!   its producer, and the honest place to find it is the producer's own
//!   refusal — not a later reader's;
//! - the digest is of the produced bytes, streamed off the artifact the
//!   provider actually reported, never restated from a plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::path::Path;

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{ArtifactKind, MechanismKey};
use vibe_safefs::Project;
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactRecord, ArtifactShape, ContentDigest, DigestAlgorithm,
    FreshnessFingerprints, ProducerIdentity, ProviderIdentity, RelativeIdentity, RelativeRoot,
    Rfc3339Timestamp, VerificationState, VerificationStatus,
};

use super::build::BuildError;
use super::cargo::{SelectedArtifact, ToolchainIdentity, VerifiedArtifact};

/// The engine-owned state home for artifact records, project-relative.
///
/// ```
/// assert_eq!(vibe_lifecycle::ARTIFACT_RECORD_DIR, ".vibe/state/artifacts");
/// ```
pub const ARTIFACT_RECORD_DIR: &str = ".vibe/state/artifacts";

/// The record epoch this engine writes.
const RECORD_SCHEMA: u32 = 1;

/// Everything the engine knows about one produced artifact at the moment
/// it decides to record it.
pub(crate) struct RecordInputs<'a> {
    pub(crate) target: &'a str,
    pub(crate) mechanism: &'a MechanismKey,
    pub(crate) provider_key: &'a str,
    pub(crate) provider_version: Option<&'a str>,
    pub(crate) provider_hash: Option<&'a str>,
    pub(crate) selected: &'a SelectedArtifact,
    pub(crate) verified: &'a VerifiedArtifact,
    pub(crate) toolchain: &'a ToolchainIdentity,
    /// The target-config fingerprint the engine computed.
    pub(crate) config_digest: &'a str,
    /// The injected RFC 3339 clock value — the same discipline the
    /// lifecycle state header follows, so a stamped record is
    /// reproducible.
    pub(crate) created_at: &'a str,
    /// The evidence summary naming what proved the verdict.
    pub(crate) evidence: String,
}

/// Build one epoch-1 artifact record.
///
/// The freshness triple is the honest one for a provider-fresh target
/// (§4.1, §5.0.5): `inputs` is ABSENT, because Cargo owns inputs the
/// engine does not model and a fabricated input digest would be a claim
/// the engine cannot support; `config` and `toolchain` are present,
/// because the engine really did hash the target's config and the
/// provider really did report its toolchain identity.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub(crate) fn build_record(inputs: &RecordInputs<'_>) -> Result<ArtifactRecord, BuildError> {
    let created_at = inputs
        .created_at
        .parse::<Rfc3339Timestamp>()
        .map_err(|error| BuildError::RecordClock {
            output: inputs.verified.output_id.clone(),
            value: inputs.created_at.to_owned(),
            reason: error.to_string(),
        })?;
    Ok(ArtifactRecord {
        schema: RECORD_SCHEMA,
        id: inputs.verified.output_id.clone(),
        kind: record_kind(inputs.selected.kind),
        shape: ArtifactShape::File,
        path_absolute: inputs.verified.path_absolute.clone(),
        path_relative: RelativeIdentity {
            root: RelativeRoot::Project,
            path: inputs.verified.path_relative.clone(),
        },
        digest: ContentDigest {
            algorithm: DigestAlgorithm::Sha256,
            value: inputs.verified.digest.clone(),
        },
        producer: ProducerIdentity {
            target: inputs.target.to_owned(),
            mechanism: inputs.mechanism.to_string(),
            provider: ProviderIdentity {
                key: inputs.provider_key.to_owned(),
                version: inputs.provider_version.map(str::to_owned),
                content_hash: inputs.provider_hash.map(str::to_owned),
            },
        },
        freshness: FreshnessFingerprints {
            inputs: None,
            config: Some(inputs.config_digest.to_owned()),
            toolchain: Some(inputs.toolchain.digest.clone()),
        },
        created_at,
        verification: VerificationState {
            status: VerificationStatus::Verified,
            evidence: Some(inputs.evidence.clone()),
        },
        media_type: None,
        platform: inputs.toolchain.host.clone(),
    })
}

/// Validate one record through the A2 cell, then publish it atomically at
/// `.vibe/state/artifacts/<output-id>.json`.
///
/// The validation is not decoration: it is what keeps this engine from
/// being the producer of an invalid record. The write is capability-
/// relative and no-follow, exactly as every other durable byte this crate
/// publishes.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub(crate) fn write_record(
    project_root: &Path,
    record: &ArtifactRecord,
) -> Result<String, BuildError> {
    validate(record).map_err(|error| BuildError::RecordInvalid {
        output: record.id.clone(),
        reason: error.to_string(),
    })?;
    let bytes = serde_json::to_vec_pretty(record).map_err(|error| BuildError::RecordEncode {
        output: record.id.clone(),
        reason: error.to_string(),
    })?;
    let relative = format!("{ARTIFACT_RECORD_DIR}/{}.json", record.id);
    let project = Project::open(project_root).map_err(|error| BuildError::RecordWrite {
        output: record.id.clone(),
        path: relative.clone(),
        reason: format!("{error:#}"),
    })?;
    project
        .write_atomic(&relative, &bytes)
        .map_err(|error| BuildError::RecordWrite {
            output: record.id.clone(),
            path: relative.clone(),
            reason: format!("{:#}", error.into_report()),
        })?;
    Ok(relative)
}

/// One target-config fingerprint over everything the engine chose for a
/// target — the `freshness.config` member of every record it produces.
///
/// Provider identity is folded in because §4.1 says so outright: "Provider
/// changes invalidate the target even when its logical mechanism name did
/// not change."
pub(crate) fn config_digest(
    mechanism: &MechanismKey,
    provider_key: &str,
    argv: &[String],
    inputs: &[String],
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"mechanism\x00");
    hash.update(mechanism.to_string().as_bytes());
    hash.update(b"\x00provider\x00");
    hash.update(provider_key.as_bytes());
    for argument in argv {
        hash.update(b"\x00argv\x00");
        hash.update(argument.as_bytes());
    }
    for declared in inputs {
        hash.update(b"\x00input\x00");
        hash.update(declared.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

/// The record vocabulary's spelling of one manifest artifact kind. Both
/// sets are the same closed §12 vocabulary, and the mapping is total for
/// exactly that reason.
const fn record_kind(kind: ArtifactKind) -> RecordKind {
    match kind {
        ArtifactKind::Executable => RecordKind::Executable,
        ArtifactKind::Archive => RecordKind::Archive,
        ArtifactKind::File => RecordKind::File,
        ArtifactKind::Directory => RecordKind::Directory,
        ArtifactKind::Skill => RecordKind::Skill,
        ArtifactKind::AgentPlugin => RecordKind::AgentPlugin,
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
