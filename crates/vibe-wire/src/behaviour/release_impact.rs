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

impl ReleaseImpact {
    /// Validate meaning the JTD shape cannot express before accepting a record.
    pub fn validate_release_impact(&self) -> Result<(), ReleaseImpactError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if !identifier(&self.change_id) {
            return Err(fault("change_id", "invalid immutable ID"));
        }
        if self.before_capture_id.trim().is_empty() || self.after_capture_id.trim().is_empty() {
            return Err(fault(
                "captures",
                "before and after identities are required",
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
            if self.acceptance.review_subject.trim().is_empty()
                || self.acceptance.review_evidence.trim().is_empty()
                || !self.evidence.iter().any(|e| {
                    matches!(
                        e.verdict,
                        crate::generated::preview::e1::release_impact::EvidenceRefVerdict::Passed
                    )
                })
            {
                return Err(fault(
                    "acceptance",
                    "review subject and passing evidence are required",
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
    fn distinct_no_impact_reasons_can_be_reviewed() {
        internal_refactor().validate_release_impact().unwrap();
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
