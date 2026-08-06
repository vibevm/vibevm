//! The fence-aware document scanner: lines → blocks → facts → markers.
//!
//! Placement semantics (PROP-043 §3.8, fact amendment): a standalone
//! marker is legal only in the preamble (document) or immediately after a
//! heading (section); inside a countable unit — paragraph, lead lines,
//! list item, table body cell — a marker must be the unit's first or last
//! token (the first token may follow the unit's `##<ID>` fact anchor); a
//! paired `<status>…</status>` wraps a fragment and counts for the unit
//! that carries it. A marked paragraph/item without a fact anchor is an
//! error (anchored-when-marked). Anything else is an issue, never a guess.
//!
//! The pipeline is split along its responsibility seams: run-matched
//! delimiters ([`delimiters`]), block collection ([`blocks`]),
//! heading/unit segmentation ([`units`]), fact segmentation ([`facts`]),
//! marker scanning ([`markers`]), and the anchor laws ([`anchors`]). This
//! module keeps the orchestrator and the shared hash.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#parsing");

mod anchors;
mod blocks;
mod delimiters;
mod facts;
mod markers;
mod units;

use crate::doc::ParsedDoc;
use anchors::check_anchor_laws;
use blocks::collect_blocks;
use facts::segment_facts;
use markers::scan_markers;
use sha2::{Digest, Sha256};
use units::collect_units;

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
    scan_markers(&mut doc);
    check_anchor_laws(&mut doc);
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
