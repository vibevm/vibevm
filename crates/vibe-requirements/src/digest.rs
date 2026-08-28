//! The three exact digests — reproducible, domain-separated,
//! length-framed; never a JSON or Rust-layout hash.
//!
//! All recipes share one primitive (architecture §4.2's frame):
//! `be64(label_len) || label || be64(value_len) || value`, seeded with
//! a domain string + `\0epoch=1\0`. Numbers and counts are canonical
//! decimal UTF-8; enums use their WIRE spelling; arrays frame their
//! count before their already-canonical elements; every optional member
//! frames a LABELED presence field (`field("<member>.present", "0|1")`)
//! before its value, and every array frames its own labeled count — so
//! a cross-language reimplementation reconstructs the byte stream from
//! the recipe alone, with no undocumented raw bytes or positions. The
//! required `truncated` member frames `field("truncated", "true|false")`,
//! never an optional-presence bit. Nothing here serialises a struct,
//! reads JSON, or depends on field declaration order — the framing
//! order is the AUTHORED SCHEMA order named in the recipes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use sha2::{Digest as _, Sha256};
use vibe_facts::SourceFileWitness;
use vibe_wire::generated::requirements_report::{
    AdoptionObservationPresence, AuthoringObservationPresence, FactStatus, FactStatusStage,
    FactStatusState, RelationSource, RelationSourceProvenance, RelationSourceState, RequirementRow,
    RequirementSourceKind, RequirementsReport, SourceResultState,
};

/// Recipe 1's domain — one source result's digest over its documents.
pub(crate) const SOURCE_DOMAIN: &[u8] = b"vibe-requirements-source-digest\0epoch=1\0";
/// Recipe 2's domain — the observation's scope digest.
pub(crate) const SCOPE_DOMAIN: &[u8] = b"vibe-requirements-scope-digest\0epoch=1\0";
/// Recipe 3's domain — the answer's own identity.
pub(crate) const OBSERVATION_DOMAIN: &[u8] = b"vibe-requirements-observation-id\0epoch=1\0";

