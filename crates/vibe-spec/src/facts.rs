//! Fact-leaf recognition (PROP-035 §5, the fact amendment).
//!
//! A `##<ID>` written as the **first token** of a paragraph or of a list item
//! (marker `-` / `*` / `+` / `N.` / `N)` at any indent) is a *fact unit* — the
//! finest grain of the document IR (PROP-014 §2.1). This module segments a
//! **text block** (a maximal run of non-blank, non-heading, non-fenced lines)
//! into its fact-carrying segments and returns the ones whose lead token is a
//! valid `##<ID>`.
//!
//! It mirrors — deliberately **without** sharing code across the separability
//! seam (PROP-035 §4; the convention is held by tests on both sides) — the host
//! Progress-Control scanner (`progress-core::parse::facts`) and the package twin
//! (`core-ai-native-specmap::mdspec`): the same list-marker set, the same
//! `[A-Za-z][A-Za-z0-9_-]*` id grammar, the same whitespace/EOL terminator, the
//! same lead-paragraph-then-items segmentation.

/// One fact segment found in a text block: the id and the block-relative line
/// range `[start, end)` it spans (lead paragraph, or an item plus its
/// continuation lines).
pub(crate) struct FactSegment {
    pub id: String,
    pub start: usize,
    pub end: usize,
}

/// Byte offset of a list item's content when the line opens one
/// (`- ` / `* ` / `+ ` / `N. ` / `N) ` at any indent), else `None`. The finest
/// fact grain lives here: a `##<ID>` first token *after the marker* mints a unit.
fn list_item_content(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    for pre in ["- ", "* ", "+ "] {
        if rest.starts_with(pre) {
            return Some(indent + pre.len());
        }
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if (1..=9).contains(&digits) {
        let after = &rest[digits..];
        if after.starts_with(". ") || after.starts_with(") ") {
            return Some(indent + digits + 2);
        }
    }
    None
}

/// A `##<ID>` fact anchor at `line[start..]` (leading whitespace skipped): `##`,
/// then a valid fact id `[A-Za-z][A-Za-z0-9_-]*`, then whitespace or end of
/// line. Returns the id.
///
/// `##` followed by an invalid id — a non-letter head (`##9bad`), an id glued to
/// a non-space glyph (`##bad!`), or a bare `##`/`###` — is ordinary prose:
/// `None`, and (unlike a malformed heading anchor) no warning (PROP-035 §5).
fn fact_id_at(line: &str, start: usize) -> Option<String> {
    let seg = line.get(start..)?;
    let lead_ws = seg.len() - seg.trim_start().len();
    let rest = seg[lead_ws..].strip_prefix("##")?;
    let id_len = rest
        .chars()
        .take_while(|&c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        .count();
    if id_len == 0 {
        return None;
    }
    let id = &rest[..id_len];
    let head_is_letter = id.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    let terminated = rest[id_len..]
        .chars()
        .next()
        .is_none_or(|c| c.is_whitespace());
    if head_is_letter && terminated {
        Some(id.to_string())
    } else {
        None
    }
}

/// Segment a text block into its `##<ID>` fact units. `lines` is the whole
/// document; the block is `lines[start..end)` (already known to hold no blank,
/// heading, or fenced line). Line ranges in the returned segments are
/// **block-relative** (`0` = `lines[start]`).
///
/// The lead paragraph (plain lines before the first list item) is one segment;
/// each list item plus its continuation lines (plain lines up to the next item)
/// is another. A segment contributes a fact only when its lead token is a valid
/// `##<ID>`.
pub(crate) fn segment_block(lines: &[String], start: usize, end: usize) -> Vec<FactSegment> {
    let len = end - start;
    // `Some(off)` — a list item opens here (content byte offset); `None` — a
    // plain line (paragraph line or an item's continuation).
    let markers: Vec<Option<usize>> = (start..end).map(|k| list_item_content(&lines[k])).collect();

    // Each segment: (anchoring block-line, marker offset on it, [lo, hi)).
    let mut segments: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut k = 0;
    // Lead paragraph: the plain lines before the first list item.
    if matches!(markers.first(), Some(None)) {
        let mut e = 0;
        while e + 1 < len && markers[e + 1].is_none() {
            e += 1;
        }
        segments.push((0, 0, 0, e + 1));
        k = e + 1;
    }
    // Every later segment opens on a list item; the following plain lines are
    // its continuation, up to the next item.
    while k < len {
        let Some(off) = markers[k] else { break };
        let mut e = k;
        while e + 1 < len && markers[e + 1].is_none() {
            e += 1;
        }
        segments.push((k, off, k, e + 1));
        k = e + 1;
    }

    let mut out = Vec::new();
    for (anchor_line, marker_off, lo, hi) in segments {
        if let Some(id) = fact_id_at(&lines[start + anchor_line], marker_off) {
            out.push(FactSegment {
                id,
                start: lo,
                end: hi,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(src: &str) -> Vec<String> {
        src.lines().map(String::from).collect()
    }

    #[test]
    fn paragraph_fact_is_recognised() {
        let l = block("##FACT-A the lead statement");
        let f = segment_block(&l, 0, l.len());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "FACT-A");
        assert_eq!((f[0].start, f[0].end), (0, 1));
    }

    #[test]
    fn lead_paragraph_spans_its_continuation_lines() {
        let l = block("##FACT-A first line\nsecond line of the same para");
        let f = segment_block(&l, 0, l.len());
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (0, 2));
    }

    #[test]
    fn list_item_facts_at_any_indent() {
        let l = block("- ##ONE first item\n  - ##TWO nested item\n* ##THREE star item");
        let f = segment_block(&l, 0, l.len());
        let ids: Vec<&str> = f.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["ONE", "TWO", "THREE"]);
        // Each item is its own unit, one line each.
        assert_eq!((f[0].start, f[0].end), (0, 1));
        assert_eq!((f[1].start, f[1].end), (1, 2));
    }

    #[test]
    fn numbered_markers_open_items() {
        let l = block("1. ##N-ONE\n2) ##N-TWO");
        let ids: Vec<String> = segment_block(&l, 0, l.len())
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, ["N-ONE", "N-TWO"]);
    }

    #[test]
    fn item_carries_its_continuation() {
        let l = block("- ##A first\n  more of A\n- ##B");
        let f = segment_block(&l, 0, l.len());
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].start, f[0].end), (0, 2)); // A + its continuation
        assert_eq!((f[1].start, f[1].end), (2, 3));
    }

    #[test]
    fn invalid_ids_are_prose() {
        // non-letter head, glued punctuation, bare ##, and a triple-# are prose.
        let l = block("##9bad here\n- ##bad! there\n##\n###ID also");
        assert!(segment_block(&l, 0, l.len()).is_empty());
    }

    #[test]
    fn underscore_and_case_are_valid_id_chars() {
        let l = block("##Mixed_Case-1 statement");
        let f = segment_block(&l, 0, l.len());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].id, "Mixed_Case-1");
    }

    #[test]
    fn a_plain_paragraph_after_items_is_not_a_lead() {
        // Only a lead paragraph (before the first item) is a paragraph fact; a
        // plain line trailing an item is that item's continuation, not its own.
        let l = block("- ##ITEM only\ntrailing continuation");
        let f = segment_block(&l, 0, l.len());
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (0, 2));
    }
}
