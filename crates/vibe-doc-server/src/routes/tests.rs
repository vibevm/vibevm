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
    let (status, headers, bytes) = get_bytes(path).await;
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

/// The same, without reading the body as text — which a picture is not.
async fn get_bytes(path: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
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
    (status, headers, bytes.to_vec())
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

/// A4.8 — the parity the two adapters have to keep
/// (`##SHELL-PARITY-TEST`).
///
/// The island the STATIC path writes (`vibe doc build --format html`) and
/// the island the SERVER glues into its template are compared byte for
/// byte, and the comparison is made by subtraction rather than by
/// rendering twice: the served page is split on the template's own two
/// halves, and what is between them must be exactly the bytes the build
/// wrote. Prediction 6 of the campaign plan says this is the test that
/// catches the first drift of the shell, and it only earns that if it
/// compares bytes — a test that asked whether both contained some
/// substring would pass through any amount of divergence.
#[tokio::test]
async fn the_island_is_the_same_bytes_through_both_adapters() {
    let tmp = tempfile::tempdir().unwrap();
    let reader = reader(&tmp);
    let marker = reader.shell.index().island_marker.clone();

    // The template this page would wear, with the marker still standing
    // where the island goes: the two halves the served page must be
    // wrapped in.
    let pages = vibe_doc::pages::read_package(tmp.path()).expect("the fixture reads");
    let dressed = crate::routes::in_shell(&reader, &pages.pages[0], &marker);
    let (before, after) = dressed
        .split_once(&marker)
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .expect("the template has a hole");

    let built = vibe_doc::build::build(
        tmp.path(),
        &SpecSources::new(),
        &vibe_doc::build::Options {
            format: vibe_doc::build::Format::Html,
            base: reader.base.clone(),
            manifest: vibe_doc::manifest::Options::at(reader.rendered_at),
            derived: std::collections::BTreeMap::new(),
        },
    )
    .expect("the fixture builds");
    let island = built
        .files
        .iter()
        .find(|file| file.path.ends_with("model/boot-lane/index.html"))
        .map(|file| String::from_utf8_lossy(&file.bytes).into_owned())
        .expect("the build wrote the page");

    let app = build_app(std::sync::Arc::new(reader));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/doc/com.example/thing-docs/0.2.0/model/boot-lane/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let served = String::from_utf8_lossy(&bytes).into_owned();

    let middle = served
        .strip_prefix(before.as_str())
        .and_then(|rest| rest.strip_suffix(after.as_str()))
        .expect("the served page is the template with something in its hole");
    assert_eq!(
        middle, island,
        "the island the server glued in is not the island the build wrote"
    );
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

/// The card's pictures come back as the bytes `vibe doc build` would
/// have written, at the addresses the manifest names them by — and at the
/// addresses a PAGE reaches them by, which are the edition's own
/// (`##CARD-SITE-COPIES`).
#[tokio::test]
async fn the_cards_pictures_are_the_bytes_the_build_writes() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path());
    let built = vibe_doc::build::build(
        tmp.path(),
        &SpecSources::new(),
        &vibe_doc::build::Options {
            format: vibe_doc::build::Format::Html,
            base: "/doc/".to_string(),
            manifest: vibe_doc::manifest::Options::at(
                Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap(),
            ),
            derived: std::collections::BTreeMap::new(),
        },
    )
    .expect("the fixture builds");
    let pictures: Vec<_> = built
        .files
        .iter()
        .filter(|file| file.path.starts_with("media/"))
        .collect();
    assert_eq!(pictures.len(), 3, "icon, banner and preview");

    for file in pictures {
        let expected_type = if file.path.ends_with(".png") {
            "image/png"
        } else {
            "image/svg+xml"
        };
        // The manifest's own spelling, under the mount…
        let (status, headers, body) = get_bytes(&format!("/doc/{}", file.path)).await;
        assert_eq!(status, StatusCode::OK, "{}", file.path);
        assert_eq!(headers[header::CONTENT_TYPE], expected_type);
        assert_eq!(
            body, file.bytes,
            "{} is not the bytes the build wrote",
            file.path
        );
        // …and the spelling a page of this edition climbs to, in both
        // spellings of the version.
        for prefix in [
            "/doc/com.example/thing-docs/0.2.0/",
            "/doc/com.example/thing-docs/latest/",
        ] {
            let (status, headers, _) = get_path(&format!("{prefix}{}", file.path)).await;
            assert_eq!(status, StatusCode::OK, "{prefix}{}", file.path);
            assert_eq!(headers[header::CONTENT_TYPE], expected_type);
        }
    }
}

/// A picture nobody published is a refusal, and a name that is not a
/// name is refused before any of it could become a path
/// (`##LOCAL-STATIC`).
#[tokio::test]
async fn a_picture_that_was_never_published_is_refused_and_never_looked_for() {
    for (address, status) in [
        ("/doc/media/nothing.svg", StatusCode::NOT_FOUND),
        ("/doc/media/vibe.toml", StatusCode::NOT_FOUND),
        ("/doc/media/..%2F..%2Fvibe.toml", StatusCode::BAD_REQUEST),
        ("/doc/media/%2e%2e", StatusCode::BAD_REQUEST),
        ("/doc/media/C:", StatusCode::BAD_REQUEST),
        (
            "/doc/com.example/thing-docs/0.2.0/media/..%2Fvibe.toml",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (actual, _, body) = get_path(address).await;
        assert_eq!(actual, status, "{address}: {body}");
        assert!(!body.contains("[package]"), "{address} served a file");
    }
}

/// The package's own page answers at its address, in both spellings of
/// the version, as the shell around an empty island: there is no document
/// behind a package page, and the values it shows are the manifest's.
#[tokio::test]
async fn the_package_page_answers_at_both_spellings_of_its_address() {
    for address in [
        "/doc/com.example/thing-docs/0.2.0/",
        "/doc/com.example/thing-docs/latest/",
    ] {
        let (status, headers, body) = get_path(address).await;
        assert_eq!(status, StatusCode::OK, "{address}");
        assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
        // The shell, with the documentation's own title in the tab.
        assert!(body.contains("<html"), "{address}: {body}");
        assert!(body.contains("<title>Thing Manual</title>"), "{body}");
        // The hole is filled, and with nothing: a package page has no
        // island, and a marker left standing would be shown to a reader.
        assert!(!body.contains("<!--vibe-doc-island-->"), "{body}");
        assert!(!body.contains("doc-page"), "{address}: {body}");
    }
}

/// The door above the mount means «this documentation», and the
/// documentation now has a page of its own to mean.
#[tokio::test]
async fn the_door_leads_to_the_package_page() {
    let (status, headers, _) = get_path("/doc/").await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(
        headers[header::LOCATION],
        "/doc/com.example/thing-docs/0.2.0/"
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
