//! Semantic invariants beside the generated preview transition wire shape.

use crate::generated::preview::e1::transition::{
    CaptureRef, PendingCaptureState, PendingDocumentationState, PendingGuideState, Transition,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CAPTURE-CATALOGUE");

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

fn optional_relative_path(
    value: Option<&str>,
    field: &'static str,
) -> Result<(), PreviewTransitionError> {
    if value.is_some_and(|source| !relative_path(source)) {
        return Err(fault(field, "source must be a safe relative path"));
    }
    Ok(())
}

enum CaptureScope {
    Dp1,
    Dp2,
    Catalogue,
}

fn validate_reference(
    reference: &CaptureRef,
    scope: CaptureScope,
) -> Result<(), PreviewTransitionError> {
    let (identity, source, content) = match scope {
        CaptureScope::Dp1 => ("dp1.capture_id", "dp1.source", "dp1"),
        CaptureScope::Dp2 => ("dp2.capture_id", "dp2.source", "dp2"),
        CaptureScope::Catalogue => ("captures.capture_id", "captures.source", "captures"),
    };
    if reference.capture_id.trim().is_empty() {
        return Err(fault(identity, "empty identity"));
    }
    optional_relative_path(Some(&reference.source), source)?;
    if !digest(&reference.capture_file_sha256, 64)
        || !digest(&reference.commit_sha1, 40)
        || !digest(&reference.git_tree_sha1, 40)
    {
        return Err(fault(content, "invalid content identity"));
    }
    Ok(())
}

impl Transition {
    /// Enumerate supplied capture references without reading files or claiming proof.
    ///
    /// Endpoint/catalogue aliases may retain alternate safe retrieval locations
    /// only when their digest, commit and tree identities agree. Catalogue entries
    /// are sorted and unique; the returned references are sorted by ID and source.
    ///
    /// ```
    /// use vibe_wire::generated::preview::e1::transition::Transition;
    /// let mut transition: Transition = serde_json::from_str(include_str!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../../formats/corpora/preview-transition/e1/valid/captured.json"
    /// )))?;
    /// let mut alternate = transition.dp1.clone();
    /// alternate.source = "evidence/alternate.json".into();
    /// transition.captures = Some(vec![alternate]);
    /// let references = transition.capture_references()?;
    /// assert_eq!(references.len(), 2);
    /// assert_eq!(references[0].source, "evidence/alternate.json");
    /// assert_eq!(references[0].capture_id, references[1].capture_id);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-081#CAPTURE-CATALOGUE"
    )]
    pub fn capture_references(&self) -> Result<Vec<CaptureRef>, PreviewTransitionError> {
        validate_reference(&self.dp1, CaptureScope::Dp1)?;
        let mut references = vec![self.dp1.clone()];
        let parts = [
            self.dp2.capture_id.as_deref(),
            self.dp2.source.as_deref(),
            self.dp2.capture_file_sha256.as_deref(),
            self.dp2.commit_sha1.as_deref(),
            self.dp2.git_tree_sha1.as_deref(),
        ];
        match self.dp2.state {
            PendingCaptureState::Pending if parts.iter().any(Option::is_some) => {
                return Err(fault("dp2", "pending capture carries identity"));
            }
            PendingCaptureState::Pending => {}
            PendingCaptureState::Candidate | PendingCaptureState::Sealed => {
                let [
                    Some(id),
                    Some(source),
                    Some(sha256),
                    Some(commit),
                    Some(tree),
                ] = parts
                else {
                    return Err(fault("dp2", "capture identity is incomplete"));
                };
                let reference = CaptureRef {
                    capture_id: id.into(),
                    source: source.into(),
                    capture_file_sha256: sha256.into(),
                    commit_sha1: commit.into(),
                    git_tree_sha1: tree.into(),
                };
                validate_reference(&reference, CaptureScope::Dp2)?;
                references.push(reference);
            }
        }
        if let Some(captures) = &self.captures {
            if captures
                .windows(2)
                .any(|pair| pair[0].capture_id >= pair[1].capture_id)
            {
                return Err(fault("captures", "catalogue IDs must be unique and sorted"));
            }
            for reference in captures {
                validate_reference(reference, CaptureScope::Catalogue)?;
                references.push(reference.clone());
            }
        }
        references.sort_by(|left, right| {
            (&left.capture_id, &left.source).cmp(&(&right.capture_id, &right.source))
        });
        for pair in references.windows(2) {
            if pair[0].capture_id == pair[1].capture_id
                && (pair[0].capture_file_sha256 != pair[1].capture_file_sha256
                    || pair[0].commit_sha1 != pair[1].commit_sha1
                    || pair[0].git_tree_sha1 != pair[1].git_tree_sha1)
            {
                return Err(fault(
                    "captures",
                    "capture ID names conflicting content identities",
                ));
            }
        }
        Ok(references)
    }

    /// Check the cross-field laws JTD cannot express. No files are opened here.
    pub fn validate_preview(&self) -> Result<(), PreviewTransitionError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if self.schema_status.trim().is_empty() {
            return Err(fault("schema_status", "empty status"));
        }
        self.capture_references()?;

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
        optional_relative_path(self.documentation.source.as_deref(), "documentation.source")?;
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
        optional_relative_path(
            self.migration_guide.source.as_deref(),
            "migration_guide.source",
        )?;
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

    fn intermediate(identity: &str) -> CaptureRef {
        let mut reference = captured().dp1;
        reference.capture_id = identity.into();
        reference.source = format!("evidence/{identity}.json");
        reference
    }

    fn set_dp2(transition: &mut Transition, reference: CaptureRef) {
        transition.dp2 = crate::generated::preview::e1::transition::PendingCapture {
            state: PendingCaptureState::Candidate,
            capture_id: Some(reference.capture_id),
            source: Some(reference.source),
            capture_file_sha256: Some(reference.capture_file_sha256),
            commit_sha1: Some(reference.commit_sha1),
            git_tree_sha1: Some(reference.git_tree_sha1),
        };
    }

    #[test]
    fn generated_catalogue_preserves_absent_empty_and_supplied_shapes() {
        let mut transition = captured();
        assert!(transition.captures.is_none());
        assert!(
            !serde_json::to_string(&transition)
                .unwrap()
                .contains("\"captures\"")
        );
        transition.captures = Some(vec![]);
        let roundtrip: Transition =
            serde_json::from_str(&serde_json::to_string(&transition).unwrap()).unwrap();
        assert_eq!(roundtrip.captures, Some(vec![]));
        let supplied: Transition = serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-transition/e1/valid/with-catalogue.json"
        ))
        .unwrap();
        assert_eq!(supplied.captures.as_ref().unwrap().len(), 1);
        assert_eq!(supplied.capture_references().unwrap().len(), 2);
        assert!(serde_json::from_str::<Transition>(include_str!(
            "../../../../formats/corpora/preview-transition/e1/invalid/wrong-catalogue-type.json"
        )).is_err());
    }

    #[test]
    fn references_are_sorted_and_retain_compatible_alternate_sources() {
        let mut transition = captured();
        let mut alternate = transition.dp1.clone();
        alternate.source = "evidence/alternate.json".into();
        transition.captures = Some(vec![alternate.clone(), intermediate("intermediate")]);
        set_dp2(&mut transition, intermediate("dp2-final"));
        let references = transition.capture_references().unwrap();
        assert_eq!(references.len(), 4);
        assert_eq!(references[0], alternate);
        assert_eq!(references[1], transition.dp1);
        assert!(
            references
                .windows(2)
                .all(|pair| (&pair[0].capture_id, &pair[0].source)
                    <= (&pair[1].capture_id, &pair[1].source))
        );
        transition.validate_preview().unwrap();
    }

    #[test]
    fn conflicting_catalogue_content_for_endpoint_identity_refuses() {
        for field in ["digest", "commit", "tree"] {
            let mut transition = captured();
            let mut conflict = transition.dp1.clone();
            match field {
                "digest" => conflict.capture_file_sha256 = "a".repeat(64),
                "commit" => conflict.commit_sha1 = "a".repeat(40),
                _ => conflict.git_tree_sha1 = "a".repeat(40),
            }
            transition.captures = Some(vec![conflict]);
            assert_eq!(
                transition.capture_references().unwrap_err().field,
                "captures",
                "{field}"
            );
            assert_eq!(
                transition.validate_preview().unwrap_err().field,
                "captures",
                "{field}"
            );
        }
    }

    #[test]
    fn equal_endpoint_identity_requires_equal_content_but_allows_alternate_source() {
        let mut transition = captured();
        let mut after = transition.dp1.clone();
        after.source = "evidence/alternate.json".into();
        set_dp2(&mut transition, after.clone());
        assert_eq!(transition.capture_references().unwrap().len(), 2);
        after.git_tree_sha1 = "a".repeat(40);
        set_dp2(&mut transition, after);
        assert_eq!(
            transition.capture_references().unwrap_err().field,
            "captures"
        );
    }

    #[test]
    fn duplicate_and_unsorted_additional_catalogue_ids_refuse() {
        for captures in [
            vec![intermediate("a"), intermediate("a")],
            vec![intermediate("b"), intermediate("a")],
        ] {
            let mut transition = captured();
            transition.captures = Some(captures);
            assert_eq!(
                transition.capture_references().unwrap_err().field,
                "captures"
            );
        }
    }

    #[test]
    fn every_reference_uses_the_same_digest_and_path_validation() {
        for field in ["id", "source", "digest", "commit", "tree"] {
            for scope in ["dp1", "dp2", "catalogue"] {
                let mut transition = captured();
                let mut bad = intermediate("intermediate");
                match field {
                    "id" => bad.capture_id = " ".into(),
                    "source" => bad.source = "../outside.json".into(),
                    "digest" => bad.capture_file_sha256 = "bad".into(),
                    "commit" => bad.commit_sha1 = "A".repeat(40),
                    _ => bad.git_tree_sha1.clear(),
                }
                match scope {
                    "dp1" => transition.dp1 = bad,
                    "dp2" => set_dp2(&mut transition, bad),
                    _ => transition.captures = Some(vec![bad]),
                }
                assert!(transition.capture_references().is_err(), "{scope}/{field}");
                assert!(transition.validate_preview().is_err(), "{scope}/{field}");
            }
        }
    }

    #[test]
    fn pure_reference_seam_refuses_pending_identity_and_incomplete_after_capture() {
        let mut transition = captured();
        transition.dp2.source = Some("evidence/after.json".into());
        assert_eq!(transition.capture_references().unwrap_err().field, "dp2");
        transition.dp2.state = PendingCaptureState::Candidate;
        assert_eq!(transition.capture_references().unwrap_err().field, "dp2");
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
