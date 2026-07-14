//! Check 7 — `spec/boot/` exists and holds only markdown files.
//! PROP-009 retired the `NN-` filename prefix; the directory holds
//! authored boot files and `vibe`-generated `INDEX.md` / `STATIC.md`
//! artifacts, none numerically prefixed.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::Manifest;

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::BootDirectory`] cell.
#[cell(seam = "Check", variant = "boot-directory")]
pub struct BootDirectoryCheck;

impl Check for BootDirectoryCheck {
    fn id(&self) -> CheckId {
        CheckId::BootDirectory
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let boot_rel = PathBuf::from("spec/boot");
        let boot = project_root.join(&boot_rel);
        if !boot.is_dir() {
            // Empty / fresh project — `vibe init` creates it. If the
            // project's vibe.toml exists but boot/ doesn't, that's a
            // structural error.
            if project_root.join(Manifest::FILENAME).exists() {
                report.err(
                    CheckId::BootDirectory,
                    Some(boot_rel),
                    None,
                    "spec/boot/ is missing — every project owns this directory; run `vibe init` if it disappeared.",
                );
            }
            return;
        }
        let entries = match fs::read_dir(&boot) {
            Ok(e) => e,
            Err(e) => {
                report.err(
                    CheckId::BootDirectory,
                    Some(boot_rel),
                    None,
                    format!("could not list boot dir: {e}"),
                );
                return;
            }
        };
        // PROP-009 §2.5 retired the `NN-` filename prefix — `vibe` owns boot
        // ordering by category band, and the generated `INDEX.md` /
        // `STATIC.md` artifacts carry no numeric prefix. Any markdown file is
        // a valid boot file; only a non-markdown stray is worth flagging.
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            if !name.ends_with(".md") {
                report.warn(
                    CheckId::BootDirectory,
                    Some(boot_rel.join(&name)),
                    None,
                    format!("non-markdown file `{name}` in spec/boot/"),
                );
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
    fn boot_dir_accepts_the_loading_model_layout() {
        // PROP-009 §2.5 retired the `NN-` prefix: the generated INDEX.md
        // / STATIC.md and any author-named boot file are all valid.
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join("spec/boot/INDEX.md"), "schema = 1\n").unwrap();
        fs::write(project.path().join("spec/boot/STATIC.md"), "# inline\n").unwrap();
        fs::write(project.path().join("spec/boot/rules.md"), "# rules\n").unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check == CheckId::BootDirectory && f.severity == Severity::Error),
            "the loading-model boot layout must not be flagged; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn boot_dir_non_markdown_file_is_a_warning() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join("spec/boot/notes.txt"), "x").unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::BootDirectory && f.severity == Severity::Warning),
            "a non-markdown file in spec/boot/ must warn; got: {:?}",
            report.findings
        );
    }
}
