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

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{Block, BlockKind, Fact, FactKind, Issue, IssueCode, ParsedDoc, Severity, Unit};
use crate::element::{self, DecodedAttrs};
use crate::model::{Granularity, Marker, MarkerForm};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

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

/// Fence state machine + block grouping.
fn collect_blocks(lines: &[&str], doc: &mut ParsedDoc) {
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

/// Heading units per the body-span rule (heading → next same-or-higher).
fn collect_units(lines: &[&str], doc: &mut ParsedDoc) {
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
            content_hash: hash_str(&body),
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
fn take_fact_id(text: &str, s: usize, e: usize) -> (Option<String>, usize) {
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
fn segment_facts(doc: &mut ParsedDoc) {
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

/// Scan every block for markers, assign granularity by position, collect
/// issues, and compute the unmarked-fact list.
fn scan_markers(doc: &mut ParsedDoc) {
    let first_heading_line = doc
        .blocks
        .iter()
        .find(|b| b.kind == BlockKind::Heading)
        .map(|b| b.line_start);
    // Line numbers of headings, to recognize "immediately after a heading":
    // a MarkerOnly block whose preceding non-Comment block is a Heading.
    let mut prev_meaningful: Option<(usize, BlockKind)> = None; // (index, kind)
    let mut placements: Vec<(usize, Granularity)> = Vec::new(); // block idx → granularity
    for (i, b) in doc.blocks.iter().enumerate() {
        match b.kind {
            BlockKind::MarkerOnly => {
                let in_preamble = first_heading_line.map(|h| b.line_start < h).unwrap_or(true);
                // A file that opens with its H1 has no preamble; the
                // standalone right after that FIRST heading governs the
                // whole document (PROP-043 §3.8 items 1–2 combined).
                let after_first_heading = matches!(prev_meaningful, Some((0, BlockKind::Heading)));
                if in_preamble || after_first_heading {
                    placements.push((i, Granularity::Document));
                } else if matches!(prev_meaningful, Some((_, BlockKind::Heading))) {
                    placements.push((i, Granularity::Section));
                } else {
                    doc.issues.push(Issue {
                        severity: Severity::Error,
                        line: b.line_start,
                        code: IssueCode::Stranded,
                        message: "standalone marker between paragraphs — attach it inside \
                                  the unit (first/last token) or move it directly \
                                  under a heading"
                            .into(),
                    });
                }
                prev_meaningful = Some((i, b.kind));
            }
            BlockKind::Comment => { /* invisible to placement adjacency */ }
            _ => prev_meaningful = Some((i, b.kind)),
        }
    }

    let blocks = doc.blocks.clone();
    for (i, b) in blocks.iter().enumerate() {
        match b.kind {
            BlockKind::Code | BlockKind::Comment | BlockKind::Heading => continue,
            BlockKind::MarkerOnly => {
                let Some((_, gran)) = placements.iter().find(|(bi, _)| *bi == i) else {
                    continue; // stranded — already reported
                };
                let span = (0usize, b.scan_text.len());
                extract_from_span(doc, b, span, span.0, *gran, true);
            }
            BlockKind::Text => {
                for (fi, f) in b.facts.iter().enumerate() {
                    let gran = match f.kind {
                        FactKind::Item => Granularity::Item,
                        FactKind::Cell => Granularity::Cell,
                        FactKind::Para | FactKind::Lead => Granularity::Paragraph,
                    };
                    let (_, content_start) = take_fact_id(&b.scan_text, f.span.0, f.span.1);
                    let had = extract_from_span(doc, b, f.span, content_start, gran, false);
                    if had {
                        doc.blocks[i].facts[fi].marked = true;
                    } else {
                        doc.unmarked_facts.push((i, fi));
                    }
                }
            }
        }
    }

    // Duplicate status markers per granularity slot (document-level).
    let mut doc_markers = doc
        .markers
        .iter()
        .filter(|m| m.granularity == Granularity::Document);
    if let (Some(first), Some(second)) = (doc_markers.next(), doc_markers.next()) {
        doc.issues.push(Issue {
            severity: Severity::Error,
            line: second.line,
            code: IssueCode::DuplicateStatus,
            message: format!(
                "second document-level status marker (first at line {})",
                first.line
            ),
        });
    }
}

/// The anchored-when-marked law + one shared id namespace (PROP-043 §3.8):
/// a marked paragraph/lead/item needs a `##<ID>`; every id — fact or
/// heading anchor — is unique per document.
fn check_anchor_laws(doc: &mut ParsedDoc) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for u in &doc.units {
        if let Some(a) = &u.anchor {
            seen.entry(a.clone()).or_insert(u.line_start);
        }
    }
    let mut new_issues = Vec::new();
    for b in &doc.blocks {
        for f in &b.facts {
            if let Some(id) = &f.id {
                if let Some(&first) = seen.get(id) {
                    new_issues.push(Issue {
                        severity: Severity::Error,
                        line: f.line,
                        code: IssueCode::DuplicateId,
                        message: format!("fact id `##{id}` already minted at line {first}"),
                    });
                } else {
                    seen.insert(id.clone(), f.line);
                }
            }
            if f.marked && f.id.is_none() && !matches!(f.kind, FactKind::Cell) {
                new_issues.push(Issue {
                    severity: Severity::Error,
                    line: f.line,
                    code: IssueCode::MissingAnchor,
                    message: "marked unit has no `##<ID>` fact anchor \
                              (anchored-when-marked, PROP-043 §3.8)"
                        .into(),
                });
            }
        }
    }
    doc.issues.extend(new_issues);
}

/// Extract markers from one span of a block. Returns true when the span
/// carries at least one marker usable as the unit's own (first/last token
/// — the first may follow the unit's fact anchor — or a fragment wrapper
/// inside it). `standalone` spans skip the position test.
fn extract_from_span(
    doc: &mut ParsedDoc,
    b: &Block,
    span: (usize, usize),
    content_start: usize,
    gran: Granularity,
    standalone: bool,
) -> bool {
    let text = &b.scan_text;
    let (s, e) = span;
    let mut found_any = false;
    let mut i = s;
    let bytes = text.as_bytes();
    while i < e {
        if text[i..e].starts_with("<status") {
            if let Some(el) = element::lex_element(text, i) {
                let line = b.line_start + text[..i].matches('\n').count();
                let d = element::decode_attrs(&el.attrs);
                push_attr_issues(doc, &d, &el.errors, line);
                let form = if el.self_closing {
                    MarkerForm::Point
                } else {
                    MarkerForm::Wrapper
                };
                let mut gran_here = gran;
                if !el.self_closing {
                    gran_here = Granularity::Fragment;
                    // A wrapper must close within this block.
                    if !text[i + el.tag_len..].contains("</status>") {
                        doc.issues.push(Issue {
                            severity: Severity::Error,
                            line,
                            code: IssueCode::WrapperMismatch,
                            message: "wrapper <status …> never closed in this paragraph".into(),
                        });
                    }
                } else if !standalone {
                    // Point marker inside a unit: legal only as the
                    // unit's first (post-anchor) or last token.
                    let before_ok = text[content_start..i].trim().is_empty();
                    let after_ok = text[i + el.tag_len..e].trim().is_empty();
                    if !before_ok && !after_ok {
                        doc.issues.push(Issue {
                            severity: Severity::Error,
                            line,
                            code: IssueCode::MidParagraph,
                            message: "point marker mid-unit — move it to the \
                                      unit's first or last token"
                                .into(),
                        });
                    }
                }
                if let Some(m) = build_marker(&d, form, gran_here, line) {
                    doc.markers.push(m);
                    // A point/shorthand marks the unit; a wrapper inside
                    // the unit counts too (an inline fact is still a
                    // marked fact — §3.8 item 6).
                    found_any = true;
                } else {
                    let missing = if d.stage.is_none() { "stage" } else { "state" };
                    doc.issues.push(Issue {
                        severity: Severity::Error,
                        line,
                        code: IssueCode::MissingAttr,
                        message: format!("<status> is missing required `{missing}`"),
                    });
                }
                i += el.tag_len;
                continue;
            }
            // `<status` that does not lex: unclosed-on-span point tag.
            let line = b.line_start + text[..i].matches('\n').count();
            doc.issues.push(Issue {
                severity: Severity::Error,
                line,
                code: IssueCode::Malformed,
                message: "unterminated <status …> tag (point markers must be \
                          self-closing `/>`)"
                    .into(),
            });
            break;
        }
        if bytes[i] == b'@'
            && (i == s || !bytes[i - 1].is_ascii_alphanumeric())
            && let Some(sh) = element::lex_shorthand(text, i)
        {
            // Position law: standalone token at start (post-anchor) or
            // end of the unit's text.
            let before_ok = text[content_start.min(i)..i].trim().is_empty();
            let after_ok = text[i + sh.len..e].trim().is_empty();
            if standalone || before_ok || after_ok {
                let line = b.line_start + text[..i].matches('\n').count();
                doc.markers.push(Marker {
                    stage: sh.stage,
                    state: sh.state,
                    action: None,
                    actionstage: None,
                    audience: Vec::new(),
                    comment: None,
                    r#ref: None,
                    form: MarkerForm::Shorthand,
                    granularity: gran,
                    line,
                });
                found_any = true;
                i += sh.len;
                continue;
            }
        }
        // Advance one char.
        let step = text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        i += step;
    }
    found_any
}

fn push_attr_issues(doc: &mut ParsedDoc, d: &DecodedAttrs, lex_errors: &[String], line: usize) {
    for e in lex_errors {
        doc.issues.push(Issue {
            severity: Severity::Error,
            line,
            code: IssueCode::Malformed,
            message: e.clone(),
        });
    }
    for (attr, value, hint) in &d.violations {
        let msg = match hint {
            Some(h) => format!("unknown {attr} value `{value}` — did you mean `{h}`?"),
            None => format!("unknown attribute or value: {attr}=\"{value}\""),
        };
        doc.issues.push(Issue {
            severity: Severity::Error,
            line,
            code: IssueCode::Vocabulary,
            message: msg,
        });
    }
}

fn build_marker(
    d: &DecodedAttrs,
    form: MarkerForm,
    gran: Granularity,
    line: usize,
) -> Option<Marker> {
    Some(Marker {
        stage: d.stage?,
        state: d.state?,
        action: d.action,
        actionstage: d.actionstage,
        audience: d.audience.clone(),
        comment: d.comment.clone(),
        r#ref: d.r#ref.clone(),
        form,
        granularity: gran,
        line,
    })
}

/// Convenience for tests and callers that only need the counters.
pub fn quick_stats(doc: &ParsedDoc) -> (usize, usize, usize) {
    (doc.fact_count, doc.unmarked_facts.len(), doc.markers.len())
}
