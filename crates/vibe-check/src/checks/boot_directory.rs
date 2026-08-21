//! Check 7 — `spec/boot/` exists and holds only spec-source files
//! (Markdown or dialect XML, PROP-045 ##LOADER-LAW). PROP-009 retired the
//! `NN-` filename prefix; the directory holds authored boot files and
//! `vibe`-generated `INDEX.md` / `STATIC.md` artifacts, none numerically
//! prefixed. An `.xml` boot file must parse as the dialect (a foreign
//! construct is a loud error, never a silent skip), and `X.md` + `X.xml`
//! beside each other are one document in two forms — an error naming both
//! (##TARGET-MIXED).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

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
        // `STATIC.md` artifacts carry no numeric prefix. Any spec-source
        // file is a valid boot file (PROP-045: `.md` or `.xml`); a stray
        // with another extension is worth flagging, an `.xml` that is not
        // the dialect is an error, and a document held in BOTH forms is
        // the split brain the mixed target forbids.
        let mut spec_files: Vec<PathBuf> = Vec::new();
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = entry.path();
            if !vibe_specdoc::is_spec_source(&path) {
                report.warn(
                    CheckId::BootDirectory,
                    Some(boot_rel.join(&name)),
                    None,
                    format!("non-spec-source file `{name}` in spec/boot/"),
                );
                continue;
            }
            // Format-specific validation: the XML form is a closed dialect —
            // a boot file that does not speak it would fail at read time in
            // the loader, so it fails here first, naming the defect.
            if path.extension().and_then(|e| e.to_str()) == Some("xml")
                && let Err(e) = vibe_specdoc::load_spec_text(&path)
            {
                report.err(
                    CheckId::BootDirectory,
                    Some(boot_rel.join(&name)),
                    None,
                    format!("XML boot file does not speak the dialect: {e}"),
                );
            }
            spec_files.push(path);
        }
        for collision in vibe_specdoc::pair_collisions_in(&spec_files) {
            let rel = collision
                .markdown
                .strip_prefix(project_root)
                .unwrap_or(&collision.markdown)
                .to_path_buf();
            report.err(CheckId::BootDirectory, Some(rel), None, collision.message());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

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
            "a non-spec-source file in spec/boot/ must warn; got: {:?}",
            report.findings
        );
    }

    /// PROP-045 ##LOADER-LAW: a dialect-XML boot file is a first-class
    /// authored boot file — and one that does not speak the dialect is an
    /// error, not a silent skip.
    #[test]
    fn boot_dir_accepts_dialect_xml_and_rejects_a_foreign_one() {
        let good = tempdir().unwrap();
        write_minimal_project(good.path());
        fs::write(
            good.path().join("spec/boot/rules.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
             <p><fact id=\"BOOT\" status=\"impl/done\">one rule</fact></p>\n</spec>",
        )
        .unwrap();
        let report = check_project(good.path(), &opts());
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check == CheckId::BootDirectory),
            "a dialect XML boot file must pass clean; got: {:?}",
            report.findings
        );

        let bad = tempdir().unwrap();
        write_minimal_project(bad.path());
        fs::write(
            bad.path().join("spec/boot/rules.xml"),
            "<spec><bogus>x</bogus></spec>",
        )
        .unwrap();
        let report = check_project(bad.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::BootDirectory
                    && f.severity == Severity::Error
                    && f.message.contains("dialect")),
            "a foreign XML boot file must be an error naming the dialect; got: {:?}",
            report.findings
        );
    }

    /// One document, one form (PROP-045 ##TARGET-MIXED): `X.md` + `X.xml`
    /// in spec/boot is a loud error naming both files.
    #[test]
    fn boot_dir_pair_collision_is_an_error() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(project.path().join("spec/boot/dup.md"), "# dup\n").unwrap();
        fs::write(
            project.path().join("spec/boot/dup.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        let hit = report
            .findings
            .iter()
            .find(|f| f.check == CheckId::BootDirectory && f.severity == Severity::Error)
            .expect("the pair must be an error");
        assert_eq!(hit.path.as_deref(), Some(Path::new("spec/boot/dup.md")));
        assert!(hit.message.contains("dup.md"), "{}", hit.message);
        assert!(hit.message.contains("dup.xml"), "{}", hit.message);
        assert!(hit.message.contains("one document, one form"));
    }
}
