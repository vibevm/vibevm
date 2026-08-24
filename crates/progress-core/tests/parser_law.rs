//! The convention-held-by-tests law (PROP-043 §2): foreign inline grammars
//! (`@spec://`, `#use`, `#embed`, `#source`, `<!-- REVIEW -->`) are opaque
//! text, fenced/inline code is never scanned, and the placement rules of
//! §3.8 — including the fact amendment (list items, table cells, `##<ID>`
//! anchors, anchored-when-marked) — hold exactly.

use progress_core::doc::{FactKind, IssueCode, Severity};
use progress_core::model::{Granularity, MarkerForm, Stage, State};
use progress_core::parse::parse_document;

#[test]
fn foreign_grammar_fixture_yields_only_the_expected_cell_anchor_errors() {
    let text = include_str!("fixtures/foreign-grammars.md");
    let doc = parse_document("fixtures/foreign-grammars.md", text);

    let missing_anchor_lines: Vec<_> = doc
        .issues
        .iter()
        .filter(|issue| issue.code == IssueCode::MissingAnchor)
        .map(|issue| issue.line)
        .collect();
    assert_eq!(
        missing_anchor_lines,
        [44, 45, 45],
        "issues: {:#?}",
        doc.issues
    );
    for i in doc
        .issues
        .iter()
        .filter(|issue| issue.code != IssueCode::MissingAnchor)
    {
        assert_ne!(
            i.severity,
            Severity::Error,
            "fixture must have no foreign-grammar false positive, got: {:?}",
            i
        );
    }
    // Exactly the intended markers, nothing from foreign grammars or code:
    // doc + section standalones, 7 paragraph shorthands (5 prose + 2 list
    // leads), 2 wrappers (fragments), 3 item shorthands + 1 item XML
    // point, 4 cell shorthands.
    assert_eq!(doc.markers.len(), 19, "markers: {:#?}", doc.markers);
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
            .filter(|m| m.granularity == Granularity::Item)
            .count(),
        4,
        "items: {:#?}",
        doc.markers
            .iter()
            .filter(|m| m.granularity == Granularity::Item)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.granularity == Granularity::Cell)
            .count(),
        4
    );
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.form == MarkerForm::Wrapper)
            .count(),
        2
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
    // Every countable unit is marked → the exhaustive counter is zero.
    assert_eq!(doc.unmarked_facts.len(), 0, "unmarked: {:#?}", {
        let mut v = Vec::new();
        for &(bi, fi) in &doc.unmarked_facts {
            v.push(doc.blocks[bi].facts[fi].clone());
        }
        v
    });
    // The fact anchors were minted and recorded.
    let ids: Vec<&str> = doc
        .blocks
        .iter()
        .flat_map(|b| b.facts.iter().filter_map(|f| f.id.as_deref()))
        .collect();
    for want in [
        "p-cite",
        "p-use",
        "lead-rules",
        "RULE-001",
        "RULE-002",
        "RULE-002a",
        "RULE-003",
        "ROW-PKGREF",
    ] {
        assert!(ids.contains(&want), "missing fact id {want}: {ids:?}");
    }
}

#[test]
fn stranded_marker_between_paragraphs_is_an_error() {
    let text = "# H {#h}\n\n##p1 First paragraph. @impl\n\n<status stage=\"test\" state=\"plan\"/>\n\n##p2 Second paragraph. @impl\n";
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
    let text = "# H {#h}\n\n##i1 Intro. @impl\n\n## S {#s}\n\n<status stage=\"impl\" state=\"done\"/>\n\n##b1 Body paragraph. @test/plan\n";
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
    let text = "# H {#h}\n\n<status stage=\"doc\" state=\"done\"/>\n\n##b1 Body. @spec/hold\n";
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
    let text = "@freeze/done\n\n# H {#h}\n\n##b1 Body. @impl\n";
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
fn unmarked_facts_are_counted_for_exhaustive() {
    let text = "# H {#h}\n\n##m1 Marked. @impl\n\nUnmarked paragraph one.\n\nUnmarked two.\n";
    let doc = parse_document("x.md", text);
    assert_eq!(doc.fact_count, 3);
    assert_eq!(doc.unmarked_facts.len(), 2);
}

#[test]
fn list_items_are_units_and_count_individually() {
    let text = "# H {#h}\n\n##lead The lead line: @spec/done\n- ##a first fact @impl/done\n- second fact, unmarked\n  continuation of the second fact\n- ##c third fact @test/plan\n";
    let doc = parse_document("x.md", text);
    // lead + 3 items.
    assert_eq!(doc.fact_count, 4, "facts: {:#?}", doc.blocks);
    assert_eq!(doc.unmarked_facts.len(), 1);
    let (bi, fi) = doc.unmarked_facts[0];
    let f = &doc.blocks[bi].facts[fi];
    assert_eq!(f.kind, FactKind::Item);
    assert!(f.id.is_none());
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.granularity == Granularity::Item)
            .count(),
        2
    );
}

#[test]
fn table_body_cells_are_units_header_and_delimiter_are_not() {
    let text = "# H {#h}\n\n| A | B |\n|---|---|\n| ##r1 left @impl/done | right |\n";
    let doc = parse_document("x.md", text);
    // 2 body cells; header/delimiter rows contribute nothing.
    assert_eq!(doc.fact_count, 2, "blocks: {:#?}", doc.blocks);
    assert_eq!(doc.unmarked_facts.len(), 1); // `right` is unmarked
    assert_eq!(
        doc.markers
            .iter()
            .filter(|m| m.granularity == Granularity::Cell)
            .count(),
        1
    );
}

#[test]
fn marked_unit_without_anchor_is_an_error() {
    let text = "# H {#h}\n\nMarked but anchor-less. @impl\n";
    let doc = parse_document("x.md", text);
    assert!(
        doc.issues
            .iter()
            .any(|i| i.code == IssueCode::MissingAnchor && i.severity == Severity::Error),
        "issues: {:#?}",
        doc.issues
    );
}

#[test]
fn duplicate_fact_id_is_an_error() {
    let text = "# H {#h}\n\n##dup One. @impl\n\n##dup Two. @impl\n";
    let doc = parse_document("x.md", text);
    assert!(
        doc.issues
            .iter()
            .any(|i| i.code == IssueCode::DuplicateId && i.severity == Severity::Error),
        "issues: {:#?}",
        doc.issues
    );
}

#[test]
fn fact_id_collides_with_heading_anchor() {
    let text = "# H {#h}\n\n##h Same as the heading anchor. @impl\n";
    let doc = parse_document("x.md", text);
    assert!(doc.issues.iter().any(|i| i.code == IssueCode::DuplicateId));
}

#[test]
fn marker_may_follow_the_fact_anchor_as_first_token() {
    let text = "# H {#h}\n\n##p1 @impl/done The marker sits right after the anchor.\n";
    let doc = parse_document("x.md", text);
    assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
    assert_eq!(doc.unmarked_facts.len(), 0);
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
