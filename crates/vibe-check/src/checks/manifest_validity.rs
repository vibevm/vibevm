//! Check 1 — `vibe.toml` parses and matches schema; `vibe.lock` (if
//! present) parses and matches schema.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

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
            // `e`'s `Display` is already a complete, self-contained sentence —
            // "failed to parse `vibe.toml`: <diagnosis> (violates …; fix: …)" —
            // so it is surfaced verbatim rather than re-wrapped (which used to
            // double the "failed to parse" framing and the filename).
            report.err(
                CheckId::ManifestValidity,
                Some(PathBuf::from(Manifest::FILENAME)),
                None,
                format!("{e}"),
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
                format!("{e}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, CheckReport, Severity, check_project};

    #[test]
    fn missing_vibe_toml_is_an_error() {
        let project = tempdir().unwrap();
        // No vibe.toml.
        fs::create_dir_all(project.path().join(vibe_core::layout::current_boot_dir())).unwrap();
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
        let msg = manifest_msg(&report);
        // The "failed to parse" marker is preserved (the existing degenerate
        // contract for this cell).
        assert!(msg.contains("failed to parse"), "{msg}");
        // The parser's diagnosis reaches the reader, while the safe wrapper
        // introduced with the provider seam never echoes the authored source
        // line (which may contain a secret).
        assert!(
            msg.contains("key with no value") && msg.contains("expected `=`"),
            "parser diagnosis missing: {msg}"
        );
        assert!(
            !msg.contains("this is = not = toml"),
            "raw manifest source leaked: {msg}"
        );
        assert!(
            msg.contains("line") && msg.contains("column"),
            "a syntax error must carry a position: {msg}"
        );
        assert!(
            msg.contains("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema"),
            "REQ citation missing: {msg}"
        );
    }

    #[test]
    fn missing_required_field_is_diagnosed_not_mislabelled_syntax() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // Syntactically valid TOML; `[package]` is missing the required
        // `group`. The diagnosis must name the field and must NOT advise
        // repairing TOML syntax — the bug this cell now closes.
        fs::write(
            project.path().join("vibe.toml"),
            "[package]\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(report.has_errors());
        let msg = manifest_msg(&report);
        assert!(msg.contains("missing field"), "must name the field: {msg}");
        assert!(msg.contains("group"), "must name `group`: {msg}");
        assert!(
            !msg.to_lowercase().contains("repair the toml syntax"),
            "a missing field is not a syntax error: {msg}"
        );
        assert!(
            msg.contains("add the missing field"),
            "remedy must be to add the field: {msg}"
        );
        assert!(
            msg.contains("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema"),
            "REQ citation missing: {msg}"
        );
    }

    /// The single `ManifestValidity` finding's message, panicking with the full
    /// report if the cell produced no such finding.
    fn manifest_msg(report: &CheckReport) -> &str {
        report
            .findings
            .iter()
            .find(|f| f.check == CheckId::ManifestValidity)
            .map(|f| f.message.as_str())
            .unwrap_or_else(|| panic!("no ManifestValidity finding: {:?}", report.findings))
    }
}
