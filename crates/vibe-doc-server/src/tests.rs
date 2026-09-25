//! What the reader is before it answers anything: where it binds, what
//! it is pointed at, and what it refuses to start on.

use axum::body::Body;
use axum::http::{Method, Request};
use chrono::{TimeZone, Utc};
use tower::util::ServiceExt;
use vibe_doc::citations::SpecSources;

use super::*;

fn when() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap()
}

fn package(dir: &Path, extra: &str) {
    std::fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"thing-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
             version = \"0.2.0\"\ntitle = \"Thing Manual\"\n\
             abstract = \"\"\"\nWhat it covers.\n\"\"\"\n\
             [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n{extra}"
        ),
    )
    .unwrap();
}

/// The address a reader answers under is the package's own coordinate
/// and version — the same one the public site mounts it at, so a link
/// written on a page works in both worlds.
#[test]
fn the_mount_is_the_packages_own_address() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path(), "");
    let reader = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when()).unwrap();
    assert_eq!(reader.mount(), "/doc/com.example/thing-docs/0.2.0/");
}

/// The host is not a setting. `##LOCAL-SERVE` binds the loopback and
/// records «binding to 0.0.0.0» as considered and rejected, so the
/// constant is the whole of the decision and there is no flag beside it.
#[test]
fn the_bind_host_is_the_loopback_and_is_not_configurable() {
    assert!(BIND_HOST.is_loopback());
    // The config a caller may set carries a port and no host — the type
    // itself is what makes the rule unbreakable from the CLI.
    let config = Config::new("x");
    assert_eq!(config.port, DEFAULT_PORT);
}

/// The policy names no external source, and `frame-ancestors` is the
/// one part a launch parameter moves.
#[test]
fn the_policy_names_no_outside_source_and_only_frames_are_a_parameter() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path(), "");
    let plain = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when()).unwrap();
    assert!(
        plain.csp.ends_with("frame-ancestors 'none'"),
        "{}",
        plain.csp
    );
    for source in ["http:", "https:", "*", "unsafe-inline", "unsafe-eval"] {
        assert!(!plain.csp.contains(source), "{source} in {}", plain.csp);
    }

    let embedded = Reader::open(
        &Config {
            frame_ancestor: Some("vscode-webview://abc".to_string()),
            ..Config::new(tmp.path())
        },
        SpecSources::new(),
        when(),
    )
    .unwrap();
    assert!(
        embedded
            .csp
            .ends_with("frame-ancestors vscode-webview://abc"),
        "{}",
        embedded.csp
    );
}

/// A shell whose head carries inline scripts is named in the policy by
/// the sha256 of each one's bytes, computed from the shell the reader is
/// actually carrying — X-035's answer, and the reason the hashes are not
/// a constant anywhere.
#[test]
fn the_policy_names_every_inline_script_of_the_shell_by_its_bytes() {
    let theme = "var t=1;";
    let scroll = "var s=2;";
    let template = format!(
        "<!doctype html><html><head><script>{theme}</script>\
         <script type=\"module\">{scroll}</script>\
         <script src=\"/doc/build/q-a.js\"></script>\
         <script type=\"qwik/state\">{{}}</script></head>\
         <body><!--vibe-doc-island--></body></html>"
    );
    let policy = content_policy(&vibe_doc_shell::Shell::bare(), &template, None);

    for body in [theme, scroll] {
        let hash = vibe_doc_shell::template::hash_of(body);
        assert!(policy.contains(&hash), "{hash} missing from {policy}");
    }
    // Two, not four: a script with a `src` is covered by `'self'`, and a
    // data block is never executed.
    assert_eq!(policy.matches("'sha256-").count(), 2, "{policy}");
    for source in ["http:", "https:", "*", "'unsafe-eval'"] {
        assert!(!policy.contains(source), "{source} in {policy}");
    }
}

