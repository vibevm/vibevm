//! The island's half of the glossary card: the two attributes a term link
//! takes, and the hidden definitions the page ends with
//! (`##READER-GLOSSARY-CARD`).

use super::*;

use crate::html;
use crate::numbering::Numbering;

/// The glossary the pages below are rendered against.
fn glossary() -> Glossary {
    let entry = |id: &str, term: &str, definition: &str| Entry {
        id: id.to_owned(),
        term: term.to_owned(),
        definition: definition.to_owned(),
    };
    Glossary {
        document: "glossary/index".to_owned(),
        entries: vec![
            entry(
                "manifest",
                "manifest",
                "What a package says about itself, in `vibe.toml`. See [the lock \
                 file](index.xml#lock-file).",
            ),
            entry(
                "lock-file",
                "lock file",
                "The resolved graph of exact versions.",
            ),
            entry(
                "store",
                "store",
                "The machine-wide cache of package content.",
            ),
        ],
    }
}

/// The bundle a page is rendered with: the site's base, and the glossary
/// when the documentation declared one.
fn content(declared: Option<Glossary>) -> Content {
    Content {
        glossary: declared,
        ..Content::new()
    }
}

/// One page, rendered as the island at `page`.
fn island(page: &str, body: &str, content: &Content) -> String {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">A page</title>\n{body}</spec>\n"
    );
    let doc =
        vibe_specdoc::from_xml_with(&xml, vibe_specdoc::Vocabulary::Doc).expect("the page parses");
    html::to_html_numbered(&doc, page, content, &Numbering::none())
}

/// A link to an entry keeps its address and takes two attributes: the entry
/// it names, and the definition that describes it.
#[test]
fn a_term_link_takes_the_entry_and_its_description() {
    let rendered = island(
        "model/two-trees.xml",
        "  <p>The [manifest](../glossary/index.xml#manifest) says what a package is.</p>\n",
        &content(Some(glossary())),
    );

    assert!(
        rendered.contains(
            "<a href=\"../../glossary/index/#manifest\" data-gloss=\"manifest\" \
             aria-describedby=\"gloss-manifest\">manifest</a>"
        ),
        "{rendered}"
    );
}

/// The definitions of the terms THIS page names, hidden, at the end of the
/// island, each once and in the order the page first names them.
#[test]
fn the_island_ends_with_the_definitions_the_page_names() {
    let rendered = island(
        "model/two-trees.xml",
        "  <p>The [store](../glossary/index.xml#store) holds what the \
         [manifest](../glossary/index.xml#manifest) asked for.</p>\n  \
         <s title=\"S\"><p>And the [store](../glossary/index.xml#store) again.</p></s>\n",
        &content(Some(glossary())),
    );

    assert!(
        rendered.contains("<aside class=\"gloss-defs\" hidden=\"\" data-gloss-defs=\"\">"),
        "{rendered}"
    );
    assert!(
        rendered.contains("<div class=\"gloss-def\" id=\"gloss-store\" data-gloss=\"store\">"),
        "{rendered}"
    );
    assert!(
        rendered.contains("<p class=\"gloss-def__term\">store</p>"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "<p class=\"gloss-def__text\">The machine-wide cache of package content.</p>"
        ),
        "{rendered}"
    );

    // Reading order, and each term once however often it is linked.
    let at = |needle: &str| rendered.find(needle).expect(needle);
    assert!(
        at("id=\"gloss-store\"") < at("id=\"gloss-manifest\""),
        "{rendered}"
    );
    assert_eq!(
        rendered.matches("id=\"gloss-store\"").count(),
        1,
        "{rendered}"
    );
    // The term nothing linked is not carried: a card cannot show what no
    // link points at.
    assert!(!rendered.contains("gloss-lock-file"), "{rendered}");
}

