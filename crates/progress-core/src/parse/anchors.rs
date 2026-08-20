//! Phase 5 — the anchor laws (anchored-when-marked, unique ids).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#parsing");

use super::facts::qualified_fact_tokens;
use crate::doc::{BlockKind, FactKind, Issue, IssueCode, ParsedDoc, Severity};
use std::collections::HashMap;

#[derive(Debug)]
struct Definition {
    id: String,
    line: usize,
    column: usize,
}

/// The anchored-when-marked law + one shared id namespace (PROP-043 §3.8):
/// a marked paragraph/lead/item needs a `##<ID>`; every id — fact or
/// heading anchor — is unique per document.
pub(super) fn check_anchor_laws(doc: &mut ParsedDoc, text: &str) {
    let mut definitions = Vec::new();
    for u in &doc.units {
        if let Some(a) = &u.anchor {
            definitions.push(Definition {
                id: a.clone(),
                line: u.line_start,
                column: 0,
            });
        }
    }

    // A code-formatted citation must use the legacy `##ID` reference spelling.
    // If its contents use `@fact:ID`, the definition spelling, count it as a
    // definition too: matching a real id then produces the B-092 duplicate.
    // Compare raw text with the parser's inline-code-blanked scan rather than
    // lexing backticks again. Fenced/comment-only blocks are not Text blocks.
    let lines: Vec<&str> = text.lines().collect();
    for b in &doc.blocks {
        if b.kind != BlockKind::Text {
            continue;
        }
        let scan_lines: Vec<&str> = b.scan_text.lines().collect();
        for (offset, line_no) in (b.line_start..=b.line_end).enumerate() {
            let Some(raw) = lines.get(line_no - 1) else {
                continue;
            };
            let scan = scan_lines.get(offset).copied().unwrap_or_default();
            let mut visible: HashMap<String, usize> = HashMap::new();
            for (id, _, _) in qualified_fact_tokens(scan, 0, scan.len()) {
                *visible.entry(id).or_default() += 1;
            }
            for (id, column, _) in qualified_fact_tokens(raw, 0, raw.len()) {
                let remaining = visible.entry(id.clone()).or_default();
                if *remaining > 0 {
                    *remaining -= 1;
                    continue;
                }
                definitions.push(Definition {
                    id,
                    line: line_no,
                    column,
                });
            }
        }
    }

    // All actual definitions come from the parser, under either spelling.
    for b in &doc.blocks {
        for f in &b.facts {
            if let Some(id) = &f.id {
                definitions.push(Definition {
                    id: id.clone(),
                    line: f.line,
                    column: 0,
                });
            }
        }
    }

    definitions.sort_by_key(|d| (d.line, d.column));
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut new_issues = Vec::new();
    for definition in definitions {
        if let Some(&first) = seen.get(&definition.id) {
            new_issues.push(Issue {
                severity: Severity::Error,
                line: definition.line,
                code: IssueCode::DuplicateId,
                message: format!(
                    "fact id `@fact:{}` is defined twice in this file: lines {first} and {}",
                    definition.id, definition.line
                ),
            });
        } else {
            seen.insert(definition.id, definition.line);
        }
    }

    for b in &doc.blocks {
        for f in &b.facts {
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

#[cfg(test)]
mod tests {
    use crate::doc::{IssueCode, Severity};
    use crate::parse::parse_document;

    fn duplicates(doc: &crate::doc::ParsedDoc) -> Vec<&crate::doc::Issue> {
        doc.issues
            .iter()
            .filter(|i| i.code == IssueCode::DuplicateId)
            .collect()
    }

    #[test]
    fn duplicate_definition_reports_both_lines() {
        let doc = parse_document("x.md", "# H {#h}\n\n@fact:SAME one\n\n@fact:SAME two\n");
        let issues = duplicates(&doc);
        assert_eq!(issues.len(), 1, "{:#?}", doc.issues);
        assert_eq!(issues[0].severity, Severity::Error);
        assert!(issues[0].message.contains("lines 3 and 5"));
    }

    #[test]
    fn definition_form_used_as_inline_citation_is_a_duplicate() {
        let doc = parse_document(
            "x.md",
            "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `@fact:SAME`\n",
        );
        let issues = duplicates(&doc);
        assert_eq!(issues.len(), 1, "{:#?}", doc.issues);
        assert!(issues[0].message.contains("lines 3 and 5"));
    }

    #[test]
    fn legacy_inline_citation_does_not_define_an_id() {
        let doc = parse_document(
            "x.md",
            "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `##SAME`\n",
        );
        assert!(duplicates(&doc).is_empty(), "{:#?}", doc.issues);
    }

    #[test]
    fn typed_and_untyped_forms_share_one_definition_namespace() {
        let doc = parse_document(
            "x.md",
            "# H {#h}\n\n\
             @fact:SAME ordinary definition\n\n\
             @fact/code:SAME typed definition\n\n\
             ```text\nclaim\n```\n",
        );
        let issues = duplicates(&doc);
        assert_eq!(issues.len(), 1, "{:#?}", doc.issues);
        assert!(issues[0].message.contains("lines 3 and 5"));
    }

    #[test]
    fn definition_form_inside_fence_is_not_a_definition() {
        let doc = parse_document(
            "x.md",
            "# H {#h}\n\n@fact:SAME definition\n\n```markdown\n@fact:SAME example\n```\n",
        );
        assert!(duplicates(&doc).is_empty(), "{:#?}", doc.issues);
    }
}
