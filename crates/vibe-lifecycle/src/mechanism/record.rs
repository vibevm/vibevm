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
//!
//! ONE home, two roles. R8-CARGO wrote this cell against the Cargo
//! adapter's own value types; every package provider produces records beside
//! the build records in the same directory, under the same laws, so
//! the inputs are the record's OWN vocabulary and the callers translate
//! into it. A second record writer — even a faithful one — would be a
//! second thing to drift.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::path::Path;

use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{ArtifactKind, MechanismKey};
use vibe_safefs::Project;
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactRecord, ArtifactShape, ContentDigest, DigestAlgorithm,
    FreshnessFingerprints, ProducerIdentity, ProviderIdentity, RelativeIdentity, RelativeRoot,
    Rfc3339Timestamp, VerificationState, VerificationStatus,
};

/// The engine-owned state home for artifact records, project-relative.
///
/// ```
/// assert_eq!(vibe_lifecycle::ARTIFACT_RECORD_DIR, ".vibe/state/artifacts");
/// ```
pub const ARTIFACT_RECORD_DIR: &str = ".vibe/state/artifacts";

/// The record epoch this engine writes.
const RECORD_SCHEMA: u32 = 1;

/// Why the engine could not stamp, validate, encode or publish one
/// artifact record.
///
/// Its own enum rather than a set of variants on either phase's error: the
/// record cell is shared by the build and package executors, so a refusal
/// it raises belongs to neither, and both carry it transparently.
///
/// ```
/// use vibe_lifecycle::RecordError;
///
/// let refusal = RecordError::Clock {
///     output: "demo.zip".into(),
///     value: "yesterday".into(),
///     reason: "input contains invalid characters".into(),
/// };
/// assert!(refusal.to_string().contains("RFC 3339"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecordError {
    /// The injected clock value is not an RFC 3339 timestamp.
    #[error(
        "artifact `{output}` cannot be stamped: `{value}` is not an RFC 3339 timestamp ({reason}) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: \
         pass the run's own RFC 3339 clock value)"
    )]
    Clock {
        output: String,
        value: String,
        reason: String,
    },

    /// The engine built a record its own A2 cell refuses. Always a bug in
    /// this engine, and it stops here rather than reaching a reader.
    #[error(
        "the artifact record for `{output}` does not satisfy the record laws: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: this is a \
         defect in the producing engine — a record \
         that does not validate is never written)"
    )]
    Invalid { output: String, reason: String },

    /// The validated record could not be serialised.
    #[error(
        "the artifact record for `{output}` could not be encoded: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: \
         this is a defect in the producing engine)"
    )]
    Encode { output: String, reason: String },

    /// The record could not be published to the engine-owned state home.
    #[error(
        "the artifact record for `{output}` could not be written to `{path}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: make the \
         selected project's `.vibe/` writable, then \
         rerun the producing phase)"
    )]
    Write {
        output: String,
        path: String,
        reason: String,
    },
}

/// The three freshness digests, as the producing executor computed them.
///
/// Every member is optional because §4.1 admits two honest postures: a
/// provider-fresh target (Cargo) has no engine-side input census and says
/// so by absence, while an engine-fresh package target really did hash its
/// complete closed input set and says so by presence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecordFreshness<'a> {
    pub(crate) inputs: Option<&'a str>,
    pub(crate) config: Option<&'a str>,
    pub(crate) toolchain: Option<&'a str>,
}

/// Everything the engine knows about one produced artifact at the moment
/// it decides to record it — the record's own vocabulary, not any one
/// provider's.
pub(crate) struct RecordInputs<'a> {
    pub(crate) target: &'a str,
    pub(crate) mechanism: &'a MechanismKey,
    pub(crate) provider_key: &'a str,
    pub(crate) provider_version: Option<&'a str>,
    pub(crate) provider_hash: Option<&'a str>,
    pub(crate) output_id: &'a str,
    pub(crate) kind: ArtifactKind,
    /// The physical shape on disk. It also decides the digest algorithm:
    /// §4 gives a file its SHA-256 and a directory its canonical tree
    /// digest, and deriving one from the other keeps a record from
    /// claiming a tree digest over a file's bytes.
    pub(crate) shape: ArtifactShape,
    /// 64 lowercase hex — over the file's bytes, or over the canonical
    /// directory manifest.
    pub(crate) digest: &'a str,
    pub(crate) path_absolute: &'a str,
    pub(crate) path_relative: &'a str,
    pub(crate) freshness: RecordFreshness<'a>,
    pub(crate) platform: Option<&'a str>,
    pub(crate) media_type: Option<&'a str>,
    /// The injected RFC 3339 clock value — the same discipline the
    /// lifecycle state header follows, so a stamped record is
    /// reproducible.
    pub(crate) created_at: &'a str,
    /// The evidence summary naming what proved the verdict.
    pub(crate) evidence: String,
}

/// Build one epoch-1 artifact record.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub(crate) fn build_record(inputs: &RecordInputs<'_>) -> Result<ArtifactRecord, RecordError> {
    build_record_with_root(inputs, RelativeRoot::Project)
}

