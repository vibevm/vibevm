//! Differential wire-parity oracle for the JSON envelopes emitted by the
//! `vibe-index` HTTP server and consumed by generated `vibe-wire` readers.
//!
//! The corpus is the meeting point: every document survives its generated
//! reader without loss, the actual route response types serialize to the same
//! structure, and one broken shape per envelope is refused loudly. The two
//! envelopes consumed by `vibe-registry` are also decoded through its public
//! client views so the live client cannot drift away from the registered wire.

use std::path::PathBuf;

use axum::response::IntoResponse;
use semver::Version;
use serde::Deserialize;
use serde_json::Value;
use vibe_index::index::quarantine::{Unavailable, recipe_for};
use vibe_index::server::error::ApiError;
use vibe_index::server::routes::{admin, capabilities, health, packages, purls};
use vibe_index::types::{Group, PackageKind, VersionEntry};
use vibe_registry::{PurlLookupResults, SearchResults};
use vibe_wire::generated::index_http::e1::admin_status_response::AdminStatusResponse;
use vibe_wire::generated::index_http::e1::capabilities_response::CapabilitiesResponse;
use vibe_wire::generated::index_http::e1::error_response::ErrorResponse;
use vibe_wire::generated::index_http::e1::health_response::HealthResponse;
use vibe_wire::generated::index_http::e1::package_delete_response::PackageDeleteResponse;
use vibe_wire::generated::index_http::e1::package_list_response::PackageListResponse;
use vibe_wire::generated::index_http::e1::package_search_response::PackageSearchResponse;
use vibe_wire::generated::index_http::e1::package_upsert_response::PackageUpsertResponse;
use vibe_wire::generated::index_http::e1::package_versions_response::PackageVersionsResponse;
use vibe_wire::generated::index_http::e1::purls_response::PurlsResponse;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("formats")
        .join("corpora")
        .join("index_http")
        .join("e1")
}

fn corpus_value(name: &str) -> Value {
    let bytes = std::fs::read(corpus_dir().join(name))
        .unwrap_or_else(|e| panic!("reading the corpus document `{name}`: {e}"));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("the corpus document `{name}` parses: {e}"))
}

fn round_trips<T>(name: &str)
where
    T: for<'de> Deserialize<'de> + serde::Serialize,
{
    let authored = corpus_value(name);
    let typed: T = serde_json::from_value(authored.clone())
        .unwrap_or_else(|e| panic!("the generated reader accepts `{name}`: {e}"));
    let returned = serde_json::to_value(&typed)
        .unwrap_or_else(|e| panic!("the generated reader serializes `{name}`: {e}"));
    assert_eq!(
        authored, returned,
        "wire drift between `{name}` and its generated-reader round trip"
    );
}

fn writer_matches<T: serde::Serialize>(name: &str, response: &T) {
    let actual = serde_json::to_value(response)
        .unwrap_or_else(|e| panic!("the actual response type for `{name}` serializes: {e}"));
    assert_eq!(
        actual,
        corpus_value(name),
        "the actual server response type drifted from `{name}`"
    );
}

fn group(value: &str) -> Group {
    Group::parse(value).unwrap_or_else(|e| panic!("fixture group `{value}` is valid: {e}"))
}

fn version(value: &str) -> Version {
    value
        .parse()
        .unwrap_or_else(|e| panic!("fixture version `{value}` is valid: {e}"))
}

fn unavailable(group_value: &str, name: &str, version_value: &str, missing: &str) -> Unavailable {
    let missing = vec![missing.to_string()];
    Unavailable {
        group: group(group_value),
        name: name.to_string(),
        version: version(version_value),
        recipe: recipe_for(&missing),
        missing,
    }
}

#[test]
fn corpus_documents_round_trip_through_generated_readers() {
    round_trips::<HealthResponse>("health_response.json");
    round_trips::<HealthResponse>("health_response-readyz.json");
    round_trips::<PackageListResponse>("package_list_response.json");
    round_trips::<PackageSearchResponse>("package_search_response.json");
    round_trips::<PackageVersionsResponse>("package_versions_response.json");
    round_trips::<PackageUpsertResponse>("package_upsert_response.json");
    round_trips::<PackageDeleteResponse>("package_delete_response.json");
    round_trips::<CapabilitiesResponse>("capabilities_response.json");
    round_trips::<PurlsResponse>("purls_response.json");
    round_trips::<AdminStatusResponse>("admin_status_response.json");
    round_trips::<ErrorResponse>("error_response.json");
}

