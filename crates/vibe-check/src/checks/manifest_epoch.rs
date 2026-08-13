//! Check — a `vibe.toml` carrying a `[package]` table but no
//! `[package].epoch` field is flagged as pre-epoch (info), so the
//! pre-epoch population stays countable until a codemod wave rewrites
//! them (PROP-044 §6.2). Absence is not `epoch = 1`: a manifest with
//! no `epoch` is read for all time by the frozen pre-epoch reader.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::Manifest;

use super::scan_local_packages;
use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The finding text for an epoch-less package manifest — fully static
/// (nothing is substituted in), so every emission is byte-identical and
/// greppable. Absence is NOT `epoch = 1` (PROP-044 §6.2): a manifest with
/// no `epoch` is read for all time by the frozen pre-epoch reader, never
/// by a later epoch's reader that merely assumes the first.
const EPOCH_ABSENT_MSG: &str = "epoch absent (pre-epoch manifest) — `[package].epoch` is unset, so this manifest is read for all time by the frozen pre-epoch reader; absence is NOT `epoch = 1` (spec://org.vibevm.core/vibevm/common/PROP-044#our-formats). Fix: add `epoch = 1` to `[package]` when this manifest is next authored.";

/// The [`CheckId::ManifestEpoch`] cell.
///
/// Walks every locally-discoverable package manifest (the project root plus
/// each `vibe.toml` under `packages/`) and, for every one that carries a
/// `[package]` table with no `epoch` field, emits a single `Info` finding.
/// A manifest that does not parse is skipped silently — `ManifestValidity`
/// already reports the parse error, and a second finding on the same file
/// would be noise. The severity is `Info` deliberately: the panel does not
/// go red (`vibe check` exits 1 only on `Error`), and the info count is the
/// signal a later codemod wave drains.
#[cell(seam = "Check", variant = "manifest-epoch")]
pub struct ManifestEpochCheck;

impl Check for ManifestEpochCheck {
    fn id(&self) -> CheckId {
        CheckId::ManifestEpoch
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for (pkg_root, _label) in scan_local_packages(project_root) {
            let manifest_path = pkg_root.join(Manifest::FILENAME);
            let manifest = match Manifest::read(&manifest_path) {
                Ok(m) => m,
                Err(_) => continue, // ManifestValidity surfaces parse errors elsewhere.
            };
            let Some(package) = manifest.package.as_ref() else {
                continue; // Not a publishable package — no epoch to check.
            };
            if package.epoch.is_none() {
                let rel = manifest_path
                    .strip_prefix(project_root)
                    .ok()
                    .map(|p| PathBuf::from(p.display().to_string().replace('\\', "/")));
                report.info(CheckId::ManifestEpoch, rel, None, EPOCH_ABSENT_MSG);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use vibe_core::manifest::Manifest;

    use super::ManifestEpochCheck;
    use crate::test_support::{opts, write_minimal_project};
    use crate::{Check, CheckId, CheckReport, Severity};

    /// Run only this cell against `project` through the seam, returning its
    /// findings — isolating the cell from every other check.
    fn cell_findings(project: &Path) -> Vec<crate::Finding> {
        let mut report = CheckReport::default();
        ManifestEpochCheck.run(project, &opts(), &mut report);
        report.findings
    }

    /// Write a `vibe.toml` carrying a `[package]` table at the given package
    /// directory (relative to the project root).
    fn write_package_manifest(project: &Path, pkg_rel: &str, body: &str) {
        let pkg = project.join(pkg_rel);
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join(Manifest::FILENAME), body).unwrap();
    }

    #[test]
    fn project_without_a_package_table_is_silent() {
        let project = tempdir().unwrap();
        // Minimal clean project carries only `[project]`.
        write_minimal_project(project.path());
        assert!(
            cell_findings(project.path()).is_empty(),
            "expected no epoch findings on a package-less project"
        );
    }

    #[test]
    fn a_package_manifest_without_epoch_yields_one_info() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        write_package_manifest(
            project.path(),
            "packages/org.demo/x/v0.1.0",
            "[package]\ngroup = \"org.demo\"\nname = \"x\"\nkind = \"tool\"\nversion = \"0.1.0\"\n",
        );
        let findings = cell_findings(project.path());
        assert_eq!(findings.len(), 1, "got: {findings:?}");
        let f = &findings[0];
        assert_eq!(f.check, CheckId::ManifestEpoch);
        assert_eq!(f.severity, Severity::Info);
        assert!(
            f.message.contains("epoch absent (pre-epoch manifest)"),
            "got: {}",
            f.message
        );
        assert!(f.message.contains("PROP-044"), "got: {}", f.message);
        // The path is the manifest relative to the project root, with
        // forward slashes (stable across platforms).
        assert_eq!(
            f.path.as_deref().and_then(|p| p.to_str()),
            Some("packages/org.demo/x/v0.1.0/vibe.toml"),
            "got: {:?}",
            f.path
        );
    }

    #[test]
    fn a_package_manifest_with_epoch_is_silent() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        write_package_manifest(
            project.path(),
            "packages/org.demo/x/v0.1.0",
            "[package]\ngroup = \"org.demo\"\nname = \"x\"\nkind = \"tool\"\nversion = \"0.1.0\"\nepoch = 1\n",
        );
        assert!(
            cell_findings(project.path()).is_empty(),
            "an epoch-1 manifest must be silent"
        );
    }

    #[test]
    fn an_unparseable_manifest_is_left_to_manifest_validity() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        write_package_manifest(
            project.path(),
            "packages/org.demo/x/v0.1.0",
            "this is = not = toml",
        );
        assert!(
            cell_findings(project.path()).is_empty(),
            "this cell must skip an unparseable manifest — ManifestValidity owns it"
        );
    }
}
