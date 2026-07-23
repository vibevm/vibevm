//! Check 8 — every package in `vibe.lock` has a materialised
//! `vibedeps/` slot on disk, and `vibedeps/` carries no slot absent
//! from the lockfile (PROP-009 §2.1).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{Lockfile, SourceKind};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::LockfileFiles`] cell.
#[cell(seam = "Check", variant = "lockfile-files")]
pub struct LockfileFilesCheck;

impl Check for LockfileFilesCheck {
    fn id(&self) -> CheckId {
        CheckId::LockfileFiles
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let lockfile_path = project_root.join(Lockfile::FILENAME);
        if !lockfile_path.exists() {
            return;
        }
        let lockfile = match Lockfile::read(&lockfile_path) {
            Ok(l) => l,
            Err(_) => {
                // Surfaced by the manifest-validity cell; don't
                // double-report.
                return;
            }
        };

        // Under the loading model (PROP-009 §2.1) a package is materialised
        // verbatim into `vibedeps/<kind>-<name>/<version>/`. Check 8 verifies
        // that the lockfile and that tree agree.

        // 1. Every locked package has its `vibedeps/` slot on disk.
        let mut expected: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for pkg in &lockfile.packages {
            let slot = format!("vibedeps/{}-{}/{}", pkg.kind, pkg.name, pkg.version);
            if !project_root.join(&slot).is_dir() {
                report.err(
                    CheckId::LockfileFiles,
                    Some(PathBuf::from(&slot)),
                    None,
                    format!(
                        "lockfile entry `{}:{}@{}` has no materialised `vibedeps/` slot — \
                         run `vibe reinstall --force`",
                        pkg.kind, pkg.name, pkg.version
                    ),
                );
            }
            expected.insert(slot);
        }

        // 2. No `vibedeps/` slot absent from the lockfile — `vibe install`
        //    prunes a slot a version bump or a dropped dependency orphans.
        let vibedeps = project_root.join("vibedeps");
        if vibedeps.is_dir() {
            for kind_name in read_subdirs(&vibedeps) {
                for version in read_subdirs(&kind_name) {
                    let rel = match version.strip_prefix(project_root) {
                        Ok(r) => r.to_string_lossy().replace('\\', "/"),
                        Err(_) => continue,
                    };
                    if !expected.contains(&rel) {
                        report.warn(
                            CheckId::LockfileFiles,
                            Some(PathBuf::from(&rel)),
                            None,
                            format!(
                                "`{rel}` is a vibedeps/ slot no lockfile entry claims \
                                 (orphan) — `vibe install` prunes these"
                            ),
                        );
                    }
                }
            }
        }

        // PROP-030 §5 (reproducibility guard): a package resolved from the
        // embedded registry of a source install records source_kind =
        // "embedded" — a machine-local path a teammate or CI cannot reproduce.
        // Warn (do not fail) so a non-portable lock does not leak into a shared
        // commit unnoticed.
        let embedded: Vec<String> = lockfile
            .packages
            .iter()
            .filter(|p| p.source_kind == Some(SourceKind::Embedded))
            .map(|p| format!("{}:{}@{}", p.kind, p.name, p.version))
            .collect();
        if !embedded.is_empty() {
            report.warn(
                CheckId::LockfileFiles,
                Some(PathBuf::from(Lockfile::FILENAME)),
                None,
                format!(
                    "{} lockfile entr{} resolved from the embedded registry of a source \
                     install (source_kind = \"embedded\") and are not portable — publish \
                     or vendor these packages before sharing the lock: {}",
                    embedded.len(),
                    if embedded.len() == 1 { "y" } else { "ies" },
                    embedded.join(", ")
                ),
            );
        }
    }
}

/// Immediate sub-directories of `dir`, sorted for deterministic output.
fn read_subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    #[test]
    fn lockfile_files_missing_slot_is_an_error() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let lockfile = r#"[meta]
generated_by = "vibe-test"
generated_at = "2026-05-04T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.1.0"
source_url = "file:///fake"
content_hash = "sha256:00"
files_written = []
"#;
        fs::write(project.path().join("vibe.lock"), lockfile).unwrap();
        // No vibedeps/flow-wal/0.1.0/ slot on disk — the error.
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LockfileFiles
                    && f.severity == Severity::Error
                    && f.message.contains("no materialised")),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn lockfile_files_orphan_vibedeps_slot_warns() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // An empty lockfile, but a vibedeps/ slot on disk — orphan.
        fs::write(
            project.path().join("vibe.lock"),
            "[meta]\ngenerated_by = \"vibe-test\"\ngenerated_at = \"2026-05-04T00:00:00Z\"\nschema_version = 5\n",
        )
        .unwrap();
        fs::create_dir_all(project.path().join("vibedeps/flow-ghost/1.0.0")).unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LockfileFiles
                    && f.severity == Severity::Warning
                    && f.message.contains("orphan")),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn embedded_source_kind_entry_warns_as_non_portable() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let lockfile = r#"[meta]
generated_by = "vibe-test"
generated_at = "2026-07-13T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.1.0"
source_url = "file:///checkout/packages"
content_hash = "sha256:00"
files_written = []
source_kind = "embedded"
"#;
        fs::write(project.path().join("vibe.lock"), lockfile).unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LockfileFiles
                    && f.severity == Severity::Warning
                    && f.message.contains("not portable")),
            "got: {:?}",
            report.findings
        );
    }

    /// PROP-030 §3.3: a package resolved from the project-local `packages/`
    /// records `source_kind = "local"` — and unlike `embedded`, it is
    /// portable (every checkout of the project carries the same tree), so
    /// the reproducibility guard does NOT warn. This is the load-bearing
    /// distinction between the two local-family tiers: embedded is
    /// machine-local (warns), local is project-local (portable).
    #[test]
    fn local_source_kind_entry_is_portable_no_warn() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let lockfile = r#"[meta]
generated_by = "vibe-test"
generated_at = "2026-07-13T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.1.0"
source_url = "file:///project/packages"
content_hash = "sha256:00"
files_written = []
source_kind = "local"
"#;
        fs::write(project.path().join("vibe.lock"), lockfile).unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check == CheckId::LockfileFiles
                    && f.severity == Severity::Warning
                    && f.message.contains("not portable")),
            "a local source_kind entry is portable and must not warn; got: {:?}",
            report.findings
        );
    }
}
