use super::*;

use std::fs;

use crate::glossary::Glossary;

/// A package that DECLARES its glossary, read the way the linter reads
/// it: the terms are the entries of the page the manifest names, and a
/// package that declares none has no terms at all (`##GLOSSARY-DECLARED`).
fn declared_glossary(entries: &str) -> Option<Glossary> {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(
        dir.path().join("vibe.toml"),
        "[glossary]
page = \"glossary/index\"
",
    )
    .expect("write");
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
    let set = crate::pages::read_package(dir.path()).expect("read");
    crate::glossary::read(dir.path(), &set).expect("read")
}

#[test]
fn the_terms_are_the_glossary_sections_longest_first() {
    let glossary = declared_glossary(
        "<fingerprint title=\"fingerprint\"><p>A hash.</p></fingerprint>\
         <freshness-fingerprint title=\"freshness fingerprint\"><p>Another.</p></freshness-fingerprint>",
    );
    let terms = terms(glossary.as_ref());
    assert_eq!(terms[0].phrase, "freshness fingerprint");
    assert_eq!(terms[1].phrase, "fingerprint");
}

/// The glossary heads an entry `index (of a registry)` because two things
/// are called an index. The words on the page are still «index».
#[test]
fn a_disambiguating_parenthesis_is_not_part_of_the_term() {
    let glossary = declared_glossary(
        "<index-registry title=\"index (of a registry)\"><p>A file.</p></index-registry>",
    );
    let terms = terms(glossary.as_ref());
    assert_eq!(terms[0].phrase, "index");
    assert_eq!(terms[0].anchor, "index-registry");
}

/// Without this, the first paragraph of a page about packages could not
/// say the word «package» (`##STYLE-PAGE-SKELETON`, 2026-09-12).
#[test]
fn the_six_ordinary_words_are_not_terms() {
    let glossary = declared_glossary(
        "<package title=\"package\"><p>A unit.</p></package>\
         <store title=\"store\"><p>A cache.</p></store>",
    );
    let found = terms(glossary.as_ref());
    let phrases: Vec<&str> = found.iter().map(|t| t.phrase.as_str()).collect();
    assert_eq!(phrases, ["store"]);
}

/// A package that declares no glossary has no terms, and the term rules
/// then find nothing (`##GLOSSARY-DECLARED`).
#[test]
fn a_package_with_no_declaration_has_no_terms() {
    assert!(terms(None).is_empty());
    // And a declaration whose page is not there is the same answer.
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(
        dir.path().join("vibe.toml"),
        "[glossary]
page = \"glossary/index\"
",
    )
    .expect("write");
    let set = crate::pages::read_package(dir.path()).expect("read");
    let missing = crate::glossary::read(dir.path(), &set).expect("read");
    assert!(terms(missing.as_ref()).is_empty());
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
