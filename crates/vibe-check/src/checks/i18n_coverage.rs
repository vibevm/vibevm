//! PROP-003 §2.10 — every file declared in `[content].files_written`
//! exists for the package's canonical language; missing translations
//! for languages declared in `[i18n].available` surface as warnings.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::path::Path;

use specmark::cell;
use vibe_core::manifest::Manifest;

use super::scan_local_packages;
use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::I18nCoverage`] cell.
///
/// For every package with an `[i18n].available` declaration, every
/// file in `[writes]` plus boot snippet must exist for the canonical
/// language (error) and SHOULD exist for every other listed language
/// (warning when missing).
#[cell(seam = "Check", variant = "i18n-coverage")]
pub struct I18nCoverageCheck;

impl Check for I18nCoverageCheck {
    fn id(&self) -> CheckId {
        CheckId::I18nCoverage
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for (pkg_root, source_label) in scan_local_packages(project_root) {
            let manifest_path = pkg_root.join(Manifest::FILENAME);
            let manifest = match Manifest::read(&manifest_path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if manifest.i18n.available.is_empty() {
                continue;
            }
            let canonical = &manifest.i18n.canonical;
            // PROP-009 retired `[writes]`; a package's only manifest-declared
            // canonical path is now its `[boot_snippet]` source.
            let logical_paths: Vec<std::path::PathBuf> = manifest
                .boot_snippet
                .as_ref()
                .map(|b| b.source.clone())
                .into_iter()
                .collect();
            let rel_manifest = manifest_path
                .strip_prefix(project_root)
                .map(|p| p.to_path_buf())
                .ok();
            // Canonical must exist.
            for logical in &logical_paths {
                let abs = pkg_root.join(logical);
                if !abs.is_file() {
                    report.err(
                        CheckId::I18nCoverage,
                        rel_manifest.clone(),
                        None,
                        format!(
                            "[{source_label}] canonical `{}` (language `{}`) missing for declared file `{}`",
                            logical.display(),
                            canonical,
                            logical.display()
                        ),
                    );
                }
            }
            // Every other declared language: warn on missing translation.
            for lang in &manifest.i18n.available {
                if lang == canonical {
                    continue;
                }
                for logical in &logical_paths {
                    let localised = vibe_core::manifest::i18n::localised_path(logical, lang);
                    let abs = pkg_root.join(&localised);
                    if !abs.is_file() {
                        report.warn(
                            CheckId::I18nCoverage,
                            rel_manifest.clone(),
                            None,
                            format!(
                                "[{source_label}] no `{}` translation of `{}` (looked for `{}`)",
                                lang,
                                logical.display(),
                                localised.display()
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
    use crate::{CheckId, check_project};

    #[test]
    fn i18n_coverage_warns_on_missing_translation() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(pkg.join("boot")).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"

[i18n]
canonical = "en"
available = ["en", "ru"]

[boot_snippet]
source = "boot/x.md"
category = "flow"
"#,
        )
        .unwrap();
        fs::write(pkg.join("boot/x.md"), "EN content").unwrap();
        // Russian sidecar deliberately missing.
        let report = check_project(project.path(), &opts());
        let warns: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::I18nCoverage)
            .collect();
        assert!(
            !warns.is_empty(),
            "expected i18n coverage warning; got: {:?}",
            report.findings
        );
        assert!(warns.iter().any(|f| f.message.contains("ru")));
    }

    #[test]
    fn i18n_coverage_clean_when_all_translations_present() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let pkg = project
            .path()
            .join("packages")
            .join("flow")
            .join("test-pkg");
        fs::create_dir_all(pkg.join("boot")).unwrap();
        fs::write(
            pkg.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "test-pkg"
kind = "flow"
version = "0.1.0"

[i18n]
canonical = "en"
available = ["en", "ru"]

[boot_snippet]
source = "boot/x.md"
category = "flow"
"#,
        )
        .unwrap();
        fs::write(pkg.join("boot/x.md"), "EN content").unwrap();
        fs::write(pkg.join("boot/x.ru.md"), "RU content").unwrap();
        let report = check_project(project.path(), &opts());
        let i18n_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::I18nCoverage)
            .collect();
        assert!(
            i18n_findings.is_empty(),
            "expected no i18n findings; got: {:?}",
            i18n_findings
        );
    }
}
