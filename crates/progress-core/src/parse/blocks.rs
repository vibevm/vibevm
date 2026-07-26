//! Phase 1 — the fence state machine and block grouping (lines → blocks).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{Block, BlockKind, ParsedDoc};
use crate::element;
use specmark::spec;

/// Fence state machine + block grouping.
pub(super) fn collect_blocks(lines: &[&str], doc: &mut ParsedDoc) {
    let mut in_fence: Option<&str> = None; // the fence marker (``` or ~~~)
    let mut cur_start: Option<usize> = None;
    let mut cur_kind = BlockKind::Text;
    let mut cur_text: Vec<String> = Vec::new();

    let flush = |start: &mut Option<usize>,
                 kind: &mut BlockKind,
                 text: &mut Vec<String>,
                 end_line: usize,
                 doc: &mut ParsedDoc| {
        if let Some(s) = start.take() {
            let joined = text.join("\n");
            doc.blocks.push(Block {
                kind: *kind,
                line_start: s,
                line_end: end_line,
                scan_text: blank_inline_code(&joined),
                facts: Vec::new(),
            });
            text.clear();
            *kind = BlockKind::Text;
        }
    };

    for (idx, raw) in lines.iter().enumerate() {
        let lineno = idx + 1;
        let trimmed = raw.trim_start();

        if let Some(marker) = in_fence {
            cur_text.push((*raw).to_string());
            if trimmed.starts_with(marker)
                && trimmed
                    .trim_end()
                    .chars()
                    .all(|c| c == marker.chars().next().unwrap_or('`'))
            {
                in_fence = None;
                flush(&mut cur_start, &mut cur_kind, &mut cur_text, lineno, doc);
            }
            continue;
        }

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            // A fence opens: close any open text block first.
            flush(
                &mut cur_start,
                &mut cur_kind,
                &mut cur_text,
                lineno.saturating_sub(1),
                doc,
            );
            in_fence = Some(if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            });
            cur_start = Some(lineno);
            cur_kind = BlockKind::Code;
            cur_text.push((*raw).to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush(
                &mut cur_start,
                &mut cur_kind,
                &mut cur_text,
                lineno.saturating_sub(1),
                doc,
            );
            continue;
        }

        if is_heading(trimmed) {
            flush(
                &mut cur_start,
                &mut cur_kind,
                &mut cur_text,
                lineno.saturating_sub(1),
                doc,
            );
            doc.blocks.push(Block {
                kind: BlockKind::Heading,
                line_start: lineno,
                line_end: lineno,
                scan_text: (*raw).to_string(),
                facts: Vec::new(),
            });
            continue;
        }

        if cur_start.is_none() {
            cur_start = Some(lineno);
            cur_kind = BlockKind::Text;
        }
        cur_text.push((*raw).to_string());
    }
    let last = lines.len();
    flush(&mut cur_start, &mut cur_kind, &mut cur_text, last, doc);

    // Reclassify special text blocks.
    for b in &mut doc.blocks {
        if b.kind != BlockKind::Text {
            continue;
        }
        let t = b.scan_text.trim();
        if is_comment_only(t) || is_thematic_break_only(t) {
            b.kind = BlockKind::Comment;
        } else if is_marker_only(t) {
            b.kind = BlockKind::MarkerOnly;
        }
    }
}

fn is_heading(trimmed: &str) -> bool {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes)
        && trimmed
            .chars()
            .nth(hashes)
            .map(|c| c == ' ')
            .unwrap_or(false)
}

fn is_comment_only(t: &str) -> bool {
    let mut rest = t.trim();
    loop {
        if !rest.starts_with("<!--") {
            return false;
        }
        match rest.find("-->") {
            Some(end) => {
                rest = rest[end + 3..].trim();
                if rest.is_empty() {
                    return true;
                }
            }
            None => return false,
        }
    }
}

/// Thematic breaks (`---`, `***`, `___`) are layout, not prose — exempt
/// from the exhaustiveness requirement like comments.
fn is_thematic_break_only(t: &str) -> bool {
    !t.is_empty()
        && t.lines().all(|l| {
            let l = l.trim();
            l.len() >= 3
                && (l.chars().all(|c| c == '-' || c == ' ')
                    || l.chars().all(|c| c == '*' || c == ' ')
                    || l.chars().all(|c| c == '_' || c == ' '))
                && l.chars().any(|c| c != ' ')
        })
}

/// True when the trimmed block text is exactly one point marker or one
/// bare shorthand token.
fn is_marker_only(t: &str) -> bool {
    if t.starts_with("<status") {
        if let Some(el) = element::lex_element(t, 0) {
            return el.self_closing && t[el.tag_len..].trim().is_empty();
        }
        return false;
    }
    if t.starts_with('@')
        && let Some(sh) = element::lex_shorthand(t, 0)
    {
        return t[sh.len..].trim().is_empty();
    }
    false
}

/// The backtick runs in `s`, as `(byte offset, run length)`.
///
/// A *run* — a maximal group of adjacent backticks — is the unit a code
/// span is delimited by, which is why the scanner counts runs rather than
/// individual backticks.
fn backtick_runs(s: &str) -> Vec<(usize, usize)> {
    let b = s.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i] == b'`' {
            i += 1;
        }
        runs.push((start, i - start));
    }
    runs
}