/// One framed field: `be64(label_len) || label || be64(value_len) || value`.
fn field(hasher: &mut Sha256, label: &[u8], value: &[u8]) {
    hasher.update((label.len() as u64).to_be_bytes());
    hasher.update(label);
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

/// A LABELED presence field — `field("<member>.present", "0|1")` — so
/// an optional's bit is reconstructable from the recipe, not from a
/// position.
fn presence(hasher: &mut Sha256, member: &[u8], present: bool) {
    let mut label = member.to_vec();
    label.extend_from_slice(b".present");
    field(hasher, &label, if present { b"1" } else { b"0" });
}

/// A LABELED array count in canonical decimal UTF-8 — each array names
/// its own count field (`document_count`, `source_count`, …), never a
/// shared ambiguous byte.
fn count(hasher: &mut Sha256, label: &[u8], n: usize) {
    field(hasher, label, n.to_string().as_bytes());
}

fn seed(domain: &[u8]) -> Sha256 {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher
}

fn finish(hasher: Sha256) -> String {
    format!("sha256:{:x}", hasher.finalize())
}

/// **Recipe 1** — `SourceResult.digest`: kind, package, document count,
/// then each sorted A2a witness's path, byte count and raw SHA-256.
/// Only `available` and `invalid` sources carry one; a host with zero
/// documents gets the canonical empty-source digest (document count 0),
/// never a missing member.
pub(crate) fn source_result_digest(
    kind: &RequirementSourceKind,
    package: &str,
    documents: &[SourceFileWitness],
) -> String {
    let mut hasher = seed(SOURCE_DOMAIN);
    field(&mut hasher, b"kind", kind_spelling(kind).as_bytes());
    field(&mut hasher, b"package", package.as_bytes());
    count(&mut hasher, b"document_count", documents.len());
    for witness in documents {
        field(&mut hasher, b"path", witness.path.as_bytes());
        field(&mut hasher, b"bytes", witness.bytes.to_string().as_bytes());
        field(&mut hasher, b"raw_sha256", witness.digest.as_bytes());
    }
    finish(hasher)
}

/// **Recipe 2** — `observation.source_digest`: the selected node, every
/// sorted available/invalid source's kind/package/digest, then every
/// sorted registry witness's path/bytes/raw SHA-256. Excludes query,
/// provider result, clock and run id — a registry-only raw edit moves
/// it, a changed question does not.
pub(crate) fn scope_digest(
    selected: &str,
    sources: &[(RequirementSourceKind, String, String)],
    registry: &[SourceFileWitness],
) -> String {
    let mut hasher = seed(SCOPE_DOMAIN);
    field(&mut hasher, b"selected", selected.as_bytes());
    count(&mut hasher, b"source_count", sources.len());
    for (kind, package, digest) in sources {
        field(&mut hasher, b"kind", kind_spelling(kind).as_bytes());
        field(&mut hasher, b"package", package.as_bytes());
        field(&mut hasher, b"source_digest", digest.as_bytes());
    }
    count(&mut hasher, b"registry_count", registry.len());
    for witness in registry {
        field(&mut hasher, b"path", witness.path.as_bytes());
        field(&mut hasher, b"bytes", witness.bytes.to_string().as_bytes());
        field(&mut hasher, b"raw_sha256", witness.digest.as_bytes());
    }
    finish(hasher)
}

/// **Recipe 3** — `observation.observation_id`: every canonical report
/// member in authored schema order except `observation_id` and
/// `observed_at`. The run join key's presence/value IS framed (a changed
/// run id moves the id); the clock alone never does.
///
/// Call this on the assembled report with `observation_id` still
/// unset — the id is never an input to itself.
pub(crate) fn observation_id(report: &RequirementsReport) -> String {
    let mut hasher = seed(OBSERVATION_DOMAIN);
    field(
        &mut hasher,
        b"requirements",
        report.requirements.to_string().as_bytes(),
    );
    let observation = &report.observation;
    field(&mut hasher, b"selected", observation.selected.as_bytes());
    field(
        &mut hasher,
        b"source_digest",
        observation.source_digest.as_bytes(),
    );
    presence(
        &mut hasher,
        b"lifecycle_run_id",
        observation.lifecycle_run_id.is_some(),
    );
    if let Some(run_id) = &observation.lifecycle_run_id {
        field(&mut hasher, b"lifecycle_run_id", run_id.as_bytes());
    }
    // The effective query, in its schema's member order.
    field(
        &mut hasher,
        b"limit",
        report.query.limit.to_string().as_bytes(),
    );
    field(
        &mut hasher,
        b"relations",
        report.query.relations.to_string().as_bytes(),
    );
    presence(
        &mut hasher,
        b"address_prefix",
        report.query.address_prefix.is_some(),
    );
    if let Some(prefix) = &report.query.address_prefix {
        field(&mut hasher, b"address_prefix", prefix.as_bytes());
    }
    // Source results.
    count(&mut hasher, b"source_count", report.sources.len());
    for source in &report.sources {
        field(
            &mut hasher,
            b"kind",
            kind_spelling(&source.source.kind).as_bytes(),
        );
        field(&mut hasher, b"package", source.source.package.as_bytes());
        field(
            &mut hasher,
            b"state",
            source_state_spelling(&source.state).as_bytes(),
        );
        presence(&mut hasher, b"digest", source.digest.is_some());
        if let Some(digest) = &source.digest {
            field(&mut hasher, b"digest", digest.as_bytes());
        }
        presence(&mut hasher, b"reason_code", source.reason_code.is_some());
        if let Some(reason) = &source.reason_code {
            field(&mut hasher, b"reason_code", reason.as_bytes());
        }
        presence(
            &mut hasher,
            b"adoption_entries",
            source.adoption_entries.is_some(),
        );
        if let Some(entries) = source.adoption_entries {
            field(
                &mut hasher,
                b"adoption_entries",
                entries.to_string().as_bytes(),
            );
        }
    }
    // Relation sources.
    count(
        &mut hasher,
        b"relation_source_count",
        report.relation_sources.len(),
    );
    for relation in &report.relation_sources {
        relation_source(&mut hasher, relation);
    }
    // Rows and their edges.
    count(&mut hasher, b"row_count", report.rows.len());
    for row in &report.rows {
        row_members(&mut hasher, row);
    }
    field(
        &mut hasher,
        b"truncated",
        if report.truncated { b"true" } else { b"false" },
    );
    finish(hasher)
}

fn relation_source(hasher: &mut Sha256, relation: &RelationSource) {
    field(hasher, b"package", relation.package.as_bytes());
    field(
        hasher,
        b"state",
        relation_state_spelling(&relation.state).as_bytes(),
    );
    field(
        hasher,
        b"provenance",
        relation_provenance_spelling(&relation.provenance).as_bytes(),
    );
    presence(hasher, b"reason_code", relation.reason_code.is_some());
    if let Some(reason) = &relation.reason_code {
        field(hasher, b"reason_code", reason.as_bytes());
    }
}

fn row_members(hasher: &mut Sha256, row: &RequirementRow) {
    field(hasher, b"address", row.address.as_bytes());
    field(hasher, b"kind", kind_spelling(&row.source.kind).as_bytes());
    field(hasher, b"package", row.source.package.as_bytes());
    // Authoring axis.
    field(
        hasher,
        b"authoring",
        authoring_spelling(&row.authoring.presence).as_bytes(),
    );
    presence(hasher, b"authoring.status", row.authoring.status.is_some());
    if let Some(status) = &row.authoring.status {
        status_members(hasher, status);
    }
    // Adoption axis.
    field(
        hasher,
        b"adoption",
        adoption_spelling(&row.adoption.presence).as_bytes(),
    );
    presence(hasher, b"adoption.status", row.adoption.status.is_some());
    if let Some(status) = &row.adoption.status {
        status_members(hasher, status);
    }
    // Relation edges, in their sorted order.
    count(hasher, b"edge_count", row.relations.len());
    for edge in &row.relations {
        field(hasher, b"verb", edge_verb_spelling(&edge.verb).as_bytes());
        field(
            hasher,
            b"edge_provenance",
            edge_provenance_spelling(&edge.provenance).as_bytes(),
        );
        field(hasher, b"symbol", edge.symbol.as_bytes());
        field(hasher, b"file", edge.file.as_bytes());
        field(hasher, b"line", edge.line.to_string().as_bytes());
    }
}

fn status_members(hasher: &mut Sha256, status: &FactStatus) {
    field(hasher, b"stage", stage_spelling(&status.stage).as_bytes());
    field(
        hasher,
        b"state",
        fact_state_spelling(&status.state).as_bytes(),
    );
}

// Wire spellings — single homes so a spelling change is one edit.

/// The wire spelling of a source kind (also the digest's).
pub fn kind_spelling(kind: &RequirementSourceKind) -> &'static str {
    match kind {
        RequirementSourceKind::Host => "host",
        RequirementSourceKind::Package => "package",
    }
}

