//! The one public assembly point.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

use std::path::PathBuf;

use vibe_wire::behaviour::requirements_report::{REQUIREMENTS_EPOCH, validate, validate_query};
use vibe_wire::generated::requirements_report as wire;
use vibe_wire::generated::shared::Timestamp;

use crate::QueryError;
use crate::digest::{observation_id, scope_digest};
use crate::provider::{ProviderSource, RelationProvider, RelationRequest, resolve};
use crate::rows;
use crate::sources::{self, SourceOutcome};

/// The validated effective query. Defaults: no prefix, `limit = 100`,
/// `relations = false`. The grammar itself belongs to the wire owner —
/// [`RequirementsQuery::try_new`] converts into the generated query and
/// refuses through [`validate_query`] before any caller touches a
/// filesystem.
///
/// ```
/// use vibe_requirements::RequirementsQuery;
///
/// let q = RequirementsQuery::default();
/// assert_eq!(q.limit(), 100);
/// assert!(!q.relations());
/// assert!(q.address_prefix().is_none());
/// assert!(RequirementsQuery::try_new(None, 0, false).is_err());
/// assert!(RequirementsQuery::try_new(Some("req-one"), 100, false).is_err());
/// assert!(RequirementsQuery::try_new(Some("spec://org.demo/doc"), 256, true).is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementsQuery {
    pub(crate) address_prefix: Option<String>,
    pub(crate) limit: u32,
    pub(crate) relations: bool,
}

impl Default for RequirementsQuery {
    fn default() -> Self {
        Self {
            address_prefix: None,
            limit: 100,
            relations: false,
        }
    }
}

impl RequirementsQuery {
    /// Build and validate the effective query in one step.
    pub fn try_new(
        address_prefix: Option<&str>,
        limit: u32,
        relations: bool,
    ) -> Result<Self, QueryError> {
        let query = Self {
            address_prefix: address_prefix.map(str::to_string),
            limit,
            relations,
        };
        validate_query(&query.effective()).map_err(|source| QueryError::InvalidQuery { source })?;
        Ok(query)
    }

    /// The effective `spec://` prefix, when one scopes the query.
    pub fn address_prefix(&self) -> Option<&str> {
        self.address_prefix.as_deref()
    }

    /// The effective row bound (1..=256).
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Whether relation enrichment was requested.
    pub fn relations(&self) -> bool {
        self.relations
    }

    /// The generated twin — the query as the answer restates it.
    pub(crate) fn effective(&self) -> wire::RequirementsQuery {
        wire::RequirementsQuery {
            limit: self.limit,
            relations: self.relations,
            address_prefix: self.address_prefix.clone(),
        }
    }
}

/// The trusted constructor inputs — never wire or MCP members. The
/// selected root is the node the query answers for; `observed_at` is
/// injected by the caller (the library never clocks itself); the
/// lifecycle run id is injected by the surface through the existing
/// read-only lifecycle peek and is validated here when present.
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// The selected workspace node's root — CLI `--path` / MCP server
    /// context authority.
    pub selected_root: PathBuf,
    /// When the observation was taken. Excluded from `observation_id`.
    pub observed_at: Timestamp,
    /// The current lifecycle run id when one exists in durable state —
    /// the join key to a verification-evidence document. The query
    /// itself creates no state and never mints one.
    pub lifecycle_run_id: Option<String>,
}

