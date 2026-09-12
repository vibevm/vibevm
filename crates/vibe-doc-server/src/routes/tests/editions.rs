//! The addresses of one edition, in both spellings of its version.
//!
//! Sliced out of `tests.rs` along that seam when the parent crossed the
//! 600-line budget. One subject holds them together: what a version
//! being spelled two ways owes a reader — the package's own page, the
//! alias law over every address the reader serves, the agent files the
//! site publishes under the edition, and the refusals that have to be
//! the same refusal under both names.
//!
//! The harness (`package`, `reader`, `get_path`, `get_bytes`) comes from
//! the parent module; this file declares none of its own.

use axum::http::header;

use super::*;

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

/// `latest` is this version, spelled the other way — for EVERY address
/// the reader serves, not for the package page alone (`##SITE-MOUNT`).
///
/// The law is stated as an equality rather than as a list of expected
/// statuses on purpose: what matters is that the two spellings are one
/// address, whatever that address answers. A test that pinned each
/// status would agree with a reader that answered `latest` correctly by
/// accident on the four forms somebody thought to write down.
#[tokio::test]
async fn latest_answers_as_the_version_it_stands_for_on_every_address() {
    for tail in [
        // The page, in the shell.
        "model/boot-lane/",
        // Both projections, as files beside it.
        "model/boot-lane.md",
        "model/boot-lane.xml",
        // A page address that lost its slash: the repair, under both.
        "model/boot-lane",
        // The package's own page.
        "",
        // The card's pictures, under the edition.
        "media/b368aadb59c54a38.svg",
        // An address this documentation does not carry: a refusal is an
        // answer, and the two spellings owe the same one.
        "model/nope/",
        // An agent file of the edition.
        "llms.txt",
        // A name the edition does not carry: a refusal is an answer too.
        "nothing.txt",
    ] {
        let numbered = get_bytes(&format!("/doc/com.example/thing-docs/0.2.0/{tail}")).await;
        let latest = get_bytes(&format!("/doc/com.example/thing-docs/latest/{tail}")).await;
        assert_eq!(latest.0, numbered.0, "status of `{tail}`");
        assert_eq!(
            latest.1.get(header::CONTENT_TYPE),
            numbered.1.get(header::CONTENT_TYPE),
            "type of `{tail}`"
        );
        assert_eq!(
            latest.1.get(header::LOCATION),
            numbered.1.get(header::LOCATION),
            "where `{tail}` sends the reader"
        );
        assert_eq!(latest.2, numbered.2, "body of `{tail}`");
    }
}

/// The picture's name above is the one the build actually writes, so the
/// alias case cannot pass by comparing two identical 404s.
#[tokio::test]
async fn the_picture_the_alias_case_names_is_one_this_package_publishes() {
    let tmp = tempfile::tempdir().unwrap();
    package(tmp.path());
    let slots = vibe_doc::media::slots(tmp.path(), "com.example/thing-docs").expect("slots read");
    assert!(
        slots
            .iter()
            .any(|slot| slot.address == "media/b368aadb59c54a38.svg"),
        "the fixture's icon moved; the alias case names a picture nobody publishes: {:?}",
        slots.iter().map(|slot| &slot.address).collect::<Vec<_>>()
    );
}

/// The agent files answer under the edition as well as at the mount's
/// root, with the same bytes — which is where the site publishes them
/// (`##SEO-LLMS-FILES`, A4.5), so an address an agent learned from the
/// web is an address this reader answers.
#[tokio::test]
async fn the_agent_files_answer_under_the_edition_in_both_spellings() {
    for (name, content_type) in [
        ("manifest.json", "application/json; charset=utf-8"),
        ("llms.txt", "text/plain; charset=utf-8"),
        ("llms-small.txt", "text/plain; charset=utf-8"),
        ("llms-medium.txt", "text/plain; charset=utf-8"),
        ("llms-full.txt", "text/plain; charset=utf-8"),
    ] {
        let (status, headers, root) = get_path(&format!("/doc/{name}")).await;
        assert_eq!(status, StatusCode::OK, "{name} at the root");
        assert_eq!(headers[header::CONTENT_TYPE], content_type, "{name}");
        assert!(root.contains("Thing Manual"), "{name}: {root}");

        for version in ["0.2.0", "latest"] {
            let (status, headers, under) =
                get_path(&format!("/doc/com.example/thing-docs/{version}/{name}")).await;
            assert_eq!(status, StatusCode::OK, "{name} under {version}");
            assert_eq!(
                headers[header::CONTENT_TYPE],
                content_type,
                "{name} under {version}"
            );
            assert_eq!(
                under, root,
                "{name} under {version} is not the root's bytes"
            );
        }
    }
}

/// A name the edition does not carry is refused where it was asked for,
/// and not sent one hop further to be refused there.
#[tokio::test]
async fn a_file_the_edition_does_not_carry_is_refused_without_a_hop() {
    for version in ["0.2.0", "latest"] {
        for name in ["nothing.txt", "llms-enormous.txt", "vibe.toml"] {
            let (status, _, body) =
                get_path(&format!("/doc/com.example/thing-docs/{version}/{name}")).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{version}/{name}: {body}");
            assert!(body.contains(name), "{version}/{name}: {body}");
            assert!(
                !body.contains("[package]"),
                "{version}/{name} served a file"
            );
        }
    }
    // …while a page that merely lost its slash is still repaired.
    let (status, headers, _) = get_path("/doc/com.example/thing-docs/latest/model/boot-lane").await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        headers[header::LOCATION],
        "/doc/com.example/thing-docs/0.2.0/model/boot-lane/"
    );
}

/// `latest` is the VERSION segment and nothing else: a document of that
/// name is still a document.
#[tokio::test]
async fn latest_is_read_only_where_a_version_stands() {
    // Not a version position — the reader has no page called `latest`,
    // and says so rather than reading the segment as a version.
    let (status, _, body) = get_path("/doc/com.example/thing-docs/0.2.0/latest/").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("latest.xml"), "{body}");
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
