//! The fence-aware document scanner: lines → blocks → facts → markers.
//!
//! Placement semantics (PROP-043 §3.8, fact amendment): a standalone
//! marker is legal only in the preamble (document) or immediately after a
//! heading (section); inside a countable unit — paragraph, lead lines,
//! list item, table body cell — a marker must be the unit's first or last
//! token (the first token may follow the unit's `##<ID>` fact anchor); a
//! paired `<status>…</status>` wraps a fragment and counts for the unit
//! that carries it. A marked countable unit without a fact anchor is an
//! error (anchored-when-marked). Anything else is an issue, never a guess.
//!
//! The pipeline is split along its responsibility seams: run-matched
//! delimiters ([`delimiters`]), block collection ([`blocks`]),
//! heading/unit segmentation ([`units`]), fact segmentation ([`facts`]),
//! the swallowed-anchor check ([`swallowed`]), marker scanning ([`markers`]),
//! and the anchor laws ([`anchors`]). This module keeps the orchestrator and
//! the shared hash.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#parsing");

mod anchor_token;
mod anchors;
mod blocks;
mod delimiters;
mod facts;
mod markers;
mod swallowed;
mod units;

use crate::doc::ParsedDoc;
use anchors::check_anchor_laws;
use blocks::collect_blocks;
use facts::{bind_covered_blocks, segment_facts};
use markers::scan_markers;
use sha2::{Digest, Sha256};
use swallowed::check_swallowed_anchors;
use units::collect_units;

// The shared grammar readers re-exported for the vibe-specdoc adapter
// (PROP-045 S1: the pivot crate rebuilds block structure OVER this parse —
// it must not re-lex the anchor/table/list/quote/fence rules or the two
// crates drift into dialects). Every one of these is a pure function over
// a text span; opening them changes no behaviour here.
pub use anchor_token::take_fact_id;
pub use delimiters::{closes_fence, fence_run};
pub use facts::{
    blockquote_prefix_len, is_delimiter_row, list_marker_len, row_cells, task_box_len,
};

/// Parse one Markdown document.
pub fn parse_document(path: &str, text: &str) -> ParsedDoc {
    let mut doc = ParsedDoc {
        path: path.to_string(),
        content_hash: content_hash(text),
        ..ParsedDoc::default()
    };
    let lines: Vec<&str> = text.lines().collect();
    collect_blocks(&lines, &mut doc);
    collect_units(&lines, &mut doc);
    segment_facts(&mut doc);
    check_swallowed_anchors(&mut doc);
    bind_covered_blocks(&mut doc);
    scan_markers(&mut doc);
    check_anchor_laws(&mut doc, text);
    doc.fact_count = doc.blocks.iter().map(|b| b.facts.len()).sum();
    doc
}

