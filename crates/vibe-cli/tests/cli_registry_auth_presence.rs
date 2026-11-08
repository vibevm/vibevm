//! Registry add preserves auth declaration presence without contacting a source.
mod common;
use common::UserScratch;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn registry_add_preserves_omission_and_explicit_none() {
    for explicit in [false, true] {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        let mut command = user.vibe();
        command
            .args([
                "registry",
                "add",
                "selected",
                "https://example.invalid/org",
                "--path",
            ])
            .arg(project.path());
        if explicit {
            command.args(["--auth", "none"]);
        }
        command.assert().success();
        let manifest =
            vibe_core::manifest::Manifest::read(project.path().join("vibe.toml")).unwrap();
        let row = manifest
            .registries
            .iter()
            .find(|row| row.name == "selected")
            .unwrap();
        assert_eq!(
            row.auth,
            explicit.then_some(vibe_core::manifest::AuthKind::None)
        );
        let text = std::fs::read_to_string(project.path().join("vibe.toml")).unwrap();
        assert_eq!(text.contains("auth = \"none\""), explicit);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn contradictory_cli_token_declaration_does_not_write_manifest() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let path = project.path().join("vibe.toml");
    let before = std::fs::read(&path).unwrap();
    user.vibe()
        .args([
            "registry",
            "add",
            "selected",
            "https://example.invalid/org",
            "--auth",
            "none",
            "--token-env",
            "UNREAD_AUTH_DECLARATION",
            "--path",
        ])
        .arg(project.path())
        .assert()
        .failure();
    assert_eq!(std::fs::read(path).unwrap(), before);
}