/// A reader wearing the bare shell sends the policy `##LOCAL-CSP` spells
/// out, to the byte — which is why the norm's own string is still a
/// constant in this crate and why this compares against it rather than
/// against a copy.
///
/// A real shell widens `style-src` to `'unsafe-inline'`, and the widening
/// is recorded here rather than discovered in a browser: the framework
/// inlines each component's stylesheet, and the reading settings write a
/// `style` attribute, which no hash can name.
#[test]
fn a_bare_reader_sends_the_policy_the_norm_spells_out() {
    let bare = content_policy(&vibe_doc_shell::Shell::bare(), "<html></html>", None);
    assert_eq!(
        bare,
        format!("{CSP_WITHOUT_FRAME_ANCESTORS}; frame-ancestors 'none'")
    );
    assert!(!bare.contains("unsafe-inline"), "{bare}");
}

/// One package is one language, so `--lang` is the caller stating which
/// one it wants; a package in another is refused at START-UP rather than
/// served under a label that is not true.
#[test]
fn a_package_in_another_language_is_refused_before_it_is_served() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path(), "[i18n]\ncanonical = \"en\"\n");
    let refused = Reader::open(
        &Config {
            lang: Some("ru".to_string()),
            ..Config::new(tmp.path())
        },
        SpecSources::new(),
        when(),
    )
    .expect_err("the package is English");
    assert!(refused.to_string().contains("thing-docs-ru"), "{refused}");
}

/// A directory that is not a package cannot be served at an address,
/// and the refusal says which rule it broke and what to do.
#[test]
fn a_directory_that_is_not_a_package_is_a_named_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let e = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when())
        .expect_err("there is no manifest");
    assert!(e.to_string().contains("LOCAL-SERVE"), "{e}");
    assert!(e.to_string().contains("vibe cache add"), "{e}");
}

/// One of the pipeline's committed fixture packages, by name.
///
/// The pair lives beside the library that owns the borrowing law rather
/// than being copied here: one fixture, one place to correct it.
fn doc_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vibe-doc/tests/fixture")
        .join(name)
}

/// A reader on the borrowed-example fixture's adaptation, in a world whose
/// checkout arm holds the source it adapts.
fn borrowed_reader() -> Reader {
    let sources = SpecSources::for_checkout(
        doc_fixture("borrowed/source"),
        Some("com.example.docs"),
        "borrowed",
    );
    Reader::open(
        &Config::new(doc_fixture("borrowed/adaptation")),
        sources,
        when(),
    )
    .expect("the adaptation opens")
}

/// One GET against the router — no listener is bound, as everywhere else
/// in this crate.
async fn served(reader: Reader, path: &str) -> String {
    let response = build_app(std::sync::Arc::new(reader))
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("the router answers");
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .expect("the body reads");
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A translation's page is served with the example its SOURCE page authors
/// — in the island a person reads and in the Markdown beside it
/// (`##LOC-EXAMPLE-REF`). Only that one source page is read per request,
/// which is the same economy the reader already applies to citations.
#[tokio::test]
async fn a_borrowed_example_is_served_with_the_source_pages_body() {
    const AT: &str = "/doc/com.example.docs/borrowed-ru/0.1.0/guide/second";

    let island = served(borrowed_reader(), &format!("{AT}/")).await;
    assert!(island.contains("data-example-ref=\"demo\""), "{island}");
    assert!(island.contains("vibe list"), "{island}");
    assert!(island.contains("error: nothing is installed"), "{island}");
    assert!(!island.contains("data-unresolved"), "{island}");

    let markdown = served(borrowed_reader(), &format!("{AT}.md")).await;
    assert!(markdown.contains("```ps1\nvibe list\n```"), "{markdown}");
    assert!(
        markdown.contains("```output\nno packages\n```"),
        "{markdown}"
    );
    assert!(
        !markdown.contains("copied from the source page"),
        "{markdown}"
    );
}
