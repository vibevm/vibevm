//! Semantic checks beside the generated authored release-impact record.

use std::collections::BTreeSet;

use crate::generated::preview::e1::release_impact::{
    AcceptanceState, DocumentationDispositionDocumentationAction, DocumentationDispositionState,
    ReleaseImpact, UserDispositionMigrationAction, UserDispositionState,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("release impact {field}: {reason}")]
pub struct ReleaseImpactError {
    pub field: &'static str,
    pub reason: &'static str,
}

fn fault(field: &'static str, reason: &'static str) -> ReleaseImpactError {
    ReleaseImpactError { field, reason }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn safe_relative_path(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':', '\0'])
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl ReleaseImpact {
    /// Validate meaning the JTD shape cannot express before accepting a record.
    pub fn validate_release_impact(&self) -> Result<(), ReleaseImpactError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if !identifier(&self.change_id) {
            return Err(fault("change_id", "invalid immutable ID"));
        }
        if let Some(paths) = &self.affected_paths
            && (paths.iter().any(|path| !safe_relative_path(path))
                || paths.windows(2).any(|pair| pair[0] >= pair[1]))
        {
            return Err(fault(
                "affected_paths",
                "paths must be safe, unique and sorted",
            ));
        }
        if !identifier(&self.before_capture_id) || !identifier(&self.after_capture_id) {
            return Err(fault(
                "captures",
                "safe before and after identities are required",
            ));
        }
        if self.implementation_subjects.is_empty() {
            return Err(fault(
                "implementation_subjects",
                "no actual contribution subject",
            ));
        }
        let mut subjects = BTreeSet::new();
        for subject in &self.implementation_subjects {
            if subject.identity.trim().is_empty() || !subjects.insert(subject.identity.as_str()) {
                return Err(fault(
                    "implementation_subjects",
                    "blank or repeated subject",
                ));
            }
        }

        let mut effects = BTreeSet::new();
        for effect in &self.effects {
            if effect.interface_id.trim().is_empty()
                || effect.capability_id.trim().is_empty()
                || effect.audience.trim().is_empty()
                || effect.platform.trim().is_empty()
            {
                return Err(fault("effects", "effect lacks its interface scope"));
            }
            if !effects.insert((
                effect.interface_id.as_str(),
                effect.capability_id.as_str(),
                effect.audience.as_str(),
                effect.platform.as_str(),
            )) {
                return Err(fault("effects", "duplicate scoped effect"));
            }
            if matches!(self.acceptance.state, AcceptanceState::Accepted)
                && (effect.contract_anchors.is_empty()
                    || effect
                        .contract_anchors
                        .iter()
                        .any(|anchor| !anchor.starts_with("spec://") || anchor.contains("/next/")))
            {
                return Err(fault(
                    "effects.contract_anchors",
                    "accepted effect needs permanent law",
                ));
            }
        }

        match self.user_impact.state {
            UserDispositionState::NoImpact
                if self.user_impact.reason.trim().is_empty()
                    || !matches!(
                        self.user_impact.migration_action,
                        UserDispositionMigrationAction::None
                    ) =>
            {
                return Err(fault(
                    "user_impact",
                    "no-impact needs a reason and no migration action",
                ));
            }
            UserDispositionState::Impact if self.user_impact.reason.trim().is_empty() => {
                return Err(fault("user_impact", "impact needs a reason"));
            }
            _ => {}
        }
        match self.documentation_impact.state {
            DocumentationDispositionState::NoImpact
                if self.documentation_impact.reason.trim().is_empty()
                    || !matches!(
                        self.documentation_impact.documentation_action,
                        DocumentationDispositionDocumentationAction::None
                    ) =>
            {
                return Err(fault(
                    "documentation_impact",
                    "no-impact needs a reason and no edit action",
                ));
            }
            DocumentationDispositionState::Impact
                if self.documentation_impact.reason.trim().is_empty()
                    || matches!(
                        self.documentation_impact.documentation_action,
                        DocumentationDispositionDocumentationAction::None
                    ) =>
            {
                return Err(fault(
                    "documentation_impact",
                    "impact needs a reason and edit action",
                ));
            }
            _ => {}
        }

        let mut evidence = BTreeSet::new();
        for entry in &self.evidence {
            if entry.sha256.as_ref().is_some_and(|value| {
                value.len() != 64
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            }) {
                return Err(fault("evidence.sha256", "invalid SHA-256 identity"));
            }
            if entry.claim_id.trim().is_empty()
                || entry.subject_id.trim().is_empty()
                || !safe_relative_path(&entry.source)
                || !evidence.insert((
                    entry.claim_id.as_str(),
                    entry.subject_id.as_str(),
                    entry.source.as_str(),
                ))
            {
                return Err(fault(
                    "evidence",
                    "blank, unsafe or repeated evidence identity",
                ));
            }
        }
        let mut related = BTreeSet::new();
        for relation in &self.relations {
            if relation.change_id == self.change_id
                || !identifier(&relation.change_id)
                || !related.insert(relation.change_id.as_str())
            {
                return Err(fault(
                    "relations",
                    "invalid, self-referential or duplicate relation",
                ));
            }
        }
        if matches!(self.acceptance.state, AcceptanceState::Accepted) {
            if matches!(self.user_impact.state, UserDispositionState::Pending)
                || matches!(
                    self.documentation_impact.state,
                    DocumentationDispositionState::Pending
                )
            {
                return Err(fault("acceptance", "impact disposition is pending"));
            }
            if !subjects.contains(self.acceptance.review_subject.as_str())
                || !safe_relative_path(&self.acceptance.review_evidence)
                || !self.evidence.iter().any(|e| {
                    e.subject_id == self.acceptance.review_subject
                        && e.source == self.acceptance.review_evidence
                        && matches!(
                        e.verdict,
                        crate::generated::preview::e1::release_impact::EvidenceRefVerdict::Passed
                    )
                })
            {
                return Err(fault(
                    "acceptance",
                    "review must bind an implementation subject and its matching passing evidence",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn internal_refactor() -> ReleaseImpact {
        serde_json::from_str(include_str!(
            "../../../../formats/corpora/release-impact/e1/valid/internal-refactor.json"
        ))
        .expect("registered release-impact corpus must parse")
    }

    #[test]
    fn path_evidence_wire_preserves_absence_and_roundtrips_supplied_fields() {
        assert!(internal_refactor().affected_paths.is_none());
        assert!(internal_refactor().evidence[0].sha256.is_none());
        let record: ReleaseImpact = serde_json::from_str(include_str!(
            "../../../../formats/corpora/release-impact/e1/valid/with-path-evidence.json"
        ))
        .unwrap();
        record.validate_release_impact().unwrap();
        assert_eq!(
            record,
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap()
        );
        assert!(serde_json::from_str::<ReleaseImpact>(include_str!("../../../../formats/corpora/release-impact/e1/invalid/wrong-affected-paths-type.json")).is_err());
    }

    #[test]
    fn optional_paths_and_digests_are_checked_only_when_supplied() {
        let mut record = internal_refactor();
        record.affected_paths = Some(vec![]);
        record.validate_release_impact().unwrap();
        for paths in [
            vec!["../bad".into()],
            vec!["z".into(), "a".into()],
            vec!["a".into(), "a".into()],
        ] {
            record.affected_paths = Some(paths);
            assert_eq!(
                record.validate_release_impact().unwrap_err().field,
                "affected_paths"
            );
        }
        record.affected_paths = Some(vec!["a".into()]);
        for digest in ["bad".into(), "A".repeat(64)] {
            record.evidence[0].sha256 = Some(digest);
            assert_eq!(
                record.validate_release_impact().unwrap_err().field,
                "evidence.sha256"
            );
        }
        record.evidence[0].sha256 = Some("a".repeat(64));
        record.validate_release_impact().unwrap();
    }

    #[test]
    fn distinct_no_impact_reasons_can_be_reviewed() {
        internal_refactor().validate_release_impact().unwrap();
        let mut record = internal_refactor();
        record.acceptance.state = AcceptanceState::Accepted;
        record.documentation_impact.state = DocumentationDispositionState::Impact;
        record.documentation_impact.documentation_action =
            DocumentationDispositionDocumentationAction::Update;
        record.documentation_impact.reason =
            "Explain the internal implementation guarantee.".into();
        record.validate_release_impact().unwrap();
    }

    #[test]
    fn accepted_review_binds_implementation_subject_and_exact_passing_source() {
        let mut record = internal_refactor();
        record.acceptance.state = AcceptanceState::Accepted;
        record.validate_release_impact().unwrap();
        for mismatch in [
            "review-subject",
            "evidence-subject",
            "evidence-source",
            "verdict",
        ] {
            let mut invalid = record.clone();
            match mismatch {
                "review-subject" => invalid.acceptance.review_subject = "unrelated".into(),
                "evidence-subject" => invalid.evidence[0].subject_id = "unrelated".into(),
                "evidence-source" => invalid.evidence[0].source = "review/unrelated.json".into(),
                _ => invalid.evidence[0].verdict =
                    crate::generated::preview::e1::release_impact::EvidenceRefVerdict::Unavailable,
            }
            assert_eq!(
                invalid.validate_release_impact().unwrap_err().field,
                "acceptance",
                "{mismatch}"
            );
        }
    }

    #[test]
    fn evidence_refuses_blank_unsafe_and_duplicate_identities() {
        for field in ["claim", "subject", "source"] {
            let mut record = internal_refactor();
            match field {
                "claim" => record.evidence[0].claim_id = " ".into(),
                "subject" => record.evidence[0].subject_id = " ".into(),
                _ => record.evidence[0].source = " ".into(),
            }
            assert_eq!(
                record.validate_release_impact().unwrap_err().field,
                "evidence"
            );
        }
        for source in [
            "/outside.json",
            "../outside.json",
            "review/../outside.json",
            "C:/outside.json",
            "review\\outside.json",
            "review//outside.json",
            "review/./outside.json",
            "review/\0.json",
        ] {
            let mut record = internal_refactor();
            record.evidence[0].source = source.into();
            assert_eq!(
                record.validate_release_impact().unwrap_err().field,
                "evidence",
                "{source:?}"
            );
        }
        let mut record = internal_refactor();
        record.evidence.push(record.evidence[0].clone());
        assert_eq!(
            record.validate_release_impact().unwrap_err().field,
            "evidence"
        );
        let mut record = internal_refactor();
        record.evidence[0].verdict =
            crate::generated::preview::e1::release_impact::EvidenceRefVerdict::Unavailable;
        record.validate_release_impact().unwrap();
    }

    #[test]
    fn unknown_state_and_unknown_field_are_refused_by_generated_reader() {
        for bad in [
            include_str!(
                "../../../../formats/corpora/release-impact/e1/invalid/unknown-impact-state.json"
            ),
            include_str!(
                "../../../../formats/corpora/release-impact/e1/invalid/unknown-field.json"
            ),
        ] {
            assert!(serde_json::from_str::<ReleaseImpact>(bad).is_err());
        }
    }

    #[test]
    fn accepted_record_refuses_pending_or_unreasoned_impact() {
        let mut record = internal_refactor();
        record.acceptance.state = AcceptanceState::Accepted;
        record.user_impact.state = UserDispositionState::Pending;
        assert_eq!(
            record.validate_release_impact().unwrap_err().field,
            "acceptance"
        );

        let mut record = internal_refactor();
        record.user_impact.reason.clear();
        assert_eq!(
            record.validate_release_impact().unwrap_err().field,
            "user_impact"
        );
    }

    #[test]
    fn an_impact_requires_a_real_documentation_action() {
        let mut record = internal_refactor();
        record.documentation_impact.state = DocumentationDispositionState::Impact;
        assert_eq!(
            record.validate_release_impact().unwrap_err().field,
            "documentation_impact"
        );
    }

    #[test]
    fn duplicate_subject_and_self_relation_are_refused() {
        let mut record = internal_refactor();
        record
            .implementation_subjects
            .push(record.implementation_subjects[0].clone());
        assert_eq!(
            record.validate_release_impact().unwrap_err().field,
            "implementation_subjects"
        );

        let mut record = internal_refactor();
        record
            .relations
            .push(crate::generated::preview::e1::release_impact::Relation {
                change_id: record.change_id.clone(),
                kind: crate::generated::preview::e1::release_impact::RelationKind::Reverts,
            });
        assert_eq!(
            record.validate_release_impact().unwrap_err().field,
            "relations"
        );
    }
}
