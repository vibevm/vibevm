//! The canonical Markdown span emitter — the XML side's hashed body.
//!
//! A unit's `contentHash` is measured over the canonical Markdown projection
//! of its span (the house form PROP-045's Markdown backend defines), NOT the
//! raw XML bytes: raw XML hashes would move on every re-indentation, while
//! the projection is deterministic and — for a canonically-spelled Markdown
//! twin — byte-equal to what `mdspec` sees over that twin. That is the
//! measured verdict of the XML wave's S4b: the Markdown corpus keeps its
//! untouched raw-span hashes (zero churn), the XML corpus hashes its own
//! canonical reading, and a document pair in the two forms yields ONE hash
//! whenever the Markdown form is the canonical spelling — pinned by the
//! parity test.
//!
//! This module mirrors the pivot crate's Markdown backend byte for byte at
//! the spacing level (blocks separated by one blank line, a section's body
//! closed by one blank line, fences run-aware, the status element's fixed
//! attribute order). It shares no code with it — PROP-014 §2.9 separability;
//! the parity test is the convention-holder on this side. Both the
//! whole-document emission and each unit's individual span go through the
//! SAME line builders here, so a unit's hash input and its rendering inside
//! the parent span can never drift apart.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use super::{XBlock, XDoc, XSection, XStatus, XUnit};

/// The whole document's canonical projection, as lines — the H1 title unit's
/// span (an anchored `<title id=…>` spans the entire document, exactly as an
/// anchored H1 does in Markdown).
pub(super) fn document_lines(doc: &XDoc) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(t) = &doc.title {
        let mut line = format!("# {}", t.text);
        if let Some(id) = &t.id {
            line.push_str(&format!(" {{#{id}}}"));
        }
        out.push(line);
        out.push(String::new());
    }
    if let Some(s) = &doc.status {
        out.push(status_element_line(s));
        out.push(String::new());
    }
    append_blocks(&doc.preamble, &mut out);
    for s in &doc.sections {
        append_section(s, 2, &mut out);
    }
    out
}

/// One section's canonical span lines: heading, optional status, blocks,
/// then the nested sections (a Markdown span runs to the next same-or-higher
/// heading, so children belong to the parent's span).
pub(super) fn section_lines(s: &XSection, level: usize) -> Vec<String> {
    let mut out = Vec::new();
    append_section(s, level, &mut out);
    out
}

fn append_section(s: &XSection, level: usize, out: &mut Vec<String>) {
    let mut line = "#".repeat(level);
    line.push(' ');
    line.push_str(&s.title);
    if let Some(id) = &s.id {
        line.push_str(&format!(" {{#{id}}}"));
    }
    out.push(line);
    out.push(String::new());
    if let Some(st) = &s.status {
        out.push(status_element_line(st));
        out.push(String::new());
    }
    append_blocks(&s.blocks, out);
    for sub in &s.sections {
        append_section(sub, level + 1, out);
    }
}

/// Blocks separated by one blank line, the run closed by one blank line
/// (only when there is at least one block).
fn append_blocks(blocks: &[XBlock], out: &mut Vec<String>) {
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        append_block(b, i, blocks, out);
    }
    if !blocks.is_empty() {
        out.push(String::new());
    }
}

fn append_block(b: &XBlock, i: usize, blocks: &[XBlock], out: &mut Vec<String>) {
    let typed = typed_id_after(blocks, i);
    match b {
        XBlock::Para { unit, .. } => {
            push_lines(&para_line(unit, typed), out);
        }
        XBlock::Quote(unit) => {
            let mut inner = Vec::new();
            push_lines(&para_line(unit, typed), &mut inner);
            for line in inner {
                if line.is_empty() {
                    out.push(">".to_string());
                } else {
                    out.push(format!("> {line}"));
                }
            }
        }
        XBlock::Fence { lang, text, .. } => {
            let run = fence_run_for(text);
            let mut open = run.clone();
            if let Some(l) = lang {
                open.push_str(l);
            }
            out.push(open);
            if !text.is_empty() {
                // The canonical form closes the content with its own line
                // break — so a text already ending in one leaves a blank
                // line before the closer. Mirror that byte exactly.
                let padded = format!("{text}\n");
                out.extend(padded.lines().map(str::to_string));
            }
            out.push(run);
        }
        XBlock::List { ordered, items } => {
            for (j, item) in items.iter().enumerate() {
                let line = item_line(item, j, *ordered, typed);
                push_lines(&line, out);
            }
        }
        XBlock::Table { rows } => {
            for (ri, row) in rows.iter().enumerate() {
                if ri == 1 {
                    // The delimiter row after the header — plain `---` cells
                    // (alignment is not modelled).
                    let width = rows.first().map(|r| r.len()).unwrap_or(1);
                    let mut del = String::from("| ");
                    for _ in 0..width.saturating_sub(1) {
                        del.push_str("--- | ");
                    }
                    del.push_str("--- |");
                    out.push(del);
                }
                let mut line = String::from("|");
                for cell in row {
                    line.push(' ');
                    line.push_str(&para_line(cell, None));
                    line.push_str(" |");
                }
                out.push(line);
            }
        }
    }
}

