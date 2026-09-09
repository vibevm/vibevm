//! Exact ownership invariant for fact terminal requirements.

use crate::doc::{FactKind, Issue, IssueCode, ParsedDoc, Severity};
use crate::model::Granularity;

pub(super) fn check_requirement_owners(doc: &mut ParsedDoc) {
    let mut issues = Vec::new();
    for fact in doc.blocks.iter().flat_map(|block| &block.facts) {
        if fact.requirements.is_none() {
            continue;
        }
        let expected = match fact.kind {
            FactKind::Para | FactKind::Lead => Granularity::Paragraph,
            FactKind::Item => Granularity::Item,
            FactKind::Cell => Granularity::Cell,
        };
        let exact = fact
            .marker_index
            .and_then(|index| doc.markers.get(index))
            .is_some_and(|marker| marker.granularity == expected);
        if !exact {
            issues.push(Issue {
                severity: Severity::Error,
                line: fact.line,
                code: IssueCode::TerminalRequirements,
                message: "terminal requirements have no exact whole-unit final status owner".into(),
            });
        }
    }
    doc.issues.extend(issues);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    #[test]
    fn every_valid_classified_fact_resolves_its_whole_unit_marker() {
        let doc = parse_document(
            "x.md",
            "@fact:A paragraph @requires:specification @spec/done\n\n- @fact:B item @requires:plan @spec/done\n\n| H |\n|---|\n| @fact:C cell @requires:decision @spec/done |\n",
        );
        assert_eq!(doc.error_count(), 0, "issues: {:#?}", doc.issues);
        for fact in doc.blocks.iter().flat_map(|block| &block.facts) {
            if fact.requirements.is_some() {
                let marker = fact
                    .marker_index
                    .and_then(|index| doc.markers.get(index))
                    .expect("classified fact has exact marker");
                assert_ne!(marker.granularity, Granularity::Fragment);
            }
        }
    }
}
