//! The requirements report's relational laws — the hand-written
//! validation cell beside the generated [`RequirementsReport`]
//! (PROP-054 §14.7 `##REF-REQUIREMENTS-WIRE`, `##FACT-QUERY-CONTRACT`;
//! R7.5 P1).
//!
//! JTD owns the FORM (the ten closed vocabularies, the optional
//! statuses, the explicit-even-when-empty lists); the laws a form
//! cannot say are named in the schema's `metadata.x-relational-laws`
//! (`schemas/requirements_report.jtd.json`) and enforced HERE, in one
//! pure pass over the generated type with typed errors. The two label
//! sets are pinned equal by `tests/requirements_report_wire.rs`, so an
//! undocumented law and an unimplemented label are both red — the seam
//! [`crate::behaviour::compile_trace_report`] and
//! [`crate::behaviour::verification_evidence`] already carry.
//!
//! The report answers on TWO source layers, and keeping them apart is
//! the point: `sources[]` says whether an authored source could be
//! read, `relation_sources[]` says what optional enrichment it could
//! offer, and only an `available` base result may own a fact row. That
//! is what makes a malformed source representable as itself instead of
//! disappearing into «no relations».
//!
//! What this cell refuses to become: a judge. It checks that four
//! typed observations are internally coherent — that a `recorded`
//! adoption carries a status, that a row's address coordinate equals
//! its source's, that a truncated answer really hit its own bound. It
//! never combines them, ranks them, or derives a verdict; that join is
//! the external orchestrator's policy
//! (`##REQUIREMENT-OBSERVATION-AXES`), and a validator that performed
//! it would smuggle the very rollup the wire refuses to carry.

use crate::behaviour::compiler_trace_index::{DIAGNOSTIC_CAP_BYTES, ScalarPreview};
use crate::behaviour::scalars::{has_control_bytes, is_lowercase_hex, relative_path_defect};
use crate::generated::requirements_report::{
    AdoptionObservationPresence, AuthoringObservationPresence, RequirementRow,
    RequirementSourceKind, RequirementsReport, SourceResultState,
};

mod coordinates;
mod errors;
mod sources;

pub use errors::{
    AddressDefect, EdgeRef, PathUnsafety, ReasonDefect, RequirementsError, SourceDefect,
    SourceStateDefect, StatusAxis,
};

use coordinates::{SCHEME, address_coordinate, address_defect};
use sources::{SourceIndex, relation_coverage, relation_sources_gate, sources_gate};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests_coherence.rs"]
mod tests_coherence;

#[cfg(test)]
#[path = "tests_relations.rs"]
mod tests_relations;

/// Every implemented law label, in schema order. Set-equal to the
/// schema's `x-relational-laws` prefixes by the wire test.
pub const IMPLEMENTED_LAWS: &[&str] = &[
    "report-identity",
    "query-bounds",
    "address-grammar",
    "prefix-scope",
    "row-order",
    "source-coherence",
    "source-result-matrix",
    "row-source-binding",
    "relation-state-matrix",
    "status-presence",
    "edge-bounds",
    "bounded-text",
    "truncation-honesty",
];

/// The requirements wire epoch this validator speaks.
pub const REQUIREMENTS_EPOCH: u32 = 1;

/// The inclusive row-bound range a query may ask for — the numbers
/// `##REF-REQUIREMENTS-SURFACES` fixes for both surfaces.
pub const LIMIT_MIN: u32 = 1;
/// See [`LIMIT_MIN`].
pub const LIMIT_MAX: u32 = 256;

/// The byte cap on an address or address prefix. Addresses are longer
/// than a diagnostic line and shorter than a document; they get their
/// own bound rather than borrowing the diagnostic cap, because the two
/// answer different questions.
pub const ADDRESS_CAP_BYTES: usize = 2 * 1024;

/// Validate one requirements report against every relational law.
/// Pure: the value in, the first broken law out.
pub fn validate(report: &RequirementsReport) -> Result<(), RequirementsError> {
    identity_gate(report)?;
    query_gate(report)?;
    let index = sources_gate(report)?;
    relation_sources_gate(report, &index)?;
    rows_gate(report, &index)?;
    relation_coverage(report)?;
    truncation_gate(report)
}

