//! The template check: a broken template is an error, a template the
//! probe cannot fully exercise is a warning, and an ordinary manifest is
//! neither.

use std::fs;

use tempfile::tempdir;

use crate::test_support::{opts, write_minimal_project};
use crate::{CheckId, CheckReport, Severity, check_project};

/// A template manifest whose `[[installer_options]]` row is spliced from
/// `step_extra`, over a `kind = "list"` source so the probe is exact.
fn template(step_extra: &str) -> String {
    format!(
        "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n\
         version = \"1.0.0\"\nepoch = 1\ntemplate = true\n\n\
         [application]\nid = \"tool\"\n\n\
         [template.versions]\nkind = \"list\"\n\n\
         [[template.versions.item]]\nversion = \"2.0.0\"\n\n\
         [[template.versions.item]]\nversion = \"3.1.4\"\n\n\
         [template.digests]\nkind = \"sha256sums\"\n\
         url = \"https://e.invalid/{{version}}/SHASUMS256.txt\"\n\n\
         [[installer_options]]\nid = \"linux-x64\"\nname = \"tool {{version}}\"\n\
         platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"\n\
         path_dirs = [\"bin\"]\n\n\
         [[installer_options.steps]]\nid = \"unpack\"\ntitle = \"Unpack {{version}}\"\n\
         kind = \"archive\"\nformat = \"tar_gz\"\n\
         source = \"https://e.invalid/tool-{{version}}-{{platform}}.tar.gz\"\n\
         sha256 = \"{{sha256:tool-{{version}}-{{platform}}.tar.gz}}\"\n{step_extra}"
    )
}

fn report_for(manifest: &str) -> CheckReport {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    fs::write(project.path().join("vibe.toml"), manifest).unwrap();
    check_project(project.path(), &opts())
}

fn findings(report: &CheckReport) -> Vec<&crate::Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.check == CheckId::TemplateRender)
        .collect()
}

#[test]
fn a_sound_template_produces_no_finding() {
    let report = report_for(&template(""));
    assert!(findings(&report).is_empty(), "{:?}", findings(&report));
}

#[test]
fn an_ordinary_manifest_is_not_examined() {
    let report = report_for(
        "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"flow\"\n\
         version = \"1.0.0\"\nepoch = 1\n",
    );
    assert!(findings(&report).is_empty());
}

#[test]
fn a_render_that_does_not_validate_is_an_error_naming_the_field() {
    // `payload_root` renders to an absolute path, which the installer
    // declaration refuses — a fault only a render can reveal.
    let report = report_for(&template("payload_root = \"/abs/{version}\"\n"));
    let finding = findings(&report)
        .first()
        .copied()
        .unwrap_or_else(|| panic!("expected a finding: {:?}", report.findings));
    assert_eq!(finding.severity, Severity::Error);
    assert!(
        finding.message.contains("skeleton render of `3.1.4`"),
        "the probe is the newest declared item: {}",
        finding.message
    );
    assert_eq!(
        finding.field.as_deref(),
        Some("installer_options[0].steps[0].payload_root"),
        "{}",
        finding.message
    );
}

#[test]
fn a_grammar_fault_the_probe_can_see_is_an_error() {
    let report = report_for(&template("description = \"{nope}\"\n"));
    let finding = findings(&report).first().copied().expect("a finding");
    assert_eq!(finding.severity, Severity::Error);
    assert!(
        finding.message.contains("not a known variable"),
        "{}",
        finding.message
    );
}

#[test]
fn a_meta_variable_leaves_the_render_unchecked_with_a_warning() {
    let report = report_for(&template("description = \"{meta.codename}\"\n"));
    let finding = findings(&report).first().copied().expect("a finding");
    assert_eq!(finding.severity, Severity::Warning);
    assert!(
        finding.message.contains("was not checked"),
        "{}",
        finding.message
    );
    assert!(!report.has_errors(), "{:?}", report.findings);
}

/// The real Node template of `vibe-template`'s own fixtures passes, which
/// is the check's acceptance against production input.
#[test]
fn the_node_template_fixture_passes_the_check() {
    let report = report_for(include_str!(
        "../../../../vibe-template/tests/fixtures/node/vibe.toml"
    ));
    assert!(findings(&report).is_empty(), "{:?}", findings(&report));
}