#[test]
fn actual_success_response_types_emit_the_corpus_structures() {
    writer_matches(
        "health_response.json",
        &health::Health {
            status: "ok",
            registry: "vibespecs".into(),
        },
    );
    writer_matches(
        "health_response-readyz.json",
        &health::Health {
            status: "ready",
            registry: "vibespecs".into(),
        },
    );

    let wal_unavailable = || {
        unavailable(
            "org.vibevm",
            "wal",
            "1.0.0-rc.1",
            "org.vibevm/wal/tombstone@1",
        )
    };
    writer_matches(
        "package_list_response.json",
        &packages::ListResponse {
            command: "list",
            registry: "vibespecs".into(),
            package_count: 1,
            returned: 1,
            offset: 0,
            limit: 50,
            packages: vec![packages::PackageRow {
                kind: Some(PackageKind::Flow),
                group: group("org.vibevm"),
                name: "wal".into(),
                latest_stable: Some(version("0.2.0")),
                versions: vec![version("0.1.0"), version("0.2.0")],
                description: Some("Write-ahead log discipline".into()),
                unavailable: vec![wal_unavailable()],
            }],
        },
    );
    writer_matches(
        "package_search_response.json",
        &packages::SearchResponse {
            command: "search",
            query: "wal".into(),
            hit_count: 1,
            hits: vec![packages::SearchHit {
                kind: PackageKind::Flow,
                group: group("org.vibevm"),
                name: "wal".into(),
                latest_stable: Some(version("0.2.0")),
                score: 3,
                matched_tokens: vec!["wal".into()],
                description: Some("Write-ahead log discipline".into()),
                unavailable: vec![wal_unavailable()],
            }],
        },
    );

    let versions_json = corpus_value("package_versions_response.json");
    let entry: VersionEntry = serde_json::from_value(versions_json["versions"][0].clone())
        .expect("the authored version fixture is an actual VersionEntry");
    writer_matches(
        "package_versions_response.json",
        &packages::PackageVersionsResponse {
            command: "package",
            kind: Some(PackageKind::Flow),
            group: group("org.vibevm"),
            name: "wal".into(),
            latest_stable: Some(version("0.2.0")),
            versions: vec![entry],
            unavailable: vec![wal_unavailable()],
        },
    );
    writer_matches(
        "package_upsert_response.json",
        &packages::UpsertResponse {
            command: "upsert",
            kind: PackageKind::Flow,
            group: group("org.vibevm"),
            name: "wal".into(),
            version: version("0.2.0"),
            created: true,
        },
    );
    writer_matches(
        "package_delete_response.json",
        &packages::DeleteResponse {
            command: "delete",
            group: group("org.vibevm"),
            name: "wal".into(),
            version: None,
            removed: true,
        },
    );
    writer_matches(
        "capabilities_response.json",
        &capabilities::Response {
            command: "capabilities",
            capability: "interface:wal".into(),
            hit_count: 1,
            hits: vec![capabilities::Hit {
                kind: PackageKind::Flow,
                group: group("org.vibevm"),
                name: "wal".into(),
                version: version("0.2.0"),
                capability_advertised: Some("interface:wal".into()),
            }],
            unavailable: vec![wal_unavailable()],
        },
    );
    writer_matches(
        "purls_response.json",
        &purls::Response {
            command: "purls",
            purl: "pkg:cargo/sqlx@0.8.0".into(),
            hit_count: 1,
            hits: vec![purls::Hit {
                kind: PackageKind::Feat,
                group: group("org.vibevm"),
                name: "sqlx-skin".into(),
                version: version("0.3.0"),
                binding_site: "package",
            }],
            unavailable: vec![unavailable(
                "org.vibevm",
                "sqlx-skin",
                "0.2.0",
                "org.vibevm/subskills/lazy-materialisation@1",
            )],
        },
    );
    writer_matches(
        "admin_status_response.json",
        &admin::Status {
            command: "admin:status",
            registry: "vibespecs".into(),
            registry_url: "https://github.com/vibespecs".into(),
            generator: "vibe-index 0.1.0-dev".into(),
            uptime_seconds: 3_600,
            read_only: false,
            package_count: 3,
            version_count: 6,
            requests_total: 417,
            mutations_total: 12,
        },
    );
}

