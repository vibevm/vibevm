//! Stage grammar and documentation-map invariants for preview checks.

use std::collections::BTreeSet;

use crate::generated::preview::e1::check_request::{
    CheckRequest, CheckRequestMode, CheckRequestStage,
};
use crate::generated::preview::e1::documentation_map::{
    DocumentationMap, TopicDisposition, TopicProofState, TopicSourceKind,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("preview check {field}: {reason}")]
pub struct PreviewCheckError {
    pub field: &'static str,
    pub reason: &'static str,
}

fn fault(field: &'static str, reason: &'static str) -> PreviewCheckError {
    PreviewCheckError { field, reason }
}

fn ordered_unique(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

impl CheckRequest {
    /// A stage is accepted only for its owning mode; no default stage is guessed.
    pub fn validate_preview(&self) -> Result<(), PreviewCheckError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if self.transition.trim().is_empty() || self.transition.contains('\0') {
            return Err(fault("transition", "missing or invalid path"));
        }
        let valid = matches!(
            (&self.mode, &self.stage),
            (
                CheckRequestMode::Capture
                    | CheckRequestMode::Contribution
                    | CheckRequestMode::Retired,
                None
            ) | (
                CheckRequestMode::Bindings,
                Some(CheckRequestStage::Roles | CheckRequestStage::Linked)
            ) | (
                CheckRequestMode::Documentation,
                Some(
                    CheckRequestStage::Intake
                        | CheckRequestStage::Updated
                        | CheckRequestStage::Verified
                )
            ) | (
                CheckRequestMode::Release,
                Some(CheckRequestStage::Reviewed | CheckRequestStage::Verified)
            )
        );
        if !valid {
            return Err(fault("stage", "stage does not belong to the selected mode"));
        }
        Ok(())
    }
}

impl DocumentationMap {
    /// A topic census may be empty before intake, but accepted topic proof is never vacuous.
    pub fn validate_preview(&self) -> Result<(), PreviewCheckError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if self.product_capture_id.trim().is_empty()
            || self.documentation_capture_id.trim().is_empty()
        {
            return Err(fault("captures", "both capture identities are required"));
        }
        let mut topics = BTreeSet::new();
        for topic in &self.topics {
            if topic.topic_id.trim().is_empty() || !topics.insert(topic.topic_id.as_str()) {
                return Err(fault("topics", "blank or repeated topic ID"));
            }
            if !ordered_unique(&topic.change_ids) || !ordered_unique(&topic.claim_anchors) {
                return Err(fault(
                    "topics",
                    "change and claim IDs must be unique and sorted",
                ));
            }
            if matches!(topic.source_kind, TopicSourceKind::Unbound)
                && (!matches!(topic.disposition, TopicDisposition::Pending)
                    || topic.source_page.is_some())
            {
                return Err(fault(
                    "topics.source_kind",
                    "unbound topic cannot claim an edit",
                ));
            }
            if matches!(
                topic.disposition,
                TopicDisposition::Existing | TopicDisposition::Retire | TopicDisposition::Redirect
            ) && topic
                .source_page
                .as_ref()
                .is_none_or(|path| path.trim().is_empty())
            {
                return Err(fault(
                    "topics.source_page",
                    "existing source page is required",
                ));
            }
            if matches!(
                topic.disposition,
                TopicDisposition::Create | TopicDisposition::Redirect
            ) && topic
                .target_page
                .as_ref()
                .is_none_or(|path| path.trim().is_empty())
            {
                return Err(fault("topics.target_page", "destination page is required"));
            }
            if matches!(topic.proof.state, TopicProofState::Checked)
                && (topic.proof.evidence.is_empty() || topic.proof.subject_ids.is_empty())
            {
                return Err(fault(
                    "topics.proof",
                    "checked topic needs subject and evidence",
                ));
            }
            if matches!(topic.proof.state, TopicProofState::Unavailable)
                && topic.proof.reason.trim().is_empty()
            {
                return Err(fault("topics.proof", "unavailable proof needs a reason"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> CheckRequest {
        serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-check-request/e1/valid/minimal.json"
        ))
        .unwrap()
    }

    #[test]
    fn stage_grammar_refuses_cross_mode_and_implicit_stages() {
        request().validate_preview().unwrap();
        let mut r = request();
        r.stage = Some(CheckRequestStage::Verified);
        assert_eq!(r.validate_preview().unwrap_err().field, "stage");
        r.mode = CheckRequestMode::Documentation;
        r.validate_preview().unwrap();
        r.stage = Some(CheckRequestStage::Reviewed);
        assert_eq!(r.validate_preview().unwrap_err().field, "stage");
        r.mode = CheckRequestMode::Release;
        r.validate_preview().unwrap();
    }

    #[test]
    fn document_map_needs_bound_pages_for_checked_topics() {
        let mut map: DocumentationMap = serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-documentation-map/e1/valid/minimal.json"
        ))
        .unwrap();
        map.validate_preview().unwrap();
        map.topics
            .push(crate::generated::preview::e1::documentation_map::Topic {
                topic_id: "cli-install".into(),
                change_ids: vec!["change-install".into()],
                claim_anchors: vec![],
                disposition: TopicDisposition::Existing,
                source_kind: TopicSourceKind::Authored,
                source_page: None,
                target_page: None,
                proof: crate::generated::preview::e1::documentation_map::TopicProof {
                    state: TopicProofState::Checked,
                    reason: String::new(),
                    subject_ids: vec![],
                    evidence: vec![],
                },
            });
        assert_eq!(
            map.validate_preview().unwrap_err().field,
            "topics.source_page"
        );
        map.topics[0].source_page = Some("howto/install.xml".into());
        assert_eq!(map.validate_preview().unwrap_err().field, "topics.proof");
    }
}
