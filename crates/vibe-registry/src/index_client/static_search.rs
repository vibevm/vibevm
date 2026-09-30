//! Search on a static mirror — the catalog read whole and scored here.
//!
//! The live route `/v1/packages` exists on a `vibe-index` server and
//! never on a static mirror, and the default index of a GitHub
//! registry — `raw.githubusercontent.com/<org>/index/<ref>`, the
//! default `vibespecs` one included — is a static mirror. A client that
//! asked the route there got the mirror's honest 404 and reported the
//! registry unreachable, so `vibe search` in a default project answered
//! nothing at all (found 2026-09-30). The mirror does publish the whole
//! catalog, `primary.jsonl`: this module reads it through the same
//! reader the documentation site uses ([`IndexClient::primary`]) and
//! scores it with the one tokeniser and ranking the server applies
//! (`vibe_wire::behaviour::index_search`), so both shapes of an index
//! answer the same hits for the same catalog (PROP-005 §2.14
//! `##INT-SEARCH`).
//!
//! What the server decides, this reader decides the same way: a version
//! whose `must_understand` names a capability this build lacks is not
//! searched — the judgement the version selector already applies
//! (PROP-044 §4.5, B-080) — and an unset `limit` means the route's own
//! default page. A record of a kind this build cannot name is skipped
//! rather than refused: it is one this build could not act on either.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#integration");

use std::collections::BTreeMap;
use std::str::FromStr;

use specmark::spec;
use vibe_core::{Group, PackageKind};
use vibe_wire::behaviour::index_search::search_packages;
use vibe_wire::generated::shared::{PackageKind as WireKind, VersionEntry};

use super::{IndexClient, IndexError, SearchHit, SearchResults};

/// The page the route serves when the caller names none
/// (`ListSearchQuery` in `vibe-index`).
const ROUTE_DEFAULT_LIMIT: usize = 50;

impl IndexClient {
    /// Search the catalog of a static mirror. `Ok(None)` when the base
    /// publishes no `primary.jsonl` — then it is an index of neither
    /// shape, and the caller keeps reporting the route's answer.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#integration",
        r = 1
    )]
    pub(super) fn search_catalog(
        &self,
        query: &str,
        kind: Option<PackageKind>,
        limit: Option<usize>,
    ) -> Result<Option<SearchResults>, IndexError> {
        let Some(entries) = self.primary()? else {
            return Ok(None);
        };
        Ok(Some(search_entries(entries, query, kind, limit)))
    }
}

