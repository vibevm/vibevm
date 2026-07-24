//! Phase 1 — the fence state machine and block grouping (lines → blocks).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{Block, BlockKind, ParsedDoc};
use crate::element;

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

/// Blank out `` `inline code` `` spans so marker scanning never fires
/// inside them (positions are preserved — replacement is space-for-byte).
fn blank_inline_code(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_code = false;
    for c in s.chars() {
        if c == '`' {
            in_code = !in_code;
            out.push('`');
        } else if in_code {
            out.push(if c == '\n' { '\n' } else { ' ' });
        } else {
            out.push(c);
        }
    }
    out
}