/// Build an epoch-1 record whose portable path is relative to an explicit
/// owner root. Native source builds use `slot` for dependency providers and
/// `project` for authored hosts; existing project-owned producers keep the
/// historical [`build_record`] convenience above.
pub(crate) fn build_record_with_root(
    inputs: &RecordInputs<'_>,
    relative_root: RelativeRoot,
) -> Result<ArtifactRecord, RecordError> {
    let created_at = inputs
        .created_at
        .parse::<Rfc3339Timestamp>()
        .map_err(|error| RecordError::Clock {
            output: inputs.output_id.to_owned(),
            value: inputs.created_at.to_owned(),
            reason: error.to_string(),
        })?;
    let algorithm = match inputs.shape {
        ArtifactShape::File => DigestAlgorithm::Sha256,
        ArtifactShape::Directory => DigestAlgorithm::Sha256Tree,
    };
    Ok(ArtifactRecord {
        schema: RECORD_SCHEMA,
        id: inputs.output_id.to_owned(),
        kind: record_kind(inputs.kind),
        shape: inputs.shape.clone(),
        path_absolute: inputs.path_absolute.to_owned(),
        path_relative: RelativeIdentity {
            root: relative_root,
            path: inputs.path_relative.to_owned(),
        },
        digest: ContentDigest {
            algorithm,
            value: inputs.digest.to_owned(),
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
            inputs: inputs.freshness.inputs.map(str::to_owned),
            config: inputs.freshness.config.map(str::to_owned),
            toolchain: inputs.freshness.toolchain.map(str::to_owned),
        },
        created_at,
        verification: VerificationState {
            status: VerificationStatus::Verified,
            evidence: Some(inputs.evidence.clone()),
        },
        media_type: inputs.media_type.map(str::to_owned),
        platform: inputs.platform.map(str::to_owned),
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
) -> Result<String, RecordError> {
    validate(record).map_err(|error| RecordError::Invalid {
        output: record.id.clone(),
        reason: error.to_string(),
    })?;
    let bytes = serde_json::to_vec_pretty(record).map_err(|error| RecordError::Encode {
        output: record.id.clone(),
        reason: error.to_string(),
    })?;
    let relative = format!("{ARTIFACT_RECORD_DIR}/{}.json", record.id);
    let project = Project::open(project_root).map_err(|error| RecordError::Write {
        output: record.id.clone(),
        path: relative.clone(),
        reason: format!("{error:#}"),
    })?;
    project
        .write_atomic(&relative, &bytes)
        .map_err(|error| RecordError::Write {
            output: record.id.clone(),
            path: relative.clone(),
            reason: format!("{:#}", error.into_report()),
        })?;
    Ok(relative)
}

/// Read back one previously written artifact record, or `None` when the
/// engine never recorded that id.
///
/// The package executor's input resolution is the only caller: §6.0.2
/// gives it the engine's own state as the ONE place a consumed build
/// output may be found ("never a guessed path"). Absence is a value here
/// rather than a refusal, because the caller — which knows the consuming
/// target and the declared input — is the one that can name it usefully.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub(crate) fn read_record(
    project_root: &Path,
    output_id: &str,
) -> Result<Option<ArtifactRecord>, String> {
    let relative = format!("{ARTIFACT_RECORD_DIR}/{output_id}.json");
    let project = Project::open(project_root).map_err(|error| format!("{error:#}"))?;
    let Some(bytes) = project
        .read_file(&relative)
        .map_err(|error| format!("{error:#}"))?
    else {
        return Ok(None);
    };
    let record: ArtifactRecord =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    validate(&record).map_err(|error| error.to_string())?;
    Ok(Some(record))
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

/// Strip the bytes that break a log line or a C string from text that came
/// out of a foreign process or an authored file, so a record's free-text
/// law cannot be violated by a toolchain banner or a resource name.
pub(crate) fn sanitize(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|byte| if byte.is_control() { ' ' } else { byte })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "no evidence recorded".to_owned()
    } else {
        trimmed.to_owned()
    }
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

/// The manifest vocabulary's spelling of one RECORDED artifact kind — the
/// exact inverse of [`record_kind`], and total for the same reason.
///
/// It is written as a match rather than derived, because that is what makes
/// a future member of either vocabulary a COMPILE error here instead of a
/// silent fallback: an input whose recorded kind this engine cannot name is
/// exactly the input a provenance gate must not wave through.
pub(crate) const fn manifest_kind(kind: &RecordKind) -> ArtifactKind {
    match kind {
        RecordKind::Executable => ArtifactKind::Executable,
        RecordKind::Archive => ArtifactKind::Archive,
        RecordKind::File => ArtifactKind::File,
        RecordKind::Directory => ArtifactKind::Directory,
        RecordKind::Skill => ArtifactKind::Skill,
        RecordKind::AgentPlugin => ArtifactKind::AgentPlugin,
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
