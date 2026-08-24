//! Check 7 — the boot directory (`current_boot_dir()`, PROP-052 L2)
//! exists and holds only spec-source files (Markdown or dialect XML,
//! PROP-045 ##LOADER-LAW). PROP-009 retired the `NN-` filename prefix;
//! the directory holds authored boot files and `vibe`-generated
//! `INDEX.md` / `STATIC.*` artifacts, none numerically prefixed. An
//! `.xml` boot file must parse as the dialect (a foreign construct is a
//! loud error, never a silent skip), and `X.md` + `X.xml` beside each
//! other are one document in two forms — an error naming both
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
        let boot_rel = vibe_core::layout::current_boot_dir();
        // Message-facing label with `/` separators (a joined PathBuf
        // renders `\` on Windows; the finding text has always been
        // forward-slashed).
        let boot_label = boot_rel.to_string_lossy().replace('\\', "/");
        let boot = project_root.join(&boot_rel);
        if !boot.is_dir() {
            // Empty / fresh project — `vibe init` creates it. If the
            // project's vibe.toml exists but boot/ doesn't, that's a
            // structural error.
            if project_root.join(Manifest::FILENAME).exists() {
                // The message names the project-relative boot dir (the
                // seam's name, so the R4 flip re-labels it for free) —
                // computed before `boot_rel` moves into the finding.
                let message = format!(
                    "{boot_label}/ is missing — every project owns this directory; run `vibe \
                     init` if it disappeared."
                );
                report.err(CheckId::BootDirectory, Some(boot_rel), None, message);
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
        let mut has_static_md = false;
        let mut has_static_xml = false;
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = entry.path();
            // The generated XML lane is intentionally a provenance-framed
            // stream of one XML document per contribution, not one root
            // document. Both generated names are therefore recognized before
            // authored-source dialect validation and pair-collision checks.
            if name == "STATIC.md" || name == "STATIC.xml" {
                has_static_md |= name == "STATIC.md";
                has_static_xml |= name == "STATIC.xml";
                continue;
            }
            if !vibe_specdoc::is_spec_source(&path) {
                report.warn(
                    CheckId::BootDirectory,
                    Some(boot_rel.join(&name)),
                    None,
                    format!("non-spec-source file `{name}` in {boot_label}/"),
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
        if has_static_md && has_static_xml {
            report.err(
                CheckId::BootDirectory,
                Some(boot_rel.clone()),
                None,
                "both STATIC.md and STATIC.xml exist — the generator owns one; delete the stray",
            );
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

    use tempfile::tempdir;
    use vibe_core::layout;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    #[test]
    fn boot_dir_accepts_the_loading_model_layout() {
        // PROP-009 §2.5 retired the `NN-` prefix: the generated INDEX.md
        // / STATIC.md and any author-named boot file are all valid.
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join(layout::current_boot_index()),
            "schema = 1\n",
        )
        .unwrap();
        fs::write(
            project.path().join(layout::current_boot_static_md()),
            "# inline\n",
        )
        .unwrap();
        fs::write(
            project
                .path()
                .join(layout::current_boot_dir())
                .join("rules.md"),
            "# rules\n",
        )
        .unwrap();
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
    fn boot_dir_accepts_either_generated_static_name_but_rejects_both() {
        for name in ["STATIC.md", "STATIC.xml"] {
            let project = tempdir().unwrap();
            write_minimal_project(project.path());
            fs::write(
                project.path().join(layout::current_boot_dir()).join(name),
                "generated stream\n",
            )
            .unwrap();
            let report = check_project(project.path(), &opts());
            assert!(
                !report.findings.iter().any(|f| {
                    f.check == CheckId::BootDirectory && f.severity == Severity::Error
                }),
                "{name} alone must pass; got: {:?}",
                report.findings
            );
        }

        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join(layout::current_boot_static_md()),
            "generated\n",
        )
        .unwrap();
        fs::write(
            project.path().join(layout::current_boot_static_xml()),
            "generated\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(report.findings.iter().any(|f| {
            f.check == CheckId::BootDirectory
                && f.severity == Severity::Error
                && f.message
                    == "both STATIC.md and STATIC.xml exist — the generator owns one; delete the stray"
        }));
    }

    #[test]
    fn boot_dir_non_markdown_file_is_a_warning() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project
                .path()
                .join(layout::current_boot_dir())
                .join("notes.txt"),
            "x",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::BootDirectory && f.severity == Severity::Warning),
            "a non-spec-source file in the boot directory must warn; got: {:?}",
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
            good.path()
                .join(layout::current_boot_dir())
                .join("rules.xml"),
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
            bad.path()
                .join(layout::current_boot_dir())
                .join("rules.xml"),
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
    /// in the boot directory is a loud error naming both files.
    #[test]
    fn boot_dir_pair_collision_is_an_error() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let boot = project.path().join(layout::current_boot_dir());
        fs::write(boot.join("dup.md"), "# dup\n").unwrap();
        fs::write(
            boot.join("dup.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        let hit = report
            .findings
            .iter()
            .find(|f| f.check == CheckId::BootDirectory && f.severity == Severity::Error)
            .expect("the pair must be an error");
        assert_eq!(
            hit.path.as_deref(),
            Some(layout::current_boot_dir().join("dup.md").as_path())
        );
        assert!(hit.message.contains("dup.md"), "{}", hit.message);
        assert!(hit.message.contains("dup.xml"), "{}", hit.message);
        assert!(hit.message.contains("one document, one form"));
    }
}
