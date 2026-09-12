//! The arXiv-style listing, and the three signals that must agree.

use super::*;
use crate::citations::SpecSources;
use crate::manifest::tests::fixture;
use crate::manifest::{self};
use vibe_wire::generated::doc_manifest::DocumentationStatus;

fn manifest_of(name: &str) -> DocManifest {
    manifest::build(&fixture(name), &SpecSources::new())
        .expect("the fixture builds")
        .manifest
}

/// A row is what an arXiv search result is: a title a person reads, the
/// coordinate, the caption, the publisher, the language, the audiences,
/// the abstract, and a link.
#[test]
fn a_row_is_a_card_a_person_can_choose_from() {
    let text = catalogue(&[manifest_of("translations/source")], "/doc/");
    assert!(text.contains("## The Pair"), "{text}");
    assert!(
        text.contains("com.example.docs/pair 0.1.0 · community · com.example.docs · en · user"),
        "{text}"
    );
    assert!(text.contains("What it covers: two pages"), "{text}");
    assert!(
        text.contains("→ /doc/com.example.docs/pair/0.1.0/"),
        "{text}"
    );
}

/// Three signals that agree: the star, the caption, and the order. A
/// confirmed documentation carries ★ and sorts above one nobody
/// confirmed.
#[test]
fn the_star_the_caption_and_the_order_say_the_same_thing() {
    let mut confirmed = manifest_of("translations/source");
    confirmed.package.status = DocumentationStatus::Primary;
    confirmed.package.title = "Zebra".to_owned();
    let community = manifest_of("manual");
    let text = catalogue(&[community, confirmed], "/doc/");
    let starred = text.find("## ★ Zebra").expect("a starred row");
    let plain = text.find("## Fixture Manual").expect("a plain row");
    assert!(starred < plain, "{text}");
    assert!(text.contains("· primary ·"), "{text}");
    assert!(text.contains("· community ·"), "{text}");
}

/// A translation's star means «named by the author of this
/// documentation», not «approved by the subject» — and the row says so in
/// words rather than leaving the mark to be guessed at.
#[test]
fn a_translations_row_says_what_its_star_means() {
    let text = catalogue(&[manifest_of("translations/adaptation")], "/doc/");
    assert!(
        text.contains("An adaptation of com.example.docs/pair, named official by the author"),
        "{text}"
    );
    assert!(text.contains("not by the subject"), "{text}");
    assert!(text.contains("· ru ·"), "{text}");
}

/// An empty registry renders a document that says it is empty, rather
/// than a heading over nothing.
#[test]
fn an_empty_catalogue_says_so() {
    let text = catalogue(&[], "/doc/");
    assert!(text.contains("No documentation was found"), "{text}");
    assert!(!text.contains("## "), "{text}");
}
