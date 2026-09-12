use super::*;

use std::fs;

fn package_with_glossary(entries: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let pages = dir.path().join("vibevm/vibespecs/glossary");
    fs::create_dir_all(&pages).expect("page dir");
    fs::write(
        pages.join("index.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">Glossary</title>\n{entries}</spec>\n"
        ),
    )
    .expect("write");
    dir
}

#[test]
fn the_terms_are_the_glossary_sections_longest_first() {
    let dir = package_with_glossary(
        "<fingerprint title=\"fingerprint\"><p>A hash.</p></fingerprint>\
         <freshness-fingerprint title=\"freshness fingerprint\"><p>Another.</p></freshness-fingerprint>",
    );
    let set = crate::pages::read_package(dir.path()).expect("read");
    let terms = terms(&set);
    assert_eq!(terms[0].phrase, "freshness fingerprint");
    assert_eq!(terms[1].phrase, "fingerprint");
}

/// The glossary heads an entry `index (of a registry)` because two things
/// are called an index. The words on the page are still «index».
#[test]
fn a_disambiguating_parenthesis_is_not_part_of_the_term() {
    let dir = package_with_glossary(
        "<index-registry title=\"index (of a registry)\"><p>A file.</p></index-registry>",
    );
    let set = crate::pages::read_package(dir.path()).expect("read");
    let terms = terms(&set);
    assert_eq!(terms[0].phrase, "index");
    assert_eq!(terms[0].anchor, "index-registry");
}

/// Without this, the first paragraph of a page about packages could not
/// say the word «package» (`##STYLE-PAGE-SKELETON`, 2026-09-12).
#[test]
fn the_six_ordinary_words_are_not_terms() {
    let dir = package_with_glossary(
        "<package title=\"package\"><p>A unit.</p></package>\
         <store title=\"store\"><p>A cache.</p></store>",
    );
    let set = crate::pages::read_package(dir.path()).expect("read");
    let found = terms(&set);
    let phrases: Vec<&str> = found.iter().map(|t| t.phrase.as_str()).collect();
    assert_eq!(phrases, ["store"]);
}

#[test]
fn a_package_with_no_glossary_has_no_terms() {
    let dir = tempfile::tempdir().expect("temp dir");
    let set = crate::pages::read_package(dir.path()).expect("read");
    assert!(terms(&set).is_empty());
}

#[test]
fn the_head_noun_inflects_in_the_three_regular_ways() {
    let registry = pattern_for("registry").expect("compiles");
    assert!(registry.is_match("two registries"));
    assert!(registry.is_match("one registry"));
    let branch = pattern_for("branch").expect("compiles");
    assert!(branch.is_match("the branches"));
    let day = pattern_for("day").expect("compiles");
    assert!(day.is_match("two days"));
    assert!(!day.is_match("two daies"));
}
