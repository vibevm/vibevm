//! The R7.5 A3 relation adapter — `vibe-trace`'s implementation of the
//! ONE landed `vibe_requirements::RelationProvider` seam (PROP-054
//! `##OPTIONAL-IR-FACT-EVIDENCE`; R7 architecture §2.3/§5.3, rulings
//! `500f7b62`/`a1095d8e`).
//!
//! The adapter answers in the PROVIDER domain only — `Available |
//! Stale | Unavailable | Invalid` with fixed bounded reason codes —
//! and never chooses the wire `current|carried` states or provenance:
//! `vibe-requirements` derives those from the base source kind. It
//! reads, never writes: no `specmap.json` is created or touched.
//!
//! **Host/current**: `Config::load` exactly once on the request's host
//! root; absent config, malformed config, or a namespace that is not
//! the host coordinate are typed losses — never a false zero-edge
//! `current`. The map is built fresh in memory
//! (`specmap_core::index::build`) per call.
//!
//! **Package/carried trust ladder** (this proves exactly "the map byte
//! published in the lock-selected package snapshot", nothing broader):
//! lock-selected root/hash from the request — no `vibedeps` rescan, no
//! manifest read, no second lock authority — then slot record →
//! `source_hash` equality → the record's own `package.specmap.json`
//! row → the file's SHA-256 against that row → parse. A mismatching
//! byte is `stale`; a matching byte that does not parse is `invalid`.
//! Transformed-source rebuilding and map-level source witnesses are
//! deliberately NOT claimed: the shipped artefact cannot prove where
//! the map was built from, only which byte was published.
//!
//! **Edge projection — coordinates only, no bodies**: `uri` selects
//! the requested address; `from_symbol`/`verb`/`provenance`/`file`/
//! `line` map exhaustively onto the generated relation; `file` becomes
//! workspace-root-relative (the source root's strict prefix + the
//! map-relative path). Spec units, code bodies, warnings, suspects,
//! `pinned_r` and `edge.reason` never cross the seam.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use specmap_core::generated::specmap::{EdgeProvenance, EdgeVerb, Specmap};
use vibe_requirements::{ProviderOutcome, ProviderSource, RelationProvider, RelationRequest};
use vibe_wire::generated::requirements_report::{
    RequirementRelation, RequirementRelationProvenance, RequirementRelationVerb,
};

use crate::foreign::{MAP_FILENAME, parse_map};

/// The read-only specmap relation provider: host maps built fresh in
/// memory, package maps proven against the lock-selected slot record.
///
/// ```
/// use vibe_requirements::{ProviderSource, RelationProvider, RelationRequest};
/// use vibe_trace::SpecmapRelationProvider;
/// use vibe_wire::generated::requirements_report::RequirementSourceKind;
///
/// let root = tempfile::TempDir::new().unwrap();
/// std::fs::write(
///     root.path().join("specmap.toml"),
///     format!(
///         "namespace = \"org.example/demo\"\nscan_roots = [\"crates/*\"]\n\
///          spec_roots = [\"{}\"]\n",
///         vibe_core::layout::current_specs_root().to_string_lossy().replace('\\', "/")
///     ),
/// )
/// .unwrap();
/// let sources = [ProviderSource {
///     kind: RequirementSourceKind::Host,
///     package: "org.example/demo",
///     root: Some(root.path()),
///     expected_content_hash: None,
/// }];
/// let addresses: Vec<String> = Vec::new();
/// let request = RelationRequest {
///     selected_root: root.path(),
///     workspace_root: root.path(),
///     sources: &sources,
///     addresses: &addresses,
/// };
/// let answer = SpecmapRelationProvider.relations(&request).unwrap();
/// assert_eq!(answer.len(), 1);
/// // An empty tree with a matching namespace is an honest zero-edge
/// // Available — the namespace gate already passed.
/// assert!(matches!(answer[0].1, vibe_requirements::ProviderOutcome::Available { .. }));
/// ```
pub struct SpecmapRelationProvider;

impl RelationProvider for SpecmapRelationProvider {
    fn relations(
        &self,
        request: &RelationRequest<'_>,
    ) -> Result<Vec<(String, ProviderOutcome)>, vibe_requirements::ProviderError> {
        // Deterministic request-source order; per-source failure never
        // fails the answer (the base query keeps its rows regardless).
        Ok(request
            .sources
            .iter()
            .map(|source| (source.package.to_string(), outcome_for(source, request)))
            .collect())
    }
}

/// One source's outcome, dispatched by kind.
fn outcome_for(source: &ProviderSource<'_>, request: &RelationRequest<'_>) -> ProviderOutcome {
    match source.kind {
        vibe_wire::generated::requirements_report::RequirementSourceKind::Host => {
            host_outcome(source, request)
        }
        vibe_wire::generated::requirements_report::RequirementSourceKind::Package => {
            package_outcome(source, request)
        }
    }
}

// --- Host / current -------------------------------------------------------

