//! The typed refusals of the requirements report's relational laws.
//! Same discipline as its two neighbours: a report is untrusted text,
//! so every scalar rides a bounded [`ScalarPreview`] and every index,
//! member name and axis is bounded by construction.
//!
//! This file owns the error enum, the law each variant witnesses, and
//! the rendering; the defect vocabularies those variants carry live in
//! the child `errors/defects.rs`.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::generated::requirements_report::{
    AdoptionObservationPresence, RelationSourceProvenance, RelationSourceState,
    RequirementSourceKind, SourceResultState,
};

mod defects;

pub use defects::{
    AddressDefect, EdgeRef, ReasonDefect, SourceDefect, SourceStateDefect, StatusAxis,
};

/// Why a project-relative forward-slashed path failed its law. The
/// GRAMMAR is shared with the evidence cell
/// ([`crate::behaviour::scalars`]); only the refusal wrapping it is
/// this cell's own.
pub use crate::behaviour::scalars::RelativePathDefect as PathUnsafety;

/// One broken relational law, typed end to end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementsError {
    /// `report-identity` — the report speaks an epoch this reader
    /// does not.
    RequirementsEpoch { found: u32 },
    /// `report-identity` — an observation digest is not `sha256:` +
    /// 64 lowercase hex.
    DigestShape {
        field: &'static str,
        value: ScalarPreview,
    },
    /// `report-identity` — the lifecycle join key is not a run id.
    LifecycleRunIdShape { run_id: ScalarPreview },
    /// `report-identity` — `observation.selected` is neither `.` nor
    /// a safe workspace-relative path.
    UnsafeSelected {
        selected: ScalarPreview,
        reason: PathUnsafety,
    },
    /// `query-bounds` — the limit is outside 1..=256.
    LimitOutOfRange { limit: u32 },
    /// `query-bounds` — the prefix is not a `spec://` prefix.
    PrefixNotSpecUri { prefix: ScalarPreview },
    /// `query-bounds` — the prefix carries a backslash or a control
    /// byte.
    UnsafePrefix { prefix: ScalarPreview },
    /// `address-grammar` — a row's address is not a full address.
    AddressGrammar {
        index: usize,
        address: ScalarPreview,
        defect: AddressDefect,
    },
    /// `prefix-scope` — a row fell outside the query's own scope.
    OutsidePrefix {
        index: usize,
        address: ScalarPreview,
        prefix: ScalarPreview,
    },
    /// `row-order` — rows are not strictly ascending by address (an
    /// out-of-order row, or a repeated one).
    RowOrder {
        index: usize,
        address: ScalarPreview,
        previous: ScalarPreview,
    },
    /// `source-coherence` — a package coordinate or a source list's
    /// order is wrong.
    SourceCoherence {
        index: usize,
        package: ScalarPreview,
        defect: SourceDefect,
    },
    /// `source-result-matrix` — a base source result's members do not
    /// follow from its state.
    SourceStateMatrix {
        index: usize,
        state: SourceResultState,
        defect: SourceStateDefect,
    },
    /// `source-result-matrix` — the base result's reason code does not
    /// follow from its state.
    SourceReason {
        index: usize,
        state: SourceResultState,
        defect: ReasonDefect,
    },
    /// `row-source-binding` — the coordinate in the row's own address
    /// is not the package its source names.
    CoordinateMismatch {
        index: usize,
        address: ScalarPreview,
        package: ScalarPreview,
    },
    /// `row-source-binding` — a row belongs to a source result that is
    /// not `available`; a malformed or missing source owns no rows.
    RowFromUnavailableSource {
        index: usize,
        package: ScalarPreview,
        state: SourceResultState,
    },
    /// `row-source-binding` — a row belongs to a source this answer
    /// never named.
    RowWithoutSource {
        index: usize,
        package: ScalarPreview,
    },
    /// `row-source-binding` — the package IS enumerated, under the
    /// other kind. Since one coordinate gets one base result, a row
    /// disagreeing with it about `host` vs `package` is naming a
    /// source that does not exist.
    RowSourceKindMismatch {
        index: usize,
        package: ScalarPreview,
        declared: RequirementSourceKind,
        base: RequirementSourceKind,
    },
    /// `relation-state-matrix` — the provenance does not follow from
    /// the state.
    RelationProvenance {
        index: usize,
        state: RelationSourceState,
        provenance: RelationSourceProvenance,
    },
    /// `relation-state-matrix` — the reason code does not follow from
    /// the state.
    RelationReason {
        index: usize,
        state: RelationSourceState,
        defect: ReasonDefect,
    },
    /// `relation-state-matrix` — `not-requested` exists exactly while
    /// the query did not ask for relations, and this source says the
    /// opposite of its own query.
    RelationRequestMismatch {
        index: usize,
        state: RelationSourceState,
        requested: bool,
    },
    /// `relation-state-matrix` — a host source claims carried
    /// provenance, or a package source claims a fresh project map.
    RelationProvenanceKind {
        index: usize,
        package: ScalarPreview,
        kind: RequirementSourceKind,
        provenance: RelationSourceProvenance,
    },
    /// `relation-state-matrix` — a row carries edges although the
    /// query loaded no map.
    EdgesWithoutRequest { index: usize },
    /// `relation-state-matrix` — relations were requested and this
    /// row's package has no relation-source result at all.
    RelationSourceMissing {
        index: usize,
        package: ScalarPreview,
    },
    /// `status-presence` — a presence word and its status member
    /// disagree. `expected` is whether a status was owed.
    StatusPresence {
        index: usize,
        axis: StatusAxis,
        expected: bool,
    },
    /// `status-presence` — the adoption presence contradicts the
    /// source kind: only a host row is `not-applicable`.
    AdoptionKind {
        index: usize,
        host: bool,
        presence: AdoptionObservationPresence,
    },
    /// `edge-bounds` — a 0 line; source lines are 1-based.
    EdgeLine { at: EdgeRef },
    /// `edge-bounds` — a blank symbol.
    EdgeBlank { at: EdgeRef, field: &'static str },
    /// `edge-bounds` — the file is not a safe repo-relative path.
    EdgeFile {
        at: EdgeRef,
        file: ScalarPreview,
        reason: PathUnsafety,
    },
    /// `row-order` — a row's edges are not strictly ascending by
    /// (verb, symbol, file, line).
    EdgeOrder { at: EdgeRef },
    /// `bounded-text` — a scalar exceeds its cap.
    ScalarOverCap {
        field: &'static str,
        bytes: usize,
        cap: usize,
    },
    /// `bounded-text` — a scalar carries CR, LF or NUL.
    UnsafeScalar {
        field: &'static str,
        value: ScalarPreview,
    },
    /// `truncation-honesty` — more rows than the query's own bound.
    RowsOverLimit { rows: u32, limit: u32 },
    /// `truncation-honesty` — a truncation claim the row count does
    /// not support.
    TruncationClaim { rows: u32, limit: u32 },
}

