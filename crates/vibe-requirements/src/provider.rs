//! The optional relation-provider seam — one call per query, wire
//! states and provenance DERIVED here from the base source kind.
//!
//! A provider answers per-package outcomes and address-associated
//! edges; it never chooses `current|carried`, provenance, or a source
//! kind, so host-carried and package-current combinations are
//! unrepresentable rather than policed. Structural provider mistakes
//! (an extra package, a duplicate result, an edge for an address the
//! query never asked about, a kind-impossible outcome, an unbounded or
//! control-bearing reason) mark the affected result — or, when they
//! cannot be attributed, the whole answer — `invalid` with a FIXED
//! reason; rich provider text is never echoed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPTIONAL-IR-FACT-EVIDENCE");

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use thiserror::Error;
use vibe_wire::generated::requirements_report::{
    RelationSource, RelationSourceProvenance, RelationSourceState, RequirementRelation,
    RequirementRelationVerb, RequirementSourceKind,
};

/// Fixed reasons for provider-shape failures — bounded, nonblank, ours.
pub(crate) const REASON_PROVIDER_MISSING: &str = "relation-provider-missing";
pub(crate) const REASON_PROVIDER_FAILED: &str = "relation-provider-failed";
pub(crate) const REASON_RESULT_MISSING: &str = "provider-result-missing";
pub(crate) const REASON_RESULT_INVALID: &str = "provider-result-invalid";
pub(crate) const REASON_ANSWER_INVALID: &str = "provider-answer-invalid";
pub(crate) const REASON_EDGE_OUT_OF_SCOPE: &str = "provider-edge-out-of-scope";
pub(crate) const REASON_KIND_IMPOSSIBLE: &str = "provider-kind-impossible";
pub(crate) const REASON_REASON_UNBOUNDED: &str = "provider-reason-unbounded";
pub(crate) const REASON_REASON_BLANK: &str = "provider-reason-blank";
pub(crate) const REASON_EDGE_MALFORMED: &str = "provider-edge-malformed";
pub(crate) const REASON_EDGE_DUPLICATE: &str = "provider-edge-duplicate";

/// The wire's shared diagnostic cap for provider-supplied reasons —
/// `x-diagnostic-cap-bytes` in the requirements schema.
const REASON_CAP_BYTES: usize = 8192;

/// One enumerated base source as the provider sees it: the wire kind,
/// the coordinate, and the materialised root when one physically exists.
#[derive(Debug, Clone)]
pub struct ProviderSource<'a> {
    /// Host or package — decides the provenance the library derives.
    pub kind: RequirementSourceKind,
    /// The `group/name` coordinate.
    pub package: &'a str,
    /// The source's root on this machine, when it exists here.
    pub root: Option<&'a Path>,
}

/// The one request: trusted roots, every enumerated base source, and
/// the sorted limited output addresses edges may attach to.
#[derive(Debug)]
pub struct RelationRequest<'a> {
    /// The selected node root the query answered for.
    pub selected_root: &'a Path,
    /// The workspace root the lock was read from.
    pub workspace_root: &'a Path,
    /// Every enumerated base source, sorted by coordinate.
    pub sources: &'a [ProviderSource<'a>],
    /// The sorted addresses that survived the limit — the ONLY addresses
    /// whose rows exist, so the only addresses edges may name.
    pub addresses: &'a [String],
}

/// One package's provider outcome. `Available` carries edges; the three
/// loss states carry a bounded machine reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderOutcome {
    /// Edges for the requested addresses of this package.
    Available {
        edges: Vec<(String, RequirementRelation)>,
    },
    /// Present data that cannot prove its own freshness.
    Stale { reason: String },
    /// No relation data could be produced for this package.
    Unavailable { reason: String },
    /// Present data that did not parse / is not trustworthy.
    Invalid { reason: String },
}

impl ProviderOutcome {
    /// The bounded reason of a loss state, or `None` for `Available`.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available { .. } => None,
            Self::Stale { reason } | Self::Unavailable { reason } | Self::Invalid { reason } => {
                Some(reason)
            }
        }
    }
}

/// A whole-provider failure. The query maps it to typed `unavailable`
/// enrichment for every enumerated source; base rows still return.
#[derive(Debug, Error)]
#[error("the relation provider failed: {0}")]
pub struct ProviderError(pub String);

