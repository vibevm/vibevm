//! Pure line-classification primitives for the markdown spec-unit scanner —
//! the syntactic helper layer the unit-building pass in `mdspec.rs` composes.
//! Split out so the main file stays within the file-length budget (the same
//! split as `mdspec/tests.rs`). None of these touch a domain type; they
//! classify raw markdown lines: heading vs fence vs list item vs `##<ID>`
//! fact anchor.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use specmark_grammar::is_valid_fact_id;

/// A heading line: 1–6 `#`, a space, text, trailing `{#anchor}`.
pub(crate) fn parse_heading(line: &str) -> Option<(usize, String, String)> {
    let trimmed = line.trim_end();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim_start();
    let open = rest.rfind("{#")?;
    if !rest.ends_with('}') {
        return None;
    }
    let anchor = &rest[open + 2..rest.len() - 1];
    let heading = rest[..open].trim_end().to_string();
    Some((hashes, heading, anchor.to_string()))
}

/// Any heading line (anchored or not) — unit spans end at these.
pub(crate) fn heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_end();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    trimmed[hashes..].starts_with(' ').then_some(hashes)
}

/// Per-line "inside a fenced code block" mask. A line whose trimmed
/// start is ``` or ~~~ toggles the fence; heading detection is
/// suppressed inside fences so worked examples in guides do not leak
/// into the unit inventory.
pub(crate) fn fence_mask(lines: &[&str]) -> Vec<bool> {
    let mut mask = Vec::with_capacity(lines.len());
    let mut in_fence = false;
    for line in lines {
        let t = line.trim_start();
        let is_boundary = t.starts_with("```") || t.starts_with("~~~");
        if is_boundary {
            // The boundary line itself counts as fenced content.
            mask.push(true);
            in_fence = !in_fence;
        } else {
            mask.push(in_fence);
        }
    }
    mask
}

/// Byte offset of a list item's content when the line opens one
/// (`- ` / `* ` / `+ ` / `N. ` / `N) ` at any indent), else `None`.
///
/// A list item is where the finest fact grain lives: a `##<ID>` written as
/// the item's first token mints its own unit (PROP-014 §2.1). This mirrors
/// — without sharing code (PROP-014 §2.9 separability; the convention is
/// held by tests on both sides) — the host Progress-Control scanner's list
/// recognition.
pub(crate) fn list_item_content(line: &str) -> Option<usize> {
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

/// A fact anchor at `line[start..]` (leading whitespace skipped): the opener,
/// a valid fact id, then whitespace or end-of-line. Returns the id and the
/// trimmed remainder of the line — the fact's lead text, kept as the unit
/// heading.
///
/// Two openers are accepted, and they mean exactly the same thing: the
/// qualified `@fact:<ID>`, which names its own key, and the legacy `##<ID>`.
/// A corpus is rewritten from one into the other in a single pass, so a
/// reader that knows only one of them would drop every unit it does not
/// recognise — which is precisely what a map is least able to survive.
///
/// An opener followed by an invalid id — a non-letter head (`##9bad`) or an
/// id run glued to a non-space glyph (`##bad!`) — is ordinary prose: `None`,
/// and (unlike a malformed heading anchor) no warning (PROP-014 §2.1). The
/// id charset is [`is_valid_fact_id`], which a heading anchor now takes too —
/// one grammar, two grains; only the reaction to a bad name differs, prose
/// here and a warning there.
pub(crate) fn fact_anchor_at(line: &str, start: usize) -> Option<(String, String)> {
    let seg = &line[start..];
    let lead_ws = seg.len() - seg.trim_start().len();
    let body = &seg[lead_ws..];
    let rest = body
        .strip_prefix("@fact:")
        .or_else(|| body.strip_prefix("##"))?;
    let id_len = rest
        .chars()
        .take_while(|&c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        .count();
    if id_len == 0 {
        return None;
    }
    let id = &rest[..id_len];
    let after = &rest[id_len..];
    if !is_valid_fact_id(id) || after.chars().next().is_some_and(|c| !c.is_whitespace()) {
        return None;
    }
    Some((id.to_string(), after.trim().to_string()))
}
