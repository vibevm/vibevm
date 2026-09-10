//! Resolve one configured project file without granting filesystem authority.
//! The declared spelling is data: it may select an existing regular file
//! below the canonical project root, never an absolute path or traversal.

use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Maximum displayed characters from an invalid configured path. Diagnostics
/// name the rejected declaration without allowing an unbounded/control-filled
/// config value to become the warning surface.
const MAX_DECLARED_DISPLAY: usize = 160;

pub(super) struct ContainedProjectFile {
    pub path: PathBuf,
    pub provenance: String,
}

pub(super) struct ContainmentError {
    reason: String,
}

impl ContainmentError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ContainmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

/// A bounded, single-line rendering of the authored declaration for warnings.
pub(super) fn bounded_declared_path(declared: &str) -> String {
    if declared.is_empty() {
        return "<empty>".to_string();
    }
    let mut display = String::new();
    let mut truncated = false;
    for character in declared.chars() {
        if display.chars().count() == MAX_DECLARED_DISPLAY {
            truncated = true;
            break;
        }
        display.push(if character.is_control() {
            '�'
        } else {
            character
        });
    }
    if truncated {
        display.push('…');
    }
    display
}

/// Resolve `declared` against `root` as one contained, existing regular file.
/// Both endpoints are canonicalised before the component-aware containment
/// check, so a symlink cannot turn a project-relative spelling into an import.
pub(super) fn resolve_project_file(
    root: &Path,
    declared: &str,
) -> Result<ContainedProjectFile, ContainmentError> {
    let declared_path = Path::new(declared);
    if declared_path.is_absolute() {
        return Err(ContainmentError::new("absolute paths are not allowed"));
    }

    let mut relative = PathBuf::new();
    for component in declared_path.components() {
        match component {
            Component::Normal(segment) => relative.push(segment),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(ContainmentError::new(
                    "parent traversal (`..`) is not allowed",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ContainmentError::new(
                    "rooted or prefixed paths are not allowed",
                ));
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(ContainmentError::new("path names no project file"));
    }

    let canonical_root = std::fs::canonicalize(root).map_err(|error| {
        ContainmentError::new(format!("project root cannot be canonicalised: {error}"))
    })?;
    let candidate = canonical_root.join(&relative);
    let canonical_candidate = std::fs::canonicalize(&candidate).map_err(|error| {
        ContainmentError::new(format!("candidate cannot be canonicalised: {error}"))
    })?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(ContainmentError::new(
            "canonical target escapes the project root",
        ));
    }
    let metadata = std::fs::metadata(&canonical_candidate).map_err(|error| {
        ContainmentError::new(format!("candidate metadata cannot be read: {error}"))
    })?;
    if !metadata.file_type().is_file() {
        return Err(ContainmentError::new(
            "canonical target is not a regular file",
        ));
    }
    let canonical_relative = canonical_candidate
        .strip_prefix(&canonical_root)
        .map_err(|_| ContainmentError::new("canonical containment could not be proven"))?;
    let provenance = crate::fwd(canonical_relative);
    if provenance.is_empty() {
        return Err(ContainmentError::new(
            "canonical target has no project-relative provenance",
        ));
    }
    Ok(ContainedProjectFile {
        path: canonical_candidate,
        provenance,
    })
}

#[cfg(test)]
mod tests;
