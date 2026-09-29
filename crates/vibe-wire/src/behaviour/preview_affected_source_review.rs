//! Pure affected-source declarations; validity establishes no actual review proof.

use std::collections::{BTreeMap, BTreeSet};

use super::preview_capture::{digest, safe_relative_path};
use crate::generated::preview::e1::affected_source_review::AffectedSourceReview;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("preview affected source review {field}: {reason}")]
pub struct AffectedSourceReviewError {
    pub field: &'static str,
    pub reason: &'static str,
}

fn fault(field: &'static str, reason: &'static str) -> AffectedSourceReviewError {
    AffectedSourceReviewError { field, reason }
}

fn path(value: &str) -> bool {
    safe_relative_path(value) && !value.contains('\0')
}

fn names(values: &[String], field: &'static str) -> Result<(), AffectedSourceReviewError> {
    let mut unique = BTreeSet::new();
    if values
        .iter()
        .any(|value| value.trim().is_empty() || !unique.insert(value))
    {
        return Err(fault(field, "blank or repeated identity"));
    }
    Ok(())
}

struct CurrentIdentity<'a> {
    sha256: &'a str,
    blob: Option<&'a str>,
}

fn retain_current<'a>(
    current: &mut BTreeMap<&'a str, CurrentIdentity<'a>>,
    location: &'a str,
    sha256: &'a str,
    blob: Option<&'a str>,
) -> Result<(), AffectedSourceReviewError> {
    if let Some(previous) = current.get_mut(location) {
        if previous.sha256 != sha256
            || previous
                .blob
                .zip(blob)
                .is_some_and(|(left, right)| left != right)
        {
            return Err(fault(
                "current_identity",
                "one current path names conflicting byte identities",
            ));
        }
        if previous.blob.is_none() {
            previous.blob = blob;
        }
    } else {
        current.insert(location, CurrentIdentity { sha256, blob });
    }
    Ok(())
}

