//! Phase 5 — the anchor laws (anchored-when-marked, unique ids).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#parsing");

use crate::doc::{Issue, IssueCode, ParsedDoc, Severity};
use std::collections::HashMap;

#[derive(Debug)]
struct Definition {
    id: String,
    line: usize,
}

/// The anchored-when-marked law + one shared id namespace (PROP-043 §3.8):
/// every marked countable unit needs a `##<ID>`; every id — fact or
/// heading anchor — is unique per document.
pub(super) fn check_anchor_laws(doc: &mut ParsedDoc) {
    let mut definitions = Vec::new();
    for u in &doc.units {
        if let Some(a) = &u.anchor {
            definitions.push(Definition {
                id: a.clone(),
                line: u.line_start,
            });
        }
    }

    // Only parsed anchors are definitions. Raw `@fact:` spellings inside
    // inline-code quotations remain opaque, as they already are to the
    // fact parser and to the block's inline-code-blanked scan text.
    for b in &doc.blocks {
        for f in &b.facts {
            if let Some(id) = &f.id {
                definitions.push(Definition {
                    id: id.clone(),
                    line: f.line,
                });
            }
        }
    }

    definitions.sort_by_key(|d| d.line);
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
            if f.marked && f.id.is_none() {
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
    fn definition_form_quoted_in_inline_code_is_not_a_definition() {
        let doc = parse_document(
            "x.md",
            "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `@fact:SAME`\n",
        );
        assert!(duplicates(&doc).is_empty(), "{:#?}", doc.issues);
    }

    #[test]
    fn real_fact_and_repeated_inline_code_quotes_in_paragraph_and_table_cell() {
        let doc = parse_document(
            "x.md",
            "# H {#h}\n\n\
             @fact:X real definition\n\n\
             @fact:PARAGRAPH `@fact:X` and `@fact:X` are quoted.\n\n\
             | Example |\n| --- |\n| @fact:CELL `@fact:X` and `@fact:X` are quoted. |\n",
        );
        assert!(duplicates(&doc).is_empty(), "{:#?}", doc.issues);
        let ids = doc
            .blocks
            .iter()
            .flat_map(|b| b.facts.iter())
            .filter_map(|f| f.id.as_deref())
            .collect::<Vec<_>>();
        assert!(
            ids.contains(&"X") && ids.contains(&"PARAGRAPH") && ids.contains(&"CELL"),
            "{ids:?}"
        );
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
    fn marked_table_cell_without_an_id_is_rejected() {
        let doc = parse_document(
            "x.md",
            "# T {#t}\n\n\
             | first | second |\n\
             | --- | --- |\n\
             | @fact:ROW x @status:impl/done | y @status:impl/done |\n",
        );
        let issues: Vec<_> = doc
            .issues
            .iter()
            .filter(|issue| issue.code == IssueCode::MissingAnchor)
            .collect();
        assert_eq!(issues.len(), 1, "{:#?}", doc.issues);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].line, 5);
        assert!(issues[0].message.contains("anchored-when-marked"));
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
