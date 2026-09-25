//! The learning path, measured — and the three links that are not
//! forward links.

use vibe_wire::generated::doc_manifest::NavigationChapter;

use super::*;

/// A chapter row as a source edition writes one.
fn chapter(id: &str, pages: &[&str]) -> NavigationChapter {
    NavigationChapter {
        id: id.to_owned(),
        title: id.to_owned(),
        pages: pages.iter().map(|p| (*p).to_owned()).collect(),
        appendix: false,
    }
}

/// The same, for pages a reader looks things up in.
fn appendix(id: &str, pages: &[&str]) -> NavigationChapter {
    NavigationChapter {
        appendix: true,
        ..chapter(id, pages)
    }
}

/// One page whose single paragraph is `text` — the prose the links are
/// read out of.
fn page(rel: &str, text: &str) -> Page {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">T</title>\n  <p>{text}</p>\n\
         </spec>\n"
    );
    Page {
        rel: rel.to_owned(),
        path: std::path::PathBuf::from(rel),
        doc: vibe_specdoc::from_xml_with(&body, vibe_specdoc::Vocabulary::Doc)
            .expect("the page parses"),
    }
}

/// Three chapters, the last one an appendix — the shape of the manual's
/// own first path, small enough to reason about.
fn path() -> Vec<NavigationChapter> {
    vec![
        chapter("start", &["start/index", "start/install"]),
        chapter("model", &["model/two-trees"]),
        appendix("reference", &["glossary/index"]),
    ]
}

/// The measurement, and the whole of what it counts: a link to a page the
/// path reaches later. A link BACK is not counted — the reader has already
/// been there, which is what a textbook's cross-reference is for.
#[test]
fn a_link_ahead_is_counted_and_a_link_back_is_not() {
    let report = measure(
        &path(),
        &[
            page(
                "start/index.xml",
                "first [the trees](../model/two-trees.xml)",
            ),
            page("start/install.xml", "back to [the start](index.xml)"),
            page(
                "model/two-trees.xml",
                "as [installing](../start/install.xml) said",
            ),
            page("glossary/index.xml", "nothing ahead of here"),
        ],
    );
    assert!(report.measured);
    assert_eq!(report.chapters, 3);
    assert_eq!(report.pages, 4);
    assert_eq!(
        report.forward_links,
        vec![ForwardLink {
            page: "start/index".to_owned(),
            target: "model/two-trees".to_owned(),
        }],
        "only the link that points ahead of the reader"
    );
    assert_eq!(report.forward_link_count, 1);
    assert!(report.render().contains("start/index -> model/two-trees"));
    assert!(report.render().contains("1 link(s) pointing ahead"));
}

/// A link into an appendix chapter is not a forward link, however far
/// ahead the page stands: looking a term up in the glossary is not being
/// sent ahead of the lesson, and declaring the chapter `appendix = true`
/// is how that is said once (`##NAV-CHAPTERS-CHECKED`).
#[test]
fn a_link_into_an_appendix_chapter_is_not_a_link_ahead() {
    let report = measure(
        &path(),
        &[
            page(
                "start/index.xml",
                "see [the glossary](../glossary/index.xml#specspace) for the words",
            ),
            page("start/install.xml", "nothing"),
            page("model/two-trees.xml", "nothing"),
            page("glossary/index.xml", "nothing"),
        ],
    );
    assert!(
        report.forward_links.is_empty(),
        "{:?}",
        report.forward_links
    );
    assert_eq!(report.forward_link_count, 0);

    // The same path with that chapter declared ordinary counts it, which
    // is what proves the appendix mark is doing the work and not the
    // folder's name.
    let numbered = vec![
        chapter("start", &["start/index", "start/install"]),
        chapter("model", &["model/two-trees"]),
        chapter("reference", &["glossary/index"]),
    ];
    let counted = measure(
        &numbered,
        &[page(
            "start/index.xml",
            "see [the glossary](../glossary/index.xml#specspace) for the words",
        )],
    );
    assert_eq!(counted.forward_link_count, 1);
}

/// Three more things that are not links to a later page, and none of them
/// is special-cased: a fragment names a place on the page that says it, a
/// citation is resolved by the resolver rather than by a path, and a page
/// off the path has no position to be later than.
#[test]
fn an_anchor_a_citation_and_a_page_off_the_path_are_not_counted() {
    let report = measure(
        &path(),
        &[page(
            "start/index.xml",
            "jump to [p07](#p07), read [the rule](spec://org.demo/lib/common/PROP-001#R), \
             visit [the web](https://vibevm.org/) and [a draft](../draft/notes.xml)",
        )],
    );
    assert!(
        report.forward_links.is_empty(),
        "{:?}",
        report.forward_links
    );
}

/// A package that declared no path is not measured, and the report says
/// which of the two it is: «no link points ahead» and «nobody declared an
/// order» print the same digit and mean opposite things.
#[test]
fn a_package_without_a_path_is_not_measured_and_says_so() {
    let report = measure(&[], &[page("start/index.xml", "nothing")]);
    assert!(!report.measured);
    assert_eq!(report.forward_link_count, 0);
    assert!(report.render().contains("declares no learning path"));
    assert!(
        !report.render().contains("link(s) pointing ahead"),
        "a package with no path prints no count: {}",
        report.render()
    );
}

/// The JSON a machine reads: the pairs in their own field and the count
/// beside them, so a reading does not have to count the list.
#[test]
fn the_json_carries_the_pairs_and_their_count() {
    let report = measure(
        &path(),
        &[page(
            "start/index.xml",
            "first [the trees](../model/two-trees.xml)",
        )],
    );
    let json: serde_json::Value =
        serde_json::from_str(&to_json(&report)).expect("the measurement is JSON");
    assert_eq!(json["forward_link_count"], 1);
    assert_eq!(json["forward_links"][0]["page"], "start/index");
    assert_eq!(json["forward_links"][0]["target"], "model/two-trees");
    assert_eq!(json["chapters"], 3);
    assert_eq!(json["measured"], true);
}

/// Over a real tree: the fixture manual declares no path, so the check
/// reads its manifest, finds none and reports the unmeasured state
/// without touching a page.
#[test]
fn a_fixture_package_that_declares_no_path_is_read_and_left_unmeasured() {
    let report = check(&crate::manifest::tests::fixture("manual")).expect("the package reads");
    assert!(!report.measured);
    assert!(report.render().contains("declares no learning path"));
}