/// Score the catalog's records — one package per `(group, name)`, its
/// usable versions ascending — and shape the answer as the route would.
pub(crate) fn search_entries(
    entries: Vec<VersionEntry>,
    query: &str,
    kind: Option<PackageKind>,
    limit: Option<usize>,
) -> SearchResults {
    let mut packages: BTreeMap<(Group, String), Vec<VersionEntry>> = BTreeMap::new();
    for entry in entries {
        if !vibe_core::capabilities::missing_capabilities(&entry.must_understand).is_empty()
            || matches!(entry.kind, WireKind::Unknown(_))
        {
            continue;
        }
        packages
            .entry((entry.group.clone(), entry.name.clone()))
            .or_default()
            .push(entry);
    }
    for versions in packages.values_mut() {
        versions.sort_by(|a, b| a.version.cmp(&b.version));
    }
    let kind_filter = kind.and_then(|k| WireKind::from_str(k.as_str()).ok());
    let hits: Vec<SearchHit> = search_packages(
        packages.values().map(|versions| versions.iter()),
        query,
        kind_filter.as_ref(),
    )
    .into_iter()
    .take(limit.unwrap_or(ROUTE_DEFAULT_LIMIT))
    .filter_map(|hit| {
        Some(SearchHit {
            kind: PackageKind::from_str(hit.kind.as_str()).ok()?,
            name: hit.name,
            latest_stable: hit.latest_stable,
            score: u32::try_from(hit.score).unwrap_or(u32::MAX),
            matched_tokens: hit.matched_tokens,
            description: hit.description,
        })
    })
    .collect();
    SearchResults {
        query: query.to_string(),
        hit_count: hits.len(),
        hits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index_client::catalog::parse_primary;

    const URL: &str = "https://example.invalid/index/primary.jsonl";

    /// One catalog line as the reader sees it; `extra` is spliced in
    /// verbatim (`"key":value,…`).
    fn line(kind: &str, group: &str, name: &str, version: &str, extra: &str) -> String {
        format!(
            r#"{{"schema_version":1,"kind":"{kind}","group":"{group}","name":"{name}","version":"{version}",{extra}"content_hash":"sha256:0","source_url":"u","source_ref":"r","registry":"example","files_count":1,"indexed_at":"2026-09-30T00:00:00Z","indexed_by":"vibe-index"}}"#
        )
    }

    fn catalog(lines: &[String]) -> Vec<VersionEntry> {
        parse_primary(URL, (lines.join("\n") + "\n").as_bytes()).expect("a catalog")
    }

    #[test]
    fn records_are_grouped_into_packages_and_ranked_as_the_route_would() {
        // Catalog order is not package order: the newer version comes first.
        let entries = catalog(&[
            line(
                "flow",
                "org.example",
                "wal",
                "0.2.0",
                r#""description":"Write-ahead log, second edition.","#,
            ),
            line(
                "feat",
                "org.example",
                "audit-log",
                "1.0.0",
                r#""description":"Append-only audit trail.","#,
            ),
            line(
                "flow",
                "org.example",
                "wal",
                "0.1.0",
                r#""description":"Write-ahead log.","#,
            ),
        ]);

        let results = search_entries(entries, "wal log", None, None);
        assert_eq!(results.query, "wal log");
        assert_eq!(results.hit_count, 2);
        assert_eq!(results.hits[0].name, "wal");
        assert_eq!(results.hits[0].kind, PackageKind::Flow);
        assert_eq!(results.hits[0].score, 2);
        assert_eq!(results.hits[0].matched_tokens, ["log", "wal"]);
        assert_eq!(
            results.hits[0].latest_stable.as_ref().unwrap().to_string(),
            "0.2.0"
        );
        assert_eq!(
            results.hits[0].description.as_deref(),
            Some("Write-ahead log, second edition.")
        );
        assert_eq!(results.hits[1].name, "audit-log");
        assert_eq!(results.hits[1].score, 1);
    }

    #[test]
    fn kind_and_limit_shape_the_answer() {
        let entries = catalog(&[
            line("flow", "org.example", "wal", "0.1.0", ""),
            line(
                "feat",
                "org.example",
                "audit-log",
                "1.0.0",
                r#""description":"wal reader","#,
            ),
        ]);

        let feats = search_entries(entries.clone(), "wal", Some(PackageKind::Feat), None);
        assert_eq!(feats.hit_count, 1);
        assert_eq!(feats.hits[0].name, "audit-log");

        // Both packages carry the one query token, so the identity order
        // decides which of them the page of one keeps.
        let one = search_entries(entries, "wal", None, Some(1));
        assert_eq!(one.hit_count, 1);
        assert_eq!(one.hits.len(), 1);
        assert_eq!(one.hits[0].name, "audit-log");
    }

    #[test]
    fn a_version_this_build_cannot_act_on_is_not_searched() {
        let entries = catalog(&[
            line(
                "flow",
                "org.example",
                "wal",
                "9.0.0",
                r#""must_understand":["b080-test-capability"],"#,
            ),
            line(
                "flow",
                "org.example",
                "wal",
                "1.0.0",
                r#""description":"the one this build reads","#,
            ),
            line("bundle", "org.example", "wal-next", "1.0.0", ""),
        ]);

        let results = search_entries(entries, "wal", None, None);
        assert_eq!(results.hit_count, 1);
        assert_eq!(
            results.hits[0].latest_stable.as_ref().unwrap().to_string(),
            "1.0.0"
        );
        assert_eq!(
            results.hits[0].description.as_deref(),
            Some("the one this build reads")
        );
    }
}