/// Blank out the **contents** of `` `inline code` `` spans so marker
/// scanning never fires inside them (one space per non-newline character,
/// so line structure and the span's own delimiters survive).
///
/// Spans are matched by backtick *run*, the way the code-span rule works:
/// a run of N backticks opens a span that only a run of exactly N closes,
/// and a run with no matching closer is literal text that opens nothing.
/// Two consequences the naive per-backtick toggle got wrong:
///
/// * a longer run **inside** a span — `` ` ```card-ops ` `` — is span
///   content, not a delimiter and not a fence opener, so it no longer
///   inverts the scanner's idea of where code is;
/// * an unpaired backtick is inert instead of swallowing the rest of the
///   block, so a stray tick can never hide the markers after it.
///
/// This does not relax the suppression itself (`##FENCE-AWARE`): what is
/// inside a span stays unscanned, and a fenced block never reaches here at
/// all. It only stops text *outside* every span from being mistaken for
/// span content, which is what makes a trailing shorthand
/// (`##SHORTHAND-STANDALONE`) visible on such a paragraph.
#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#element")]
#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#shorthand")]
fn blank_inline_code(s: &str) -> String {
    let runs = backtick_runs(s);
    let mut blanked = vec![false; s.len()];
    let mut r = 0usize;
    while r < runs.len() {
        let (open, len) = runs[r];
        match runs[r + 1..].iter().position(|&(_, l)| l == len) {
            Some(rel) => {
                let close = r + 1 + rel;
                for slot in &mut blanked[open + len..runs[close].0] {
                    *slot = true;
                }
                r = close + 1;
            }
            // No closer of the same width: literal backticks. Resume after
            // the run — a later run may still open a span of its own.
            None => r += 1,
        }
    }
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.char_indices() {
        if blanked[i] && c != '\n' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    /// A paragraph that quotes a fenced-code backtick inside an inline span
    /// still ends where the author says it ends, so the trailing shorthand
    /// is the unit's last token and marks it.
    #[test]
    fn trailing_marker_survives_a_quoted_triple_backtick() {
        let text = "# H {#h}\n\n\
                    ##CARD-OPS Authored as a fenced ` ```card-ops ` block of `key: value` \
                    fields. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.markers.len(), 1, "markers: {:#?}", doc.markers);
    }

    /// One, two and four backtick delimiters behave alike: whatever a span
    /// holds is content, and the marker after it is still the last token.
    #[test]
    fn any_delimiter_width_leaves_the_trailing_marker_visible() {
        for span in ["`a b`", "``a ` b``", "````a ``` b````", "`` ` ``"] {
            let text = format!("# H {{#h}}\n\n##U A unit quoting {span} here. @impl/done\n");
            let doc = parse_document("x.md", &text);
            assert_eq!(doc.error_count(), 0, "span {span}: {:#?}", doc.issues);
            assert_eq!(
                doc.unmarked_facts.len(),
                0,
                "span {span}: {:#?}",
                doc.blocks
            );
        }
    }

    /// An unterminated span is literal text, not an opener: it cannot eat
    /// the rest of the block, and the scanner cannot hang on it. The marker
    /// sits on the line *after* the stray tick, which is exactly the text
    /// the old toggle blanked.
    #[test]
    fn an_unterminated_code_span_swallows_nothing() {
        let text = "# H {#h}\n\n\
                    ##U-1 A stray ` tick opens nothing,\n\
                    and the marker ending the unit is still found. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.markers.len(), 1, "markers: {:#?}", doc.markers);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        let ids: Vec<&str> = doc
            .blocks
            .iter()
            .flat_map(|b| b.facts.iter().filter_map(|f| f.id.as_deref()))
            .collect();
        assert_eq!(ids, ["U-1"], "blocks: {:#?}", doc.blocks);
    }

    /// The suppression itself is untouched: a marker written inside a code
    /// span, or inside a fenced block, is still not a marker.
    #[test]
    fn markers_inside_code_spans_and_fences_stay_unrecognised() {
        let text = "# H {#h}\n\n\
                    ##U A unit naming `@impl/done` and `<status stage=\"idea\" state=\"plan\"/>` \
                    in code. @spec/done\n\n\
                    ```markdown\n\
                    @test/plan\n\
                    <status stage=\"idea\" state=\"plan\"/>\n\
                    ##NOT-AN-ID\n\
                    ```\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        // Exactly one marker: the paragraph's own trailing `@spec/done`.
        assert_eq!(doc.markers.len(), 1, "markers: {:#?}", doc.markers);
        assert_eq!(
            doc.blocks
                .iter()
                .flat_map(|b| b.facts.iter().filter_map(|f| f.id.as_deref()))
                .collect::<Vec<_>>(),
            ["U"],
            "blocks: {:#?}",
            doc.blocks
        );
    }

    /// Run matching, directly: contents blanked, delimiters kept, an
    /// unpaired run left alone, byte offsets of every kept char unmoved.
    #[test]
    fn code_span_contents_are_blanked_and_delimiters_kept() {
        assert_eq!(blank_inline_code("a `bc` d"), "a `  ` d");
        assert_eq!(blank_inline_code("a ` ``` ` d"), "a `     ` d");
        assert_eq!(blank_inline_code("a ``b`c`` d"), "a ``   `` d");
        assert_eq!(blank_inline_code("a ` b c"), "a ` b c");
        assert_eq!(blank_inline_code("a `b\nc` d"), "a ` \n ` d");
    }
}
