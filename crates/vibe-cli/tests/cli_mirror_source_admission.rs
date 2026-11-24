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
        .unwrap_or_else(|_| panic!("fixture CLI could not be started"))
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

// Never include captured output or parsed response values in assertion failures.
fn error_record(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("fixture error record was not valid JSON"))
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
    assert!(excluded.status.code() == Some(1), "excluded source status mismatch");
    let shown = String::from_utf8_lossy(&excluded.stderr);
    assert!(shown.contains("source_policy_excluded  registry gh is disabled by the source policy [hosting github]"), "missing policy refusal headline");
    assert!(shown.contains("configured source https://github.com/vibevm is excluded by exclude_hosting selector github"), "missing configured-source explanation");
    assert!(!shown.contains("would read"), "configured source misrepresented as transport endpoint");
    assert!(shown.contains("vibe registry list"), "missing policy refusal fix");
    assert!(shown.contains("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY"), "missing policy reference");
    assert!(excluded.stdout.is_empty(), "excluded source produced progress output");
    assert!(!shown.contains("Reading the target"), "target was read before admission");
    assert!(!root.join("target/_index").exists(), "target index was created");

    let unknown = invoke(root, "https://github.com/vibevm", unusable, false);
    assert!(unknown.status.code() == Some(1), "unknown source status mismatch");
    let shown = String::from_utf8_lossy(&unknown.stderr);
    assert!(shown.contains("registry_not_found  no registry named https://github.com/vibevm"), "missing unknown source headline");
    assert!(shown.contains("configured: corp · mirror · gh"), "missing configured registry list");
    assert!(shown.contains("vibe registry add <name> https://github.com/vibevm"), "missing unknown source fix");
    assert!(unknown.stdout.is_empty(), "unknown source produced progress output");

    let hostile = invoke(root, "https://fixture:secret@example.invalid/\u{1b}[31m", unusable, true);
    assert!(hostile.status.code() == Some(1), "hostile source status mismatch");
    let record = error_record(&hostile);
    assert!(record["ok"] == false, "hostile source record was not a refusal");
    let error = record["error"].as_str().unwrap_or_else(|| panic!("missing hostile source error"));
    assert!(error.starts_with("registry_not_found: "), "hostile source error code mismatch");
    assert!(!error.contains("secret"), "credential redaction failed");
    assert!(!error.contains('\u{1b}'), "terminal control escaping failed");
    assert!(hostile.stdout.is_empty(), "hostile source produced progress output");

    let other = format!(
        "{}[[registry]]\nname='blocked'\ntype='git'\nurl='https://fixture:secret@blocked.example.invalid/org'\n",
        local_row(root, "corp", "source")
    );
    fixture(root, "[source_policy]\nexclude_hosting=['host:blocked.example.invalid']\n", &other);
    let excluded = invoke(root, "blocked", unusable, true);
    assert!(excluded.status.code() == Some(1), "blocked source status mismatch");
    let record = error_record(&excluded);
    assert!(record["ok"] == false, "blocked source record was not a refusal");
    let error = record["error"].as_str().unwrap_or_else(|| panic!("missing blocked source error"));
    assert!(error.starts_with("source_policy_excluded: "), "blocked source error code mismatch");
    assert!(error.contains("hosting host:blocked.example.invalid"), "missing exact-host rule");
    assert!(error.contains("configured source "), "missing configured-source meaning");
    assert!(error.contains("blocked.example.invalid/org"), "missing configured source address");
    assert!(error.contains("is excluded by exclude_hosting selector host:blocked.example.invalid"), "missing configured-source exclusion explanation");
    assert!(!error.contains("would read"), "configured source misrepresented as transport endpoint");
    assert!(!error.contains("secret"), "credential redaction failed");
    assert!(!error.contains("raw.githubusercontent.com"), "invented transport endpoint");
    assert!(excluded.stdout.is_empty(), "blocked source produced progress output");

    // A local source is not hosting: the same exclusion leaves it admitted.
    fixture(root, "[source_policy]\nexclude_hosting=['github']\n", &rows);
    let permitted = invoke(root, "corp", "mirror", true);
    assert!(permitted.status.success(), "permitted local source failed");
    let report: vibe_wire::generated::registry_mirror_report::RegistryMirrorReport =
        serde_json::from_slice(&permitted.stdout)
            .unwrap_or_else(|_| panic!("local source report was not valid JSON"));
    assert!(report.ok, "local source report failed");
    assert!(report.source == "corp", "local source identity mismatch");
    assert!(report.dry_run, "local source report was not a dry run");
    assert!(report.versions.is_empty(), "empty local source offered versions");
    assert!(!root.join("target/_index").exists(), "dry run created a target index");
}
