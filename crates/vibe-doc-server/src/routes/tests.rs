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
/// back as the island INSIDE the shell (`##SHELL-SERVE-SOURCES`). These
/// tests wear the bare shell, so the page is a document with typography
/// and no script — which is the configuration a machine without Node
/// gets, and a readable page.
#[tokio::test]
async fn a_page_address_returns_the_island_inside_the_shell() {
    let (status, headers, body) =
        get_path("/doc/com.example/thing-docs/0.2.0/model/boot-lane/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
    // The island.
    assert!(body.contains("doc-page"), "{body}");
    assert!(body.contains("boot lane"), "{body}");
    // The shell around it, with the page's own title in the tab.
    assert!(body.contains("<html"), "{body}");
    assert!(body.contains("<title>Boot lane</title>"), "{body}");
    // …and the marker is gone: the island went where it was.
    assert!(!body.contains("<!--vibe-doc-island-->"), "{body}");
    assert!(
        !body.contains("<script"),
        "the bare shell carries no script"
    );
}

/// The bare shell's stylesheet is a FILE, and it is served.
#[tokio::test]
async fn the_bare_shells_stylesheet_is_served_from_the_binary() {
    let (status, headers, body) = get_path("/doc/fallback.css").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "text/css; charset=utf-8");
    assert!(body.contains("--measure"), "{body}");
}

/// A name the shell does not carry is a refusal, not a directory
/// listing and not an index (`##LOCAL-STATIC`).
#[tokio::test]
async fn a_shell_file_that_does_not_exist_is_a_refusal() {
    for address in ["/doc/assets/", "/doc/build/", "/doc/assets/nothing.js"] {
        let (status, _, _) = get_path(address).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{address}");
    }
}

/// The resolver follows a citation into the page it names, and drops a
/// fragment the page does not carry (F-15).
#[tokio::test]
async fn the_resolver_follows_a_citation_to_the_page() {
    let (status, headers, _) =
        get_path("/doc/resolve?uri=spec://com.example/thing-docs/model/boot-lane%23root").await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(
        headers[header::LOCATION],
        "/doc/com.example/thing-docs/0.2.0/model/boot-lane/#root"
    );

    let (status, headers, _) =
        get_path("/doc/resolve?uri=spec://com.example/thing-docs/model/boot-lane%23gone").await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(
        headers[header::LOCATION],
        "/doc/com.example/thing-docs/0.2.0/model/boot-lane/"
    );
}

/// A citation this reader cannot answer for is reported, never guessed.
#[tokio::test]
async fn the_resolver_refuses_what_it_does_not_serve() {
    for (address, status) in [
        ("/doc/resolve", StatusCode::BAD_REQUEST),
        (
            "/doc/resolve?uri=https://example.com/",
            StatusCode::BAD_REQUEST,
        ),
        (
            "/doc/resolve?uri=spec://org.other/other-docs/page",
            StatusCode::NOT_FOUND,
        ),
        (
            "/doc/resolve?uri=spec://com.example/thing-docs@9.9.9/model/boot-lane",
            StatusCode::NOT_FOUND,
        ),
        (
            "/doc/resolve?uri=spec://com.example/thing-docs/model/absent",
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (actual, _, body) = get_path(address).await;
        assert_eq!(actual, status, "{address}: {body}");
    }
}