impl RequirementsError {
    /// The implemented-law label this violation witnesses.
    #[must_use]
    pub fn law(&self) -> &'static str {
        use RequirementsError as E;
        match self {
            E::RequirementsEpoch { .. }
            | E::DigestShape { .. }
            | E::LifecycleRunIdShape { .. }
            | E::UnsafeSelected { .. } => "report-identity",
            E::LimitOutOfRange { .. } | E::PrefixNotSpecUri { .. } | E::UnsafePrefix { .. } => {
                "query-bounds"
            }
            E::AddressGrammar { .. } => "address-grammar",
            E::OutsidePrefix { .. } => "prefix-scope",
            E::RowOrder { .. } | E::EdgeOrder { .. } => "row-order",
            E::SourceCoherence { .. } => "source-coherence",
            E::SourceStateMatrix { .. } | E::SourceReason { .. } => "source-result-matrix",
            E::CoordinateMismatch { .. }
            | E::RowFromUnavailableSource { .. }
            | E::RowWithoutSource { .. }
            | E::RowSourceKindMismatch { .. } => "row-source-binding",
            E::RelationProvenance { .. }
            | E::RelationReason { .. }
            | E::RelationRequestMismatch { .. }
            | E::RelationProvenanceKind { .. }
            | E::EdgesWithoutRequest { .. }
            | E::RelationSourceMissing { .. } => "relation-state-matrix",
            E::StatusPresence { .. } | E::AdoptionKind { .. } => "status-presence",
            E::EdgeLine { .. } | E::EdgeBlank { .. } | E::EdgeFile { .. } => "edge-bounds",
            E::ScalarOverCap { .. } | E::UnsafeScalar { .. } => "bounded-text",
            E::RowsOverLimit { .. } | E::TruncationClaim { .. } => "truncation-honesty",
        }
    }
}

