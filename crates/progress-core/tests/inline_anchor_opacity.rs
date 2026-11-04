//! Quoted anchor spellings are content, while real definitions stay unique.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS");

use progress_core::{doc::IssueCode, parse::parse_document};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS"
)]
fn quoted_anchors_do_not_add_definitions() {
    for quote in [
        "`@fact:SAME`",
        "``@fact/code:SAME ` example``",
        "````@fact:SAME ``` example````",
    ] {
        let source = format!(
            "# H {{#h}}\n\n@fact:SAME definition\n\n@fact:OTHER see {quote}\n\n| Example |\n| --- |\n| @fact:CELL {quote} |\n"
        );
        let doc = parse_document("quoted.md", &source);
        assert_eq!(doc.error_count(), 0, "{quote}: {:?}", doc.issues);
        let ids: Vec<_> = doc
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .filter_map(|f| f.id.as_deref())
            .collect();
        assert_eq!(ids, ["SAME", "OTHER", "CELL"]);
        assert_eq!(doc.fact_count, 3);
        assert_eq!(
            doc.units
                .iter()
                .filter_map(|u| u.anchor.as_deref())
                .collect::<Vec<_>>(),
            ["h"]
        );
        assert!(doc.blocks.iter().any(|b| b.source_text.contains(quote)));
    }
}

#[test]
fn real_heading_fact_and_typed_fact_collisions_still_refuse() {
    for source in [
        "# H {#h}\n\n@fact:SAME one\n\n@fact:SAME two\n",
        "# H {#SAME}\n\n@fact:SAME definition\n",
        "# H {#h}\n\n@fact:SAME definition\n\n@fact/code:SAME code\n\n```text\nbody\n```\n",
    ] {
        let doc = parse_document("duplicate.md", source);
        let issues: Vec<_> = doc
            .issues
            .iter()
            .filter(|i| i.code == IssueCode::DuplicateId)
            .collect();
        assert_eq!(issues.len(), 1, "{:?}", doc.issues);
        assert!(issues[0].message.contains("lines"));
    }
}

#[test]
fn unmatched_backticks_do_not_hide_real_definitions_or_missing_anchors() {
    let doc = parse_document(
        "unmatched.md",
        "# H {#h}\n\n@fact:SAME stray ` tick\n\n@fact:SAME second definition\n",
    );
    assert!(doc.issues.iter().any(|i| i.code == IssueCode::DuplicateId));
    let doc = parse_document(
        "missing.md",
        "# H {#h}\n\nUnanchored `@fact:QUOTED` text @impl/done\n",
    );
    assert!(
        doc.issues
            .iter()
            .any(|i| i.code == IssueCode::MissingAnchor)
    );
    assert!(
        doc.blocks
            .iter()
            .flat_map(|b| &b.facts)
            .all(|f| f.id.is_none())
    );
}