/// Answer one requirements query over one selected project.
///
/// The ONE constructor of report members: surfaces build a
/// [`RequirementsQuery`] and a [`QueryContext`], call this, and render
/// ([`text::render`]) or serialise what comes back. The assembled
/// report is validated through the P1 relational validator before it is
/// returned.
pub fn query(
    query: &RequirementsQuery,
    context: &QueryContext,
    provider: Option<&dyn RelationProvider>,
) -> Result<wire::RequirementsReport, QueryError> {
    // Refuse an unacceptable question BEFORE any filesystem access,
    // through the wire owner's own grammar.
    validate_query(&query.effective()).map_err(|source| QueryError::InvalidQuery { source })?;
    if let Some(run_id) = &context.lifecycle_run_id
        && !is_run_id(run_id)
    {
        return Err(QueryError::InvalidRunId {
            run_id: run_id.clone(),
        });
    }

    // One discovery epoch for the whole answer.
    let selected = vibe_workspace::Workspace::discover_selected(&context.selected_root)
        .map_err(|source| QueryError::Workspace { source })?;
    let workspace_root = selected.workspace.root.clone();
    let selected_str = selected.selected.as_str().to_string();

    // The registry snapshot: one read, parsed from the same bytes whose
    // witnesses ride the scope digest. Malformed ⇒ typed abort.
    let snapshot = vibe_facts::Registry::load_with_witnesses(&selected.selected_root)
        .map_err(|source| QueryError::Registry { source })?;

    // The source universe, then one A2a observation per root-bearing
    // source, with coordinate-level prefix pruning. Enumeration reuses
    // the SAME epoch — no second selected-manifest read.
    let coords = sources::enumerate(&workspace_root, &selected, &snapshot.registry)?;
    let prefix_coordinate = query.address_prefix.as_deref().and_then(prefix_coordinate);
    let mut outcomes: Vec<SourceOutcome> = Vec::new();
    for coord in &coords {
        if let Some(scope) = &prefix_coordinate
            && &coord.package != scope
        {
            continue;
        }
        match coord.root.as_deref() {
            Some(_) => outcomes.push(sources::observe(coord)?),
            None => outcomes.push(sources::orphan_or_unavailable(coord, &snapshot.registry)?),
        }
    }

    // Rows: the A1 join per available source, merged, prefix-filtered,
    // globally sorted, then cut once.
    let mut all_rows = Vec::new();
    for outcome in &outcomes {
        if outcome.result.state != wire::SourceResultState::Available {
            continue;
        }
        let joined = vibe_facts::join_adoption(
            &snapshot.registry,
            &outcome.result.source.package,
            facts_kind(&outcome.result.source.kind),
            &outcome.facts,
        )
        .map_err(|source| QueryError::Invariant(format!("adoption join failed: {source}")))?;
        all_rows.extend(rows::build(
            &outcome.result.source.kind,
            &outcome.result.source.package,
            &joined,
        ));
    }
    if let Some(prefix) = &query.address_prefix {
        all_rows.retain(|row| row.address.starts_with(prefix.as_str()));
    }
    all_rows.sort_by(|a, b| a.address.cmp(&b.address));
    rows::refuse_duplicate_addresses(&all_rows)?;
    let total = rows::checked_row_count(all_rows.len())?;
    let truncated = total > query.limit;
    all_rows.truncate(query.limit as usize);

    // The optional enrichment layer: one provider call at most, only
    // when requested, over the limited address set.
    let addresses: Vec<String> = all_rows.iter().map(|row| row.address.clone()).collect();
    let provider_sources: Vec<ProviderSource<'_>> = outcomes
        .iter()
        .map(|outcome| ProviderSource {
            kind: outcome.result.source.kind.clone(),
            package: outcome.result.source.package.as_str(),
            root: coord_root(&coords, &outcome.result.source.package),
            // The lock's exact authority for the coordinate, projected
            // from the enumeration's one lock read — trusted request
            // metadata for the provider's own trust decision (A2c).
            expected_content_hash: coord_content_hash(&coords, &outcome.result.source.package),
        })
        .collect();
    let request = RelationRequest {
        selected_root: &selected.selected_root,
        workspace_root: &workspace_root,
        sources: &provider_sources,
        addresses: &addresses,
    };
    let resolved = resolve(query.relations, provider, &request);
    for row in &mut all_rows {
        if let Some(edges) = resolved.edges.get(&row.address) {
            row.relations = edges.clone();
        }
    }

    // The three exact digests, then the observation.
    let digest_sources: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| {
            outcome.result.digest.as_ref().map(|digest| {
                (
                    outcome.result.source.kind.clone(),
                    outcome.result.source.package.clone(),
                    digest.clone(),
                )
            })
        })
        .collect();
    let source_digest = scope_digest(&selected_str, &digest_sources, &snapshot.witnesses);
    let mut report = wire::RequirementsReport {
        requirements: REQUIREMENTS_EPOCH,
        observation: wire::RequirementsObservation {
            observation_id: String::new(),
            observed_at: context.observed_at,
            selected: selected_str,
            source_digest,
            lifecycle_run_id: context.lifecycle_run_id.clone(),
        },
        query: query.effective(),
        sources: outcomes.into_iter().map(|outcome| outcome.result).collect(),
        relation_sources: resolved.sources,
        rows: all_rows,
        truncated,
    };
    report.observation.observation_id = observation_id(&report);

    validate(&report).map_err(|source| QueryError::Wire { source })?;
    Ok(report)
}

/// The coordinate a `spec://` prefix scopes to, when it names one
/// (`spec://<group>/<name>/…`). A prefix without a full coordinate
/// cannot prune and does not try.
fn prefix_coordinate(prefix: &str) -> Option<String> {
    let rest = prefix.strip_prefix("spec://")?;
    let mut parts = rest.split('/');
    let group = parts.next()?.trim();
    let name = parts.next()?.trim();
    if group.is_empty() || name.is_empty() || name.contains('#') {
        return None;
    }
    Some(format!("{group}/{name}"))
}

/// 32 bytes of `[0-9a-f]` — the run-id shape the evidence wire and
/// lifecycle state hold themselves to. Lowercase HEX, not lowercase
/// alphanumeric: `g`..`z` refuse exactly like uppercase.
fn is_run_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

fn facts_kind(kind: &wire::RequirementSourceKind) -> vibe_facts::SourceKind {
    match kind {
        wire::RequirementSourceKind::Host => vibe_facts::SourceKind::Host,
        wire::RequirementSourceKind::Package => vibe_facts::SourceKind::Package,
    }
}

fn coord_root<'a>(
    coords: &'a [sources::SourceCoord],
    package: &str,
) -> Option<&'a std::path::Path> {
    coords
        .iter()
        .find(|coord| coord.package == package)
        .and_then(|coord| coord.root.as_deref())
}

/// The lock's exact content hash for one enumerated coordinate, from
/// the enumeration's own single lock read — `None` for the host and
/// registry-only orphans.
fn coord_content_hash<'a>(coords: &'a [sources::SourceCoord], package: &str) -> Option<&'a str> {
    coords
        .iter()
        .find(|coord| coord.package == package)
        .and_then(|coord| coord.content_hash.as_deref())
}
