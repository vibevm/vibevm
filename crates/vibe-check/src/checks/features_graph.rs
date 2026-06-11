//! PROP-003 §2.10 — `[features]` table is internally consistent
//! (no cycles to unknown features, no exclusive-group violations,
//! every `subskill:<path>` reference resolves on disk).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::path::Path;

use specmark::cell;
use vibe_core::manifest::Manifest;

use super::scan_local_packages;
use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::FeaturesGraph`] cell.
///
/// Walks every package available to the project (lockfile + project tree
/// `packages/`) and validates its `[features]` table. Diagnostics from
/// the resolver's [`vibe_resolver::validate_features_table`] surface as
/// warnings since a misconfigured table won't break install for
/// projects that don't activate any of its features but is still worth
/// surfacing.
#[cell(seam = "Check", variant = "features-graph")]
pub struct FeaturesGraphCheck;

impl Check for FeaturesGraphCheck {
    fn id(&self) -> CheckId {
        CheckId::FeaturesGraph
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for (pkg_root, source_label) in scan_local_packages(project_root) {
            let manifest_path = pkg_root.join(Manifest::FILENAME);
            let manifest = match Manifest::read(&manifest_path) {
                Ok(m) => m,
                Err(_) => continue, // ManifestValidity check surfaces this elsewhere.
            };
            if manifest.features.is_empty() {
                continue;
            }
            let findings = vibe_resolver::validate_features_table(&manifest.features);
            let rel = manifest_path
                .strip_prefix(project_root)
                .map(|p| p.to_path_buf())
                .ok();
            for f in findings {
                report.warn(
                    CheckId::FeaturesGraph,
                    rel.clone(),
                    None,
                    format!("[{source_label}] {f}"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, check_project};

    fn write_pkg_with_features(project: &Path, features_section: &str) {
        write_minimal_project(project);
        let pkg = project.join("packages").join("flow").join("test-pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            format!(
                r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"

[features]
{features_section}
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn features_graph_passes_clean() {
        let project = tempdir().unwrap();
        write_pkg_with_features(
            project.path(),
            r#"default = ["a"]
a = []
"#,
        );
        let report = check_project(project.path(), &opts());
        let feature_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::FeaturesGraph)
            .collect();
        assert!(
            feature_findings.is_empty(),
            "expected no features findings, got: {:?}",
            feature_findings
        );
    }

    #[test]
    fn features_graph_flags_unknown_referenced_feature() {
        let project = tempdir().unwrap();
        write_pkg_with_features(
            project.path(),
            r#"a = ["bogus-feature"]
"#,
        );
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::FeaturesGraph
                    && f.message.contains("unknown feature")),
            "expected unknown-feature finding; got: {:?}",
            report.findings
        );
    }
}
