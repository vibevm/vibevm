//! Semantic invariants beside the generated preview transition wire shape.

use crate::generated::preview::e1::transition::{
    PendingCaptureState, PendingDocumentationState, PendingGuideState, Transition,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("preview transition {field}: {reason}")]
pub struct PreviewTransitionError {
    pub field: &'static str,
    pub reason: &'static str,
}

fn fault(field: &'static str, reason: &'static str) -> PreviewTransitionError {
    PreviewTransitionError { field, reason }
}

fn digest(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl Transition {
    /// Check the cross-field laws JTD cannot express. No files are opened here.
    pub fn validate_preview(&self) -> Result<(), PreviewTransitionError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if self.schema_status.trim().is_empty() {
            return Err(fault("schema_status", "empty status"));
        }
        if self.dp1.capture_id.trim().is_empty() {
            return Err(fault("dp1.capture_id", "empty identity"));
        }
        if !relative_path(&self.dp1.source) {
            return Err(fault("dp1.source", "source must be a safe relative path"));
        }
        if !digest(&self.dp1.capture_file_sha256, 64)
            || !digest(&self.dp1.commit_sha1, 40)
            || !digest(&self.dp1.git_tree_sha1, 40)
        {
            return Err(fault("dp1", "invalid content identity"));
        }

        let dp2_parts = [
            self.dp2.capture_id.as_deref(),
            self.dp2.source.as_deref(),
            self.dp2.capture_file_sha256.as_deref(),
            self.dp2.commit_sha1.as_deref(),
            self.dp2.git_tree_sha1.as_deref(),
        ];
        match self.dp2.state {
            PendingCaptureState::Pending if dp2_parts.iter().any(Option::is_some) => {
                return Err(fault("dp2", "pending capture carries identity"));
            }
            PendingCaptureState::Candidate | PendingCaptureState::Sealed
                if dp2_parts.iter().any(Option::is_none) =>
            {
                return Err(fault("dp2", "capture identity is incomplete"));
            }
            _ => {}
        }
        if let Some(source) = &self.dp2.source {
            if !relative_path(source) {
                return Err(fault("dp2.source", "source must be a safe relative path"));
            }
        }
        for (field, value, width) in [
            ("dp2.capture_file_sha256", &self.dp2.capture_file_sha256, 64),
            ("dp2.commit_sha1", &self.dp2.commit_sha1, 40),
            ("dp2.git_tree_sha1", &self.dp2.git_tree_sha1, 40),
        ] {
            if value.as_ref().is_some_and(|v| !digest(v, width)) {
                return Err(fault(field, "invalid content identity"));
            }
        }

        let documentation_parts = [
            self.documentation.capture_id.as_deref(),
            self.documentation.source.as_deref(),
            self.documentation.capture_file_sha256.as_deref(),
        ];
        if matches!(
            self.documentation.state,
            PendingDocumentationState::PendingOwnerBinding
        ) {
            if documentation_parts.iter().any(Option::is_some) {
                return Err(fault("documentation", "unbound source carries identity"));
            }
        } else if documentation_parts.iter().any(Option::is_none) {
            return Err(fault("documentation", "source binding is incomplete"));
        }
        if let Some(source) = &self.documentation.source {
            if !relative_path(source) {
                return Err(fault(
                    "documentation.source",
                    "source must be a safe relative path",
                ));
            }
        }
        if self
            .documentation
            .capture_file_sha256
            .as_ref()
            .is_some_and(|v| !digest(v, 64))
        {
            return Err(fault(
                "documentation.capture_file_sha256",
                "invalid content identity",
            ));
        }

        if matches!(self.migration_guide.state, PendingGuideState::Pending) {
            if self.migration_guide.source.is_some() || self.migration_guide.source_sha256.is_some()
            {
                return Err(fault(
                    "migration_guide",
                    "pending guide carries an artifact",
                ));
            }
        } else if self.migration_guide.source.is_none()
            || self.migration_guide.source_sha256.is_none()
        {
            return Err(fault("migration_guide", "guide artifact is incomplete"));
        }
        if let Some(source) = &self.migration_guide.source {
            if !relative_path(source) {
                return Err(fault(
                    "migration_guide.source",
                    "source must be a safe relative path",
                ));
            }
        }
        if self
            .migration_guide
            .source_sha256
            .as_ref()
            .is_some_and(|v| !digest(v, 64))
        {
            return Err(fault(
                "migration_guide.source_sha256",
                "invalid content identity",
            ));
        }

        if self
            .change_records
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(fault("change_records", "IDs must be unique and sorted"));
        }
        if self.proof.documentation_verified
            && !matches!(
                self.documentation.state,
                PendingDocumentationState::Verified
            )
        {
            return Err(fault(
                "proof.documentation_verified",
                "documentation is not verified",
            ));
        }
        if self.proof.migration_rehearsed
            && !matches!(self.migration_guide.state, PendingGuideState::Verified)
        {
            return Err(fault("proof.migration_rehearsed", "guide is not verified"));
        }
        if self.proof.final_product_panel_passed
            && matches!(self.dp2.state, PendingCaptureState::Pending)
        {
            return Err(fault(
                "proof.final_product_panel_passed",
                "DP2 remains pending",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captured() -> Transition {
        serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-transition/e1/valid/captured.json"
        ))
        .expect("registered preview transition corpus must parse")
    }

    #[test]
    fn captured_before_state_is_valid_and_later_proof_is_pending() {
        let transition = captured();
        transition.validate_preview().unwrap();
        assert!(!transition.proof.final_product_panel_passed);
    }

    #[test]
    fn wrong_epoch_type_and_unknown_field_are_refused_by_generated_reader() {
        for bad in [
            include_str!(
                "../../../../formats/corpora/preview-transition/e1/invalid/wrong-epoch-type.json"
            ),
            include_str!(
                "../../../../formats/corpora/preview-transition/e1/invalid/unknown-field.json"
            ),
        ] {
            assert!(serde_json::from_str::<Transition>(bad).is_err());
        }
    }

    #[test]
    fn a_pending_dp2_cannot_claim_identity_or_final_proof() {
        let mut transition = captured();
        transition.dp2.capture_id = Some("candidate".into());
        assert_eq!(transition.validate_preview().unwrap_err().field, "dp2");

        let mut transition = captured();
        transition.proof.final_product_panel_passed = true;
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "proof.final_product_panel_passed"
        );
    }

    #[test]
    fn unknown_epoch_is_refused_after_shape_parsing() {
        let mut transition = captured();
        transition.schema = 2;
        assert_eq!(transition.validate_preview().unwrap_err().field, "schema");
    }

    #[test]
    fn documentation_and_migration_proof_require_verified_sources() {
        let mut transition = captured();
        transition.proof.documentation_verified = true;
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "proof.documentation_verified"
        );

        let mut transition = captured();
        transition.proof.migration_rehearsed = true;
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "proof.migration_rehearsed"
        );
    }

    #[test]
    fn duplicate_change_ids_and_parent_paths_are_refused() {
        let mut transition = captured();
        transition.change_records = vec!["C-1".into(), "C-1".into()];
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "change_records"
        );

        let mut transition = captured();
        transition.dp1.source = "../outside.json".into();
        assert_eq!(
            transition.validate_preview().unwrap_err().field,
            "dp1.source"
        );
    }
}
