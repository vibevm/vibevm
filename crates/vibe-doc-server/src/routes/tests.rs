//! The routes, exercised against the router itself — no TCP listener is
//! bound, so the tests are hermetic and cheap, exactly as `vibe-index`'s
//! are.

use axum::body::Body;
use axum::http::{Method, Request};
use chrono::{TimeZone, Utc};
use tower::util::ServiceExt;
use vibe_doc::citations::SpecSources;

use super::*;
use crate::{Config, Reader, build_app};

const PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">Boot lane</title>\n  \
  <p>The boot lane is what a session reads first.</p>\n\
</spec>\n";

fn package(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("vibevm/vibespecs/model")).unwrap();
    std::fs::write(
        dir.join("vibe.toml"),
        "[package]\nname = \"thing-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
         version = \"0.2.0\"\ntitle = \"Thing Manual\"\nabstract = \"\"\"\nWhat it covers.\n\"\"\"\n\
         [i18n]\ncanonical = \"en\"\n\
         [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("vibevm/vibespecs/model/boot-lane.xml"), PAGE).unwrap();
}

fn reader(tmp: &tempfile::TempDir) -> Reader {
    package(tmp.path());
    let when = Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap();
    Reader::open(&Config::new(tmp.path()), SpecSources::new(), when).expect("the package opens")
}

async fn get_path(path: &str) -> (StatusCode, axum::http::HeaderMap, String) {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_app(std::sync::Arc::new(reader(&tmp)));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

#[tokio::test]
async fn the_health_route_answers_the_shape_the_index_server_answers() {
    let (status, _, body) = get_path("/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"status\":\"ok\""), "{body}");
}

/// The whole reader in one case: a page address ending in a slash comes
/// back as the island — content HTML and no page furniture, because the
/// shell that wraps it is a separate thing (`##PIPE-SHELL-PARSES-NOTHING`).
#[tokio::test]
async fn a_page_address_returns_the_bare_island() {
    let (status, headers, body) =
        get_path("/doc/com.example/thing-docs/0.2.0/model/boot-lane/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
    assert!(body.contains("doc-page"), "{body}");
    assert!(body.contains("boot lane"), "{body}");
    assert!(
        !body.contains("<html"),
        "an island carries no page furniture"
    );
    assert!(!body.contains("<script"), "and no script");
}

/// Both projections lie beside the page as files, and each says what it
/// is (`##SITE-TRAILING-SLASH`).
#[tokio::test]
async fn the_projections_answer_beside_the_page() {
    for (suffix, content_type, marker) in [
        (".md", "text/markdown; charset=utf-8", "[p01]"),
        (".xml", "application/xml; charset=utf-8", "p=\"1\""),
    ] {
        let (status, headers, body) = get_path(&format!(
            "/doc/com.example/thing-docs/0.2.0/model/boot-lane{suffix}"
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{suffix}");
        assert_eq!(headers[header::CONTENT_TYPE], content_type);
        assert!(body.contains(marker), "{suffix}: {body}");
    }
}

/// An address that lost its slash is repaired once, permanently, rather
/// than on every visit.
#[tokio::test]
async fn a_page_address_without_its_slash_redirects_to_the_one_with_it() {
    let (status, headers, _) = get_path("/doc/com.example/thing-docs/0.2.0/model/boot-lane").await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        headers[header::LOCATION],
        "/doc/com.example/thing-docs/0.2.0/model/boot-lane/"
    );
}

/// The machine files the agent endpoints promise, at the mount's root.
#[tokio::test]
async fn the_manifest_and_the_llms_tiers_are_served() {
    let (status, headers, body) = get_path("/doc/manifest.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "application/json; charset=utf-8"
    );
    assert!(body.contains("\"thing-docs\""), "{body}");

    for tier in [
        "llms.txt",
        "llms-small.txt",
        "llms-medium.txt",
        "llms-full.txt",
    ] {
        let (status, headers, body) = get_path(&format!("/doc/{tier}")).await;
        assert_eq!(status, StatusCode::OK, "{tier}");
        assert_eq!(headers[header::CONTENT_TYPE], "text/plain; charset=utf-8");
        assert!(body.contains("Thing Manual"), "{tier}: {body}");
    }
}

/// The form `##LOCAL-STATIC` forbids, in the three spellings the
/// phase-0 probe found a live server falling to. Every one of them must
/// be refused BEFORE anything touches the filesystem, and none of them
/// may return a file.
#[tokio::test]
async fn no_spelling_of_a_traversal_reaches_the_filesystem() {
    for attempt in [
        "/doc/com.example/thing-docs/0.2.0/../../../../vibe.toml",
        "/doc/com.example/thing-docs/0.2.0/..%2F..%2Fvibe.toml",
        "/doc/com.example/thing-docs/0.2.0/%2e%2e%2fvibe.toml",
        "/doc/com.example/thing-docs/0.2.0/model/../../../vibe.toml.md",
    ] {
        let (status, _, body) = get_path(attempt).await;
        assert!(
            status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST,
            "{attempt} answered {status}"
        );
        assert!(!body.contains("[package]"), "{attempt} served a file");
    }
}

/// An escape that decodes to nothing is a refusal, not a guess.
#[tokio::test]
async fn a_malformed_escape_is_refused() {
    let (status, _, _) = get_path("/doc/com.example/thing-docs/0.2.0/model/%zz/").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Every response carries the policy and the sniff guard — including
/// the refusals, which is where a policy with a hole in it would show.
#[tokio::test]
async fn every_response_carries_the_policy_and_the_sniff_guard() {
    for path in [
        "/doc/com.example/thing-docs/0.2.0/model/boot-lane/",
        "/doc/com.example/thing-docs/0.2.0/nothing-here/",
        "/somewhere/else",
    ] {
        let (_, headers, _) = get_path(path).await;
        let csp = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
        assert!(csp.contains("default-src 'self'"), "{path}: {csp}");
        assert!(csp.contains("frame-ancestors 'none'"), "{path}: {csp}");
        assert!(!csp.contains("unsafe-inline"), "{path}: {csp}");
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    }
}

/// The reader has no CORS layer at all — which is stricter than any
/// setting of one, and the state `##LOCAL-CSP` asks for.
#[tokio::test]
async fn the_reader_sends_no_cross_origin_permission() {
    let (_, headers, _) = get_path("/doc/com.example/thing-docs/0.2.0/model/boot-lane/").await;
    assert!(headers.get("access-control-allow-origin").is_none());
}

/// A page this documentation does not carry is a 404 that says so, not a
/// blank 200.
#[tokio::test]
async fn an_unknown_page_is_a_refusal_that_names_it() {
    let (status, _, body) = get_path("/doc/com.example/thing-docs/0.2.0/model/nope/").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("model/nope.xml"), "{body}");
}

/// The decoder's own law: it reads an escape or it refuses, and it never
/// guesses.
#[test]
fn the_decoder_reads_or_refuses() {
    assert_eq!(decode("/doc/a%20b"), Some("/doc/a b".to_string()));
    assert_eq!(decode("/doc/plain"), Some("/doc/plain".to_string()));
    assert_eq!(decode("/doc/%2"), None);
    assert_eq!(decode("/doc/%zz"), None);
    // A separator smuggled through an escape decodes to a separator, and
    // that is exactly why the segment check runs on the DECODED path.
    assert_eq!(decode("/a%2Fb"), Some("/a/b".to_string()));
}
