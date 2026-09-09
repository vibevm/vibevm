//! Phase 4 — marker scanning: placement law, granularity, and issues.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#parsing");

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
                extract_from_span(doc, b, span, span.0, *gran, true, None);
            }
            BlockKind::Text => {
                for (fi, f) in b.facts.iter().enumerate() {
                    let gran = match f.kind {
                        FactKind::Item => Granularity::Item,
                        FactKind::Cell => Granularity::Cell,
                        FactKind::Para | FactKind::Lead => Granularity::Paragraph,
                    };
                    let (_, content_start) = take_fact_id(&b.scan_text, f.span.0, f.span.1);
                    let (had, marker_index) = extract_from_span(
                        doc,
                        b,
                        f.span,
                        content_start,
                        gran,
                        false,
                        f.id.as_deref(),
                    );
                    if had {
                        doc.blocks[i].facts[fi].marked = true;
                        doc.blocks[i].facts[fi].marker_index = marker_index;
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
    owner: Option<&str>,
) -> (bool, Option<usize>) {
    let text = &b.scan_text;
    let (s, e) = span;
    let mut found_any = false;
    let mut whole_unit_marker = None;
    let mut i = s;
    let bytes = text.as_bytes();
    while i < e {
        if text[i..e].starts_with("<status") {
            if let Some(scan_el) = element::lex_element(text, i) {
                let line = b.line_start + text[..i].matches('\n').count();
                let Some(el) = element::lex_element(&b.source_text, i) else {
                    doc.issues.push(Issue {
                        severity: Severity::Error,
                        line,
                        code: IssueCode::Malformed,
                        message:
                            "status marker found by the scanner cannot be lexed from its raw source"
                                .into(),
                    });
                    i += scan_el.tag_len;
                    continue;
                };
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
                    let marker_index = doc.markers.len();
                    doc.markers.push(m);
                    if gran_here == gran && gran_here != Granularity::Fragment {
                        flag_duplicate_whole_unit(doc, whole_unit_marker, marker_index, owner);
                        whole_unit_marker = Some(marker_index);
                    }
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
                let marker_index = doc.markers.len();
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
                flag_duplicate_whole_unit(doc, whole_unit_marker, marker_index, owner);
                whole_unit_marker = Some(marker_index);
                found_any = true;
                i += sh.len;
                continue;
            }
        }
        // Advance one char.
        let step = text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        i += step;
    }
    (found_any, whole_unit_marker)
}

fn flag_duplicate_whole_unit(
    doc: &mut ParsedDoc,
    first: Option<usize>,
    second: usize,
    owner: Option<&str>,
) {
    let Some(first) = first else { return };
    let owner = owner
        .map(|id| format!("fact `{id}`"))
        .unwrap_or_else(|| "unit".into());
    doc.issues.push(Issue {
        severity: Severity::Error,
        line: doc.markers[second].line,
        code: IssueCode::DuplicateStatus,
        message: format!(
            "second whole-unit status for {owner} (first at line {})",
            doc.markers[first].line
        ),
    });
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

#[cfg(test)]
mod tests {
    use super::super::parse_document;
    use crate::model::Granularity;

    #[test]
    fn point_marker_attributes_keep_inline_code_verbatim() {
        let source = "# H {#h}\n\n\
                      <status stage=\"impl\" state=\"work\" \
                      comment=\"shipped with `471e3b1b`\" ref=\"`release-proof`\"/>\n";
        let doc = parse_document("x.md", source);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        let marker = doc.document_marker().expect("document marker");
        assert_eq!(marker.comment.as_deref(), Some("shipped with `471e3b1b`"));
        assert_eq!(marker.r#ref.as_deref(), Some("`release-proof`"));
    }

    #[test]
    fn same_line_cells_keep_exact_marker_indices_through_serde() {
        let source = "| A | B |\n|---|---|\n| @fact:A one @requires:external <status stage=\"impl\" state=\"done\" ref=\"left\"/> | @fact:B two @requires:external <status stage=\"test\" state=\"done\" ref=\"right\"/> |\n";
        let doc = parse_document("x.md", source);
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        let facts: Vec<_> = doc.blocks.iter().flat_map(|block| &block.facts).collect();
        let left = facts[0].marker_index.expect("left marker");
        let right = facts[1].marker_index.expect("right marker");
        assert_ne!(left, right, "same physical line still has two owners");
        assert_eq!(doc.markers[left].r#ref.as_deref(), Some("left"));
        assert_eq!(doc.markers[right].r#ref.as_deref(), Some("right"));

        let json = serde_json::to_string(&doc).expect("serialize parse sidecar");
        let decoded: crate::doc::ParsedDoc =
            serde_json::from_str(&json).expect("decode parse sidecar");
        let facts: Vec<_> = decoded
            .blocks
            .iter()
            .flat_map(|block| &block.facts)
            .collect();
        assert_eq!(
            decoded.markers[facts[0].marker_index.unwrap()].r#ref,
            Some("left".into())
        );
        assert_eq!(
            decoded.markers[facts[1].marker_index.unwrap()].r#ref,
            Some("right".into())
        );
    }

    #[test]
    fn two_whole_unit_statuses_are_duplicate_with_or_without_requirements() {
        for source in [
            "@fact:A @spec/done body @impl/done\n",
            "@fact:A @spec/done body @requires:specification @impl/done\n",
        ] {
            let doc = parse_document("x.md", source);
            assert!(
                doc.issues.iter().any(|issue| {
                    issue.code == crate::doc::IssueCode::DuplicateStatus
                        && issue.message.contains("fact `A`")
                        && issue.message.contains("first at line")
                }),
                "{source}: {:#?}",
                doc.issues
            );
        }
    }

    #[test]
    fn fragment_wrapper_plus_one_whole_unit_status_remains_legal() {
        let doc = parse_document(
            "x.md",
            "@fact:A <status stage=\"spec\" state=\"done\">part</status> rest @requires:specification @impl/done\n",
        );
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        let fact = &doc.blocks[0].facts[0];
        assert_eq!(
            doc.markers[fact.marker_index.unwrap()].granularity,
            Granularity::Paragraph
        );
    }
}
