//! Phase 3b — the swallowed-anchor check.
//!
//! A fact anchor is the FIRST token of its paragraph: `@fact:<ID>`,
//! `@fact/<type>:<ID>`, or the legacy `##<ID>`. When two anchored facts sit on
//! neighbouring lines with NO blank line between them, Markdown folds them into
//! a single paragraph — only the first keeps its address; the second's anchor
//! becomes body text of the first. Its marker still parses, its prose reads
//! identically, `vibe progress check` stays silent, yet no verdict can ever
//! bind to the swallowed anchor again, because it no longer has an address.
//!
//! This phase walks each fact's body, line by line, and flags any further
//! anchor found at a line start as an [`IssueCode::SwallowedAnchor`]. It reuses
//! [`crate::parse::facts::take_fact_id`] — an existing entry point over the ONE
//! anchor grammar reader ([`facts::parse_anchor`]) — rather than a second
//! recogniser, so a fact's own anchor and a swallowed one can never drift apart
//! into two slightly different grammars (the failure mode `parse_anchor`'s
//! doc-comment warns this markup has already paid for three times).
//!
//! [`facts::parse_anchor`]: crate::parse::facts::parse_anchor

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{BlockKind, Issue, IssueCode, ParsedDoc, Severity};
use crate::parse::facts::take_fact_id;