fn source_state_spelling(state: &SourceResultState) -> &'static str {
    match state {
        SourceResultState::Available => "available",
        SourceResultState::Unavailable => "unavailable",
        SourceResultState::Invalid => "invalid",
        SourceResultState::Orphaned => "orphaned",
    }
}

fn relation_state_spelling(state: &RelationSourceState) -> &'static str {
    match state {
        RelationSourceState::NotRequested => "not-requested",
        RelationSourceState::Current => "current",
        RelationSourceState::Carried => "carried",
        RelationSourceState::Stale => "stale",
        RelationSourceState::Unavailable => "unavailable",
        RelationSourceState::Invalid => "invalid",
    }
}

fn relation_provenance_spelling(provenance: &RelationSourceProvenance) -> &'static str {
    match provenance {
        RelationSourceProvenance::None => "none",
        RelationSourceProvenance::FreshProjectMap => "fresh-project-map",
        RelationSourceProvenance::CarriedPackageMap => "carried-package-map",
    }
}

fn authoring_spelling(presence: &AuthoringObservationPresence) -> &'static str {
    match presence {
        AuthoringObservationPresence::Marked => "marked",
        AuthoringObservationPresence::Unmarked => "unmarked",
    }
}

fn adoption_spelling(presence: &AdoptionObservationPresence) -> &'static str {
    match presence {
        AdoptionObservationPresence::NotApplicable => "not-applicable",
        AdoptionObservationPresence::Absent => "absent",
        AdoptionObservationPresence::Indeterminate => "indeterminate",
        AdoptionObservationPresence::Recorded => "recorded",
    }
}

fn edge_verb_spelling(
    verb: &vibe_wire::generated::requirements_report::RequirementRelationVerb,
) -> &'static str {
    use vibe_wire::generated::requirements_report::RequirementRelationVerb as V;
    match verb {
        V::Implements => "implements",
        V::Verifies => "verifies",
        V::Documents => "documents",
        V::Deviates => "deviates",
        V::Informs => "informs",
    }
}

fn edge_provenance_spelling(
    provenance: &vibe_wire::generated::requirements_report::RequirementRelationProvenance,
) -> &'static str {
    use vibe_wire::generated::requirements_report::RequirementRelationProvenance as P;
    match provenance {
        P::Authored => "authored",
        P::Generated => "generated",
        P::Proposed => "proposed",
    }
}

fn stage_spelling(stage: &FactStatusStage) -> &'static str {
    match stage {
        FactStatusStage::Unknown => "unknown",
        FactStatusStage::Idea => "idea",
        FactStatusStage::Spec => "spec",
        FactStatusStage::Impl => "impl",
        FactStatusStage::Test => "test",
        FactStatusStage::Doc => "doc",
        FactStatusStage::Freeze => "freeze",
    }
}

fn fact_state_spelling(state: &FactStatusState) -> &'static str {
    match state {
        FactStatusState::Hold => "hold",
        FactStatusState::Plan => "plan",
        FactStatusState::Work => "work",
        FactStatusState::Done => "done",
        FactStatusState::Void => "void",
    }
}
