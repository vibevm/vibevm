//! `evidence_id`: one domain-separated digest over every canonical identity
//! and comparison member except itself and `observed_at` (PROP-054
//! `##EVIDENCE-WIRE-AND-SURFACES`).
//!
//! The recipe is a WRITER, not a serialization. Members are framed in the
//! schema's own vocabulary order — the JSON insertion order of
//! `formats/vocabularies.json`, down to `files` before `bytes` — under the
//! shared `be64(label_len)||label||be64(value_len)||value` frame, with
//! canonical decimal counts, wire enum spellings and an explicit `0|1`
//! presence byte before every optional. Nothing here consults `serde`, JSON
//! key order or Rust field order, so another language reproduces the id from
//! the schema alone.
//!
//! `observed_at` is excluded on purpose: re-reading the same claim at another
//! wall-clock second is the same claim.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES");

use specmark::spec;
use vibe_wire::generated::shared::{
    ArtifactWitness, DigestWitness, EvidenceStatus, InputMeasurement, VerificationEvidence,
};

use crate::state::FramedHash;

/// The evidence identity's own epoch domain.
const SEED: &[u8] = b"vibe-verification-evidence-id\0epoch=1\0";

/// The id of one assembled member.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES")]
pub(super) fn evidence_id(member: &VerificationEvidence) -> String {
    let mut hash = FramedHash::seeded(SEED);
    // Root order: `evidence`, [evidence_id], `status`, [observed_at], `run`,
    // `inputs`, `artifacts` — the two excluded members simply do not appear.
    hash.count("evidence", member.evidence as usize);
    hash.field("status", status(&member.status).as_bytes());
    hash.field("run.run_id", member.run.run_id.as_bytes());
    hash.field("run.selected", member.run.selected.as_bytes());
    hash.field("run.requested", member.run.requested.as_bytes());
    hash.count("run.chain.count", member.run.chain.len());
    for phase in &member.run.chain {
        hash.field("run.chain.item", phase.as_bytes());
    }
    hash.field("run.started", member.run.started.as_bytes());
    hash.count("inputs.count", member.inputs.len());
    for row in &member.inputs {
        input(&mut hash, row);
    }
    hash.count("artifacts.count", member.artifacts.len());
    for row in &member.artifacts {
        artifact(&mut hash, row);
    }
    hash.finish()
}

/// `execution, phase, declaration_fingerprint, patterns, status`, then the
/// four optionals in their own declared order.
fn input(hash: &mut FramedHash, row: &InputMeasurement) {
    hash.field("inputs.execution", row.execution.as_bytes());
    hash.field("inputs.phase", row.phase.as_bytes());
    hash.field(
        "inputs.declaration_fingerprint",
        row.declaration_fingerprint.as_bytes(),
    );
    hash.count("inputs.patterns.count", row.patterns.len());
    for pattern in &row.patterns {
        hash.field("inputs.patterns.item", pattern.as_bytes());
    }
    hash.field("inputs.status", status(&row.status).as_bytes());
    optional(
        hash,
        "inputs.measured_run_id",
        row.measured_run_id.as_deref(),
    );
    optional_witness(hash, "inputs.measured", row.measured.as_ref());
    optional_witness(hash, "inputs.observed", row.observed.as_ref());
    optional(hash, "inputs.reason_code", row.reason_code.as_deref());
}

/// `id, kind, path, status`, then the same four optionals.
fn artifact(hash: &mut FramedHash, row: &ArtifactWitness) {
    hash.field("artifacts.id", row.id.as_bytes());
    hash.field("artifacts.kind", row.kind.as_bytes());
    hash.field("artifacts.path", row.path.as_bytes());
    hash.field("artifacts.status", status(&row.status).as_bytes());
    optional(
        hash,
        "artifacts.measured_run_id",
        row.measured_run_id.as_deref(),
    );
    optional_witness(hash, "artifacts.measured", row.measured.as_ref());
    optional_witness(hash, "artifacts.observed", row.observed.as_ref());
    optional(hash, "artifacts.reason_code", row.reason_code.as_deref());
}

/// An optional scalar: the presence bit first, ALWAYS, so an absent value can
/// never collide with an authored empty one.
fn optional(hash: &mut FramedHash, label: &str, value: Option<&str>) {
    hash.presence(&format!("{label}.present"), value.is_some());
    if let Some(value) = value {
        hash.field(label, value.as_bytes());
    }
}

/// An optional witness: `algorithm, digest, files, bytes` — the vocabulary's
/// own order, counts included, because a counted manifest and an uncounted
/// content digest are different claims.
fn optional_witness(hash: &mut FramedHash, label: &str, value: Option<&DigestWitness>) {
    hash.presence(&format!("{label}.present"), value.is_some());
    let Some(value) = value else { return };
    hash.field(&format!("{label}.algorithm"), value.algorithm.as_bytes());
    hash.field(&format!("{label}.digest"), value.digest.as_bytes());
    let files = value.files.map(|files| files.to_string());
    optional(hash, &format!("{label}.files"), files.as_deref());
    optional(hash, &format!("{label}.bytes"), value.bytes.as_deref());
}

/// The closed wire spelling — never a Rust debug rendering.
const fn status(status: &EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Matched => "matched",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Stale => "stale",
        EvidenceStatus::Unavailable => "unavailable",
        EvidenceStatus::Unstable => "unstable",
    }
}
