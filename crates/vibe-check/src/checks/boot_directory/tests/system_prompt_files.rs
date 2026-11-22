//! The boot directory check and the opted-in system prompt's generated files.

use super::*;

/// The opted-in system prompt's generated artifacts and vibe's own ignore
/// file are accepted silently; a foreign `.gitignore` there still warns.
#[test]
fn boot_dir_accepts_generated_system_prompt_files() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    let boot = project.path().join(layout::current_boot_dir());
    fs::write(boot.join(layout::SYSTEM_MD), "x").unwrap();
    fs::write(boot.join(layout::SYSTEM_META_JSON), "{}").unwrap();
    fs::write(boot.join(layout::AGENT_LANE_MD), "x").unwrap();
    fs::write(
        boot.join(".gitignore"),
        vibe_workspace::system_prompt::BOOT_IGNORE_BODY,
    )
    .unwrap();
    let report = check_project(project.path(), &opts());
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.check == CheckId::BootDirectory),
        "generated system-prompt files must not warn; got: {:?}",
        report.findings
    );

    fs::write(boot.join(".gitignore"), "*.tmp\n").unwrap();
    let report = check_project(project.path(), &opts());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check == CheckId::BootDirectory && f.severity == Severity::Warning),
        "a foreign .gitignore in the boot directory must still warn; got: {:?}",
        report.findings
    );
}