impl std::error::Error for RequirementsError {}

/// The wire spelling of a base source-result state.
fn source_state_spelling(state: &SourceResultState) -> &'static str {
    use SourceResultState as S;
    match state {
        S::Available => "available",
        S::Unavailable => "unavailable",
        S::Invalid => "invalid",
        S::Orphaned => "orphaned",
    }
}

/// The wire spelling of a relation-source state.
fn state_spelling(state: &RelationSourceState) -> &'static str {
    use RelationSourceState as S;
    match state {
        S::NotRequested => "not-requested",
        S::Current => "current",
        S::Carried => "carried",
        S::Stale => "stale",
        S::Unavailable => "unavailable",
        S::Invalid => "invalid",
    }
}

/// The wire spelling of a relation-source provenance.
fn provenance_spelling(provenance: &RelationSourceProvenance) -> &'static str {
    use RelationSourceProvenance as P;
    match provenance {
        P::FreshProjectMap => "fresh-project-map",
        P::CarriedPackageMap => "carried-package-map",
        P::None => "none",
    }
}

/// The wire spelling of a source kind.
fn kind_spelling(kind: &RequirementSourceKind) -> &'static str {
    match kind {
        RequirementSourceKind::Host => "host",
        RequirementSourceKind::Package => "package",
    }
}

/// The wire spelling of an adoption presence.
fn adoption_spelling(presence: &AdoptionObservationPresence) -> &'static str {
    use AdoptionObservationPresence as A;
    match presence {
        A::NotApplicable => "not-applicable",
        A::Absent => "absent",
        A::Indeterminate => "indeterminate",
        A::Recorded => "recorded",
    }
}

