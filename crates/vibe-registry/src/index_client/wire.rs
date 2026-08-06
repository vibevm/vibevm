//! Wire-decode shapes for the index HTTP surface — the JSON envelopes
//! returned by the `by-name/<name>.json` static route and the live
//! `/v1/packages` / `/v1/purls/{purl}` server routes (PROP-005 §2.4,
//! §2.10; PROP-008 §2.8). Split out of `mod.rs` purely so that file
//! stays under its length budget once auth was added — the shapes are
//! independent of the client's HTTP / auth logic.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http");

use semver::Version;
use serde::Deserialize;
use vibe_core::{Group, PackageKind};

/// Decoded `by-name/<name>.json` — the candidate set for one bare name
/// (PROP-008 §2.8). Only the fields the resolver's version selector
/// needs are read; the rest of the on-disk shape is tolerated.
#[derive(Debug, Deserialize)]
pub(super) struct NameEntryView {
    #[serde(default)]
    pub packages: Vec<PackageEntryView>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PackageEntryView {
    pub group: Group,
    #[serde(default)]
    pub versions: Vec<VersionEntryView>,
}

#[derive(Debug, Deserialize)]
pub(super) struct VersionEntryView {
    pub version: Version,
}

/// Decoded body of the structured search route. Mirrors the wire
/// shape produced by `vibe_index::server::routes::packages::SearchResponse`.
/// Extra fields on the wire (today: `command`) are tolerated
/// silently — kept simple so a server-side envelope addition does
/// not force a client bump.
///
/// `Serialize` is derived alongside `Deserialize` so the CLI-side
/// `~/.vibe/search-cache/` layer can persist a decoded result and
/// load it back without a separate cache-only schema.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchResults {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub hit_count: usize,
    #[serde(default)]
    pub hits: Vec<SearchHit>,
}

/// One package matched by the index's search backend.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: PackageKind,
    pub name: String,
    #[serde(default)]
    pub latest_stable: Option<Version>,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub matched_tokens: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Decoded body of the structured PURL-lookup route. Mirrors
/// `vibe_index::server::routes::purls::Response`.
#[derive(Debug, Clone, Deserialize)]
pub struct PurlLookupResults {
    #[serde(default)]
    pub purl: String,
    #[serde(default)]
    pub hit_count: usize,
    #[serde(default)]
    pub hits: Vec<PurlLookupHit>,
}

/// One concrete `(kind, name, version)` whose package- or subskill-level
/// `describes` matched the queried PURL.
#[derive(Debug, Clone, Deserialize)]
pub struct PurlLookupHit {
    pub kind: PackageKind,
    pub name: String,
    pub version: Version,
    pub binding_site: BindingSite,
}

/// Where the PURL match originated on the matched entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingSite {
    /// PURL declared on the entry's top-level `describes` field.
    Package,
    /// PURL declared on a subskill within the entry.
    Subskill,
}

impl std::fmt::Display for BindingSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingSite::Package => f.write_str("package"),
            BindingSite::Subskill => f.write_str("subskill"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_results_decode_minimal_envelope() {
        let body = serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.1.0",
                    "score": 3,
                    "matched_tokens": ["wal"],
                    "description": "Write-ahead log"
                }
            ]
        });
        let parsed: SearchResults = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.query, "wal");
        assert_eq!(parsed.hit_count, 1);
        assert_eq!(parsed.hits.len(), 1);
        assert_eq!(parsed.hits[0].kind, PackageKind::Flow);
        assert_eq!(parsed.hits[0].name, "wal");
        assert_eq!(parsed.hits[0].score, 3);
        assert_eq!(
            parsed.hits[0].latest_stable.as_ref().unwrap().to_string(),
            "0.1.0"
        );
        assert_eq!(parsed.hits[0].matched_tokens, vec!["wal".to_string()]);
        assert_eq!(
            parsed.hits[0].description.as_deref(),
            Some("Write-ahead log")
        );
    }

    #[test]
    fn search_hit_tolerates_missing_optional_fields() {
        let body = serde_json::json!({
            "kind": "feat",
            "name": "atomic-commits"
        });
        let parsed: SearchHit = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.kind, PackageKind::Feat);
        assert_eq!(parsed.name, "atomic-commits");
        assert_eq!(parsed.score, 0);
        assert!(parsed.latest_stable.is_none());
        assert!(parsed.matched_tokens.is_empty());
        assert!(parsed.description.is_none());
    }

    #[test]
    fn purl_lookup_results_decode_full_envelope() {
        let body = serde_json::json!({
            "command": "purls",
            "purl": "pkg:cargo/sqlx@0.8.0",
            "hit_count": 2,
            "hits": [
                {
                    "kind": "flow",
                    "name": "sqlx-skin",
                    "version": "0.1.0",
                    "binding_site": "package"
                },
                {
                    "kind": "stack",
                    "name": "rust",
                    "version": "0.2.0",
                    "binding_site": "subskill"
                }
            ]
        });
        let parsed: PurlLookupResults = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.purl, "pkg:cargo/sqlx@0.8.0");
        assert_eq!(parsed.hit_count, 2);
        assert_eq!(parsed.hits.len(), 2);
        assert_eq!(parsed.hits[0].kind, PackageKind::Flow);
        assert_eq!(parsed.hits[0].binding_site, BindingSite::Package);
        assert_eq!(parsed.hits[1].binding_site, BindingSite::Subskill);
    }

    #[test]
    fn binding_site_display_renders_lowercase_word() {
        assert_eq!(format!("{}", BindingSite::Package), "package");
        assert_eq!(format!("{}", BindingSite::Subskill), "subskill");
    }

    #[test]
    fn name_entry_view_extracts_candidate_groups() {
        // `name_candidates` decodes `by-name/<name>.json` into a
        // `NameEntryView` and maps each package to its `group` — the
        // candidate set short-name resolution (PROP-008 §2.6) walks.
        // Two groups under one bare name is a collision (§2.7). The
        // surrounding `name` / `indexed_at` fields are tolerated.
        let body = serde_json::json!({
            "name": "wal",
            "indexed_at": "2026-05-22T00:00:00Z",
            "packages": [
                { "group": "org.vibevm", "versions": [{ "version": "0.1.0" }] },
                { "group": "com.acme", "versions": [{ "version": "0.2.0" }] }
            ]
        });
        let parsed: NameEntryView = serde_json::from_value(body).unwrap();
        let groups: Vec<String> = parsed
            .packages
            .iter()
            .map(|p| p.group.to_string())
            .collect();
        assert_eq!(groups, vec!["org.vibevm", "com.acme"]);
    }
}