/// The optional relation seam. Called AT MOST ONCE per query and only
/// when the query asked for relations; a `relations = false` query
/// never constructs, dereferences or calls a provider.
///
/// The canonical shape: answer every requested source with one outcome,
/// attaching edges only for the addresses the request named.
///
/// ```
/// use std::path::Path;
/// use vibe_requirements::{
///     ProviderError, ProviderOutcome, ProviderSource, RelationProvider, RelationRequest,
/// };
/// use vibe_wire::generated::requirements_report::{
///     RequirementRelation, RequirementRelationProvenance, RequirementRelationVerb,
/// };
///
/// struct Static;
/// impl RelationProvider for Static {
///     fn relations(
///         &self,
///         request: &RelationRequest<'_>,
///     ) -> Result<Vec<(String, ProviderOutcome)>, ProviderError> {
///         Ok(request
///             .sources
///             .iter()
///             .map(|source: &ProviderSource<'_>| {
///                 let package = source.package.to_string();
///                 let edges = request
///                     .addresses
///                     .iter()
///                     .filter(|address| address.starts_with(&format!(
///                         "spec://{}/", source.package
///                     )))
///                     .map(|address| {
///                         (
///                             address.clone(),
///                             RequirementRelation {
///                                 verb: RequirementRelationVerb::Verifies,
///                                 provenance: RequirementRelationProvenance::Authored,
///                                 symbol: "x::t".to_string(),
///                                 file: "crates/x/src/lib.rs".to_string(),
///                                 line: 1,
///                             },
///                         )
///                     })
///                     .collect();
///                 (package, ProviderOutcome::Available { edges })
///             })
///             .collect())
///     }
/// }
///
/// let root = Path::new(".");
/// let sources = [ProviderSource {
///     kind: vibe_wire::generated::requirements_report::RequirementSourceKind::Package,
///     package: "org.example/pkg",
///     root: Some(root),
/// }];
/// let addresses = vec!["spec://org.example/pkg/RULE#P".to_string()];
/// let request = RelationRequest {
///     selected_root: root,
///     workspace_root: root,
///     sources: &sources,
///     addresses: &addresses,
/// };
/// let answer = Static.relations(&request).unwrap();
/// assert_eq!(answer.len(), 1);
/// assert!(matches!(answer[0].1, ProviderOutcome::Available { .. }));
/// ```
pub trait RelationProvider {
    /// Answer for every requested source. Results are per-package
    /// `(coordinate, outcome)` pairs; the library validates the shape.
    fn relations(
        &self,
        request: &RelationRequest<'_>,
    ) -> Result<Vec<(String, ProviderOutcome)>, ProviderError>;
}

/// The resolved enrichment layer: one wire `RelationSource` per
/// enumerated source plus the edges keyed by address.
pub(crate) struct ResolvedRelations {
    pub sources: Vec<RelationSource>,
    pub edges: BTreeMap<String, Vec<RequirementRelation>>,
}