#[tokio::test]
async fn actual_api_error_writer_emits_the_error_corpus() {
    let detail = "`org.vibevm/missing` is not in the index (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: check the (group, name) identity, or publish the package first)";
    let response = ApiError::not_found(detail).into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the actual ApiError response body is readable");
    let actual: Value =
        serde_json::from_slice(&body).expect("the actual ApiError response body is JSON");
    assert_eq!(actual, corpus_value("error_response.json"));
}

#[test]
fn registry_client_views_decode_the_live_server_corpora() {
    let search: SearchResults =
        serde_json::from_value(corpus_value("package_search_response.json"))
            .expect("the public registry search view accepts the live server envelope");
    assert_eq!(search.query, "wal");
    assert_eq!(search.hit_count, 1);
    assert_eq!(search.hits[0].name, "wal");
    assert_eq!(search.hits[0].score, 3);

    let purls: PurlLookupResults = serde_json::from_value(corpus_value("purls_response.json"))
        .expect("the public registry PURL view accepts the live server envelope");
    assert_eq!(purls.purl, "pkg:cargo/sqlx@0.8.0");
    assert_eq!(purls.hit_count, 1);
    assert_eq!(purls.hits[0].name, "sqlx-skin");
    assert_eq!(purls.hits[0].binding_site.to_string(), "package");
}

#[test]
fn broken_documents_are_rejected_loudly() {
    let health = serde_json::json!({ "status": "warming", "registry": "vibespecs" });
    assert!(serde_json::from_value::<HealthResponse>(health).is_err());

    let list = serde_json::json!({
        "command": "list", "registry": "vibespecs", "package_count": 1,
        "returned": 1, "offset": 0, "limit": 50, "packages": 7
    });
    assert!(serde_json::from_value::<PackageListResponse>(list).is_err());

    let search = serde_json::json!({
        "command": "search", "query": "wal", "hit_count": 1,
        "hits": [{ "kind": "flow", "group": "org.vibevm", "name": "wal",
                   "latest_stable": "0.2.0", "score": "high",
                   "matched_tokens": ["wal"], "description": null }]
    });
    assert!(serde_json::from_value::<PackageSearchResponse>(search).is_err());

    let versions = serde_json::json!({
        "command": "package", "kind": "flow", "group": "org.vibevm",
        "name": "wal", "latest_stable": "0.2.0", "versions": {},
        "unavailable": []
    });
    assert!(serde_json::from_value::<PackageVersionsResponse>(versions).is_err());

    let upsert = serde_json::json!({
        "command": "upsert", "kind": "flow", "group": "org.vibevm",
        "name": "wal", "version": "0.2.0", "created": "yes"
    });
    assert!(serde_json::from_value::<PackageUpsertResponse>(upsert).is_err());

    let delete = serde_json::json!({
        "command": "delete", "group": "org.vibevm", "name": "wal",
        "version": null, "removed": "yes"
    });
    assert!(serde_json::from_value::<PackageDeleteResponse>(delete).is_err());

    let capabilities = serde_json::json!({
        "command": "capabilities", "capability": "interface:wal",
        "hit_count": 1, "hits": 3, "unavailable": []
    });
    assert!(serde_json::from_value::<CapabilitiesResponse>(capabilities).is_err());

    let purls = serde_json::json!({
        "command": "purls", "purl": "pkg:cargo/sqlx@0.8.0", "hit_count": 1,
        "hits": [{ "kind": "feat", "group": "org.vibevm", "name": "sqlx-skin",
                   "version": "0.3.0", "binding_site": "workspace" }],
        "unavailable": []
    });
    assert!(serde_json::from_value::<PurlsResponse>(purls).is_err());

    let admin = serde_json::json!({
        "command": "admin:status", "registry": "vibespecs",
        "registry_url": "https://github.com/vibespecs", "generator": "vibe-index",
        "uptime_seconds": "3600", "read_only": false, "package_count": 3,
        "version_count": 6, "requests_total": 417, "mutations_total": 12
    });
    assert!(serde_json::from_value::<AdminStatusResponse>(admin).is_err());

    let error = serde_json::json!({
        "type": "vibe-index/error/not-found", "title": "resource not found",
        "status": "404", "detail": "missing"
    });
    assert!(serde_json::from_value::<ErrorResponse>(error).is_err());
}
