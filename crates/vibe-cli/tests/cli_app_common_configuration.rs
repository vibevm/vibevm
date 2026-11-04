//! Public install -g refuses unconnected common scopes before app execution.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES");

#[cfg(test)]
mod common;
use common::app_registry::AppRegistry;
use serde_json::Value;
use std::fs;

#[cfg(test)]
fn refusal(package_scope: bool) {
    let registry = AppRegistry::new();
    let sentinel = registry.root.path().join("STEP_MUST_NOT_RUN");
    let option = format!(
        "result={{external='/tmp/common-config-test'}}\n[[installer_options.steps]]\nid='setup'\ntitle='Setup'\nkind='shell'\ncommand=\"touch '{}'\"\n",
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
    assert_eq!(output.status.code(), Some(2), "{stdout}\n{stderr}");
    let envelope = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|v| v["ok"] == false)
        .expect("JSON refusal");
    assert_eq!(envelope["code"], "configuration_unsupported", "{envelope}");
    let scope = if package_scope {
        "scope `package`"
    } else {
        "scope `installer main`"
    };
    assert!(stderr.contains(scope), "{stderr}");
    assert!(!sentinel.exists());
    assert!(!registry.settings().join("applications/index.json").exists());
    assert!(!registry.settings().join("cache/app-work").exists());
}

#[test]
fn global_install_refuses_package_common_configuration_without_app_publication() {
    refusal(true);
}
#[test]
fn global_install_refuses_selected_option_common_configuration_without_app_publication() {
    refusal(false);
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