/// `report-identity`: the epoch, the two digests a reader joins
/// answers by, the optional lifecycle join key, and the node the
/// answer is about.
fn identity_gate(report: &RequirementsReport) -> Result<(), RequirementsError> {
    if report.requirements != REQUIREMENTS_EPOCH {
        return Err(RequirementsError::RequirementsEpoch {
            found: report.requirements,
        });
    }
    let observation = &report.observation;
    for (field, value) in [
        ("observation_id", &observation.observation_id),
        ("source_digest", &observation.source_digest),
    ] {
        if !is_sha256(value) {
            return Err(RequirementsError::DigestShape {
                field,
                value: preview(value),
            });
        }
    }
    if let Some(run_id) = observation.lifecycle_run_id.as_deref()
        && !is_lowercase_hex(run_id, 32)
    {
        return Err(RequirementsError::LifecycleRunIdShape {
            run_id: preview(run_id),
        });
    }
    if observation.selected != "."
        && let Some(reason) = relative_path_defect(&observation.selected)
    {
        return Err(RequirementsError::UnsafeSelected {
            selected: preview(&observation.selected),
            reason,
        });
    }
    Ok(())
}

/// `query-bounds`: the answer restates the question, so the question
/// it restates must be one the surfaces would have accepted.
fn query_gate(report: &RequirementsReport) -> Result<(), RequirementsError> {
    validate_query(&report.query)
}

/// The pure `query-bounds` entry over ONE effective query — the exact
/// grammar the full report validator applies to `report.query`, exposed
/// so a query library can refuse an unacceptable question BEFORE any
/// filesystem access, through the wire owner rather than a copy of the
/// prefix/cap/limit grammar (R7.5 A2b).
///
/// ```
/// use vibe_wire::behaviour::requirements_report::validate_query;
/// use vibe_wire::generated::requirements_report::RequirementsQuery;
///
/// let mut query = RequirementsQuery { limit: 100, relations: false, address_prefix: None };
/// assert!(validate_query(&query).is_ok());
/// query.limit = 0;
/// assert!(validate_query(&query).is_err());
/// query.limit = 100;
/// query.address_prefix = Some("req-one".to_string());
/// assert!(validate_query(&query).is_err()); // never a bare fact id
/// ```
pub fn validate_query(
    query: &crate::generated::requirements_report::RequirementsQuery,
) -> Result<(), RequirementsError> {
    if !(LIMIT_MIN..=LIMIT_MAX).contains(&query.limit) {
        return Err(RequirementsError::LimitOutOfRange { limit: query.limit });
    }
    let Some(prefix) = query.address_prefix.as_deref() else {
        return Ok(());
    };
    if prefix.len() > ADDRESS_CAP_BYTES {
        return Err(RequirementsError::ScalarOverCap {
            field: "address_prefix",
            bytes: prefix.len(),
            cap: ADDRESS_CAP_BYTES,
        });
    }
    if !prefix.starts_with(SCHEME) {
        return Err(RequirementsError::PrefixNotSpecUri {
            prefix: preview(prefix),
        });
    }
    if prefix.contains('\\') || has_control_bytes(prefix) {
        return Err(RequirementsError::UnsafePrefix {
            prefix: preview(prefix),
        });
    }
    Ok(())
}

/// Every per-row law: bounded text, the full address grammar, the
/// query's own scope, sorted unique rows, the base-source binding, the
/// two status matrices, and the edges' own bounds.
fn rows_gate(
    report: &RequirementsReport,
    sources: &SourceIndex<'_>,
) -> Result<(), RequirementsError> {
    let mut previous: Option<&str> = None;
    for (index, row) in report.rows.iter().enumerate() {
        row_text_gate(row)?;
        if let Some(defect) = address_defect(&row.address) {
            return Err(RequirementsError::AddressGrammar {
                index,
                address: preview(&row.address),
                defect,
            });
        }
        if let Some(prefix) = report.query.address_prefix.as_deref()
            && !row.address.starts_with(prefix)
        {
            return Err(RequirementsError::OutsidePrefix {
                index,
                address: preview(&row.address),
                prefix: preview(prefix),
            });
        }
        if let Some(previous) = previous
            && previous >= row.address.as_str()
        {
            return Err(RequirementsError::RowOrder {
                index,
                address: preview(&row.address),
                previous: preview(previous),
            });
        }
        previous = Some(&row.address);
        binding_gate(index, row, sources)?;
        status_gate(index, row)?;
        edges_gate(index, row)?;
    }
    Ok(())
}

