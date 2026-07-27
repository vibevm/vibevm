//! Phase 1 — the fence state machine and block grouping (lines → blocks).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use super::delimiters::{blank_inline_code, closes_fence, fence_run};
use crate::doc::{Block, BlockKind, ParsedDoc};
use crate::element;
use specmark::spec;

/// Fence state machine + block grouping.
pub(super) fn collect_blocks(lines: &[&str], doc: &mut ParsedDoc) {
    let mut in_fence: Option<(char, usize)> = None; // fence char + opening run
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

    // A leading YAML frontmatter block is structure, not a paragraph: emit
    // it whole and start the scan under it (see [`frontmatter_len`]).
    let fm = frontmatter_len(lines);
    if fm > 0 {
        doc.blocks.push(Block {
            kind: BlockKind::Comment,
            line_start: 1,
            line_end: fm,
            scan_text: blank_inline_code(&lines[..fm].join("\n")),
            facts: Vec::new(),
        });
    }

    for (idx, raw) in lines.iter().enumerate().skip(fm) {
        let lineno = idx + 1;
        let trimmed = raw.trim_start();

        if let Some((ch, open_len)) = in_fence {
            cur_text.push((*raw).to_string());
            if closes_fence(trimmed, ch, open_len) {
                in_fence = None;
                flush(&mut cur_start, &mut cur_kind, &mut cur_text, lineno, doc);
            }
            continue;
        }

        if let Some(open) = fence_run(trimmed) {
            // A fence opens: close any open text block first.
            flush(
                &mut cur_start,
                &mut cur_kind,
                &mut cur_text,
                lineno.saturating_sub(1),
                doc,
            );
            in_fence = Some(open);
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

/// A `---` fence line: the delimiter alone, at column 0. Trailing spaces
/// are invisible and tolerated; leading ones are not, because an indented
/// `---` is a line *inside* a YAML block scalar, not a fence.
fn is_frontmatter_fence(line: &str) -> bool {
    line.trim_end() == "---"
}

/// How many lines the document's leading `---`-delimited YAML frontmatter
/// occupies, or 0 when the document has none.
///
/// Frontmatter is **structure**, like the comment, the thematic break and
/// the fence this module already exempts: it is the metadata envelope the
/// tooling that loads the file consumes, not a paragraph anyone wrote. It
/// carries no blank line, so `##COUNTABLE-UNITS` sees one paragraph and
/// demands a marker for it; `##ANCHORED-WHEN-MARKED` then demands a
/// `##<ID>` as that unit's first token, and its first token is the opening
/// `---`, where no anchor can legally stand — YAML would stop parsing.
/// The two obligations are unsatisfiable together on this block, which is
/// what makes it structure and not prose.
///
/// The recogniser is as narrow as `facts::task_box_len`, and for the same
/// reason: accept the structure only where it cannot mean anything else.
///
/// * **Line 1 only.** A `---` further down keeps its present meaning — a
///   thematic break, or a setext underline inside a text block.
/// * **Column 0 only**, both fences (see [`is_frontmatter_fence`]).
/// * **The scan stops at a blank line.** A blank there means the leading
///   `---` opened nothing: it was a thematic break and what follows is
///   prose. Without that stop the scan resynchronises on the *next*
///   thematic break in the file and swallows every unit between the two —
///   F-084's failure exactly, a loss that reads like an absence.
///
/// A scan that finds no closer answers 0, and the document then parses
/// precisely as it did before this rule existed.
#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#granularity")]
#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#placement")]
fn frontmatter_len(lines: &[&str]) -> usize {
    if !lines.first().is_some_and(|l| is_frontmatter_fence(l)) {
        return 0;
    }
    for (idx, line) in lines.iter().enumerate().skip(1) {
        if line.trim().is_empty() {
            return 0;
        }
        if is_frontmatter_fence(line) {
            return idx + 1;
        }
    }
    0
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

    /// The shape of the nine `SKILL.md` files: frontmatter, the document
    /// marker, the heading, the prose. The envelope is one Comment block
    /// spanning its own four lines and owes no marker; everything under it
    /// parses exactly as it would without it.
    #[test]
    fn leading_frontmatter_is_structure_and_not_a_countable_unit() {
        let text = "---\nname: a-skill\ndescription: What it does.\n---\n\n\
                    <status stage=\"impl\" state=\"done\"/>\n\n\
                    # A skill {#root}\n\n\
                    ##DO-THE-THING You run the thing. @impl/done\n";
        let doc = parse_document("SKILL.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.blocks[0].kind, BlockKind::Comment);
        assert_eq!((doc.blocks[0].line_start, doc.blocks[0].line_end), (1, 4));
        assert_eq!(doc.fact_count, 1, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["DO-THE-THING"]);
    }

    /// A document that is *only* frontmatter has no units at all — closer
    /// on the last line, with or without the trailing newline.
    #[test]
    fn a_document_that_is_only_frontmatter_has_no_units() {
        for text in ["---\nname: a\n---\n", "---\nname: a\n---"] {
            let doc = parse_document("SKILL.md", text);
            assert_eq!(doc.error_count(), 0, "{text:?}: {:#?}", doc.issues);
            assert_eq!(doc.blocks.len(), 1, "{text:?}: {:#?}", doc.blocks);
            assert_eq!(doc.blocks[0].kind, BlockKind::Comment, "{text:?}");
            assert_eq!(doc.fact_count, 0, "{text:?}: {:#?}", doc.blocks);
        }
    }

    /// A closer glued to the heading under it still closes there: the
    /// heading is a heading, not the envelope's last line.
    #[test]
    fn a_closer_followed_immediately_by_a_heading_ends_the_envelope() {
        let text = "---\nname: a\n---\n# A skill {#root}\n\n\
                    ##DO-IT You run it. @impl/done\n";
        let doc = parse_document("SKILL.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(
            (doc.blocks[0].kind, doc.blocks[0].line_end),
            (BlockKind::Comment, 3)
        );
        assert_eq!(doc.blocks[1].kind, BlockKind::Heading);
        assert_eq!(doc.blocks[1].line_start, 4);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["DO-IT"]);
    }

    /// An unterminated leading `---` opens nothing: with no closer the
    /// document parses exactly as it did before the rule existed, and the
    /// text under the dashes keeps whatever status it always had.
    #[test]
    fn an_unterminated_leading_fence_changes_nothing() {
        // No closer anywhere: `---` + keys are one text block, one unit.
        let text = "---\nname: a\ndescription: no closer ever\n";
        let doc = parse_document("SKILL.md", text);
        assert_eq!(doc.blocks.len(), 1, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.blocks[0].kind, BlockKind::Text);
        assert_eq!(doc.unmarked_facts.len(), 1, "blocks: {:#?}", doc.blocks);
        // A heading on line 2 ends the block at the dashes, which are then
        // a thematic break — the pre-existing behaviour, unchanged.
        let text = "---\n# A doc {#root}\n\n##DO-IT You run it. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.blocks[0].kind, BlockKind::Comment);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
    }

    /// An indented `---` inside the envelope is a line of a YAML block
    /// scalar, not the closer: the envelope runs on to the real fence, and
    /// the keys after the lookalike do not spill out as a paragraph.
    #[test]
    fn an_indented_dash_line_inside_frontmatter_is_not_the_closer() {
        let text = "---\nname: a\nbody: |\n  ---\nmore: b\n---\n\n\
                    # A skill {#root}\n\n\
                    ##DO-IT You run it. @impl/done\n";
        let doc = parse_document("SKILL.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(
            (doc.blocks[0].kind, doc.blocks[0].line_end),
            (BlockKind::Comment, 6)
        );
        assert_eq!(doc.fact_count, 1, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
    }

    /// The rule fires at line 1 and nowhere else: a `---`-delimited block
    /// further down is whatever it was before — here a text block, and so
    /// a countable unit that nobody marked.
    #[test]
    fn a_dashed_block_below_line_one_is_not_frontmatter() {
        let text = "# A doc {#root}\n\n---\nname: a\n---\n\n\
                    ##DO-IT You run it. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.unmarked_facts.len(), 1, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["DO-IT"]);
    }

    /// NEGATIVE CONTROL. Three dashes, a blank line, prose: a thematic
    /// break at line 1 of a frontmatter-less document, which must keep
    /// parsing as a thematic break.
    ///
    /// The second case is the one that catches an unbounded scan: a second
    /// thematic break further down is not the first one's closer, and a
    /// scanner that resynchronises on it swallows `##UNIT-1` whole and
    /// reports the loss as nothing at all (F-084).
    #[test]
    fn a_line_one_thematic_break_stays_a_thematic_break() {
        let text = "---\n\n# A doc {#root}\n\n##UNIT-1 One unit. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.blocks[0].kind, BlockKind::Comment);
        assert_eq!((doc.blocks[0].line_start, doc.blocks[0].line_end), (1, 1));
        assert_eq!(doc.fact_count, 1, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["UNIT-1"]);

        let text = "---\n\n##UNIT-1 First unit. @impl/done\n\n---\n\n\
                    ##UNIT-2 Second unit. @impl/done\n";
        let doc = parse_document("x.md", text);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        assert_eq!(doc.fact_count, 2, "blocks: {:#?}", doc.blocks);
        assert_eq!(doc.unmarked_facts.len(), 0, "blocks: {:#?}", doc.blocks);
        assert_eq!(fact_ids(&doc), ["UNIT-1", "UNIT-2"]);
        assert_eq!(doc.markers.len(), 2, "markers: {:#?}", doc.markers);
    }
}
