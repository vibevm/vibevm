//! The `unavailable` half of the server's end-to-end guards (F62C):
//! the tests that pin WHAT THE ANSWER SAYS about a version this build
//! refuses to act on — where the parent file holds the tests that pin
//! the shapes of the answers that say nothing unusual.
//!
//! Out of line for the 600-line file budget, by the crate's own idiom
//! (`scanner_e2e.rs` → `scanner_e2e/journal_form.rs`): the parent
//! declares the module with `#[path]`, so the module-tree position —
//! and therefore `use super::*` — reaches the fixtures above: one
//! `entry` / `now` / `req` / `body_to_json` set, not a second copy.

use super::*;

/// The quarantine fixture, per R55.6: its OWN minimal state — the
/// shared `populated_state()` (and the seven guards pinning its
/// counts) stays untouched. One package, two versions: `0.1.0`
/// usable, `0.2.0` refused (an unknown reader capability). RAM-built
/// and handed to `AppState::new` directly, exactly like
/// `populated_state()` — the shorter, journal-free path the survey
/// named; the predicate reads the RECORD, not the loader's carrier,
/// so no `load_from` is needed for the refusal to be visible.
fn quarantined_state() -> (tempfile::TempDir, AppState) {
    let tmp = tempfile::tempdir().unwrap();
    let mut idx = Index::new(
        "vibespecs",
        "https://example.invalid/vibespecs",
        NamingConvention::Fqdn,
        now(),
    );
    idx.upsert(entry(
        PackageKind::Flow,
        "wal",
        "0.1.0",
        Some("WAL discipline"),
        &["interface:wal"],
        None,
    ));
    let mut refused = entry(
        PackageKind::Flow,
        "wal",
        "0.2.0",
        Some("WAL discipline"),
        &["interface:wal"],
        None,
    );
    refused.must_understand = vec!["some-future-capability".into()];
    idx.upsert(refused);
    idx.write_to(tmp.path(), &WriteCtx { at: now() }).unwrap();
    let state = AppState::new(tmp.path().to_path_buf(), true, idx);
    (tmp, state)
}

/// §3.5.1 — `GET /v1/packages/{g}/{n}`: the refused version is NOT
/// among `versions` but IS named in `unavailable`, with its missing
/// list and the one-home recipe.
#[tokio::test]
async fn package_versions_names_the_refused_version() {
    let (_tmp, state) = quarantined_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages/org.vibevm/wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    let versions: Vec<&str> = body["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, vec!["0.1.0"]);
    let unavailable = body["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["group"], "org.vibevm");
    assert_eq!(unavailable[0]["name"], "wal");
    assert_eq!(unavailable[0]["version"], "0.2.0");
    assert_eq!(
        unavailable[0]["missing"],
        serde_json::json!(["some-future-capability"])
    );
    assert!(
        unavailable[0]["recipe"]
            .as_str()
            .unwrap()
            .contains("this build does not understand `some-future-capability`")
    );
}

/// §3.5.2 — `GET /v1/packages/{g}/{n}/{v}` on the refused version:
/// status stays 404 (R55.4), the body's extension member carries
/// `missing` and `recipe`, and its `type`/`title` DIFFER from the
/// answer for a version that never existed. One test compares BOTH
/// answers — otherwise the difference is not proven.
#[tokio::test]
async fn single_version_refused_differs_from_missing() {
    let (_tmp, state) = quarantined_state();
    let app = build_app(state);
    let app_missing = app.clone();

    // A: the refused version — 404 with a speaking body.
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages/org.vibevm/wal/0.2.0"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let refused: serde_json::Value = body_to_json(resp.into_body()).await;

    // B: a version that never existed — 404 with today's plain body.
    let resp = app_missing
        .oneshot(req(Method::GET, "/v1/packages/org.vibevm/wal/9.9.9"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let missing: serde_json::Value = body_to_json(resp.into_body()).await;

    // Same status — the difference lives in the body.
    assert_eq!(
        refused["type"], "vibe-index/error/unavailable",
        "refusal is its own word, not 'resource not found'"
    );
    assert_eq!(missing["type"], "vibe-index/error/not-found");
    assert_ne!(refused["title"], missing["title"]);
    // The extension member: RFC 7807 allows it; only the refusal has it.
    let row = refused["unavailable"].as_object().unwrap();
    assert_eq!(
        row["missing"],
        serde_json::json!(["some-future-capability"])
    );
    assert!(
        row["recipe"]
            .as_str()
            .unwrap()
            .contains("this build does not understand")
    );
    assert!(
        missing.get("unavailable").is_none(),
        "a version that never existed keeps its plain not-found body"
    );
}

/// §3.5.3 — `GET /v1/packages?q=`: the hit carries the refusal rows of
/// its package (the hit names a package, not a version).
#[tokio::test]
async fn search_hit_names_the_refused_version() {
    let (_tmp, state) = quarantined_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages?q=wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["command"], "search");
    let hits = body["hits"].as_array().unwrap();
    let wal = hits
        .iter()
        .find(|h| h["name"] == "wal")
        .expect("wal hit (its usable 0.1.0 scores it)");
    let unavailable = wal["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["version"], "0.2.0");
}

/// §3.5.4 — `GET /v1/capabilities/{cap}`: the envelope names the
/// refused version that WOULD have matched the capability.
#[tokio::test]
async fn capabilities_names_the_refused_version() {
    let (_tmp, state) = quarantined_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/capabilities/interface:wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    // Only the usable 0.1.0 is served as a hit…
    assert_eq!(body["hit_count"], 1);
    assert_eq!(body["hits"][0]["version"], "0.1.0");
    // …and the refused 0.2.0 — which advertises the same capability —
    // is named beside it.
    let unavailable = body["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["name"], "wal");
    assert_eq!(unavailable[0]["version"], "0.2.0");
    assert_eq!(
        unavailable[0]["missing"],
        serde_json::json!(["some-future-capability"])
    );
}