/// `row-source-binding`: a row belongs to an `available` base source
/// result with the same `(kind, package)`, and the coordinate parsed
/// out of the row's own address is that same package. A row whose
/// address says one package while its source says another is two
/// claims wearing one row.
fn binding_gate(
    index: usize,
    row: &RequirementRow,
    sources: &SourceIndex<'_>,
) -> Result<(), RequirementsError> {
    let coordinate = address_coordinate(&row.address).unwrap_or_default();
    if coordinate != row.source.package {
        return Err(RequirementsError::CoordinateMismatch {
            index,
            address: preview(coordinate),
            package: preview(&row.source.package),
        });
    }
    match sources.get(row.source.package.as_str()) {
        Some(entry) if entry.kind != &row.source.kind => {
            Err(RequirementsError::RowSourceKindMismatch {
                index,
                package: preview(&row.source.package),
                declared: row.source.kind.clone(),
                base: entry.kind.clone(),
            })
        }
        Some(entry) if entry.state == SourceResultState::Available => Ok(()),
        Some(entry) => Err(RequirementsError::RowFromUnavailableSource {
            index,
            package: preview(&row.source.package),
            state: entry.state.clone(),
        }),
        None => Err(RequirementsError::RowWithoutSource {
            index,
            package: preview(&row.source.package),
        }),
    }
}

/// `bounded-text` over one row's own scalars — the address by the
/// address cap, everything else by the shared diagnostic cap, and no
/// value carrying a byte a reader cannot print.
fn row_text_gate(row: &RequirementRow) -> Result<(), RequirementsError> {
    if row.address.len() > ADDRESS_CAP_BYTES {
        return Err(RequirementsError::ScalarOverCap {
            field: "address",
            bytes: row.address.len(),
            cap: ADDRESS_CAP_BYTES,
        });
    }
    if has_control_bytes(&row.address) {
        return Err(RequirementsError::UnsafeScalar {
            field: "address",
            value: preview(&row.address),
        });
    }
    bounded(&row.source.package, "source.package")?;
    Ok(())
}

/// `status-presence`: a presence word and a status member are one
/// statement, and the SOURCE KIND decides which adoption words are
/// even legal — a host-authored fact has no consumer overlay to
/// consult, and a package fact always has one to answer about.
fn status_gate(index: usize, row: &RequirementRow) -> Result<(), RequirementsError> {
    let authoring_owes = matches!(row.authoring.presence, AuthoringObservationPresence::Marked);
    if authoring_owes != row.authoring.status.is_some() {
        return Err(RequirementsError::StatusPresence {
            index,
            axis: StatusAxis::Authoring,
            expected: authoring_owes,
        });
    }
    let adoption_owes = matches!(row.adoption.presence, AdoptionObservationPresence::Recorded);
    if adoption_owes != row.adoption.status.is_some() {
        return Err(RequirementsError::StatusPresence {
            index,
            axis: StatusAxis::Adoption,
            expected: adoption_owes,
        });
    }
    let not_applicable = matches!(
        row.adoption.presence,
        AdoptionObservationPresence::NotApplicable
    );
    let host = matches!(row.source.kind, RequirementSourceKind::Host);
    if host != not_applicable {
        return Err(RequirementsError::AdoptionKind {
            index,
            host,
            presence: row.adoption.presence.clone(),
        });
    }
    Ok(())
}

/// `edge-bounds` and the edge half of `row-order`: 1-based lines,
/// nonblank symbols, safe repo-relative files, and a sorted unique
/// edge set keyed by the WIRE spelling of the verb.
fn edges_gate(index: usize, row: &RequirementRow) -> Result<(), RequirementsError> {
    let mut previous: Option<(&str, &str, &str, u32)> = None;
    for (edge_index, edge) in row.relations.iter().enumerate() {
        let at = EdgeRef {
            row: index,
            edge: edge_index,
        };
        validate_edge(at, edge)?;
        let key = (
            verb_spelling(edge),
            edge.symbol.as_str(),
            edge.file.as_str(),
            edge.line,
        );
        if let Some(previous) = previous
            && previous >= key
        {
            return Err(RequirementsError::EdgeOrder { at });
        }
        previous = Some(key);
    }
    Ok(())
}