impl std::fmt::Display for RequirementsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RequirementsError as E;
        match self {
            E::RequirementsEpoch { found } => write!(
                f,
                "requirements = {found}; this reader speaks epoch {}",
                super::REQUIREMENTS_EPOCH
            ),
            E::DigestShape { field, value } => write!(
                f,
                "observation.{field} {value} is not `sha256:` followed by 64 lowercase hex characters"
            ),
            E::LifecycleRunIdShape { run_id } => write!(
                f,
                "observation.lifecycle_run_id {run_id} is not exactly 32 lowercase hex characters"
            ),
            E::UnsafeSelected { selected, reason } => write!(
                f,
                "observation.selected {selected} {} — it is `.` or a workspace-relative forward-slashed path",
                reason.phrase()
            ),
            E::LimitOutOfRange { limit } => write!(
                f,
                "query.limit = {limit} is outside {}..={}; zero answers nothing and an unbounded query is not bounded metadata",
                super::LIMIT_MIN,
                super::LIMIT_MAX
            ),
            E::PrefixNotSpecUri { prefix } => write!(
                f,
                "query.address_prefix {prefix} is not a `spec://` prefix; a bare fact id names nothing resolvable"
            ),
            E::UnsafePrefix { prefix } => write!(
                f,
                "query.address_prefix {prefix} carries a backslash, CR, LF or NUL"
            ),
            E::AddressGrammar {
                index,
                address,
                defect,
            } => write!(f, "rows[{index}].address {address} {}", defect.phrase()),
            E::OutsidePrefix {
                index,
                address,
                prefix,
            } => write!(
                f,
                "rows[{index}].address {address} falls outside the query's own prefix {prefix}"
            ),
            E::RowOrder {
                index,
                address,
                previous,
            } => write!(
                f,
                "rows[{index}].address {address} does not follow {previous}; rows are sorted by address and each appears once"
            ),
            E::SourceCoherence {
                index,
                package,
                defect,
            } => write!(f, "index {index}: package {package} {}", defect.phrase()),
            E::SourceStateMatrix {
                index,
                state,
                defect,
            } => write!(
                f,
                "sources[{index}]: state `{}` {}",
                source_state_spelling(state),
                defect.phrase()
            ),
            E::SourceReason {
                index,
                state,
                defect,
            } => write!(
                f,
                "sources[{index}]: state `{}` {}",
                source_state_spelling(state),
                defect.phrase()
            ),
            E::CoordinateMismatch {
                index,
                address,
                package,
            } => write!(
                f,
                "rows[{index}]: the address coordinate {address} is not the source package {package}; \
                 a row that names two packages is two claims"
            ),
            E::RowFromUnavailableSource {
                index,
                package,
                state,
            } => write!(
                f,
                "rows[{index}]: package {package} is `{}`, and only an `available` source owns fact rows",
                source_state_spelling(state)
            ),
            E::RowWithoutSource { index, package } => write!(
                f,
                "rows[{index}]: package {package} has no source result; a row with no source is an unsourced claim"
            ),
            E::RowSourceKindMismatch {
                index,
                package,
                declared,
                base,
            } => write!(
                f,
                "rows[{index}]: the row calls {package} a `{}` source while the base layer calls it a `{}`; one coordinate gets one source result",
                kind_spelling(declared),
                kind_spelling(base)
            ),
            E::RelationProvenance {
                index,
                state,
                provenance,
            } => write!(
                f,
                "relation_sources[{index}]: state `{}` cannot carry provenance `{}`",
                state_spelling(state),
                provenance_spelling(provenance)
            ),
            E::RelationReason {
                index,
                state,
                defect,
            } => write!(
                f,
                "relation_sources[{index}]: state `{}` {}",
                state_spelling(state),
                defect.phrase()
            ),
            E::RelationRequestMismatch {
                index,
                state,
                requested,
            } => write!(
                f,
                "relation_sources[{index}]: state `{}` with query.relations = {requested}; \
                 `not-requested` exists exactly while relations were not asked for",
                state_spelling(state)
            ),
            E::RelationProvenanceKind {
                index,
                package,
                kind,
                provenance,
            } => write!(
                f,
                "relation_sources[{index}]: {package} is a `{}` source, so it cannot carry `{}` — a host map is built fresh, a package map is carried",
                kind_spelling(kind),
                provenance_spelling(provenance)
            ),
            E::EdgesWithoutRequest { index } => write!(
                f,
                "rows[{index}] carries relation edges although query.relations is false; \
                 a query that did not ask loaded no map"
            ),
            E::RelationSourceMissing { index, package } => write!(
                f,
                "rows[{index}]: relations were requested but package {package} has no relation-source result, even to say it has no edges"
            ),
            E::StatusPresence {
                index,
                axis,
                expected,
            } => write!(
                f,
                "rows[{index}].{}: its presence word {} a status member",
                axis.as_str(),
                if *expected { "owes" } else { "forbids" }
            ),
            E::AdoptionKind {
                index,
                host,
                presence,
            } => write!(
                f,
                "rows[{index}]: a `{}` row cannot carry adoption `{}` — only a host-authored fact has no consumer overlay to consult",
                if *host { "host" } else { "package" },
                adoption_spelling(presence)
            ),
            E::EdgeLine { at } => {
                write!(f, "{at}.line is 0; source lines are 1-based")
            }
            E::EdgeBlank { at, field } => write!(f, "{at}.{field} is blank"),
            E::EdgeFile { at, file, reason } => {
                write!(f, "{at}.file {file} {}", reason.phrase())
            }
            E::EdgeOrder { at } => write!(
                f,
                "{at} does not follow its predecessor; edges are sorted by (verb, symbol, file, line) — the verb by its WIRE spelling — and each appears once"
            ),
            E::ScalarOverCap { field, bytes, cap } => {
                write!(f, "`{field}` is {bytes} bytes, over the {cap} byte cap")
            }
            E::UnsafeScalar { field, value } => {
                write!(f, "`{field}` {value} carries CR, LF or NUL")
            }
            E::RowsOverLimit { rows, limit } => write!(
                f,
                "the report carries {rows} rows over its own limit of {limit}"
            ),
            E::TruncationClaim { rows, limit } => write!(
                f,
                "the report claims truncation with {rows} rows and a limit of {limit}; \
                 a truncated answer reached its bound"
            ),
        }
    }
}
