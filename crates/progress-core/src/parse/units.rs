//! Phase 2 — heading/unit segmentation (the body-span rule).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#parsing");

use super::content_hash;
use crate::doc::{BlockKind, ParsedDoc, Unit};

/// Heading units per the body-span rule (heading → next same-or-higher).
pub(super) fn collect_units(lines: &[&str], doc: &mut ParsedDoc) {
    let heads: Vec<(usize, usize, String, Option<String>)> = doc
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::Heading)
        .map(|b| {
            let raw = b.scan_text.trim_start();
            let level = raw.chars().take_while(|c| *c == '#').count();
            let title_raw = raw[level..].trim();
            let (title, anchor) = split_anchor(title_raw);
            (b.line_start, level, title, anchor)
        })
        .collect();
    for (i, (start, level, title, anchor)) in heads.iter().enumerate() {
        let end = heads
            .iter()
            .skip(i + 1)
            .find(|(_, l2, _, _)| l2 <= level)
            .map(|(s2, _, _, _)| s2 - 1)
            .unwrap_or(lines.len());
        // trim_end: trailing blank lines before the next heading are
        // boundary noise and must not shift the unit's baseline identity.
        let body: String = lines[*start - 1..end.min(lines.len())]
            .join("\n")
            .trim_end()
            .to_string();
        doc.units.push(Unit {
            heading: title.clone(),
            level: *level,
            anchor: anchor.clone(),
            line_start: *start,
            line_end: end,
            content_hash: content_hash(&body),
        });
    }
}

fn split_anchor(title: &str) -> (String, Option<String>) {
    if let Some(open) = title.rfind("{#")
        && let Some(close_rel) = title[open..].find('}')
    {
        let anchor = title[open + 2..open + close_rel].trim().to_string();
        let clean = title[..open].trim().to_string();
        if !anchor.is_empty() {
            return (clean, Some(anchor));
        }
    }
    (title.trim().to_string(), None)
}
