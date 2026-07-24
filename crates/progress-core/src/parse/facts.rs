//! Phase 3 — fact segmentation (paragraphs, lead lines, list items, table cells).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{BlockKind, Fact, FactKind, ParsedDoc};

/// Byte offset of a list item's content when the line opens one
/// (`- ` / `* ` / `+ ` / `N. ` / `N) `), else None.
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

/// True when every cell of the table row is a `---`/`:--:`-style rule.
fn is_delimiter_row(cells: &[(usize, usize)], text: &str) -> bool {
    !cells.is_empty()
        && cells.iter().all(|&(s, e)| {
            let t = text[s..e].trim();
            !t.is_empty() && t.chars().all(|c| c == '-' || c == ':') && t.chars().any(|c| c == '-')
        })
}

/// Byte spans of the cells of one `|`-delimited row (segments between
/// bars, plus a leading/trailing bar-less segment when non-empty).
fn row_cells(text: &str, s: usize, e: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let bars: Vec<usize> = (s..e).filter(|&i| bytes[i] == b'|').collect();
    let mut out = Vec::new();
    if let Some(&first) = bars.first() {
        if !text[s..first].trim().is_empty() {
            out.push((s, first));
        }
        for w in bars.windows(2) {
            out.push((w[0] + 1, w[1]));
        }
        if let Some(&last) = bars.last()
            && !text[last + 1..e].trim().is_empty()
        {
            out.push((last + 1, e));
        }
    }
    out
}

/// The `##<ID>` fact anchor at the start of a span: `(id, content_start)`
/// where `content_start` is the byte just past the id (for the marker
/// position law). No id ⇒ content_start == span start.
pub(super) fn take_fact_id(text: &str, s: usize, e: usize) -> (Option<String>, usize) {
    let seg = &text[s..e];
    let lead_ws = seg.len() - seg.trim_start().len();
    let t = &seg[lead_ws..];
    if let Some(rest) = t.strip_prefix("##") {
        let id_len = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .count();
        if id_len > 0
            && rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && rest[id_len..]
                .chars()
                .next()
                .is_none_or(|c| c.is_whitespace())
        {
            let id = rest[..id_len].to_string();
            return (Some(id), s + lead_ws + 2 + id_len);
        }
    }
    (None, s)
}

/// Segment every Text block into countable facts (PROP-043 §8):
/// a plain paragraph, the lead lines before the first list item, each
/// list item (with its continuation lines), each non-empty table body
/// cell. Header + delimiter rows of a table are structure, not facts.
pub(super) fn segment_facts(doc: &mut ParsedDoc) {
    for b in &mut doc.blocks {
        if b.kind != BlockKind::Text {
            continue;
        }
        let text = b.scan_text.clone();
        // Line starts (byte offsets) inside the block text.
        let mut offs: Vec<usize> = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                offs.push(i + 1);
            }
        }
        let n = offs.len();
        let span_of = |li: usize| -> (usize, usize) {
            let s = offs[li];
            let e = if li + 1 < n {
                offs[li + 1] - 1
            } else {
                text.len()
            };
            (s, e)
        };

        #[derive(Clone, Copy, PartialEq)]
        enum L {
            Item(usize),
            Table,
            Plain,
        }
        let classes: Vec<L> = (0..n)
            .map(|li| {
                let (s, e) = span_of(li);
                let line = &text[s..e];
                if line.trim_start().starts_with('|') {
                    L::Table
                } else if let Some(off) = list_item_content(line) {
                    L::Item(s + off)
                } else {
                    L::Plain
                }
            })
            .collect();

        let has_structure = classes.iter().any(|c| !matches!(c, L::Plain));
        if !has_structure {
            b.facts
                .push(mk_fact(FactKind::Para, &text, 0, text.len(), b.line_start));
            continue;
        }

        // Table delimiter/header rows to skip (structure, not facts).
        let mut structural: Vec<bool> = vec![false; n];
        for li in 0..n {
            if classes[li] == L::Table {
                let (s, e) = span_of(li);
                let cells = row_cells(&text, s, e);
                if is_delimiter_row(&cells, &text) {
                    structural[li] = true;
                    if li > 0 && classes[li - 1] == L::Table {
                        structural[li - 1] = true; // the header row
                    }
                }
            }
        }

        let mut li = 0usize;
        // Lead: plain lines before the first item/table line.
        if classes[0] == L::Plain {
            let mut end_li = 0;
            while end_li + 1 < n && classes[end_li + 1] == L::Plain {
                end_li += 1;
            }
            let (s, _) = span_of(0);
            let (_, e) = span_of(end_li);
            b.facts
                .push(mk_fact(FactKind::Lead, &text, s, e, b.line_start));
            li = end_li + 1;
        }
        while li < n {
            match classes[li] {
                L::Item(content) => {
                    // The item runs until the next item/table line.
                    let mut end_li = li;
                    while end_li + 1 < n && classes[end_li + 1] == L::Plain {
                        end_li += 1;
                    }
                    let (_, e) = span_of(end_li);
                    b.facts.push(mk_fact(
                        FactKind::Item,
                        &text,
                        content,
                        e,
                        b.line_start + li,
                    ));
                    li = end_li + 1;
                }
                L::Table => {
                    if !structural[li] {
                        let (s, e) = span_of(li);
                        for (cs, ce) in row_cells(&text, s, e) {
                            if !text[cs..ce].trim().is_empty() {
                                b.facts.push(mk_fact(
                                    FactKind::Cell,
                                    &text,
                                    cs,
                                    ce,
                                    b.line_start + li,
                                ));
                            }
                        }
                    }
                    li += 1;
                }
                L::Plain => {
                    // Plain lines after a table with no items: their own unit.
                    let start = li;
                    while li + 1 < n && classes[li + 1] == L::Plain {
                        li += 1;
                    }
                    let (s, _) = span_of(start);
                    let (_, e) = span_of(li);
                    b.facts
                        .push(mk_fact(FactKind::Para, &text, s, e, b.line_start + start));
                    li += 1;
                }
            }
        }
    }
}

fn mk_fact(kind: FactKind, text: &str, s: usize, e: usize, line: usize) -> Fact {
    // Cells may carry an id too (first cell of a row ⇒ the row's address,
    // any other cell ⇒ that cell's — §3.8 table addressing); only the
    // anchored-when-marked obligation exempts cells.
    let (id, _) = take_fact_id(text, s, e);
    Fact {
        kind,
        id,
        line,
        span: (s, e),
        marked: false,
    }
}
