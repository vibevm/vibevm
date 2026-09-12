//! The composed pages, read back through the pivot that will read them.

use super::*;

fn manifest(text: &str) -> Manifest {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(tmp.path().join("vibe.toml"), text).expect("writing");
    super::super::card::read(tmp.path()).expect("a manifest")
}

fn card() -> Manifest {
    manifest(
        "[package]\nname = \"wal\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n\
         kind = \"flow\"\nlicense = \"UPL-1.0\"\nauthors = [\"A\", \"B\"]\n\
         keywords = [\"log\"]\ndescription = \"A log.\"\n\
         \n[[skill]]\nname = \"wal\"\npath = \"skills/wal\"\n",
    )
}

fn parsed(page: &str) -> vibe_specdoc::doc::SpecDoc {
    vibe_specdoc::from_xml_with(page, vibe_specdoc::doc::Vocabulary::Doc)
        .unwrap_or_else(|e| panic!("the pivot refused: {e}\n{page}"))
}

#[test]
fn the_manifest_page_reads_as_a_page() {
    let page = manifest_page(&card(), &Related::default());
    parsed(&page);
    assert!(page.contains("org.example/wal"), "{page}");
    assert!(page.contains("UPL-1.0"), "{page}");
    assert!(page.contains("A, B"), "{page}");
    assert!(page.contains("<skills title="), "{page}");
}

#[test]
fn the_readme_page_reads_as_a_page() {
    let page = readme_page(
        &card(),
        "# wal\n\nA log.\n\n## Install\n\n```sh\nvibe install\n```\n",
    );
    parsed(&page);
    assert!(page.contains("<wal title=\"wal\">"), "{page}");
    assert!(page.contains("<install title=\"Install\">"), "{page}");
    assert!(page.contains("lang=\"sh\""), "{page}");
}

/// The anchor is a slug of the heading, so the same heading gives the
/// same anchor at every render — an anchor is immutable once published.
#[test]
fn one_heading_gives_one_anchor_at_every_render() {
    let page = readme_page(&card(), "## Getting started\n\ntext\n");
    assert!(page.contains("<getting-started title="), "{page}");
    assert_eq!(page, readme_page(&card(), "## Getting started\n\ntext\n"));
}

/// The same heading twice is legal in a README and illegal as an anchor.
#[test]
fn a_repeated_heading_gets_a_second_anchor_and_the_page_still_reads() {
    let page = readme_page(&card(), "## Use\n\none\n\n## Use\n\ntwo\n");
    parsed(&page);
    assert!(page.contains("<use title="), "{page}");
    assert!(page.contains("<use-2 title="), "{page}");
}

/// A heading may begin with a number, and an element name may not.
#[test]
fn a_heading_that_starts_with_a_digit_still_makes_a_legal_name() {
    let page = readme_page(&card(), "## 2. Install\n\ntext\n");
    parsed(&page);
    assert!(page.contains("<s-2-install title="), "{page}");
}

/// Text before the first heading belongs to the page, not to a section
/// that has not been opened.
#[test]
fn prose_before_the_first_heading_stands_at_the_top_of_the_page() {
    let page = readme_page(&card(), "A sentence.\n\n## Later\n\nmore\n");
    parsed(&page);
    let sentence = page.find("A sentence.").expect("the sentence");
    let section = page.find("<later title=").expect("the section");
    assert!(sentence < section, "{page}");
}

/// Markup a README carries must never become markup of the page.
#[test]
fn angle_brackets_and_ampersands_in_a_readme_are_text() {
    let page = readme_page(
        &card(),
        "# <b>Bold</b> & more\n\n<script>alert(1)</script>\n",
    );
    parsed(&page);
    assert!(!page.contains("<b>"), "{page}");
    assert!(!page.contains("<script>"), "{page}");
    assert!(page.contains("&lt;script&gt;"), "{page}");
    assert!(page.contains("&amp; more"), "{page}");
}

#[test]
fn the_boot_snippet_page_says_who_reads_it() {
    let card = manifest(
        "[package]\nname = \"a\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n\
         \n[boot_snippet]\npath = \"snippet.md\"\n",
    );
    let snippet = card.table("boot_snippet").expect("the table").clone();
    let page = boot_snippet_page(&card, &snippet);
    parsed(&page);
    assert!(page.contains("read by the agent"), "{page}");
    assert!(page.contains("snippet.md"), "{page}");
}

/// A manifest with nothing optional in it is still a reference page.
#[test]
fn a_manifest_with_no_optional_field_still_makes_a_page() {
    let card = manifest("[package]\nname = \"a\"\ngroup = \"org.example\"\nversion = \"1.0.0\"\n");
    let page = manifest_page(&card, &Related::default());
    parsed(&page);
    assert!(page.contains("<the-manifest title="), "{page}");
}

fn shelf_row(coordinate: &str, rank: crate::site::shelves::Rank) -> crate::site::shelves::Row {
    let (publisher, name) = coordinate.rsplit_once('/').expect("a coordinate");
    crate::site::shelves::Row {
        coordinate: coordinate.into(),
        version: "1.0.0".into(),
        title: name.into(),
        publisher: publisher.into(),
        lang: "en".into(),
        rank,
    }
}

/// Three signals, and the page has to carry all three: the mark, the
/// word beside it, and the order the rows arrived in.
#[test]
fn a_shelf_carries_the_mark_the_word_and_the_publisher() {
    use crate::site::shelves::Rank;
    let related = Related {
        documentation: vec![
            shelf_row("org.example/wal-book", Rank::Primary),
            shelf_row("org.volunteers/wal-notes", Rank::Community),
        ],
        ..Related::default()
    };
    let page = manifest_page(&card(), &related);
    parsed(&page);
    assert!(page.contains("★ primary"), "{page}");
    assert!(page.contains("community"), "{page}");
    assert!(page.contains("org.volunteers"), "{page}");
    let first = page.find("wal-book").expect("the primary row");
    let second = page.find("wal-notes").expect("the community row");
    assert!(first < second, "the ranked order did not survive");
}

/// «Nobody has documented this» is what an absent shelf says, and it
/// says it better than a heading over an empty table.
#[test]
fn a_shelf_with_no_rows_is_not_printed_at_all() {
    let page = manifest_page(&card(), &Related::default());
    parsed(&page);
    assert!(!page.contains("<documentation title="), "{page}");
    assert!(!page.contains("<dependants title="), "{page}");
}

/// Structure, and never a distance: «behind by» would need a history
/// the project does not keep.
#[test]
fn an_adaptation_page_states_the_mirror_and_refuses_to_state_a_distance() {
    let related = Related::default().adapting(Some(super::super::Adaptation {
        source: "org.example/wal-docs".into(),
        pages: 12,
        divergences: 2,
    }));
    let page = manifest_page(&card(), &related);
    parsed(&page);
    assert!(page.contains("org.example/wal-docs"), "{page}");
    assert!(page.contains("2 pages do not mirror"), "{page}");
    assert!(page.contains("STRUCTURE and not of meaning"), "{page}");
}

#[test]
fn an_adaptation_that_mirrors_completely_says_so() {
    let related = Related::default().adapting(Some(super::super::Adaptation {
        source: "org.example/wal-docs".into(),
        pages: 12,
        divergences: 0,
    }));
    assert!(
        manifest_page(&card(), &related).contains("Every page mirrors it"),
        "the clean state is not stated"
    );
}
