//! Phase 0 — run-matched delimiters: what a backtick or tilde *run* opens
//! and what closes it, for both the block fence and the inline code span.
//!
//! The two used to live apart — the fence in the block state machine, the
//! span beside it — and they drifted: the span was rewritten to match by
//! run (F-084) and the fence was left matching by prefix, so a four-backtick
//! block quoting three-backtick ones was closed by its own content. They are
//! one cell now because they are one rule: **a delimiter is a run, and only
//! a run of at least the same width closes it.**

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use specmark::spec;

/// The fence run a line opens with — its character and how many of it —
/// or `None` when the line opens no fence at all.
///
/// The run **length** is what makes the scanner nestable. A block opened
/// with a longer run may hold shorter ones as content, which is how any
/// document that quotes fenced markdown has to be written:
///
/// `````text
/// ````markdown
/// ```
/// a command the quoted document fences
/// ```
/// ````
/// `````
///
/// (That example needs a five-backtick wrapper for exactly the reason it
/// documents: rustdoc's own scanner is run-matched, so a four-backtick
/// wrapper is closed by the four-backtick line inside it and the prose
/// below compiles as a doctest.)
///
/// Matching on the `` ``` `` *prefix* alone reads that inner opener as the
/// outer block's closer, and everything after it inverts: the code between
/// the inner fences becomes prose the exhaustive gate demands a marker for,
/// while the prose after them becomes code it ignores. The demand cannot be
/// satisfied — `##FENCE-AWARE` means a marker written there is not read as
/// one, and writing it would edit a skeleton consumers copy verbatim.
pub(super) fn fence_run(trimmed: &str) -> Option<(char, usize)> {
    let ch = trimmed.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|&c| c == ch).count();
    (len >= 3).then_some((ch, len))
}

/// Whether `trimmed` closes a fence of character `ch` opened at run length
/// `open`: the same character, a run at least as long, and nothing else on
/// the line — an info string makes a line an opener, never a closer.
pub(super) fn closes_fence(trimmed: &str, ch: char, open: usize) -> bool {
    fence_run(trimmed).is_some_and(|(c, n)| c == ch && n >= open)
        && trimmed.trim_end().chars().all(|c| c == ch)
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
pub(super) fn blank_inline_code(s: &str) -> String {
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

    /// Ids of every fact the document produced, in document order.
    fn fact_ids(doc: &crate::doc::ParsedDoc) -> Vec<String> {
        doc.blocks
            .iter()
            .flat_map(|b| b.facts.iter().filter_map(|f| f.id.clone()))
            .collect()
    }

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
        assert_eq!(fact_ids(&doc), ["U-1"], "blocks: {:#?}", doc.blocks);
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
        assert_eq!(fact_ids(&doc), ["U"], "blocks: {:#?}", doc.blocks);
    }

    /// The shape every document that quotes fenced markdown has to use: a
    /// four-backtick block holding three-backtick ones. The inner opener is
    /// content, so the code between the inner fences stays code and the
    /// whole outer block owes exactly nothing.
    ///
    /// Matched by prefix, the inner opener closed the outer block and the
    /// parse inverted: `acme init` became a paragraph the exhaustive gate
    /// demanded a marker for, while the `**Expected.**` prose after it
    /// became code the gate could not see. Eleven such units stood in
    /// `manual-tests`, and none of them could be satisfied — a marker
    /// written inside a fence is not read as one.
    #[test]
    fn a_longer_fence_is_not_closed_by_a_shorter_run() {
        let text = "# H {#h}\n\n\
                    ##LEAD The rule, stated once. @impl/done\n\n\
                    ````\n\
                    3. Initialise the project.\n\n\
                    \x20  ```\n\
                    \x20  acme init\n\
                    \x20  ```\n\n\
                    \x20  **Expected.** It exits 0.\n\
                    ````\n\n\
                    ##TAIL The prose that follows it. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(
            doc.unmarked_facts.len(),
            0,
            "the quoted block owes nothing: {:#?}",
            doc.blocks
        );
        assert_eq!(
            fact_ids(&doc),
            ["LEAD", "TAIL"],
            "blocks: {:#?}",
            doc.blocks
        );
    }

    /// The converse still holds, so the fix cannot strand a block: a fence
    /// opened with three backticks is closed by a longer run, exactly as it
    /// was before. An unclosed outer block would swallow the rest of the
    /// file, which is the failure mode worth a test of its own.
    #[test]
    fn a_shorter_fence_is_closed_by_a_longer_run() {
        let text = "# H {#h}\n\n\
                    ```\n\
                    code\n\
                    ````\n\n\
                    ##AFTER The prose after the block. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["AFTER"], "blocks: {:#?}", doc.blocks);
    }

    /// A tilde fence is not closed by backticks of any width, and the
    /// reverse likewise — the character has to match before the run does.
    #[test]
    fn a_fence_is_never_closed_by_the_other_fence_character() {
        let text = "# H {#h}\n\n\
                    ~~~\n\
                    ```\n\
                    still inside the tilde block\n\
                    ```\n\
                    ~~~\n\n\
                    ##AFTER The prose after the block. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["AFTER"], "blocks: {:#?}", doc.blocks);
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