/// Reduce both spellings of a marker to one canonical form, so that a text
/// which changed only its markup SPELLING hashes to the same value.
///
/// The canonical form is the LEGACY one, and that direction is deliberate:
/// every hash already recorded in a baseline or a campaign cache was computed
/// over legacy text, and canonicalising forward keeps all of them valid. A
/// corpus-wide rewrite of the spelling therefore disturbs no verdict, no seal
/// and no baseline — while any change to an id, a stage, a state or a single
/// word of prose still moves the hash exactly as before.
///
/// When the legacy spelling is finally retired, the canon flips and every
/// hash is recomputed once, deliberately, instead of drifting silently now.
fn canonical_markup(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains("@fact:") && !s.contains("@status:") {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(s.replace("@fact:", "##").replace("@status:", "@"))
}

/// The content identity of a text: sha256, lowercase hex (PROP-043 §7.1 —
/// "path, content-hash, …"). Also the baseline identity of a unit body.
///
/// The digest is taken over the text's [`canonical_markup`], not its raw
/// bytes: this number answers "has the content changed?", and a marker
/// rewritten from one legal spelling into the other has not changed content.
///
/// Public because a caller that wants to know whether a file changed must
/// ask *the same question the parser answers*: this is the number
/// [`ParsedDoc::content_hash`] carries and the number the cache's currency
/// test compares. One definition, so a cache hit can never stand for
/// something other than what the parse would have produced.
pub fn content_hash(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(canonical_markup(s).as_bytes());
    format!("{:x}", h.finalize())
}

/// Convenience for tests and callers that only need the counters.
pub fn quick_stats(doc: &ParsedDoc) -> (usize, usize, usize) {
    (doc.fact_count, doc.unmarked_facts.len(), doc.markers.len())
}

#[cfg(test)]
mod qualified_form_tests {
    use super::*;

    /// The qualified spellings must produce the SAME parse as the legacy ones
    /// — same ids, same counts, nothing left unmarked. This is the property a
    /// corpus-wide rewrite rests on: if the two forms ever disagree, the
    /// migration silently changes meaning instead of spelling.
    #[test]
    fn qualified_and_legacy_forms_parse_identically() {
        let legacy = "# One {#one}\n\n\
             ##a1 First claim. @impl/done\n\n\
             ##a2 Second claim. @spec/work\n";
        let qualified = "# One {#one}\n\n\
             @fact:a1 First claim. @status:impl/done\n\n\
             @fact:a2 Second claim. @status:spec/work\n";

        let l = parse_document("a.md", legacy);
        let q = parse_document("a.md", qualified);

        assert_eq!(quick_stats(&l).0, quick_stats(&q).0, "fact count");
        assert_eq!(quick_stats(&l).1, quick_stats(&q).1, "unmarked");
        assert_eq!(quick_stats(&l).2, quick_stats(&q).2, "markers");
        assert_eq!(quick_stats(&q).1, 0, "no fact left unmarked");

        let ids = |d: &ParsedDoc| -> Vec<String> {
            d.blocks
                .iter()
                .flat_map(|b| b.facts.iter().filter_map(|f| f.id.clone()))
                .collect()
        };
        assert_eq!(ids(&l), ids(&q), "fact ids must be identical");
        assert_eq!(ids(&q), vec!["a1".to_string(), "a2".to_string()]);
    }

    /// A heading is `## ` with a space and must stay a heading — the whole
    /// reason the legacy marker is written closed up. The rewrite must not be
    /// able to touch one.
    #[test]
    fn headings_are_untouched_by_either_form() {
        let d = parse_document(
            "a.md",
            "# Title {#t}\n\n## A real heading {#h}\n\n@fact:x A claim. @status:impl/done\n",
        );
        assert_eq!(quick_stats(&d).0, 1, "the heading is not a fact");
        assert_eq!(quick_stats(&d).1, 0, "and the one fact is marked");
    }

    /// The property the whole migration rests on: rewriting the SPELLING
    /// leaves every recorded hash valid, so no verdict comes due, no seal
    /// breaks and no baseline goes suspect.
    #[test]
    fn rewriting_the_spelling_does_not_move_the_hash() {
        let legacy = "# One {#one}\n\n\
             ##a1 First claim. @impl/done\n\n\
             ##a2 Second claim. @spec/work\n";
        let qualified = "# One {#one}\n\n\
             @fact:a1 First claim. @status:impl/done\n\n\
             @fact:a2 Second claim. @status:spec/work\n";

        assert_eq!(
            content_hash(legacy),
            content_hash(qualified),
            "a spelling change must not move the content identity"
        );

        let l = parse_document("a.md", legacy);
        let q = parse_document("a.md", qualified);
        assert_eq!(l.content_hash, q.content_hash, "document identity");
        assert_eq!(l.units.len(), q.units.len());
        for (lu, qu) in l.units.iter().zip(q.units.iter()) {
            assert_eq!(lu.content_hash, qu.content_hash, "unit identity");
        }
    }

    /// …and the normalisation must not blunt the instrument: a real edit
    /// still moves the hash, in every place a real edit can happen.
    #[test]
    fn a_real_edit_still_moves_the_hash() {
        let base = "@fact:a1 First claim. @status:impl/done\n";
        // prose changed
        assert_ne!(
            content_hash(base),
            content_hash("@fact:a1 First claim, reworded. @status:impl/done\n")
        );
        // id changed
        assert_ne!(
            content_hash(base),
            content_hash("@fact:a2 First claim. @status:impl/done\n")
        );
        // state changed
        assert_ne!(
            content_hash(base),
            content_hash("@fact:a1 First claim. @status:impl/work\n")
        );
        // stage changed
        assert_ne!(
            content_hash(base),
            content_hash("@fact:a1 First claim. @status:spec/done\n")
        );
    }

    // ---- `@fact/code:` — a fence becomes a fact's body ---------------------

    fn binding_issues(d: &ParsedDoc) -> Vec<&crate::doc::Issue> {
        d.issues
            .iter()
            .filter(|i| i.code == crate::doc::IssueCode::FenceBinding)
            .collect()
    }

    /// The point of the whole feature: a claim inside a fence stops belonging
    /// to nobody.
    #[test]
    fn a_typed_anchor_binds_the_fence_below_it() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n\
             @fact/code:PANEL the panel runs this @status:impl/done\n\
             ```bash\n\
             bash tools/self-check.sh\n\
             ```\n",
        );
        assert!(binding_issues(&d).is_empty(), "{:?}", d.issues);
        let f = d
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .find(|f| f.id.as_deref() == Some("PANEL"))
            .expect("fact");
        assert_eq!(f.covers, Some((4, 6)), "the fence's line range");
    }

    /// An untyped anchor covers only its own paragraph — the default the
    /// owner asked for: a fence is an example until someone says otherwise.
    #[test]
    fn an_untyped_anchor_covers_nothing() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n\
             @fact:PLAIN just prose @status:impl/done\n\
             ```bash\n\
             echo hi\n\
             ```\n",
        );
        assert!(binding_issues(&d).is_empty());
        let f = d
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .find(|f| f.id.as_deref() == Some("PLAIN"))
            .expect("fact");
        assert_eq!(f.covers, None);
    }

    /// An unimplemented type is an ERROR, not a silent skip. A grammar that
    /// ignores what it cannot do promises what it cannot do.
    #[test]
    fn an_unknown_type_is_an_error() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n@fact/image:PIC a picture @status:impl/done\n",
        );
        let issues = binding_issues(&d);
        assert_eq!(issues.len(), 1, "{:?}", d.issues);
        assert_eq!(issues[0].line, 3);
        assert_eq!(
            issues[0].message,
            "unknown fact type `image`; the one implemented type is `code`"
        );
    }

    /// A typed anchor with no fence after it names a body it does not have.
    #[test]
    fn a_typed_anchor_without_its_block_is_an_error() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n@fact/code:NOPE no fence follows @status:impl/done\n\nplain text\n",
        );
        let issues = binding_issues(&d);
        assert_eq!(issues.len(), 1, "{:?}", d.issues);
        assert_eq!(
            issues[0].message,
            "`@fact/code:NOPE` is not followed by a fenced block"
        );
    }

    /// Blank separator lines are layout, not intervening objects. The bound
    /// body is durable source, not merely a line-range hint: a renderer can
    /// print it and a verdict can stand against its own hash.
    #[test]
    fn blank_lines_are_allowed_and_the_fence_is_in_the_fact_body() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n\
             @fact/code:PANEL the panel runs this @status:impl/done\n\n\n\n\
             ```bash\n\
             cargo test -p progress-core\n\
             ```\n",
        );
        assert!(binding_issues(&d).is_empty(), "{:?}", d.issues);
        let fact = d
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .find(|f| f.id.as_deref() == Some("PANEL"))
            .expect("fact");
        assert!(fact.body.starts_with("@fact/code:PANEL"), "{}", fact.body);
        assert!(
            fact.body
                .contains("```bash\ncargo test -p progress-core\n```"),
            "{}",
            fact.body
        );
        assert_eq!(fact.content_hash, content_hash(&fact.body));
    }

    /// The fence is part of this fact's identity, not just the enclosing
    /// file's identity: changing one command makes this fact stale.
    #[test]
    fn editing_the_bound_fence_moves_the_fact_body_hash() {
        let parse = |command: &str| {
            let text = format!(
                "# T {{#t}}\n\n@fact/code:RUN execute this @status:impl/done\n\n```bash\n{command}\n```\n"
            );
            let d = parse_document("a.md", &text);
            let fact = d
                .blocks
                .iter()
                .flat_map(|b| &b.facts)
                .find(|f| f.id.as_deref() == Some("RUN"))
                .expect("fact");
            (fact.body.clone(), fact.content_hash.clone())
        };
        let (before_body, before_hash) = parse("cargo check");
        let (after_body, after_hash) = parse("cargo test");
        assert_ne!(before_body, after_body);
        assert_ne!(before_hash, after_hash);
    }

    /// Any meaningful block breaks adjacency, even when a fence appears
    /// later. The parser refuses the anchor at its own line instead of
    /// searching forward and silently claiming across an intervening object.
    #[test]
    fn another_block_between_the_fact_and_fence_is_an_error() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n\
             @fact/code:RUN execute this @status:impl/done\n\n\
             <!-- an intervening block -->\n\n\
             ```bash\ntrue\n```\n",
        );
        let issues = binding_issues(&d);
        assert_eq!(issues.len(), 1, "{:?}", d.issues);
        assert_eq!(issues[0].line, 3);
        assert_eq!(
            issues[0].message,
            "`@fact/code:RUN` is not followed by a fenced block"
        );
    }

    /// Binding changes ownership, not scanning: markers and definition-looking
    /// tokens inside the fence remain opaque while the source still prints as
    /// part of the owning fact's body.
    #[test]
    fn a_bound_fence_remains_opaque_to_marker_and_anchor_scans() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n\
             @fact/code:RUN execute this @status:impl/done\n\n\
             ```markdown\n\
             @fact:INSIDE example only @status:idea/plan\n\
             ```\n",
        );
        assert_eq!(d.error_count(), 0, "{:?}", d.issues);
        assert_eq!(d.markers.len(), 1, "{:?}", d.markers);
        let ids: Vec<&str> = d
            .blocks
            .iter()
            .flat_map(|b| b.facts.iter().filter_map(|f| f.id.as_deref()))
            .collect();
        assert_eq!(ids, ["RUN"]);
        let fact = d
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .find(|f| f.id.as_deref() == Some("RUN"))
            .expect("fact");
        assert!(fact.body.contains("@fact:INSIDE"));
    }

    /// The default remains an example: an ordinary fact's printable body and
    /// body hash stop at its own paragraph even when a fence follows.
    #[test]
    fn an_untyped_fact_body_still_excludes_the_following_fence() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n@fact:PLAIN prose @status:impl/done\n\n```text\nexample\n```\n",
        );
        let fact = d
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .find(|f| f.id.as_deref() == Some("PLAIN"))
            .expect("fact");
        assert_eq!(fact.body, "@fact:PLAIN prose @status:impl/done");
        assert_eq!(fact.content_hash, content_hash(&fact.body));
        assert!(!fact.body.contains("example"));
    }

    /// Fact bodies preserve source bytes even though marker scanning blanks
    /// inline code internally. Non-ASCII text is the offset-sensitive case.
    #[test]
    fn ordinary_fact_body_preserves_inline_code_verbatim() {
        let d = parse_document(
            "a.md",
            "# T {#t}\n\n@fact:TEXT Keep `привет @status:idea/plan` opaque. @status:impl/done\n",
        );
        assert_eq!(d.error_count(), 0, "{:?}", d.issues);
        assert_eq!(d.markers.len(), 1);
        let fact = d
            .blocks
            .iter()
            .flat_map(|b| &b.facts)
            .find(|f| f.id.as_deref() == Some("TEXT"))
            .expect("fact");
        assert_eq!(
            fact.body,
            "@fact:TEXT Keep `привет @status:idea/plan` opaque. @status:impl/done"
        );
    }

    /// Mixed spellings inside one document must both be read: during the
    /// rewrite a half-migrated file exists on disk, and it may not lose facts.
    #[test]
    fn a_half_migrated_document_keeps_every_fact() {
        let d = parse_document(
            "a.md",
            "# One {#one}\n\n\
             ##old Legacy spelling. @impl/done\n\n\
             @fact:new Qualified spelling. @status:impl/done\n",
        );
        assert_eq!(quick_stats(&d).0, 2);
        assert_eq!(quick_stats(&d).1, 0);
    }
}
