//! The Markdown projection's own laws. The shared-numbers law lives in
//! `tests/projections.rs`, where all three projections are compared.

use std::collections::BTreeMap;

use super::*;
use crate::citations::{RuleText, Source};
use crate::numbering::number_blocks;

fn page(body: &str) -> SpecDoc {
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n{body}\
         </spec>\n"
    );
    vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc).expect("page parses")
}

/// The address the rendered page is given. Any page address serves these
/// laws, and a borrowed example is resolved against exactly this one.
const PAGE: &str = "guide/one.xml";

fn rendered(body: &str, content: &Content) -> String {
    let doc = page(body);
    to_markdown_numbered(&doc, PAGE, content, &number_blocks(&doc))
}

/// The number opens the block.
#[test]
fn the_number_opens_the_block_it_names() {
    let md = rendered("  <p>one</p>\n  <p>two</p>\n", &Content::new());
    assert!(md.contains("[p01] one"), "{md}");
    assert!(md.contains("[p02] two"), "{md}");
}

/// A list's first item is where the block begins, so the label opens it;
/// the later items carry nothing.
#[test]
fn a_lists_label_opens_its_first_item_only() {
    let md = rendered(
        "  <list ordered=\"false\"><item>a</item><item>b</item></list>\n",
        &Content::new(),
    );
    assert!(md.contains("- [p01] a"), "{md}");
    assert!(md.contains("\n- b"), "{md}");
    assert_eq!(md.matches("[p01]").count(), 1, "{md}");
}

/// A fence and a table have no place for a label inside their first
/// line, so it stands on the line above — still the first thing the
/// block prints.
#[test]
fn a_fence_and_a_table_take_their_label_on_the_line_above() {
    let md = rendered(
        "  <fence lang=\"rust\">let x = 1;</fence>\n  \
           <table><tr><td>a</td></tr></table>\n",
        &Content::new(),
    );
    assert!(md.contains("[p01]\n```rust\nlet x = 1;\n```"), "{md}");
    assert!(md.contains("[p02]\n| a |"), "{md}");
}

/// The reader's Markdown carries the rule's TEXT, which is the whole
/// difference from the pivot's projection of the source.
#[test]
fn a_rule_projects_its_text_and_its_address() {
    let uri = "spec://org.demo/lib/common/PROP-001#A";
    let mut rules = BTreeMap::new();
    rules.insert(
        uri.to_owned(),
        RuleText {
            uri: uri.to_owned(),
            anchor: "A".to_owned(),
            text: "The rule says a thing.".to_owned(),
            lang: "en".to_owned(),
            source: Source::Checkout,
            path: std::path::PathBuf::from("x.xml"),
        },
    );
    let content = Content {
        rules,
        ..Content::new()
    };
    let md = rendered(&format!("  <rule ref=\"{uri}\"/>\n"), &content);
    assert!(md.contains("> [p01] The rule says a thing."), "{md}");
    assert!(md.contains(&format!("> <{uri}>")), "{md}");
    assert!(!md.contains("~r"), "a citation is live and unpinned");
}

/// A `derived` block carries the text this build generated; without it,
/// the provenance line, so the file never claims an empty output.
#[test]
fn a_derived_block_projects_the_generated_text_or_its_provenance() {
    let mut content = Content::new();
    content.derived.insert(
        Content::derived_key(vibe_specdoc::doc::DerivedKind::CliHelp, "vibe --help"),
        "Usage: vibe".to_owned(),
    );
    let md = rendered(
        "  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n",
        &content,
    );
    assert!(md.contains("```text\nUsage: vibe\n```"), "{md}");

    let bare = rendered(
        "  <derived kind=\"cli-help\" ref=\"vibe --help\"/>\n",
        &Content::new(),
    );
    assert!(
        bare.contains("generated from cli-help: vibe --help"),
        "{bare}"
    );
}

/// An example is the command and the output it must produce, in adjacent
/// fences, so a reader can copy one and compare the other.
#[test]
fn an_example_projects_as_adjacent_fences() {
    let md = rendered(
        "  <example id=\"v\" fixture=\"none\"><run>vibe --version</run>\
           <expect>vibe 1.0.0</expect></example>\n",
        &Content::new(),
    );
    assert!(md.contains("```sh\nvibe --version\n```"), "{md}");
    assert!(md.contains("```output\nvibe 1.0.0\n```"), "{md}");
}

/// A guarded slot names its condition, and the block is kept: one file
/// serves every platform, as one build does.
#[test]
fn a_guarded_slot_names_its_condition_and_keeps_its_block() {
    let md = rendered(
        "  <p when=\"os:windows\">backslash</p>\n  <p when=\"os:linux\">slash</p>\n",
        &Content::new(),
    );
    assert!(md.contains("## os:windows\n\n[p01] backslash"), "{md}");
    assert!(md.contains("## os:linux\n\n[p02] slash"), "{md}");
}

/// A fence whose content holds a fence run is quoted by a longer one.
#[test]
fn a_fence_quoting_a_fence_takes_a_longer_run() {
    let md = rendered(
        "  <fence lang=\"md\">```sh&#10;echo hi&#10;```</fence>\n",
        &Content::new(),
    );
    assert!(md.contains("````md"), "{md}");
}

/// A page's anchors survive the projection: a heading keeps `{#id}` and
/// a fact keeps `@fact:`.
#[test]
fn anchors_survive_into_the_markdown() {
    let md = rendered(
        "  <p><A-RULE fact=\"true\" status=\"spec/done\">Text.</A-RULE></p>\n  \
           <section id=\"s\" title=\"S\"><p>x</p></section>\n",
        &Content::new(),
    );
    assert!(
        md.contains("[p01] @fact:A-RULE Text. @status:spec/done"),
        "{md}"
    );
    assert!(md.contains("## S {#s}"), "{md}");
}

/// Without a numbering the projection carries no labels — the neutral
/// element costs a caller no branch.
#[test]
fn without_a_numbering_no_label_appears() {
    let doc = page("  <p>one</p>\n");
    let md = to_markdown(&doc, &Content::new());
    assert!(!md.contains('['), "{md}");
    assert!(md.contains("one"), "{md}");
}
