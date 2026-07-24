//! The convention-held-by-tests law (PROP-043 §2): foreign inline grammars
//! (`@spec://`, `#use`, `#embed`, `#source`, `<!-- REVIEW -->`) are opaque
//! text, fenced/inline code is never scanned, and the placement rules of
//! §3.8 hold exactly.

use progress_core::doc::{IssueCode, Severity};
use progress_core::model::{Granularity, MarkerForm, Stage, State};
use progress_core::parse::parse_document;

#[test]
fn foreign_grammar_fixture_yields_zero_false_matches() {
    let text = include_str!("fixtures/foreign-grammars.md");
    let doc = parse_document("fixtures/foreign-grammars.md", text);

    for i in &doc.issues {
        assert_ne!(
            i.severity,
            Severity::Error,
            "fixture must be clean, got: {:?}",
            i
        );
    }
    // Exactly the intended markers, nothing from foreign grammars or code:
    // doc marker, section marker, @impl, @spec/done, @doc/done,
    // @doc/plan, wrapper fragment, @impl/done.
    assert_eq!(doc.markers.len(), 8, "markers: {:#?}", doc.markers);
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.granularity == Granularity::Document)
            .count(),
        1
    );
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.granularity == Granularity::Section)
            .count(),
        1
    );
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.form == MarkerForm::Wrapper)
            .count(),
        1
    );
    // `@spec/done` parsed as shorthand (stage=spec), NOT as `@spec://…`.
    assert!(
        doc.markers
            .iter()
            .any(|m| m.stage == Stage::Spec && m.state == State::Done)
    );
    // Nothing from inside the fence leaked: no idea/plan marker exists.
    assert!(
        !doc.markers
            .iter()
            .any(|m| m.stage == Stage::Idea && m.state == State::Plan)
    );
    // Every prose paragraph is marked → exhaustive counter is zero.
    assert_eq!(doc.unmarked_paragraphs.len(), 0);
}

#[test]
fn stranded_marker_between_paragraphs_is_an_error() {
    let text = "# H {#h}\n\nFirst paragraph. @impl\n\n<status stage=\"test\" state=\"plan\"/>\n\nSecond paragraph. @impl\n";
    let doc = parse_document("x.md", text);
    assert!(
        doc.issues
            .iter()
            .any(|i| i.code == IssueCode::Stranded && i.severity == Severity::Error),
        "issues: {:#?}",
        doc.issues
    );
}

#[test]
fn section_marker_is_the_standalone_right_after_a_heading() {
    // The SECOND heading: its standalone is a section marker. (After the
    // FIRST heading of a preamble-less file it would be the document
    // marker — see the fallback test below.)
    let text = "# H {#h}\n\nIntro. @impl\n\n## S {#s}\n\n<status stage=\"impl\" state=\"done\"/>\n\nBody paragraph. @test/plan\n";
    let doc = parse_document("x.md", text);
    assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
    assert!(
        doc.markers
            .iter()
            .any(|m| m.granularity == Granularity::Section
                && m.stage == Stage::Impl
                && m.state == State::Done)
    );
}

#[test]
fn first_h1_standalone_governs_a_preambleless_document() {
    // Every spec in this repo opens with its H1 — there is no preamble to
    // put a document marker into. The standalone right after that first
    // heading IS the document marker (PROP-043 §3.8, pilot amendment).
    let text = "# H {#h}\n\n<status stage=\"doc\" state=\"done\"/>\n\nBody. @spec/hold\n";
    let doc = parse_document("x.md", text);
    assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
    let dm = doc.document_marker().expect("document marker");
    assert_eq!((dm.stage, dm.state), (Stage::Doc, State::Done));
    let r = progress_core::rollup::rollup_doc(&doc);
    // Explicit (doc/done) wins over computed worst-of (spec/hold).
    assert_eq!(r.explicit, Some((Stage::Doc, State::Done)));
    assert_eq!(r.effective, Some((Stage::Doc, State::Done)));
}

#[test]
fn preamble_marker_governs_the_document() {
    let text = "@freeze/done\n\n# H {#h}\n\nBody. @impl\n";
    let doc = parse_document("x.md", text);
    let dm = doc.document_marker().expect("document marker");
    assert_eq!((dm.stage, dm.state), (Stage::Freeze, State::Done));
}

#[test]
fn mid_paragraph_point_marker_is_an_error() {
    let text = "# H {#h}\n\nSome text <status stage=\"impl\" state=\"work\"/> more text.\n";
    let doc = parse_document("x.md", text);
    assert!(doc.issues.iter().any(|i| i.code == IssueCode::MidParagraph));
}

#[test]
fn unmarked_paragraphs_are_counted_for_exhaustive() {
    let text = "# H {#h}\n\nMarked. @impl\n\nUnmarked paragraph one.\n\nUnmarked two.\n";
    let doc = parse_document("x.md", text);
    assert_eq!(doc.paragraph_count, 3);
    assert_eq!(doc.unmarked_paragraphs.len(), 2);
}

#[test]
fn vocabulary_typo_carries_a_hint() {
    let text = "# H {#h}\n\nBody. <status stage=\"impl\" state=\"work\" action=\"rewrok\"/>\n";
    let doc = parse_document("x.md", text);
    let issue = doc
        .issues
        .iter()
        .find(|i| i.code == IssueCode::Vocabulary)
        .expect("vocabulary issue");
    assert!(issue.message.contains("rework"), "{}", issue.message);
}
