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
//! The pipeline is split along its responsibility seams: block collection
//! ([`blocks`]), heading/unit segmentation ([`units`]), fact segmentation
//! ([`facts`]), marker scanning ([`markers`]), and the anchor laws
//! ([`anchors`]). This module keeps the orchestrator and the shared hash.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

mod anchors;
mod blocks;
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
        content_hash: hash_str(text),
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

fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())
}

/// Convenience for tests and callers that only need the counters.
pub fn quick_stats(doc: &ParsedDoc) -> (usize, usize, usize) {
    (doc.fact_count, doc.unmarked_facts.len(), doc.markers.len())
}