/// A paragraph unit's canonical line:
/// `[@fact[/code]:ID ]text[ status]` joined with single spaces (so an empty
/// body still reads `@fact:X @status:s/s`).
pub(super) fn para_line(u: &XUnit, typed: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(id) = fact_prefix(u, typed) {
        parts.push(id);
    }
    if !u.text.is_empty() {
        parts.push(u.text.clone());
    }
    if let Some(st) = u.fact.as_ref().and_then(|f| f.status.as_ref()) {
        parts.push(status_suffix(st));
    }
    parts.join(" ")
}

/// A list item's canonical line: the marker, a GFM task box if any, then the
/// unit body (fact anchor included).
pub(super) fn item_line(u: &XUnit, index: usize, ordered: bool, typed: Option<&str>) -> String {
    let marker = if ordered {
        format!("{}. ", index + 1)
    } else {
        "- ".to_string()
    };
    let (box_prefix, rest) = split_task_box(&u.text);
    let mut parts: Vec<String> = Vec::new();
    if let Some(id) = fact_prefix(u, typed) {
        parts.push(id);
    }
    if !rest.is_empty() {
        parts.push(rest.to_string());
    }
    if let Some(st) = u.fact.as_ref().and_then(|f| f.status.as_ref()) {
        parts.push(status_suffix(st));
    }
    format!("{marker}{box_prefix}{}", parts.join(" "))
}

/// The text that follows the anchor on the canonical line — the fact unit's
/// `heading`, what the Markdown scanner reads back: `[text][ status]`.
pub(super) fn unit_body(u: &XUnit) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !u.text.is_empty() {
        parts.push(u.text.clone());
    }
    if let Some(st) = u.fact.as_ref().and_then(|f| f.status.as_ref()) {
        parts.push(status_suffix(st));
    }
    parts.join(" ")
}

/// The fact-anchor prefix, honouring the `@fact/code:` typed spelling.
fn fact_prefix(u: &XUnit, typed: Option<&str>) -> Option<String> {
    let f = u.fact.as_ref().filter(|f| f.is_meaningful())?;
    let id = f.id.as_ref()?;
    let typed = f.id.as_deref().is_some_and(|id| Some(id) == typed);
    Some(if typed {
        format!("@fact/code:{id}")
    } else {
        format!("@fact:{id}")
    })
}

/// The `@fact/code` look-ahead: the id the block AFTER `i` binds its fence
/// to — the typed spelling applies to that fact alone.
pub(super) fn typed_id_after(blocks: &[XBlock], i: usize) -> Option<&str> {
    match blocks.get(i + 1) {
        Some(XBlock::Fence { fact: Some(id), .. }) => Some(id.as_str()),
        _ => None,
    }
}

/// Split a multi-line string into pushed lines.
fn push_lines(joined: &str, out: &mut Vec<String>) {
    out.extend(joined.lines().map(str::to_string));
}

/// The status suffix: the qualified shorthand when the payload is just
/// stage/state, the point element when it carries more.
pub(super) fn status_suffix(st: &XStatus) -> String {
    if st.action.is_none()
        && st.actionstage.is_none()
        && st.audience.is_empty()
        && st.comment.is_none()
        && st.r#ref.is_none()
    {
        format!("@status:{}/{}", st.stage, st.state)
    } else {
        status_element_line(st)
    }
}

/// The `<status …/>` point element, Markdown spelling (fixed attribute
/// order: stage, state, action, actionstage, audience, comment, ref).
pub(super) fn status_element_line(s: &XStatus) -> String {
    let mut out = format!("<status stage=\"{}\" state=\"{}\"", s.stage, s.state);
    if let Some(a) = &s.action {
        out.push_str(&format!(" action=\"{a}\""));
    }
    if let Some(a) = &s.actionstage {
        out.push_str(&format!(" actionstage=\"{a}\""));
    }
    if !s.audience.is_empty() {
        out.push_str(&format!(" audience=\"{}\"", s.audience.join(",")));
    }
    if let Some(c) = &s.comment {
        out.push_str(&format!(" comment=\"{c}\""));
    }
    if let Some(r) = &s.r#ref {
        out.push_str(&format!(" ref=\"{r}\""));
    }
    out.push_str("/>");
    out
}

/// The fence run that safely wraps `text`: at least three backticks, one
/// more than the longest run inside it (run-matching — a quoted fence stays
/// quoted).
fn fence_run_for(text: &str) -> String {
    let longest = text
        .lines()
        .map(|l| {
            let b = l.trim_start().as_bytes();
            let mut best = 0usize;
            let mut i = 0usize;
            while i < b.len() {
                if b[i] == b'`' {
                    let start = i;
                    while i < b.len() && b[i] == b'`' {
                        i += 1;
                    }
                    best = best.max(i - start);
                } else {
                    i += 1;
                }
            }
            best
        })
        .max()
        .unwrap_or(0);
    "`".repeat(3.max(longest + 1))
}

/// Split a leading GFM task box (`[ ] `, `[x] `, `[X] `) off an item's
/// text; the box re-attaches before the fact anchor on emission.
pub(super) fn split_task_box(text: &str) -> (&str, &str) {
    let b = text.as_bytes();
    if b.len() >= 3 && b[0] == b'[' && b[2] == b']' && matches!(b[1], b' ' | b'x' | b'X') {
        let mut i = 3;
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
            i += 1;
        }
        // `[ ]glued` is prose, not a box — require the spacing or the end of
        // the text, exactly like the scanner's box grammar.
        if i > 3 || b.len() == 3 {
            return (&text[..i], &text[i..]);
        }
    }
    ("", text)
}
