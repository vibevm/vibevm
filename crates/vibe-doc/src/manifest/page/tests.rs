//! What a page row takes from a page, and what it refuses to invent.

use super::*;
use crate::pages::Page;

fn doc(body: &str) -> SpecDoc {
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">A page</title>\n{body}\
         </spec>\n"
    );
    vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc).expect("page parses")
}

fn page(body: &str) -> Page {
    Page {
        rel: "guide/a.xml".to_owned(),
        path: std::path::PathBuf::from("guide/a.xml"),
        doc: doc(body),
    }
}

/// A task page is the one that opens with the request a person hands to
/// an agent, so the genre is read off the blocks and never declared —
/// a declaration and the blocks could disagree.
#[test]
fn a_page_carrying_a_prompt_is_a_task_page() {
    let task = doc("  <prompt id=\"p\" assert=\"none\">Do the thing.</prompt>\n");
    assert_eq!(genre_of(&task), PageGenre::Task);
}

#[test]
fn a_page_with_no_prompt_is_a_concept_page() {
    assert_eq!(genre_of(&doc("  <p>prose</p>\n")), PageGenre::Concept);
}

/// The audiences come from every `<status>` on the page, document-level
/// and unit-level alike, and arrive in the vocabulary's own order rather
/// than the order the page happened to mention them.
#[test]
fn audiences_are_collected_in_the_vocabularys_order() {
    let d = doc(
        "  <status stage=\"doc\" state=\"work\" audience=\"dev\"/>\n  \
           <p><F fact=\"true\" status=\"doc/work\" audience=\"user\">text</F></p>\n",
    );
    assert_eq!(audiences_of(&d), vec![Audience::User, Audience::Dev]);
}

#[test]
fn a_page_that_marks_no_audience_claims_none() {
    assert!(audiences_of(&doc("  <p>prose</p>\n")).is_empty());
}

/// Every named anchor, in document order: the title, then section ids and
/// fact ids as they are met. The positional `pNN` numbers are not here.
#[test]
fn anchors_are_the_ids_in_document_order() {
    let d = doc(
        "  <p><LEAD fact=\"true\" status=\"doc/work\">lead</LEAD></p>\n  \
           <section id=\"one\" title=\"One\">\n    \
             <p><IN-ONE fact=\"true\" status=\"doc/work\">x</IN-ONE></p>\n    \
             <section id=\"two\" title=\"Two\">\n      <p>plain</p>\n    </section>\n  \
           </section>\n",
    );
    assert_eq!(anchors_of(&d), vec!["root", "LEAD", "one", "IN-ONE", "two"]);
}

/// The summary is the first paragraph, taken verbatim. The style law
/// requires that paragraph to stand alone; composing a second one here
/// would put text in front of readers that no check governs.
#[test]
fn the_summary_is_the_first_paragraph_of_the_page() {
    let d = doc("  <p>What this is, in plain words.</p>\n  <p>And the rest.</p>\n");
    assert_eq!(summary_of(&d), "What this is, in plain words.");
}

#[test]
fn a_page_with_no_paragraph_has_no_summary() {
    let d = doc("  <fence lang=\"sh\">vibe list</fence>\n");
    assert_eq!(summary_of(&d), "");
}

/// The two rates of `##READER-META-AND-PRINT`: the source language reads
/// faster than an adaptation, so the same page is a longer read in
/// Russian than in English.
#[test]
fn an_adaptation_reads_slower_than_the_source_language() {
    let body: String = std::iter::repeat_n("word", 500)
        .collect::<Vec<_>>()
        .join(" ");
    let d = doc(&format!("  <p>{body}</p>\n"));
    // 502 words with the title: three minutes at 250 and at 200 alike.
    assert_eq!(reading_time_min(&d, "en"), 3);
    assert_eq!(reading_time_min(&d, "ru"), 3);
    let short: String = std::iter::repeat_n("word", 450)
        .collect::<Vec<_>>()
        .join(" ");
    let d = doc(&format!("  <p>{short}</p>\n"));
    // 452 words: two minutes for a reader of the source language, three
    // for a reader of an adaptation.
    assert_eq!(reading_time_min(&d, "en"), 2);
    assert_eq!(reading_time_min(&d, "ru"), 3);
}

/// A page with almost nothing on it still takes a minute: zero minutes
/// is not a reading time anybody can use.
#[test]
fn the_shortest_page_still_reads_for_one_minute() {
    assert_eq!(reading_time_min(&doc("  <p>one</p>\n"), "en"), 1);
}

/// Fences, examples and prompts are read too. A reference page made of
/// tables that claimed a one-minute read would be lying to the reader
/// who is about to spend ten.
#[test]
fn the_count_includes_what_the_reader_sees_in_a_fence() {
    let words: String = std::iter::repeat_n("token", 600)
        .collect::<Vec<_>>()
        .join(" ");
    let d = doc(&format!("  <fence lang=\"text\">{words}</fence>\n"));
    assert_eq!(reading_time_min(&d, "en"), 3);
}

/// A `derived` block holds an address and no text, so it weighs nothing
/// until the product is run — and a manifest build deliberately does not
/// run it.
#[test]
fn a_derived_block_weighs_nothing_in_the_reading_time() {
    let with = doc("  <p>one</p>\n  <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n");
    let without = doc("  <p>one</p>\n");
    assert_eq!(
        reading_time_min(&with, "en"),
        reading_time_min(&without, "en")
    );
}

/// A page that has never been read aloud carries no date, which is a
/// state worth showing rather than a zero to fill in.
#[test]
fn a_row_carries_a_read_aloud_date_only_when_the_package_recorded_one() {
    let row = row(&page("  <p>text</p>\n"), "en", &Reviews::none());
    assert_eq!(row.reviewed_at, None);
    assert_eq!(row.path, "guide/a.xml");
    assert_eq!(row.title, "A page");
}
