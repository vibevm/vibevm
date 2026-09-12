//! Unit tests for [`super`], out-of-line per the file-length budget.
//!
//! The source documentation is planted in the project's OWN package
//! root (`vibevm/vibepacks/<group>/<name>/v<version>/`), which is the
//! first place the cell looks and the only one a test may write: the
//! machine store belongs to the developer running the suite.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::test_support::opts;
use crate::{CheckId, CheckReport, Severity, check_project};

/// Write the adaptation's own manifest. `i18n` is the line every case
/// varies, so it is a parameter rather than a fixture.
fn write_translation(root: &Path, i18n: &str, documents: &str) {
    fs::write(
        root.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.vibevm.core"
name = "vibevm-docs-ru"
kind = "doc"
version = "0.1.0"
title = "Руководство VibeVM"
abstract = "Что покрывает, для кого, что предполагает известным, чего не покрывает."
{i18n}
{documents}
[translates]
package = "org.vibevm.core/vibevm-docs"
version = "^0.1"
"#
        ),
    )
    .unwrap();
    fs::write(root.join("README.md"), "# Руководство\n").unwrap();
}

/// Plant the source documentation in the project's own package root.
fn write_source(root: &Path, version: &str, documents: &str) {
    let dir = root
        .join(vibe_core::layout::current_packages_root())
        .join("org.vibevm.core")
        .join("vibevm-docs")
        .join(format!("v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.vibevm.core"
name = "vibevm-docs"
kind = "doc"
version = "{version}"
title = "VibeVM Manual"
abstract = "Four answers."

{documents}"#
        ),
    )
    .unwrap();
}

const SUBJECT: &str = "[[documents]]\npackage = \"org.vibevm.core/vibevm\"\nversion = \"^1.0\"\n";
const OTHER_SUBJECT: &str =
    "[[documents]]\npackage = \"org.vibevm.world/multi-user-planning\"\nversion = \"^1.0\"\n";

fn translation_findings(report: &CheckReport) -> Vec<&crate::Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.check == CheckId::DocTranslation)
        .collect()
}

#[test]
fn an_adaptation_that_names_its_language_and_its_subjects_passes_clean() {
    let project = tempdir().unwrap();
    write_translation(project.path(), "\n[i18n]\ncanonical = \"ru\"\n", SUBJECT);
    write_source(project.path(), "0.1.0", SUBJECT);
    let report = check_project(project.path(), &opts());
    assert!(
        translation_findings(&report).is_empty(),
        "a well-formed adaptation owes nothing: {:?}",
        report.findings
    );
}

/// The default is `en`, so silence would make every adaptation claim
/// English. The key must be written — and the message must name the
/// field that works, because the campaign plan that preceded PROP-057
/// told authors to write `lang`.
#[test]
fn an_adaptation_that_never_names_its_language_is_an_error() {
    let project = tempdir().unwrap();
    write_translation(project.path(), "", SUBJECT);
    write_source(project.path(), "0.1.0", SUBJECT);
    let report = check_project(project.path(), &opts());
    let hit = translation_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Error)
        .expect("an unnamed language is an error");
    assert!(hit.message.contains("[i18n] canonical"), "{}", hit.message);
    assert!(hit.message.contains("no `lang` field"), "{}", hit.message);
    assert!(hit.message.contains("LOC-LANGUAGE-FIELD"));
}

/// An `[i18n]` table without `canonical` is silence too — the parsed
/// manifest fills the default in either way, which is exactly why the
/// question is asked of the file's text.
#[test]
fn an_i18n_table_without_canonical_is_still_silence() {
    let project = tempdir().unwrap();
    write_translation(project.path(), "\n[i18n]\npreferred = \"ru\"\n", SUBJECT);
    write_source(project.path(), "0.1.0", SUBJECT);
    let report = check_project(project.path(), &opts());
    assert!(
        translation_findings(&report)
            .into_iter()
            .any(|f| f.severity == Severity::Error && f.message.contains("LOC-LANGUAGE-FIELD")),
        "an [i18n] table that never says `canonical` has not named a language: {:?}",
        report.findings
    );
}

#[test]
fn documenting_something_else_than_the_source_is_an_error() {
    let project = tempdir().unwrap();
    write_translation(
        project.path(),
        "\n[i18n]\ncanonical = \"ru\"\n",
        OTHER_SUBJECT,
    );
    write_source(project.path(), "0.1.0", SUBJECT);
    let report = check_project(project.path(), &opts());
    let hit = translation_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Error)
        .expect("diverging subjects are an error");
    assert!(
        hit.message.contains("LOC-DOCUMENTS-MATCH"),
        "{}",
        hit.message
    );
    assert!(
        hit.message.contains("org.vibevm.world/multi-user-planning"),
        "the message names what this package documents: {}",
        hit.message
    );
    assert!(
        hit.message.contains("org.vibevm.core/vibevm@^1.0"),
        "and what the source documents: {}",
        hit.message
    );
}

