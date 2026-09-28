//! Pure checks over a generated preview source capture.

use std::collections::BTreeSet;

use crate::generated::preview::e1::capture::Capture;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("preview capture {field}: {reason}")]
pub struct PreviewCaptureError {
    pub field: &'static str,
    pub reason: &'static str,
}

fn fault(field: &'static str, reason: &'static str) -> PreviewCaptureError {
    PreviewCaptureError { field, reason }
}

fn digest(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl Capture {
    /// Validate local consistency before checking referenced Git objects.
    pub fn validate_preview(&self) -> Result<(), PreviewCaptureError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if self.capture_id.trim().is_empty() {
            return Err(fault("capture_id", "empty identity"));
        }
        if !digest(&self.repository_commit_sha1, 40) || !digest(&self.repository_tree_sha1, 40) {
            return Err(fault("repository", "invalid Git identity"));
        }
        if !safe_relative_path(&self.source_record) || !digest(&self.source_record_sha256, 64) {
            return Err(fault("source_record", "invalid raw record reference"));
        }
        if self.packages.is_empty() || self.files.is_empty() || self.surfaces.is_empty() {
            return Err(fault("inventory", "source inventory is empty"));
        }

        let mut packages = BTreeSet::new();
        for package in &self.packages {
            if package.coordinate.trim().is_empty()
                || package.kind.trim().is_empty()
                || !safe_relative_path(&package.slot)
                || !digest(&package.git_tree_sha1, 40)
                || !digest(&package.manifest_sha256, 64)
                || !packages.insert(package.coordinate.as_str())
            {
                return Err(fault("packages", "invalid or repeated package identity"));
            }
        }
        let mut files = BTreeSet::new();
        for file in &self.files {
            if !safe_relative_path(&file.path)
                || !digest(&file.sha256, 64)
                || !digest(&file.git_blob_sha1, 40)
                || !files.insert(file.path.as_str())
            {
                return Err(fault("files", "invalid or repeated file identity"));
            }
        }
        let mut surfaces = BTreeSet::new();
        for surface in &self.surfaces {
            if !safe_relative_path(&surface.source)
                || !digest(&surface.source_sha256, 64)
                || !files.contains(surface.source.as_str())
                || surface.identifiers.is_empty()
                || !surfaces.insert(surface.source.as_str())
            {
                return Err(fault(
                    "surfaces",
                    "source-declared surface lacks exact file identity",
                ));
            }
            if self
                .files
                .iter()
                .find(|file| file.path == surface.source)
                .map(|file| file.sha256.as_str())
                != Some(surface.source_sha256.as_str())
            {
                return Err(fault(
                    "surfaces",
                    "surface source hash differs from file capture",
                ));
            }
            let mut names = BTreeSet::new();
            if surface
                .identifiers
                .iter()
                .any(|name| name.trim().is_empty() || !names.insert(name))
            {
                return Err(fault(
                    "surfaces.identifiers",
                    "blank or repeated declaration",
                ));
            }
        }
        let mut toolchains = BTreeSet::new();
        for toolchain in &self.toolchains {
            if toolchain.name.trim().is_empty()
                || toolchain.version.trim().is_empty()
                || !toolchains.insert(toolchain.name.as_str())
            {
                return Err(fault("toolchains", "blank or repeated toolchain"));
            }
        }
        let mut exclusions = BTreeSet::new();
        for exclusion in &self.excluded_from_product_delta {
            if !safe_relative_path(exclusion.path.trim_end_matches('/'))
                || exclusion.reason.trim().is_empty()
                || !exclusions.insert(exclusion.path.as_str())
            {
                return Err(fault(
                    "excluded_from_product_delta",
                    "invalid or repeated exclusion",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dp1() -> Capture {
        serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-capture/e1/valid/dp1.json"
        ))
        .expect("registered preview capture corpus must parse")
    }

    #[test]
    fn captured_source_and_unavailable_artifacts_are_explicit() {
        let capture = dp1();
        capture.validate_preview().unwrap();
        assert_eq!(capture.packages.len(), 48);
        assert!(matches!(
            capture.artifact_state,
            crate::generated::preview::e1::capture::CaptureArtifactState::Unavailable
        ));
    }

    #[test]
    fn wrong_hash_type_and_unknown_field_are_refused_by_generated_reader() {
        let mut wrong_type: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-capture/e1/valid/dp1.json"
        ))
        .unwrap();
        wrong_type["packages"][0]["git_tree_sha1"] = serde_json::json!(17);
        assert!(serde_json::from_value::<Capture>(wrong_type).is_err());

        let mut unknown_field: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-capture/e1/valid/dp1.json"
        ))
        .unwrap();
        unknown_field["unregistered_claim"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Capture>(unknown_field).is_err());
    }

    #[test]
    fn duplicate_package_and_wrong_surface_hash_are_refused() {
        let mut capture = dp1();
        capture.packages.push(capture.packages[0].clone());
        assert_eq!(capture.validate_preview().unwrap_err().field, "packages");

        let mut capture = dp1();
        capture.surfaces[0].source_sha256 = "0".repeat(64);
        assert_eq!(capture.validate_preview().unwrap_err().field, "surfaces");
    }

    #[test]
    fn parent_path_and_unsupported_epoch_are_refused() {
        let mut capture = dp1();
        capture.source_record = "../outside.json".into();
        assert_eq!(
            capture.validate_preview().unwrap_err().field,
            "source_record"
        );

        let mut capture = dp1();
        capture.schema = 2;
        assert_eq!(capture.validate_preview().unwrap_err().field, "schema");
    }
}
