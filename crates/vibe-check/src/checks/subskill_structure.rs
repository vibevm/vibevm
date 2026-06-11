//! PROP-003 §2.10 — every `subskills/<path>/vibe-subskill.toml` in
//! scope parses, fields are well-formed, `delivery` matches PROP-003
//! §2.5.0, and lazy-push / lazy-pull subskills carry the load-bearing
//! `description` field.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::path::Path;

use specmark::cell;

use super::scan_local_packages;
use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::SubskillStructure`] cell.
///
/// Walks every package and inspects its `subskills/` tree. For each
/// `vibe-subskill.toml`: ensure it parses, validate via
/// [`SubskillManifest::validation_findings`], and that paths declared
/// in the manifest's `[content].files_written` exist on disk relative
/// to the subskill's own directory.
///
/// [`SubskillManifest::validation_findings`]: vibe_core::manifest::SubskillManifest::validation_findings
#[cell(seam = "Check", variant = "subskill-structure")]
pub struct SubskillStructureCheck;

impl Check for SubskillStructureCheck {
    fn id(&self) -> CheckId {
        CheckId::SubskillStructure
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for (pkg_root, source_label) in scan_local_packages(project_root) {
            let subskills_dir = pkg_root.join("subskills");
            if !subskills_dir.is_dir() {
                continue;
            }
            for entry in walkdir::WalkDir::new(&subskills_dir)
                .max_depth(6)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                if entry.file_name() != vibe_core::manifest::SubskillManifest::FILENAME {
                    continue;
                }
                let manifest_path = entry.path().to_path_buf();
                let rel = manifest_path
                    .strip_prefix(project_root)
                    .map(|p| p.to_path_buf())
                    .ok();
                let manifest = match vibe_core::manifest::SubskillManifest::read(&manifest_path) {
                    Ok(m) => m,
                    Err(e) => {
                        report.err(
                            CheckId::SubskillStructure,
                            rel,
                            None,
                            format!("[{source_label}] subskill manifest fails to parse: {e}"),
                        );
                        continue;
                    }
                };
                for finding in manifest.validation_findings() {
                    report.err(
                        CheckId::SubskillStructure,
                        rel.clone(),
                        None,
                        format!("[{source_label}] {finding}"),
                    );
                }
                // PROP-003 §2.5.5: practical depth cap is 3 (`a/b/c` is
                // three components). At 4 or more, warn — the package is
                // almost certainly better split into separate packages.
                let depth = manifest.subskill.path.matches('/').count() + 1;
                if depth >= 4 {
                    report.warn(
                        CheckId::SubskillStructure,
                        rel.clone(),
                        None,
                        format!(
                            "[{source_label}] subskill `{}` nested {depth} levels deep — PROP-003 §2.5.5 caps practical depth at 3. Consider splitting into separate packages.",
                            manifest.subskill.path
                        ),
                    );
                }
                // Every declared file exists relative to the subskill's
                // own root.
                let sub_root = manifest_path.parent().unwrap_or(&manifest_path);
                for f in &manifest.content.files_written {
                    let absolute = sub_root.join(f);
                    if !absolute.is_file() {
                        report.err(
                            CheckId::SubskillStructure,
                            rel.clone(),
                            None,
                            format!(
                                "[{source_label}] subskill `{}` declares file `{}` which is missing on disk",
                                manifest.subskill.path,
                                f.display()
                            ),
                        );
                    }
                }
            }
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
    fn subskill_structure_flags_lazy_push_without_description() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"
"#,
        )
        .unwrap();
        // Subskill with delivery=lazy-push but no description.
        let sub = pkg.join("subskills/stack/rust");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join("vibe-subskill.toml"),
            r#"[subskill]
path = "stack/rust"
delivery = "lazy-push"
"#,
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report.findings.iter().any(|f| {
                f.check == CheckId::SubskillStructure
                    && f.message.contains("requires a non-empty `description`")
            }),
            "expected subskill description finding; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn subskill_structure_flags_missing_declared_file() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"
"#,
        )
        .unwrap();
        let sub = pkg.join("subskills/stack/rust");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join("vibe-subskill.toml"),
            r#"[subskill]
path = "stack/rust"

[content]
files_written = ["spec/missing.md"]
"#,
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report.findings.iter().any(|f| {
                f.check == CheckId::SubskillStructure && f.message.contains("missing on disk")
            }),
            "expected missing-file finding; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn subskill_structure_warns_on_excessive_nesting_depth() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"
"#,
        )
        .unwrap();
        // 4-level nested subskill: a/b/c/d.
        let sub = pkg.join("subskills/a/b/c/d");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join("vibe-subskill.toml"),
            r#"[subskill]
path = "a/b/c/d"
"#,
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::SubskillStructure
                    && f.severity == Severity::Warning
                    && f.message.contains("nested 4 levels deep")),
            "expected depth-4 warning; got {:?}",
            report.findings
        );
    }
}
