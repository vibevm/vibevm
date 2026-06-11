//! Check 1 — `vibe.toml` parses and matches schema; `vibe.lock` (if
//! present) parses and matches schema.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{Lockfile, Manifest};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::ManifestValidity`] cell.
#[cell(seam = "Check", variant = "manifest-validity")]
pub struct ManifestValidityCheck;

impl Check for ManifestValidityCheck {
    fn id(&self) -> CheckId {
        CheckId::ManifestValidity
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let manifest_path = project_root.join(Manifest::FILENAME);
        if !manifest_path.exists() {
            report.err(
                CheckId::ManifestValidity,
                Some(PathBuf::from(Manifest::FILENAME)),
                None,
                format!(
                    "no `{}` in project root — every vibevm project carries one. Run `vibe init`.",
                    Manifest::FILENAME
                ),
            );
            return;
        }
        if let Err(e) = Manifest::read(&manifest_path) {
            report.err(
                CheckId::ManifestValidity,
                Some(PathBuf::from(Manifest::FILENAME)),
                None,
                format!("`{}` failed to parse: {e}", Manifest::FILENAME),
            );
        }

        let lockfile_path = project_root.join(Lockfile::FILENAME);
        if !lockfile_path.exists() {
            // Empty project — fine. `vibe install` will create one.
            return;
        }
        if let Err(e) = Lockfile::read(&lockfile_path) {
            report.err(
                CheckId::ManifestValidity,
                Some(PathBuf::from(Lockfile::FILENAME)),
                None,
                format!("`{}` failed to parse: {e}", Lockfile::FILENAME),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    #[test]
    fn missing_vibe_toml_is_an_error() {
        let project = tempdir().unwrap();
        // No vibe.toml.
        fs::create_dir_all(project.path().join("spec/boot")).unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::ManifestValidity && f.severity == Severity::Error),
            "expected ManifestValidity error; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn malformed_vibe_toml_is_an_error() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join("vibe.toml"), "this is = not = toml").unwrap();
        let report = check_project(project.path(), &opts());
        assert!(report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::ManifestValidity
                    && f.message.contains("failed to parse")),
        );
    }
}
