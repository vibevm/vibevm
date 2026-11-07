//! Public install -g keeps package and selected-installer scopes distinct.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES");

#[cfg(test)]
mod common;
use common::app_registry::AppRegistry;
use serde_json::Value;
use std::fs;

#[cfg(test)]
fn installs(package_scope: bool) {
    let registry = AppRegistry::new();
    let sentinel = registry.root.path().join("STEP_RAN");
    let option = format!(
        "result={{external='{}'}}\n[[installer_options.steps]]\nid='setup'\ntitle='Setup'\nkind='shell'\ncommand=\"touch '{}'\"\n",
        registry.root.path().display(),
        sentinel.display()
    );
    let package = registry.package("common-config", "1.0.0", "common-config", &option);
    let path = package.join("vibe.toml");
    let definition = if package_scope {
        "\n[configuration]\n"
    } else {
        "\n[installer_options.configuration]\n"
    };
    let text = fs::read_to_string(&path).unwrap() + definition;
    vibe_core::manifest::Manifest::parse_str(&text).unwrap();
    fs::write(&path, text).unwrap();
    let output = registry.app(
        &["install", "-g"],
        "org.example/common-config",
        &["--offline", "--unattended", "--json"],
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(sentinel.exists());
    let index: Value = serde_json::from_slice(
        &fs::read(registry.settings().join("applications/index.json")).unwrap(),
    )
    .unwrap();
    // Recorded source evidence selects schema 5 in both configuration scopes.
    assert_eq!(index["protocol"], "vibe-user-applications/5");
    let record = &index["installations"]["org.example/common-config@1.0.0"];
    assert!(record["source_evidence"].is_object());
    assert_eq!(record.get("configuration").is_some(), !package_scope);
}

#[test]
fn global_install_keeps_package_configuration_separate_from_installer() {
    installs(true);
}
#[test]
fn global_install_records_selected_installer_common_configuration() {
    installs(false);
}

#[test]
fn legacy_describe_and_steps_refuse_incomplete_common_option_views() {
    let registry = AppRegistry::new();
    registry.package("common-view","1.0.0","common-view","result={external='/tmp/common-view'}\n[installer_options.configuration]\n[[installer_options.steps]]\nid='setup'\ntitle='Setup'\nkind='shell'\ncommand='true'\n");
    let mut wrong = Vec::new();
    for verb in ["describe", "steps"] {
        for machine in [false, true] {
            let mut extra = vec!["--offline", "--unattended"];
            if machine {
                extra.push("--json");
            }
            let output = registry.app(&["app", verb], "org.example/common-view", &extra);
            let stderr = String::from_utf8(output.stderr).unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            let code_ok = if machine {
                stderr
                    .lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .any(|v| v["code"] == "configuration_unsupported")
            } else {
                stderr.contains("configuration_unsupported")
            };
            if output.status.code() != Some(2)
                || !code_ok
                || !stderr.contains("installer main")
                || !stdout.is_empty()
            {
                wrong.push((verb, machine, stdout, stderr));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "legacy views falsely presented rich configuration: {wrong:?}"
    );
}

#[test]
fn existing_registry_failure_keeps_its_failure_category_after_wrapping() {
    let registry = AppRegistry::new();
    fs::create_dir_all(registry.registry()).unwrap();
    let output = registry.app(
        &["install", "-g"],
        "org.example/absent",
        &["--offline", "--unattended", "--json"],
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    let envelope = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|v| v["ok"] == false)
        .unwrap();
    assert_eq!(envelope["code"], "registry_failed", "{envelope}");
}