/// Fixed host reason codes.
const HOST_ROOT_ABSENT: &str = "host-root-absent";
const HOST_HASH_UNEXPECTED: &str = "host-content-hash-unexpected";
const CONFIG_ABSENT: &str = "project-map-config-absent";
const CONFIG_INVALID: &str = "project-map-config-invalid";
const NAMESPACE_MISMATCH: &str = "project-map-namespace-mismatch";

fn host_outcome(source: &ProviderSource<'_>, request: &RelationRequest<'_>) -> ProviderOutcome {
    let Some(root) = source.root else {
        return unavailable(HOST_ROOT_ABSENT);
    };
    if source.expected_content_hash.is_some() {
        // The landed query never sends a lock hash for the host; a
        // request that does is malformed, and the honest answer is a
        // typed loss, never a guess.
        return invalid(HOST_HASH_UNEXPECTED);
    }
    // Root containment is decided BEFORE any source I/O: an outside
    // root refuses without touching a config, record or map.
    let prefix = match checked_prefix(root, request) {
        Ok(prefix) => prefix,
        Err(outcome) => return outcome,
    };
    let config = match specmap_core::config::Config::load(root) {
        Ok(Some(config)) => config,
        Ok(None) => return unavailable(CONFIG_ABSENT),
        Err(_) => return invalid(CONFIG_INVALID),
    };
    // The minted URIs use the config's namespace; the host identity is
    // the request's coordinate. A mismatch builds the WRONG tree, and
    // a zero-edge `current` would claim "built and found nothing" —
    // so it is a typed unavailable, never a false current.
    if config.namespace != source.package {
        return unavailable(NAMESPACE_MISMATCH);
    }
    let map = specmap_core::index::build(root, &config);
    finish_with_edges(&map, source, request, &prefix)
}

// --- Package / carried trust ladder ---------------------------------------

/// Fixed carried reason codes — the ladder's rungs, in order.
const SLOT_ABSENT: &str = "package-slot-absent";
const LOCK_HASH_MISSING: &str = "lock-content-hash-missing";
const SLOT_RECORD_UNAVAILABLE: &str = "slot-record-unavailable";
const SOURCE_HASH_MISMATCH: &str = "slot-source-hash-mismatch";
const MAP_NOT_SHIPPED: &str = "carried-map-not-shipped";
const MAP_UNAVAILABLE: &str = "carried-map-unavailable";
const MAP_MODIFIED: &str = "carried-map-modified";
const MAP_UNPARSEABLE: &str = "carried-map-unparseable";
const ROOT_OUTSIDE_WORKSPACE: &str = "relation-root-outside-workspace";

fn package_outcome(source: &ProviderSource<'_>, request: &RelationRequest<'_>) -> ProviderOutcome {
    let Some(root) = source.root else {
        return unavailable(SLOT_ABSENT);
    };
    let Some(expected_hash) = source.expected_content_hash else {
        // The request owes the lock's authority for every locked
        // package; its absence is malformed metadata.
        return invalid(LOCK_HASH_MISSING);
    };
    // Root containment BEFORE any slot-record read.
    let prefix = match checked_prefix(root, request) {
        Ok(prefix) => prefix,
        Err(outcome) => return outcome,
    };
    // In-place and record-less slots land here too: the record IS the
    // trust ladder's first rung, and without it there is nothing to
    // prove the carried byte against.
    let record = match vibe_workspace::vibedeps::read_slot_record(root) {
        Ok(record) => record,
        Err(_) => return unavailable(SLOT_RECORD_UNAVAILABLE),
    };
    if record.source_hash.as_str() != expected_hash {
        return stale(SOURCE_HASH_MISMATCH);
    }
    let Some(row) = record.files.iter().find(|file| file.path == MAP_FILENAME) else {
        return unavailable(MAP_NOT_SHIPPED);
    };
    // The ONE capability-relative raw read. The shared safefs cell
    // refuses symlink/reparse/hardlink, pins the slot root, rechecks the
    // final name after EOF, and the pre/post EntryProof pair catches a
    // replacement between sizing and reading. No full slot walk.
    let bytes = match read_owned_map(root) {
        Ok(bytes) => bytes,
        Err(()) => return unavailable(MAP_UNAVAILABLE),
    };
    carried_outcome_from_bytes(&bytes, &row.sha256, source, request, &prefix)
}

/// Read the carried map as one identity-bound byte value through the
/// shared capability filesystem cell. The observed file length is only
/// the allocation/read fence; acceptance also requires the same opaque
/// object and length before and after the bounded read.
fn read_owned_map(root: &std::path::Path) -> Result<Vec<u8>, ()> {
    let project = vibe_safefs::Project::open(root).map_err(|_| ())?;
    let directory = project.root_dir().map_err(|_| ())?;
    let Some((before, before_len)) = project
        .inspect_file_in(&directory, MAP_FILENAME)
        .map_err(|_| ())?
    else {
        return Err(());
    };
    let cap = usize::try_from(before_len).map_err(|_| ())?;
    let Some(bytes) = project
        .read_file_bounded_in(&directory, MAP_FILENAME, cap)
        .map_err(|_| ())?
    else {
        return Err(());
    };
    let Some((after, after_len)) = project
        .inspect_file_in(&directory, MAP_FILENAME)
        .map_err(|_| ())?
    else {
        return Err(());
    };
    if before != after || before_len != after_len || bytes.len() as u64 != after_len {
        return Err(());
    }
    Ok(bytes)
}