/// A source that is on no disk here leaves the rule UNCHECKED, and the
/// finding says so — a warning naming the reason and the remedy, never a
/// silent pass and never an error blaming the author for a package that
/// is simply not on this machine.
#[test]
fn an_unreachable_source_warns_with_its_reason() {
    let project = tempdir().unwrap();
    write_translation(project.path(), "\n[i18n]\ncanonical = \"ru\"\n", SUBJECT);
    let report = check_project(project.path(), &opts());
    let hit = translation_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Warning)
        .expect("an unreachable source is a warning");
    assert!(
        hit.message.contains("unchecked here, not"),
        "{}",
        hit.message
    );
    assert!(hit.message.contains("vibe cache add"), "{}", hit.message);
    assert!(
        !translation_findings(&report)
            .iter()
            .any(|f| f.severity == Severity::Error),
        "an absent source is not the author's defect: {:?}",
        report.findings
    );
}

/// The constraint picks the version: a source outside `^0.1` is not the
/// source, and the cell says the rule is unchecked rather than comparing
/// against the wrong package.
#[test]
fn a_source_outside_the_constraint_is_not_the_source() {
    let project = tempdir().unwrap();
    write_translation(project.path(), "\n[i18n]\ncanonical = \"ru\"\n", SUBJECT);
    write_source(project.path(), "0.2.0", OTHER_SUBJECT);
    let report = check_project(project.path(), &opts());
    assert!(
        translation_findings(&report)
            .into_iter()
            .any(|f| f.severity == Severity::Warning),
        "0.2.0 does not satisfy `^0.1`: {:?}",
        report.findings
    );
}

/// A package that adapts nothing is not the mirror half's business at
/// all — that half is gated on `[translates]`.
#[test]
fn a_package_that_adapts_nothing_is_untouched() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let report = check_project(project.path(), &opts());
    assert!(translation_findings(&report).is_empty());
}

/// A documentation package that lists languages is claiming to hold text
/// it does not hold: a translation is another package, and the site
/// computes the available languages from the `translates` edges at every
/// render. The two would then disagree about what exists.
#[test]
fn a_documentation_package_that_lists_languages_is_an_error() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.vibevm.core\"\nname = \"vibevm-docs\"\nkind = \"doc\"\n\
             version = \"0.1.0\"\ntitle = \"VibeVM Manual\"\nabstract = \"Four answers.\"\n\
             [i18n]\ncanonical = \"en\"\navailable = [\"en\", \"ru\"]\n\n{SUBJECT}"
        ),
    )
    .unwrap();
    fs::write(project.path().join("README.md"), "# manual\n").unwrap();

    let report = check_project(project.path(), &opts());
    let found = translation_findings(&report);
    assert_eq!(found.len(), 1, "{report:#?}");
    assert_eq!(found[0].severity, Severity::Error);
    assert!(
        found[0].message.contains("\"en\", \"ru\""),
        "{}",
        found[0].message
    );
    assert!(
        found[0].message.contains("LOC-LANGUAGE-FIELD"),
        "{}",
        found[0].message
    );
}

/// The rule is the `doc` kind's. Every other kind localises inside one
/// package, which is what the field is for.
#[test]
fn an_ordinary_package_may_list_its_languages() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\
         [i18n]\ncanonical = \"en\"\navailable = [\"en\", \"ru\"]\n",
    )
    .unwrap();
    let report = check_project(project.path(), &opts());
    assert!(translation_findings(&report).is_empty(), "{report:#?}");
}

/// The source is looked for in the checked root's ANCESTORS too. An
/// adaptation under development is checked both from the project root
/// and with `--path <its own slot>`, and only the first has the package
/// root beneath it; without the walk the second warned that a package in
/// the same tree was unreachable.
#[test]
fn a_source_above_the_checked_slot_is_found() {
    let project = tempdir().unwrap();
    write_source(project.path(), "0.1.0", SUBJECT);

    // The adaptation, checked at its own slot rather than at the root.
    let slot = project
        .path()
        .join(vibe_core::layout::current_packages_root())
        .join("org.vibevm.core")
        .join("vibevm-docs-ru")
        .join("v0.1.0");
    fs::create_dir_all(&slot).unwrap();
    write_translation(&slot, "[i18n]\ncanonical = \"ru\"", SUBJECT);

    let report = check_project(&slot, &opts());
    assert!(
        translation_findings(&report).is_empty(),
        "the source sits three levels up, in this project's own registry: {report:#?}"
    );
}