/// The pure per-edge shape law — 1-based line, nonblank symbol, safe
/// repo-relative file — extracted from `edge-bounds` so a relation
/// provider validates its edges through the ONE path grammar, never a
/// second one (R7.5 A2b follow-up C6). `at` only names the error's
/// position; the law itself is position-independent.
///
/// ```
/// use vibe_wire::behaviour::requirements_report::{EdgeRef, validate_edge};
/// use vibe_wire::generated::requirements_report::{
///     RequirementRelation, RequirementRelationProvenance, RequirementRelationVerb,
/// };
///
/// let at = EdgeRef { row: 0, edge: 0 };
/// let edge = RequirementRelation {
///     verb: RequirementRelationVerb::Verifies,
///     provenance: RequirementRelationProvenance::Authored,
///     symbol: "x::t".to_string(),
///     file: "crates/x/src/lib.rs".to_string(),
///     line: 1,
/// };
/// assert!(validate_edge(at, &edge).is_ok());
/// let broken = RequirementRelation { line: 0, ..edge };
/// assert!(validate_edge(at, &broken).is_err());
/// ```
pub fn validate_edge(
    at: EdgeRef,
    edge: &crate::generated::requirements_report::RequirementRelation,
) -> Result<(), RequirementsError> {
    // The complete per-edge scalar law lives here, including the
    // bounded-text half. A provider calling this helper must not pass an
    // over-cap/control-bearing edge only for the later full-report gate to
    // reject it under a different path.
    bounded(&edge.symbol, "relations.symbol")?;
    bounded(&edge.file, "relations.file")?;
    if edge.line == 0 {
        return Err(RequirementsError::EdgeLine { at });
    }
    if edge.symbol.trim().is_empty() {
        return Err(RequirementsError::EdgeBlank {
            at,
            field: "symbol",
        });
    }
    if let Some(reason) = relative_path_defect(&edge.file) {
        return Err(RequirementsError::EdgeFile {
            at,
            file: preview(&edge.file),
            reason,
        });
    }
    Ok(())
}

/// `truncation-honesty`: the row set never exceeds its own bound, and
/// a claim of truncation means the bound was actually reached.
fn truncation_gate(report: &RequirementsReport) -> Result<(), RequirementsError> {
    let rows = u32::try_from(report.rows.len()).unwrap_or(u32::MAX);
    if rows > report.query.limit {
        return Err(RequirementsError::RowsOverLimit {
            rows,
            limit: report.query.limit,
        });
    }
    if report.truncated && rows != report.query.limit {
        return Err(RequirementsError::TruncationClaim {
            rows,
            limit: report.query.limit,
        });
    }
    Ok(())
}

/// The WIRE spelling of an edge's verb — the sort key's first
/// component. It is deliberately NOT the generated enum's declaration
/// order: the two differ (`Verifies` is declared before `Documents`
/// while `documents` sorts before `verifies`), and sorting by the Rust
/// discriminant would produce an order no JSON reader could reproduce.
fn verb_spelling(
    edge: &crate::generated::requirements_report::RequirementRelation,
) -> &'static str {
    use crate::generated::requirements_report::RequirementRelationVerb as V;
    match edge.verb {
        V::Implements => "implements",
        V::Verifies => "verifies",
        V::Documents => "documents",
        V::Deviates => "deviates",
        V::Informs => "informs",
    }
}

/// `bounded-text` on one ordinary scalar: within the shared diagnostic
/// cap and free of bytes a reader cannot print.
fn bounded(value: &str, field: &'static str) -> Result<(), RequirementsError> {
    if value.len() > DIAGNOSTIC_CAP_BYTES {
        return Err(RequirementsError::ScalarOverCap {
            field,
            bytes: value.len(),
            cap: DIAGNOSTIC_CAP_BYTES,
        });
    }
    if has_control_bytes(value) {
        return Err(RequirementsError::UnsafeScalar {
            field,
            value: preview(value),
        });
    }
    Ok(())
}

/// One bounded preview — the trace index cell's type, reused.
fn preview(value: &str) -> ScalarPreview {
    ScalarPreview::of(value)
}

/// `sha256:` + 64 lowercase hex — the shared spelling.
fn is_sha256(value: &str) -> bool {
    crate::behaviour::scalars::is_sha256(value)
}