/// Detect fact anchors swallowed into another fact's body.
///
/// For every fact of every `Text` block, scan the lines *after* its own anchor
/// line: if any of them opens with a fact anchor, that anchor has lost its
/// address (the two facts parse as one paragraph because no blank line
/// separates them) and is reported as [`IssueCode::SwallowedAnchor`].
///
/// **Line start.** A swallowed anchor is recognised by the same rule that
/// recognises a fact's own first token: leading whitespace and a blockquote
/// `>` prefix are skipped, then [`take_fact_id`] is asked for an anchor. A list
/// marker (`- `) needs no special case here, because a line that opens one is
/// its own countable unit — segmentation never leaves it inside another fact's
/// body — so the two positions share one grammar instead of two.
///
/// **Placement.** Immediately after [`facts::segment_facts`] and before
/// [`facts::bind_covered_blocks`]. The check reads only the segmented fact
/// spans (`Fact::span`, `Fact::line`, `Fact::id`) and the block's `scan_text`;
/// it consumes nothing `bind_covered_blocks`, the marker scan, or the anchor
/// laws produce, and nothing they produce reads it back. It therefore stands
/// with the other per-fact body inspections, ahead of the cross-block
/// typed-anchor binding.
///
/// [`facts::segment_facts`]: crate::parse::facts::segment_facts
/// [`facts::bind_covered_blocks`]: crate::parse::facts::bind_covered_blocks
pub(super) fn check_swallowed_anchors(doc: &mut ParsedDoc) {
    let mut issues: Vec<Issue> = Vec::new();

    for b in &doc.blocks {
        if b.kind != BlockKind::Text {
            continue;
        }
        let scan = &b.scan_text;
        for f in &b.facts {
            // Byte offsets (into `scan`) of each line start inside this fact's
            // body. Line 0 carries the fact's own anchor — already recorded as
            // `f.id` — so a swallowed anchor can only live on a later line.
            let body_s = f.span.0;
            let body_e = f.span.1;
            let mut line_starts: Vec<usize> = vec![body_s];
            for (i, ch) in scan[body_s..body_e].char_indices() {
                if ch == '\n' {
                    line_starts.push(body_s + i + 1);
                }
            }

            for k in 1..line_starts.len() {
                let ls = line_starts[k];
                let le = if k + 1 < line_starts.len() {
                    line_starts[k + 1] - 1 // stop just before this line's '\n'
                } else {
                    body_e
                };
                // Reuse the one anchor reader: an anchor at this line start is
                // a fact that lost its address to the paragraph above it.
                let (Some(swallowed), _) = take_fact_id(scan, ls, le) else {
                    continue;
                };
                let line = f.line + k; // body line k is `f.line + k` in source
                let own = match &f.id {
                    Some(id) => format!("fact `{id}`"),
                    None => "an unanchored paragraph".to_string(),
                };
                issues.push(Issue {
                    severity: Severity::Error,
                    line,
                    code: IssueCode::SwallowedAnchor,
                    message: format!(
                        "line {line}: {own} swallows a second fact anchor `{swallowed}` — \
                         with no blank line between them they parse as one paragraph, so \
                         `{swallowed}` became body text and lost its address (its marker \
                         still stands, but no verdict can bind to it). \
                         Fix: insert a blank line before `{swallowed}`."
                    ),
                });
            }
        }
    }

    doc.issues.extend(issues);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::IssueCode;
    use crate::parse::parse_document;

    /// Every `SwallowedAnchor` issue the document produced.
    fn swallowed(doc: &ParsedDoc) -> Vec<&Issue> {
        doc.issues
            .iter()
            .filter(|i| i.code == IssueCode::SwallowedAnchor)
            .collect()
    }

    /// Id of every fact the document produced, in document order.
    fn fact_ids(doc: &ParsedDoc) -> Vec<String> {
        doc.blocks
            .iter()
            .flat_map(|b| b.facts.iter().filter_map(|f| f.id.clone()))
            .collect()
    }

    // ---- ПРОВЕРЬ-4: the verbatim form from the task brief is caught ----------

    /// The exact three-line shape from the brief: three anchored facts on
    /// neighbouring lines, no blank line between them. They fold into ONE
    /// paragraph, so only `rename-note` keeps an address — the other two are
    /// swallowed. The check must name both, and must name the fix.
    #[test]
    fn verbatim_section1_form_is_caught() {
        let text = "# Doc {#doc}\n\n\
             @fact:rename-note **Terminology…** some text @status:doc/done\n\
             @fact:status-line **Status:** record. @status:doc/done\n\
             @fact:authority-line **Authority:** contract. @status:doc/done\n";
        let doc = parse_document("design.md", text);

        // The swallowing itself: only the first anchor survives as an address.
        assert_eq!(fact_ids(&doc), ["rename-note"], "blocks: {:#?}", doc.blocks);

        let issues = swallowed(&doc);
        assert_eq!(issues.len(), 2, "issues: {:#?}", doc.issues);
        let names: Vec<usize> = issues.iter().map(|i| i.line).collect();
        assert_eq!(names, [4, 5], "lines of the swallowed anchors");
        for i in &issues {
            assert!(
                i.message.contains("blank line"),
                "names the fix: {}",
                i.message
            );
            assert!(i.severity == Severity::Error);
        }
        assert!(
            issues.iter().any(|i| i.message.contains("`status-line`")),
            "names status-line: {:?}",
            issues
        );
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("`authority-line`")),
            "names authority-line: {:?}",
            issues
        );
    }

    /// The lost address is invisible to EVERY other check. Strip the markers
    /// and the only issues left are these — proving the defect passed silently
    /// before this phase existed.
    #[test]
    fn the_lost_address_is_silent_without_this_check() {
        let text = "# Doc {#doc}\n\n\
             @fact:rename-note A terminology note.\n\
             @fact:status-line A status record.\n\
             @fact:authority-line The contract.\n";
        let doc = parse_document("design.md", text);

        assert_eq!(fact_ids(&doc), ["rename-note"], "two addresses were lost");
        assert!(!doc.issues.is_empty(), "the check catches them");
        assert!(
            doc.issues
                .iter()
                .all(|i| i.code == IssueCode::SwallowedAnchor),
            "no other check notices the lost address: {:#?}",
            doc.issues
        );
        assert_eq!(swallowed(&doc).len(), 2);
    }

    // ---- ПРОВЕРЬ-5: the five false positives that must NOT fire -------------

    /// 2.2.1 — a mention of `@fact:` MID-LINE (not at a line start) is prose.
    #[test]
    fn mid_line_mention_is_not_swallowed() {
        let text = "# Doc {#doc}\n\n\
             @fact:A The grammar reads the key @fact:not-an-anchor mid-sentence,\n\
             and continues on the next line with @fact:also-not one too.\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["A"]);
        assert!(swallowed(&doc).is_empty(), "issues: {:#?}", doc.issues);
    }

    /// 2.2.2 — a mention of `@fact:` INSIDE inline code is blanked from the
    /// scan before any anchor is looked for.
    #[test]
    fn mention_inside_inline_code_is_not_swallowed() {
        let text = "# Doc {#doc}\n\n\
             @fact:A The shorthand `@fact:example` is blanked from the scan.\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["A"]);
        assert!(swallowed(&doc).is_empty(), "issues: {:#?}", doc.issues);
    }

    /// 2.2.3 — a list where every item is its OWN fact (each opens with `- `)
    /// is three separate anchored units, not one fact with swallowed anchors.
    #[test]
    fn separate_list_items_are_not_swallowed() {
        let text = "# Doc {#doc}\n\n\
             - @fact:A first item\n\
             - @fact:B second item\n\
             - @fact:C third item\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["A", "B", "C"], "blocks: {:#?}", doc.blocks);
        assert!(swallowed(&doc).is_empty(), "issues: {:#?}", doc.issues);
    }

    /// 2.2.4 — a table where each body cell is its OWN fact (a single-line
    /// span with no second line to swallow).
    #[test]
    fn separate_table_cells_are_not_swallowed() {
        let text = "# Doc {#doc}\n\n\
             | @fact:h1 | @fact:h2 |\n\
             |---|---|\n\
             | @fact:c1 | @fact:c2 |\n";
        let doc = parse_document("x.md", text);
        // Body-row cells are the facts (the header row is structure).
        assert_eq!(fact_ids(&doc), ["c1", "c2"], "blocks: {:#?}", doc.blocks);
        assert!(swallowed(&doc).is_empty(), "issues: {:#?}", doc.issues);
    }

    /// 2.2.5 — a Markdown heading `## Heading` (two hashes + a space) is NOT a
    /// fact anchor, and `parse_anchor` is what keeps them apart. The grammar
    /// level is checked directly (it is the mechanism this phase relies on),
    /// and a real heading block between two facts causes no false positive.
    #[test]
    fn heading_is_not_a_fact_anchor() {
        // Grammar level: a `## ` heading is rejected, a real `##<ID>` is read.
        assert_eq!(take_fact_id("## A heading", 0, 11).0, None);
        assert_eq!(take_fact_id("##  Two hashes", 0, 14).0, None);
        assert_eq!(take_fact_id("### Three", 0, 9).0, None);
        assert_eq!(
            take_fact_id("##AN-ID rest", 0, 12).0.as_deref(),
            Some("AN-ID")
        );

        // Integration: a heading is its own block, never a swallowed anchor.
        let text = "# Doc {#doc}\n\n\
             @fact:A a claim above.\n\n\
             ## A section heading\n\n\
             @fact:B a claim below.\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["A", "B"]);
        assert!(swallowed(&doc).is_empty(), "issues: {:#?}", doc.issues);
    }

    // ---- the two anchor spellings, and the blockquote prefix ---------------

    /// Both the qualified `@fact:<ID>` and the legacy `##<ID>` spellings are
    /// swallowed the same way: the one grammar reader serves both.
    #[test]
    fn both_anchor_spellings_are_caught() {
        let text = "# Doc {#doc}\n\n\
             @fact:FIRST qualified form\n\
             ##SECOND legacy form\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["FIRST"]);
        let issues = swallowed(&doc);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("`SECOND`"));
        assert_eq!(issues[0].line, 4); // 1 heading, 2 blank, 3 FIRST, 4 SECOND
    }

    /// A blockquote prefix is skipped just as it is for a fact's own anchor,
    /// so two quoted facts on neighbouring lines are still caught.
    #[test]
    fn blockquoted_swallowed_anchor_is_caught() {
        let text = "# Doc {#doc}\n\n\
             > @fact:QUOTE-A a quoted norm\n\
             > @fact:QUOTE-B another quoted norm\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["QUOTE-A"]);
        let issues = swallowed(&doc);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("`QUOTE-B`"));
    }

    /// A swallowed anchor on the continuation line of a list item is caught
    /// too — the item's body spans its soft-wrapped continuation lines.
    #[test]
    fn swallowed_anchor_on_a_list_continuation_is_caught() {
        let text = "# Doc {#doc}\n\n\
             - @fact:ITEM an item\n\
               @fact:SWALLOWED glued to the item's body\n";
        let doc = parse_document("x.md", text);
        assert_eq!(fact_ids(&doc), ["ITEM"]);
        let issues = swallowed(&doc);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("`SWALLOWED`"));
    }
}