/// Resolve the enrichment layer for the enumerated sources.
///
/// `requested` is the effective query's `relations` flag. When false,
/// every source gets an explicit `not-requested/none` row and no edge
/// exists — and the provider argument is never touched.
pub(crate) fn resolve(
    requested: bool,
    provider: Option<&dyn RelationProvider>,
    request: &RelationRequest<'_>,
) -> ResolvedRelations {
    if !requested {
        return ResolvedRelations {
            sources: request
                .sources
                .iter()
                .map(|source| RelationSource {
                    package: source.package.to_string(),
                    state: RelationSourceState::NotRequested,
                    provenance: RelationSourceProvenance::None,
                    reason_code: None,
                })
                .collect(),
            edges: BTreeMap::new(),
        };
    }

    let (outcomes, poisoned): (
        BTreeMap<String, Result<ProviderOutcome, &'static str>>,
        bool,
    ) = match provider {
        None => (all_outcomes(request, REASON_PROVIDER_MISSING), false),
        Some(provider) => match provider.relations(request) {
            Ok(results) => index_outcomes(request, results),
            Err(_) => (all_outcomes(request, REASON_PROVIDER_FAILED), false),
        },
    };

    let mut sources = Vec::with_capacity(request.sources.len());
    let mut edges: BTreeMap<String, Vec<RequirementRelation>> = BTreeMap::new();
    for source in request.sources {
        // A poisoned answer is invalid for EVERY enumerated source —
        // including sources the provider's answer omitted.
        let outcome = if poisoned {
            ProviderOutcome::Invalid {
                reason: REASON_ANSWER_INVALID.to_string(),
            }
        } else {
            match outcomes.get(source.package).cloned() {
                Some(Ok(outcome)) => outcome,
                Some(Err(fixed)) => ProviderOutcome::Invalid {
                    reason: fixed.to_string(),
                },
                None => ProviderOutcome::Unavailable {
                    reason: REASON_RESULT_MISSING.to_string(),
                },
            }
        };
        sources.push(RelationSource {
            package: source.package.to_string(),
            state: wire_state(&outcome, source),
            provenance: wire_provenance(&outcome, source),
            reason_code: bounded_reason(&outcome),
        });
        if let ProviderOutcome::Available {
            edges: package_edges,
        } = outcome
        {
            for (address, edge) in package_edges {
                edges.entry(address).or_default().push(edge);
            }
        }
    }
    // Canonical per-address order only — the wire sorts edges by
    // (verb, symbol, file, line). Duplicates were already refused per
    // outcome in `validate_outcome`; nothing is deduplicated silently.
    for row_edges in edges.values_mut() {
        row_edges.sort_by_key(edge_key);
    }
    ResolvedRelations { sources, edges }
}

/// Index a provider's answer by package, applying the shape laws:
/// an extra package poisons the whole answer (fixed
/// `provider-answer-invalid`); a duplicate marks that package's result
/// invalid; per-result validity (reason bounds, edge scope, kind
/// possibility) is checked per source.
fn index_outcomes(
    request: &RelationRequest<'_>,
    results: Vec<(String, ProviderOutcome)>,
) -> (
    BTreeMap<String, Result<ProviderOutcome, &'static str>>,
    bool,
) {
    let requested: BTreeMap<&str, ProviderSource<'_>> = request
        .sources
        .iter()
        .map(|source| (source.package, source.clone()))
        .collect();
    let mut indexed: BTreeMap<String, Result<ProviderOutcome, &'static str>> = BTreeMap::new();
    let mut poisoned = false;
    for (package, outcome) in results {
        let Some(source) = requested.get(package.as_str()) else {
            // A package this query never enumerated cannot be answered
            // for — unattributable, so the whole answer is invalid
            // (including sources the answer omitted; `resolve` applies
            // that below).
            poisoned = true;
            continue;
        };
        let outcome = validate_outcome(outcome, source.clone(), request);
        if indexed.insert(package.clone(), Ok(outcome)).is_some() {
            // A duplicate result for one package is a shape failure
            // of that package's answer.
            indexed.insert(package, Err(REASON_RESULT_INVALID));
        }
    }
    (indexed, poisoned)
}