/// A definition's own inline markup is rendered as the page's is, and its
/// links are resolved against the page carrying it.
#[test]
fn a_definitions_markup_and_links_are_the_islands_own() {
    let rendered = island(
        "model/two-trees.xml",
        "  <p>The [manifest](../glossary/index.xml#manifest) is a file.</p>\n",
        &content(Some(glossary())),
    );

    assert!(rendered.contains("<code>vibe.toml</code>"), "{rendered}");
    // Resolved as everything else in this island is, and with no lens of
    // its own: a definition that opened a second card would show the same
    // paragraph twice.
    assert!(
        rendered.contains("<a href=\"../index/#lock-file\">the lock file</a>"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("gloss-lock-file"),
        "a definition's links carry no card of their own: {rendered}"
    );
}

/// The glossary page itself shows nothing: the entries are defined there,
/// and a card over a definition would repeat the paragraph under the
/// cursor.
#[test]
fn the_glossary_page_carries_neither_attribute_nor_block() {
    let rendered = island(
        "glossary/index.xml",
        "  <p>See the [store](index.xml#store) and the [manifest](index.xml#manifest).</p>\n",
        &content(Some(glossary())),
    );

    assert!(!rendered.contains("data-gloss"), "{rendered}");
    assert!(!rendered.contains("aria-describedby"), "{rendered}");
    // And the links themselves are untouched.
    assert!(
        rendered.contains("<a href=\"../index/#store\">store</a>"),
        "{rendered}"
    );
}

/// Without a declared glossary nothing changes — the rollback case, and the
/// state of every documentation written before the table existed.
#[test]
fn a_documentation_with_no_glossary_renders_exactly_as_before() {
    let body = "  <p>The [manifest](../glossary/index.xml#manifest) says what a package \
                is.</p>\n";
    let with = island("model/two-trees.xml", body, &content(Some(glossary())));
    let without = island("model/two-trees.xml", body, &content(None));

    assert!(!without.contains("data-gloss"), "{without}");
    assert!(!without.contains("gloss-defs"), "{without}");
    assert!(
        without.contains("<a href=\"../../glossary/index/#manifest\">manifest</a>"),
        "{without}"
    );
    assert_ne!(with, without);
}

/// Only an anchor of the declared glossary page is a term. Its neighbours
/// are ordinary links, and so is an anchor the glossary does not define.
#[test]
fn only_an_entry_of_the_declared_page_is_a_term() {
    let rendered = island(
        "model/two-trees.xml",
        "  <p>Not a term: [a page](../model/boot-lane.xml#store), \
         [an anchor](#store), \
         [a citation](spec://com.example/subject/common/PROP-001#store), \
         [the web](https://vibevm.org/#store), \
         [no entry](../glossary/index.xml#nothing-of-the-sort), \
         [no anchor](../glossary/index.xml).</p>\n",
        &content(Some(glossary())),
    );

    assert!(!rendered.contains("data-gloss"), "{rendered}");
    assert!(!rendered.contains("gloss-defs"), "{rendered}");
}

/// The block is apparatus and takes no number, so a page's `pNN` addresses
/// mean the same thing whether or not it carries definitions
/// (`##READER-NUMBERED-BLOCKS`).
#[test]
fn the_definitions_take_no_block_number() {
    let body = "  <p>The [manifest](../glossary/index.xml#manifest) says what a package \
                is.</p>\n";
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">A page</title>\n{body}</spec>\n"
    );
    let doc =
        vibe_specdoc::from_xml_with(&xml, vibe_specdoc::Vocabulary::Doc).expect("the page parses");
    let numbering = crate::numbering::number_blocks(&doc);
    let rendered = html::to_html_numbered(
        &doc,
        "model/two-trees.xml",
        &content(Some(glossary())),
        &numbering,
    );

    assert_eq!(numbering.len(), 1, "one paragraph, one number");
    assert_eq!(rendered.matches("data-p=\"").count(), 1, "{rendered}");
    assert!(
        rendered.contains("<aside class=\"gloss-defs\""),
        "{rendered}"
    );
}