impl AffectedSourceReview {
    /// Check declaration/reference consistency without reading bytes or native sources.
    ///
    /// ```
    /// use vibe_wire::generated::preview::e1::affected_source_review::AffectedSourceReview;
    /// let review: AffectedSourceReview = serde_json::from_str(r#"{
    ///   "schema":1,"review_id":"empty declared scope","before_capture_id":"before",
    ///   "after_capture_id":"after","source_record":"evidence/review.md",
    ///   "source_record_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ///   "paths":[],"interfaces":[],"topics":[],"contract_sources":[]
    /// }"#).unwrap();
    /// review.validate_affected_source_review().unwrap();
    /// // A valid empty declaration does not prove the actual delta is empty.
    /// ```
    pub fn validate_affected_source_review(&self) -> Result<(), AffectedSourceReviewError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if [
            &self.review_id,
            &self.before_capture_id,
            &self.after_capture_id,
        ]
        .iter()
        .any(|identity| identity.trim().is_empty())
        {
            return Err(fault(
                "identity",
                "review and capture identities must be nonblank",
            ));
        }
        if !path(&self.source_record) || !digest(&self.source_record_sha256, 64) {
            return Err(fault(
                "source_record",
                "invalid retained review source reference",
            ));
        }
        let mut paths = BTreeMap::new();
        let mut current = BTreeMap::new();
        for binding in &self.paths {
            if !path(&binding.path) || paths.insert(binding.path.as_str(), binding).is_some() {
                return Err(fault("paths", "unsafe or repeated product path"));
            }
            for identity in binding.before.iter().chain(binding.after.iter()) {
                if !digest(&identity.git_blob_sha1, 40) || !digest(&identity.sha256, 64) {
                    return Err(fault("paths.identity", "invalid endpoint byte identity"));
                }
            }
            if let Some(after) = &binding.after {
                retain_current(
                    &mut current,
                    &binding.path,
                    &after.sha256,
                    Some(&after.git_blob_sha1),
                )?;
            }
            names(&binding.interface_ids, "paths.interface_ids")?;
            if binding.interface_ids.is_empty() {
                if binding
                    .user_no_impact_reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
                    || binding
                        .documentation_no_impact_reason
                        .as_ref()
                        .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(fault(
                        "paths.no_impact",
                        "unmapped path requires separate user and documentation reasons",
                    ));
                }
            } else if binding.user_no_impact_reason.is_some()
                || binding.documentation_no_impact_reason.is_some()
            {
                return Err(fault(
                    "paths.no_impact",
                    "mapped path cannot carry no-impact reasons",
                ));
            }
        }
        let mut topics = BTreeSet::new();
        for topic in &self.topics {
            if topic.topic_id.trim().is_empty()
                || topic.obligation.trim().is_empty()
                || !topics.insert(topic.topic_id.as_str())
            {
                return Err(fault("topics", "blank or repeated topic obligation"));
            }
        }
        let mut interfaces = BTreeMap::new();
        let mut required_topics = BTreeSet::new();
        for interface in &self.interfaces {
            if [
                &interface.interface_id,
                &interface.capability_id,
                &interface.audience,
                &interface.platform,
                &interface.before,
                &interface.after,
            ]
            .iter()
            .any(|value| value.trim().is_empty())
                || interfaces
                    .insert(interface.interface_id.as_str(), interface)
                    .is_some()
            {
                return Err(fault(
                    "interfaces",
                    "blank scope or repeated interface identity",
                ));
            }
            names(&interface.source_paths, "interfaces.source_paths")?;
            if interface.source_paths.is_empty() {
                return Err(fault(
                    "interfaces.source_paths",
                    "interface requires product source paths",
                ));
            }
            for source in &interface.source_paths {
                let Some(binding) = paths.get(source.as_str()) else {
                    return Err(fault(
                        "interfaces.source_paths",
                        "interface names an unknown product path",
                    ));
                };
                if !path(source) || !binding.interface_ids.contains(&interface.interface_id) {
                    return Err(fault(
                        "interfaces.source_paths",
                        "product path membership is not reciprocal",
                    ));
                }
            }
            names(&interface.contract_anchors, "interfaces.contract_anchors")?;
            if interface.contract_anchors.is_empty()
                || interface
                    .contract_anchors
                    .iter()
                    .any(|anchor| !anchor.starts_with("spec://") || anchor.contains("/next/"))
            {
                return Err(fault(
                    "interfaces.contract_anchors",
                    "interface requires permanent contract references",
                ));
            }
            names(&interface.topic_ids, "interfaces.topic_ids")?;
            if interface.topic_ids.is_empty()
                || interface
                    .topic_ids
                    .iter()
                    .any(|id| !topics.contains(id.as_str()))
            {
                return Err(fault(
                    "interfaces.topic_ids",
                    "interface requires known topic obligations",
                ));
            }
            required_topics.extend(interface.topic_ids.iter().map(String::as_str));
        }
        for binding in &self.paths {
            for identity in &binding.interface_ids {
                let Some(interface) = interfaces.get(identity.as_str()) else {
                    return Err(fault(
                        "paths.interface_ids",
                        "product path names an unknown interface",
                    ));
                };
                if !interface.source_paths.contains(&binding.path) {
                    return Err(fault(
                        "paths.interface_ids",
                        "interface path membership is not reciprocal",
                    ));
                }
            }
        }
        if topics != required_topics {
            return Err(fault("topics", "unreferenced topic obligation"));
        }
        let mut sources = BTreeSet::new();
        for source in &self.contract_sources {
            if source.group.trim().is_empty()
                || source.name.trim().is_empty()
                || source.source_profile.trim().is_empty()
                || !sources.insert((source.group.as_str(), source.name.as_str()))
            {
                return Err(fault(
                    "contract_sources",
                    "blank or repeated contract source authority",
                ));
            }
            if !path(&source.manifest_path) || !digest(&source.manifest_sha256, 64) {
                return Err(fault(
                    "contract_sources.manifest",
                    "invalid manifest reference",
                ));
            }
            if paths
                .get(source.manifest_path.as_str())
                .is_some_and(|binding| binding.after.is_none())
            {
                return Err(fault(
                    "current_identity",
                    "current manifest contradicts declared endpoint absence",
                ));
            }
            retain_current(
                &mut current,
                &source.manifest_path,
                &source.manifest_sha256,
                None,
            )?;
            if source
                .spec_root_components
                .iter()
                .any(|component| !path(component) || component.contains('/'))
            {
                return Err(fault(
                    "contract_sources.spec_root_components",
                    "unsafe specification root component",
                ));
            }
            let root = source.spec_root_components.join("/");
            let mut documents = BTreeSet::new();
            for document in &source.documents {
                if !path(&document.path)
                    || !documents.insert(document.path.as_str())
                    || (!root.is_empty() && !document.path.starts_with(&format!("{root}/")))
                    || !digest(&document.git_blob_sha1, 40)
                    || !digest(&document.sha256, 64)
                {
                    return Err(fault(
                        "contract_sources.documents",
                        "unsafe, repeated, out-of-root or invalid contract document",
                    ));
                }
                if paths
                    .get(document.path.as_str())
                    .is_some_and(|binding| binding.after.is_none())
                {
                    return Err(fault(
                        "current_identity",
                        "current contract document contradicts declared endpoint absence",
                    ));
                }
                retain_current(
                    &mut current,
                    &document.path,
                    &document.sha256,
                    Some(&document.git_blob_sha1),
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "preview_affected_source_review/tests.rs"]
mod tests;
