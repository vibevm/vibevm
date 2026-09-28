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

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
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
        if self.change_records.iter().any(|id| !identifier(id)) {
            return Err(fault("change_records", "unsafe change ID"));
        }
        if let Some(scope) = &self.contribution_scope {
            if !identifier(&scope.before_capture_id) || !identifier(&scope.after_capture_id) {
                return Err(fault("contribution_scope", "unsafe capture ID"));
            }
            if scope
                .record_digests
                .iter()
                .any(|record| !identifier(&record.change_id) || !digest(&record.sha256, 64))
                || scope
                    .record_digests
                    .windows(2)
                    .any(|pair| pair[0].change_id >= pair[1].change_id)
                || scope
                    .record_digests
                    .iter()
                    .map(|record| &record.change_id)
                    .ne(self.change_records.iter())
            {
                return Err(fault(
                    "contribution_scope.record_digests",
                    "sorted exact record IDs and SHA-256 bindings are required",
                ));
            }
            for record in &scope.record_digests {
                if record.required_claim_ids.as_ref().is_some_and(|claims| {
                    claims.is_empty()
                        || claims.iter().any(|claim| claim.trim().is_empty())
                        || claims.windows(2).any(|pair| pair[0] >= pair[1])
                }) {
                    return Err(fault(
                        "contribution_scope.required_claim_ids",
                        "supplied claim selection must be nonempty, unique and sorted",
                    ));
                }
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
#[path = "preview_transition/tests.rs"]
mod tests;
