//! The glossary as an entity: what a declaration reads as, what an entry
//! is, and what a defect of one is (`##GLOSSARY-DECLARED`,
//! `##GLOSSARY-CHECKED`).

use super::*;

use std::path::PathBuf;

/// A package on disk: its manifest body and its pages, by address.
fn package(manifest: &str, pages: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary package");
    std::fs::write(dir.path().join("vibe.toml"), manifest).expect("the manifest");
    for (rel, body) in pages {
        let at: PathBuf = dir.path().join("vibevm/vibespecs").join(rel);
        std::fs::create_dir_all(at.parent().expect("a folder")).expect("the folder");
        std::fs::write(&at, body).expect("the page");
    }
    dir
}

/// A glossary page with two entries, the second of which opens with a list
/// instead of a paragraph.
const GLOSSARY_PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
     <spec xmlns=\"https://vibevm.org/spec/1\">\n\
       <title id=\"root\">Glossary</title>\n\
       <p>One definition per term.</p>\n\
       <lock-file title=\"lock file\">\n\
         <p>The resolved graph, written by `vibe install` and read by every consumer.</p>\n\
         <p>A second paragraph, which is not the definition.</p>\n\
       </lock-file>\n\
       <manifest title=\"manifest\">\n\
         <p>What a package says about itself, in `vibe.toml`.</p>\n\
       </manifest>\n\
     </spec>\n";

const DECLARING: &str = "[package]\nname = \"m\"\ngroup = \"com.example.docs\"\nkind = \"doc\"\n\
                         \n[glossary]\npage = \"glossary/index\"\n";

fn read_set(dir: &std::path::Path) -> PageSet {
    crate::pages::read_package(dir).expect("the package reads")
}

/// The declaration is one line of the manifest, read as data like the rest
/// of the card.
#[test]
fn a_declaration_is_read_off_the_manifest() {
    let dir = package(DECLARING, &[]);
    assert_eq!(
        declared(dir.path()).expect("the manifest reads").as_deref(),
        Some("glossary/index")
    );

    // And a package that declares none has none. The path means nothing by
    // itself (`##GLOSSARY-DECLARED`).
    let bare = package(
        "[package]\nname = \"m\"\n",
        &[("glossary/index.xml", GLOSSARY_PAGE)],
    );
    assert_eq!(declared(bare.path()).expect("the manifest reads"), None);
    assert_eq!(
        read(bare.path(), &read_set(bare.path())).expect("no glossary"),
        None
    );
}

/// Each top-level section is one entry: the anchor a link names, the term
/// the title states, and the FIRST paragraph and no other.
#[test]
fn an_entry_is_a_section_with_its_first_paragraph() {
    let dir = package(DECLARING, &[("glossary/index.xml", GLOSSARY_PAGE)]);
    let glossary = read(dir.path(), &read_set(dir.path()))
        .expect("the package reads")
        .expect("the glossary is declared and present");

    assert_eq!(glossary.document, "glossary/index");
    assert_eq!(glossary.entries.len(), 2);
    let first = &glossary.entries[0];
    assert_eq!(first.id, "lock-file");
    assert_eq!(first.term, "lock file");
    assert!(first.definition.starts_with("The resolved graph"));
    assert!(
        !first.definition.contains("second paragraph"),
        "one paragraph is the definition: {}",
        first.definition
    );

    // The definition keeps the page's own inline Markdown: what a code
    // span is, is the island's decision and not this module's.
    assert!(glossary.entries[1].definition.contains("`vibe.toml`"));
    assert_eq!(
        glossary.entry("manifest").map(|e| e.term.as_str()),
        Some("manifest")
    );
    assert!(glossary.entry("nothing-of-the-sort").is_none());
}

/// A declaration that points at nothing is a defect of the package and not
/// of the render: a build still renders every page it has.
#[test]
fn a_glossary_page_that_is_not_there_is_a_defect_and_not_a_refusal() {
    let dir = package(DECLARING, &[("guide/one.xml", GLOSSARY_PAGE)]);
    let set = read_set(dir.path());

    assert_eq!(read(dir.path(), &set).expect("the package reads"), None);
    assert_eq!(
        check(dir.path(), &set).expect("the check runs"),
        vec![Defect::MissingPage {
            document: "glossary/index".to_owned()
        }]
    );
}

/// An entry states a term and a definition, and the check names the one
/// that does not.
#[test]
fn an_entry_needs_a_term_and_a_definition() {
    let page = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n\
           <title id=\"root\">Glossary</title>\n\
           <lock-file title=\"lock file\">\n\
             <list ordered=\"false\"><item>Not a definition.</item></list>\n\
           </lock-file>\n\
           <manifest title=\"\">\n\
             <p>What a package says about itself.</p>\n\
           </manifest>\n\
         </spec>\n";
    let dir = package(DECLARING, &[("glossary/index.xml", page)]);
    let found = check(dir.path(), &read_set(dir.path())).expect("the check runs");

    assert_eq!(
        found,
        vec![
            Defect::EntryWithoutDefinition {
                document: "glossary/index".to_owned(),
                term: "lock file".to_owned(),
            },
            Defect::EntryWithoutTerm {
                document: "glossary/index".to_owned(),
                id: "manifest".to_owned(),
            },
        ]
    );
    for defect in &found {
        assert!(!defect.render().is_empty());
    }
}

/// A glossary page the pivot refuses hides its own entries, so it is
/// reported as that rather than counted clean.
#[test]
fn an_unreadable_glossary_page_is_reported_as_one() {
    let dir = package(DECLARING, &[("glossary/index.xml", "not xml at all")]);
    assert_eq!(
        check(dir.path(), &read_set(dir.path())).expect("the check runs"),
        vec![Defect::UnreadablePage {
            document: "glossary/index".to_owned()
        }]
    );
}

/// A package that declares no glossary is not a package with a broken one.
#[test]
fn a_package_with_no_declaration_has_nothing_to_check() {
    let dir = package(
        "[package]\nname = \"m\"\n",
        &[("guide/one.xml", GLOSSARY_PAGE)],
    );
    assert!(
        check(dir.path(), &read_set(dir.path()))
            .expect("the check runs")
            .is_empty()
    );
}
