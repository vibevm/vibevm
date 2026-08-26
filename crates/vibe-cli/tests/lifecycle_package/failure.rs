use std::fs;

use specmark::verifies;

use crate::common::UserScratch;

use super::support::{RECOVER_KEY, command, json_documents, lifecycle_state, project};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
fn package_binding_failure_stops_every_later_contribution() {
    let user = UserScratch::new();
    let project = project(true, &["claude"], false);
    let fake_home = tempfile::tempdir().unwrap();
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str(
        r#"
[[extension]]
id = "after-failure"
point = "phase:package"
handler = { kind = "script", base = "scripts/after" }
"#,
    );
    fs::write(manifest_path, manifest).unwrap();
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/after.sh"),
        "printf reached > after-package.txt\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/after.ps1"),
        "Set-Content -LiteralPath after-package.txt -Value reached -NoNewline\n",
    )
    .unwrap();
    fs::create_dir_all(project.path().join(".vibe")).unwrap();
    fs::write(
        project.path().join(".vibe/package-skills.toml"),
        "schema = 1\n[[binding]]\nunknown = true\n",
    )
    .unwrap();

    let output = command(&user, project.path(), fake_home.path(), true)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let documents = json_documents(&output.stdout);
    assert_eq!(documents.len(), 2, "{documents:#?}");
    assert_eq!(documents[1]["ok"], false);
    let rows = documents[1]["contributions"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["key"], RECOVER_KEY);
    assert_eq!(rows[0]["status"], "fail");
    assert!(!project.path().join("after-package.txt").exists());
    assert_eq!(
        lifecycle_state(project.path()).execution[RECOVER_KEY].status,
        vibe_wire::generated::lifecycle_state::ExecutionRecordStatus::Fail
    );
}
