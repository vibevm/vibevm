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
    /// The short spelling this registry offers for another package
    /// (PROP-008 §2.6). `default` — every index written before aliases
    /// existed reads as offering none, which is what it does.
    #[serde(default)]
    pub alias: Option<AliasView>,
}

/// The `alias` member of a `by-name` file: the package a spelling
/// stands for. It sits BESIDE `packages`, so a reader that ignores it
/// sees the candidate set it always saw.
#[derive(Debug, Deserialize)]
pub(super) struct AliasView {
    pub group: Group,
    pub name: String,
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
    /// Capabilities the version's record says a reader must understand
    /// to act on it (PROP-044 §4.5). `default` — records without the
    /// field (every pre-capability index) read as declaring nothing,
    /// exactly as before.
    #[serde(default)]
    pub must_understand: Vec<String>,
    /// Whether the record's manifest declared `[package] template =
    /// true` — a **template revision** rather than a produced version
    /// (PROP-075 `#TEMPLATE-FLAG`, `#TEMPLATE-REVISION-NOT-A-VERSION`).
    /// `default` — a record without the field, and every index that
    /// predates it, reads as an ordinary version.
    #[serde(default)]
    pub template: bool,
}

/// One `by-name` version entry as the client reads it: the version
/// plus the capability declarations its record carries (PROP-044
/// §4.5). The version selector needs both — a version whose
/// `must_understand` names a capability this reader lacks must be
/// skipped, not picked ([B-080]).
///
/// [B-080]: ../../../BACKLOG.md#b-080
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexVersion {
    pub version: Version,
    /// Empty on records that declare no capabilities — and on every
    /// entry served by an index that predates the field.
    pub must_understand: Vec<String>,
    /// `true` for a **template revision**: the version of the template
    /// itself, never a version of the thing it produces (PROP-075
    /// `#TEMPLATE-REVISION-NOT-A-VERSION`). A revision is not a
    /// candidate — it is neither listed nor selectable — which is the
    /// same answer the directory-shaped path reaches by splitting the
    /// manifests on disk.
    pub template: bool,
}

impl From<VersionEntryView> for IndexVersion {
    fn from(v: VersionEntryView) -> Self {
        IndexVersion {
            version: v.version,
            must_understand: v.must_understand,
            template: v.template,
        }
    }
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
    fn version_entry_view_reads_must_understand_and_tolerates_its_absence() {
        // B-080 (PROP-044 §4.5): the by-name record carries
        // `must_understand` and the view reads it; a record without
        // the field — every pre-capability index — declares nothing.
        let with_caps: VersionEntryView = serde_json::from_value(serde_json::json!({
            "version": "2.0.0",
            "must_understand": ["b080-test-capability"]
        }))
        .unwrap();
        assert_eq!(with_caps.version.to_string(), "2.0.0");
        assert_eq!(
            with_caps.must_understand,
            vec!["b080-test-capability".to_string()]
        );
        let carried: IndexVersion = with_caps.into();
        assert_eq!(carried.must_understand.len(), 1);

        let plain: VersionEntryView =
            serde_json::from_value(serde_json::json!({ "version": "1.0.0" })).unwrap();
        assert_eq!(plain.version.to_string(), "1.0.0");
        assert!(plain.must_understand.is_empty());
    }

    #[test]
    fn version_entry_view_reads_the_template_flag_and_tolerates_its_absence() {
        // PROP-075 `#TEMPLATE-REVISION-NOT-A-VERSION`: the by-name
        // record says whether a published version is the template's own
        // revision, and the view reads it. A record without the field —
        // every index that predates it, and every ordinary package —
        // reads as an ordinary version, so nothing an older index
        // serves changes meaning.
        let revision: VersionEntryView = serde_json::from_value(serde_json::json!({
            "version": "1.1.0",
            "template": true
        }))
        .unwrap();
        assert!(revision.template);
        let carried: IndexVersion = revision.into();
        assert!(carried.template, "the flag survives the decode");

        let produced: VersionEntryView = serde_json::from_value(serde_json::json!({
            "version": "22.23.4",
            "template": false
        }))
        .unwrap();
        assert!(!produced.template);

        let older: VersionEntryView =
            serde_json::from_value(serde_json::json!({ "version": "1.0.0" })).unwrap();
        assert!(!older.template, "a record without the field is a version");
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
