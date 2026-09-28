//! Pure retained documentation declaration/reference consistency, not intake proof.

use super::preview_capture::{digest, safe_relative_path};
use crate::generated::preview::e1::documentation_capture::{DocumentationCapture, SourceSubject};
use std::collections::{BTreeMap, BTreeSet};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-CAPTURE-MODEL");

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("preview documentation capture {field}: {reason}")]
pub struct DocumentationCaptureError {
    pub field: &'static str,
    pub reason: &'static str,
}

fn fault(field: &'static str, reason: &'static str) -> DocumentationCaptureError {
    DocumentationCaptureError { field, reason }
}

// The owning path grammar is shared; this input additionally forbids NUL.
fn path(value: &str) -> bool {
    safe_relative_path(value) && !value.contains('\0')
}

fn byte_identity(
    sha256: &str,
    blob: Option<&str>,
    git: bool,
    field: &'static str,
) -> Result<(), DocumentationCaptureError> {
    if !digest(sha256, 64)
        || blob.is_some_and(|value| !digest(value, 40))
        || (git && blob.is_none())
    {
        return Err(fault(field, "invalid byte identity or missing Git blob"));
    }
    Ok(())
}

struct ByteIdentity<'a> {
    sha256: &'a str,
    blob: Option<&'a str>,
}

fn retain_identity<'a>(
    index: &mut BTreeMap<String, ByteIdentity<'a>>,
    location: String,
    sha256: &'a str,
    blob: Option<&'a str>,
) -> Result<(), DocumentationCaptureError> {
    if let Some(previous) = index.get_mut(&location) {
        if previous.sha256 != sha256
            || previous
                .blob
                .zip(blob)
                .is_some_and(|(left, right)| left != right)
        {
            return Err(fault(
                "byte_identity",
                "one repository path names conflicting byte identities",
            ));
        }
        if previous.blob.is_none() {
            previous.blob = blob;
        }
    } else {
        index.insert(location, ByteIdentity { sha256, blob });
    }
    Ok(())
}

impl DocumentationCapture {
    /// Check retained value and reference invariants without opening sources.
    /// Foreign profiles/recipes and unresolved page ownership remain declarations.
    pub fn validate_documentation_capture(&self) -> Result<(), DocumentationCaptureError> {
        if self.schema != 1 {
            return Err(fault("schema", "unsupported epoch"));
        }
        if self.capture_id.trim().is_empty()
            || self.product_baseline_capture_id.trim().is_empty()
            || self.source_profile.trim().is_empty()
        {
            return Err(fault(
                "identity",
                "capture, product baseline and source profile must be nonblank",
            ));
        }
        if !path(&self.source_record) || !digest(&self.source_record_sha256, 64) {
            return Err(fault("source_record", "invalid raw observation reference"));
        }
        if self
            .source_root_components
            .iter()
            .any(|component| !path(component) || component.contains('/'))
        {
            return Err(fault(
                "source_root_components",
                "unsafe directory component",
            ));
        }
        let git = match &self.source_subject {
            SourceSubject::Git(subject) => {
                if subject.repository_key.trim().is_empty()
                    || subject.repository_identity.trim().is_empty()
                    || !digest(&subject.commit_sha1, 40)
                    || !digest(&subject.tree_sha1, 40)
                {
                    return Err(fault("source_subject", "invalid Git source declaration"));
                }
                true
            }
            SourceSubject::Content(subject) => {
                if subject.repository_key.trim().is_empty()
                    || subject.repository_identity.trim().is_empty()
                    || subject.recipe_id.trim().is_empty()
                    || !digest(&subject.tree_sha256, 64)
                {
                    return Err(fault(
                        "source_subject",
                        "invalid content source declaration",
                    ));
                }
                false
            }
        };
        let prefix = self.source_root_components.join("/");
        let mut files = BTreeSet::new();
        let mut identities = BTreeMap::new();
        for file in &self.files {
            if !path(&file.path) || !files.insert(file.path.as_str()) {
                return Err(fault("files", "unsafe or repeated source file path"));
            }
            byte_identity(&file.sha256, file.git_blob_sha1.as_deref(), git, "files")?;
            let repository_path = if prefix.is_empty() {
                file.path.clone()
            } else {
                format!("{prefix}/{}", file.path)
            };
            retain_identity(
                &mut identities,
                repository_path,
                &file.sha256,
                file.git_blob_sha1.as_deref(),
            )?;
        }
        let mut roles = BTreeMap::<&str, Vec<_>>::new();
        for input in &self.build_inputs {
            if !path(&input.path) {
                return Err(fault("build_inputs", "unsafe build input path"));
            }
            byte_identity(
                &input.sha256,
                input.git_blob_sha1.as_deref(),
                git,
                "build_inputs",
            )?;
            let purposes = roles.entry(input.path.as_str()).or_default();
            if purposes.contains(&&input.purpose) {
                return Err(fault("build_inputs", "repeated path and purpose"));
            }
            purposes.push(&input.purpose);
            retain_identity(
                &mut identities,
                input.path.clone(),
                &input.sha256,
                input.git_blob_sha1.as_deref(),
            )?;
        }
        let mut exclusions = BTreeSet::new();
        for exclusion in &self.exclusions {
            let relative = exclusion.path.strip_suffix('/').unwrap_or(&exclusion.path);
            if !path(relative)
                || exclusion.reason.trim().is_empty()
                || !exclusions.insert(exclusion.path.as_str())
            {
                return Err(fault(
                    "exclusions",
                    "unsafe, unreasoned or repeated exclusion",
                ));
            }
        }
        let mut pages = BTreeSet::new();
        for page in &self.pages {
            if !path(&page.path)
                || !pages.insert(page.path.as_str())
                || !files.contains(page.path.as_str())
            {
                return Err(fault("pages", "unsafe, repeated or uncaptured page"));
            }
            if self.exclusions.iter().any(|exclusion| {
                if exclusion.path.ends_with('/') {
                    page.path.starts_with(&exclusion.path)
                } else {
                    page.path == exclusion.path
                }
            }) {
                return Err(fault("pages", "selected page is also excluded"));
            }
            let mut links = BTreeSet::new();
            for source in &page.generated_from {
                if !path(source)
                    || !links.insert(source.as_str())
                    || !identities.contains_key(source)
                {
                    return Err(fault(
                        "pages.generated_from",
                        "unsafe, repeated or uncaptured source link",
                    ));
                }
            }
        }
        let mut names = BTreeSet::new();
        for toolchain in &self.toolchains {
            if toolchain.name.trim().is_empty()
                || toolchain.version.trim().is_empty()
                || !names.insert(toolchain.name.as_str())
            {
                return Err(fault(
                    "toolchains",
                    "blank or repeated toolchain declaration",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "preview_documentation_capture/tests.rs"]
mod tests;
