//! Mirror source admission precedes target access, using isolated local fixtures.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn invoke(root: &Path, source: &str, target: &str, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
    command
        .env_clear()
        .current_dir(root.join("project"))
        .env("HOME", root.join("home"))
        .env("USERPROFILE", root.join("home"))
        .env("VIBE_SETTINGS", root.join("settings"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("NO_COLOR", "1")
        .env("TERM", "xterm")
        .env("COLUMNS", "200")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    for key in ["PATH", "SystemRoot", "SYSTEMROOT", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    if json {
        command.arg("--json");
    } else {
        command.arg("--plain");
    }
    command
        .args(["registry", "mirror", source, "--into", target, "--dry-run"])
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn fixture(root: &Path, policy: &str, rows: &str) {
    for directory in ["home", "settings", "project", "source", "target"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::write(root.join("settings/config.toml"), "").unwrap();
    fs::write(
        root.join("project/vibe.toml"),
        format!("[project]\nname='consumer'\nversion='0.1.0'\n{policy}\n{rows}"),
    )
    .unwrap();
}

fn local_row(root: &Path, name: &str, directory: &str) -> String {
    let address = reqwest::Url::from_directory_path(root.join(directory)).unwrap();
    format!("[[registry]]\nname='{name}'\ntype='localdir'\nurl='{address}'\n")
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY"
)]
fn mirror_source_is_checked_before_target() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let rows = format!(
        "{}{}[[registry]]\nname='gh'\ntype='github'\nurl='https://github.com/vibevm'\n",
        local_row(root, "corp", "source"),
        local_row(root, "mirror", "target")
    );
    fixture(root, "[source_policy]\nexclude_hosting=['github']\n", &rows);
    // An invalid target spelling proves admission precedes even target resolution.
    let unusable = "https://[invalid-target";
    let excluded = invoke(root, "gh", unusable, false);
    assert_eq!(excluded.status.code(), Some(1), "{}", text(&excluded));
    let shown = text(&excluded);
    assert!(shown.contains("source_policy_excluded  registry gh is disabled by the source policy [hosting github]"), "{shown}");
    assert!(shown.contains("mirroring it would read https://github.com/vibevm, which exclude_hosting forbids"), "{shown}");
    assert!(shown.contains("vibe registry list"), "{shown}");
    assert!(shown.contains("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY"), "{shown}");
    assert!(excluded.stdout.is_empty(), "{shown}");
    assert!(!shown.contains("Reading the target"), "{shown}");
    assert!(!root.join("target/_index").exists());

    let unknown = invoke(root, "https://github.com/vibevm", unusable, false);
    assert_eq!(unknown.status.code(), Some(1), "{}", text(&unknown));
    let shown = text(&unknown);
    assert!(shown.contains("registry_not_found  no registry named https://github.com/vibevm"), "{shown}");
    assert!(shown.contains("configured: corp · mirror · gh"), "{shown}");
    assert!(shown.contains("vibe registry add <name> https://github.com/vibevm"), "{shown}");
    assert!(unknown.stdout.is_empty(), "{shown}");

    let hostile = invoke(root, "https://fixture:secret@example.invalid/\u{1b}[31m", unusable, true);
    assert_eq!(hostile.status.code(), Some(1), "{}", text(&hostile));
    let record: serde_json::Value = serde_json::from_slice(&hostile.stderr).unwrap();
    assert_eq!(record["ok"], false);
    let error = record["error"].as_str().unwrap();
    assert!(error.starts_with("registry_not_found: "), "{record}");
    assert!(!error.contains("secret"), "{record}");
    assert!(!error.contains('\u{1b}'), "{record}");
    assert!(hostile.stdout.is_empty());

    let other = format!(
        "{}[[registry]]\nname='blocked'\ntype='git'\nurl='https://fixture:secret@blocked.example.invalid/org'\n",
        local_row(root, "corp", "source")
    );
    fixture(root, "[source_policy]\nexclude_hosting=['host:blocked.example.invalid']\n", &other);
    let excluded = invoke(root, "blocked", unusable, true);
    assert_eq!(excluded.status.code(), Some(1), "{}", text(&excluded));
    let record: serde_json::Value = serde_json::from_slice(&excluded.stderr).unwrap();
    assert_eq!(record["ok"], false);
    let error = record["error"].as_str().unwrap();
    assert!(error.starts_with("source_policy_excluded: "), "{record}");
    assert!(error.contains("hosting host:blocked.example.invalid"), "{record}");
    assert!(error.contains("blocked.example.invalid/org"), "{record}");
    assert!(!error.contains("secret"), "{record}");
    assert!(!error.contains("raw.githubusercontent.com"), "{record}");
    assert!(excluded.stdout.is_empty());

    // A local source is not hosting: the same exclusion leaves it admitted.
    fixture(root, "[source_policy]\nexclude_hosting=['github']\n", &rows);
    let permitted = invoke(root, "corp", "mirror", true);
    assert!(permitted.status.success(), "{}", text(&permitted));
    let report: vibe_wire::generated::registry_mirror_report::RegistryMirrorReport =
        serde_json::from_slice(&permitted.stdout).unwrap();
    assert!(report.ok);
    assert_eq!(report.source, "corp");
    assert!(report.dry_run);
    assert!(report.versions.is_empty());
    assert!(!root.join("target/_index").exists());
}