/// The carried ladder's byte-level core: hash and parse the SAME bytes.
/// Pure over its inputs — the disk is not touched — which is exactly
/// the seam that proves the accepted edges came from the hashed byte.
pub(crate) fn carried_outcome_from_bytes(
    bytes: &[u8],
    row_sha256: &str,
    source: &ProviderSource<'_>,
    request: &RelationRequest<'_>,
    prefix: &str,
) -> ProviderOutcome {
    if vibe_workspace::vibedeps::sha256_bytes(bytes) != row_sha256 {
        return stale(MAP_MODIFIED);
    }
    let map = match parse_map(bytes) {
        Ok(map) => map,
        Err(_) => return invalid(MAP_UNPARSEABLE),
    };
    finish_with_edges(&map, source, request, prefix)
}

/// The checked workspace-relative prefix for a source root — the one
/// containment decision, computed before any source I/O and reused by
/// the edge projection (never stripped twice).
fn checked_prefix(
    root: &std::path::Path,
    request: &RelationRequest<'_>,
) -> Result<String, ProviderOutcome> {
    root.strip_prefix(request.workspace_root)
        .map(vibe_core::machine_json_path)
        .map_err(|_| invalid(ROOT_OUTSIDE_WORKSPACE))
}

// --- Edge projection ------------------------------------------------------

/// Project a proven map's edges onto the request: filter to requested
/// addresses inside this source's coordinate, map fields exhaustively,
/// and make `file` workspace-root-relative. A source root outside the
/// workspace is a typed invalid for that source.
fn finish_with_edges(
    map: &Specmap,
    source: &ProviderSource<'_>,
    request: &RelationRequest<'_>,
    prefix: &str,
) -> ProviderOutcome {
    let namespace = format!("spec://{}/", source.package);
    let mut edges = Vec::new();
    for edge in &map.edges {
        // The adapter filters; it never asks the library to drop.
        if !request.addresses.contains(&edge.uri) || !edge.uri.starts_with(&namespace) {
            continue;
        }
        // The workspace-root host keeps map-relative files unchanged
        // (its strict prefix is empty); every other root — a selected
        // member or a package slot — prefixes the map-relative path.
        let file = if prefix.is_empty() {
            edge.file.clone()
        } else {
            format!("{prefix}/{}", edge.file)
        };
        edges.push((
            edge.uri.clone(),
            RequirementRelation {
                verb: map_verb(&edge.verb),
                provenance: map_provenance(&edge.provenance),
                symbol: edge.fromSymbol.clone(),
                file,
                line: edge.line,
            },
        ));
    }
    ProviderOutcome::Available { edges }
}

fn map_verb(verb: &EdgeVerb) -> RequirementRelationVerb {
    match verb {
        EdgeVerb::Deviates => RequirementRelationVerb::Deviates,
        EdgeVerb::Documents => RequirementRelationVerb::Documents,
        EdgeVerb::Implements => RequirementRelationVerb::Implements,
        EdgeVerb::Informs => RequirementRelationVerb::Informs,
        EdgeVerb::Verifies => RequirementRelationVerb::Verifies,
    }
}

fn map_provenance(provenance: &EdgeProvenance) -> RequirementRelationProvenance {
    match provenance {
        EdgeProvenance::Authored => RequirementRelationProvenance::Authored,
        EdgeProvenance::Generated => RequirementRelationProvenance::Generated,
        EdgeProvenance::Proposed => RequirementRelationProvenance::Proposed,
    }
}

fn unavailable(reason: &'static str) -> ProviderOutcome {
    ProviderOutcome::Unavailable {
        reason: reason.to_string(),
    }
}

fn stale(reason: &'static str) -> ProviderOutcome {
    ProviderOutcome::Stale {
        reason: reason.to_string(),
    }
}

fn invalid(reason: &'static str) -> ProviderOutcome {
    ProviderOutcome::Invalid {
        reason: reason.to_string(),
    }
}

/// The one map edge shape the tests build fixtures from — coordinates
/// only, mirroring the engine's canonical JSON.
#[cfg(test)]
pub(crate) fn fixture_edge(
    uri: &str,
    verb: EdgeVerb,
    provenance: EdgeProvenance,
    from_symbol: &str,
    file: &str,
    line: u32,
) -> specmap_core::generated::specmap::Edge {
    specmap_core::generated::specmap::Edge {
        file: file.to_string(),
        fromSymbol: from_symbol.to_string(),
        line,
        provenance,
        uri: uri.to_string(),
        verb,
        pinnedR: None,
        reason: None,
    }
}

/// Name the map artefact this adapter reads (kept beside the fixture
/// helper so the tests never re-spell it).
#[cfg(test)]
pub(crate) const FIXTURE_MAP_FILENAME: &str = MAP_FILENAME;