/// Apply the per-result shape laws; failures become a FIXED invalid
/// reason rather than echoed provider text. The laws: a loss reason
/// must be nonblank, bounded and free of control bytes; a source with
/// no root here may only answer `Unavailable` (anything data-bearing is
/// kind-impossible); every edge must pass the wire's own per-edge shape
/// law (through the ONE public helper — no second path grammar), name
/// an address the query actually emitted, sit inside the outcome's own
/// `spec://<source.package>/` namespace, and never repeat an exact
/// (address, verb, symbol, file, line) key.
fn validate_outcome(
    outcome: ProviderOutcome,
    source: ProviderSource<'_>,
    request: &RelationRequest<'_>,
) -> ProviderOutcome {
    let invalid = |reason: &'static str| ProviderOutcome::Invalid {
        reason: reason.to_string(),
    };
    if let Some(reason) = outcome.reason() {
        if reason.trim().is_empty() {
            return invalid(REASON_REASON_BLANK);
        }
        if reason.len() > REASON_CAP_BYTES
            || reason.bytes().any(|b| matches!(b, b'\r' | b'\n' | b'\0'))
        {
            return invalid(REASON_REASON_UNBOUNDED);
        }
    }
    if source.root.is_none() && !matches!(outcome, ProviderOutcome::Unavailable { .. }) {
        // A rootless source has no map to read, prove stale, or fail to
        // parse — only an honest `Unavailable` is possible.
        return invalid(REASON_KIND_IMPOSSIBLE);
    }
    if let ProviderOutcome::Available { edges } = &outcome {
        let namespace = format!("spec://{}/", source.package);
        let mut seen: BTreeSet<(String, (String, String, String), u32)> = BTreeSet::new();
        for (address, edge) in edges {
            // The wire's own per-edge shape law, through the one public
            // helper — malformed edges never reach the final validator.
            if vibe_wire::behaviour::requirements_report::validate_edge(
                vibe_wire::behaviour::requirements_report::EdgeRef { row: 0, edge: 0 },
                edge,
            )
            .is_err()
            {
                return invalid(REASON_EDGE_MALFORMED);
            }
            // Globally requested AND inside this outcome's own
            // namespace: a host result cannot attach a package row's
            // edge.
            if !request.addresses.contains(address) || !address.starts_with(&namespace) {
                return invalid(REASON_EDGE_OUT_OF_SCOPE);
            }
            let key = (address.clone(), edge_symbol_file_verb(edge), edge.line);
            if !seen.insert(key) {
                return invalid(REASON_EDGE_DUPLICATE);
            }
        }
    }
    outcome
}

/// (verb, symbol, file) — the key halves of one edge, spelled by the
/// wire's own sort key.
fn edge_symbol_file_verb(edge: &RequirementRelation) -> (String, String, String) {
    (
        verb_spelling(&edge.verb).to_string(),
        edge.symbol.clone(),
        edge.file.clone(),
    )
}

fn all_outcomes(
    request: &RelationRequest<'_>,
    reason: &'static str,
) -> BTreeMap<String, Result<ProviderOutcome, &'static str>> {
    request
        .sources
        .iter()
        .map(|source| {
            (
                source.package.to_string(),
                Ok(ProviderOutcome::Unavailable {
                    reason: reason.to_string(),
                }),
            )
        })
        .collect()
}

/// The wire state for one outcome: the provider's loss word, or the
/// kind-derived success word (`current` for host, `carried` for
/// package).
fn wire_state(outcome: &ProviderOutcome, source: &ProviderSource<'_>) -> RelationSourceState {
    match outcome {
        ProviderOutcome::Available { .. } => match source.kind {
            RequirementSourceKind::Host => RelationSourceState::Current,
            RequirementSourceKind::Package => RelationSourceState::Carried,
        },
        ProviderOutcome::Stale { .. } => RelationSourceState::Stale,
        ProviderOutcome::Unavailable { .. } => RelationSourceState::Unavailable,
        ProviderOutcome::Invalid { .. } => RelationSourceState::Invalid,
    }
}

/// The wire provenance for one outcome: derived from state + kind, so
/// the validator's kind constraint holds by construction.
fn wire_provenance(
    outcome: &ProviderOutcome,
    source: &ProviderSource<'_>,
) -> RelationSourceProvenance {
    match outcome {
        ProviderOutcome::Unavailable { .. } => RelationSourceProvenance::None,
        _ => match source.kind {
            RequirementSourceKind::Host => RelationSourceProvenance::FreshProjectMap,
            RequirementSourceKind::Package => RelationSourceProvenance::CarriedPackageMap,
        },
    }
}

fn bounded_reason(outcome: &ProviderOutcome) -> Option<String> {
    outcome.reason().map(|reason| reason.to_string())
}

/// The wire's edge sort key — (verb, symbol, file, line) by WIRE
/// spelling, matching the validator's own ordering.
fn edge_key(edge: &RequirementRelation) -> (String, String, String, u32) {
    (
        verb_spelling(&edge.verb).to_string(),
        edge.symbol.clone(),
        edge.file.clone(),
        edge.line,
    )
}

fn verb_spelling(verb: &RequirementRelationVerb) -> &'static str {
    match verb {
        RequirementRelationVerb::Implements => "implements",
        RequirementRelationVerb::Verifies => "verifies",
        RequirementRelationVerb::Documents => "documents",
        RequirementRelationVerb::Deviates => "deviates",
        RequirementRelationVerb::Informs => "informs",
    }
}
