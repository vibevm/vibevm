//! Phase 4 — marker scanning: placement law, granularity, and issues.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use super::facts::take_fact_id;
use crate::doc::{Block, BlockKind, FactKind, Issue, IssueCode, ParsedDoc, Severity};
use crate::element::{self, DecodedAttrs};
use crate::model::{Granularity, Marker, MarkerForm};

/// Scan every block for markers, assign granularity by position, collect
/// issues, and compute the unmarked-fact list.
pub(super) fn scan_markers(doc: &mut ParsedDoc) {
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
