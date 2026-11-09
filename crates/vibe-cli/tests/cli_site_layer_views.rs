//! The applied site profile is a registry layer of its own wherever the
//! layers are shown (PROP-002 `#registry-layers`, PROP-019 `#site-profile`):
//! `vibe registry list` labels its rows `site` (and lists it alone with
//! `--scope site`), and `vibe show config` lists every layer's rows with the
//! layer as provenance and reports the profile itself.

mod common;

use std::fs;

use common::UserScratch;
use serde_json::Value;

const PROFILE: &str = "schema = 1\nname = \"corp-site\"\n\n\
    [[registry]]\nname = \"corp\"\ntype = \"LocalDir\"\nurl = \"file:///site/corp\"\n\n\
    [[mirror]]\nof = \"corp\"\nurl = \"file:///site/corp-mirror\"\npriority = 5\n";

fn json(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON report")
}

fn setup() -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    fs::write(user.settings.join("site-profile.toml"), PROFILE).unwrap();
    fs::write(
        user.settings.join("registry.toml"),
        "[[registry]]\nname = \"mine\"\nurl = \"file:///home/me/registry\"\n",
    )
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n\
         [[registry]]\nname = \"team\"\nurl = \"file:///team/registry\"\n",
    )
    .unwrap();
    (user, project)
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers"
)]
fn registry_list_shows_site_rows_with_site_scope() {
    let (user, project) = setup();
    let report = json(
        user.vibe()
            .args(["--json", "registry", "list", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows: Vec<(String, String)> = report["registries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_string(),
                row["scope"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("team".to_string(), "project".to_string()),
            ("mine".to_string(), "user".to_string()),
            ("corp".to_string(), "site".to_string()),
        ]
    );

    let site = json(
        user.vibe()
            .args(["--json", "registry", "list", "--scope", "site", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows = site["registries"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "corp");
    assert_eq!(rows[0]["scope"], "site");
    assert_eq!(rows[0]["mirrors"][0]["url"], "file:///site/corp-mirror");

    // The site layer stays read-only for the editing commands.
    let refused = user
        .vibe()
        .args([
            "registry",
            "add",
            "x",
            "file:///x",
            "--scope",
            "site",
            "--path",
        ])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("vibe self apply"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn show_config_lists_every_layer_and_the_site_profile() {
    let (user, project) = setup();
    let report = json(
        user.vibe()
            .args(["--json", "show", "config", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows: Vec<(&str, &str)> = report["registries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap(),
                row["provenance"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("team", "vibe.toml"),
            ("mine", "~/.vibe/registry.toml"),
            ("corp", "site-profile.toml"),
        ]
    );
    let mirrors = report["mirrors"].as_array().unwrap();
    assert_eq!(mirrors.len(), 1);
    assert_eq!(mirrors[0]["provenance"], "site-profile.toml");
    assert_eq!(report["site_profile"]["loaded"], true);
    assert_eq!(report["site_profile"]["name"], "corp-site");
    assert!(
        report["site_profile"]["path"]
            .as_str()
            .unwrap()
            .ends_with("site-profile.toml")
    );

    let text = user
        .vibe()
        .args(["show", "config", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("source: site-profile.toml"), "{text}");
    assert!(text.contains("(applied: corp-site)"), "{text}");
}
