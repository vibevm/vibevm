//! Phase 5 — the anchor laws (anchored-when-marked, unique ids).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#parsing");

use crate::doc::{FactKind, Issue, IssueCode, ParsedDoc, Severity};
use std::collections::HashMap;

/// The anchored-when-marked law + one shared id namespace (PROP-043 §3.8):
/// a marked paragraph/lead/item needs a `##<ID>`; every id — fact or
/// heading anchor — is unique per document.
pub(super) fn check_anchor_laws(doc: &mut ParsedDoc) {
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
