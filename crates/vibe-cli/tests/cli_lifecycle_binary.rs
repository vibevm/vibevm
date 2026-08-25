mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

fn binary_package(registry: &Path, group: &str, name: &str, message: &str) {
    let root = registry.join(group).join(name).join("v0.1.0");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("vibe.toml"),
        format!(
            r#"[package]
group = "{group}"
name = "{name}"
kind = "tool"
version = "0.1.0"

[[binary]]
name = "runner"
crate = "."

[[extension]]
id = "binary-handler"
point = "phase:build"
handler = {{ kind = "binary", name = "runner" }}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}-fixture"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "runner"
path = "src/main.rs"
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("build.rs"),
        r#"fn main() {
    assert!(
        std::env::var_os("VIBEVM_PUBLISH_TOKEN_GITHUB").is_none(),
        "package build inherited a publish credential"
    );
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        format!(
            r###"use std::io::Read;
fn main() {{
    let mut wire = String::new();
    std::io::stdin().read_to_string(&mut wire).unwrap();
    assert!(wire.contains("\"envelope\":1"));
    eprintln!("structured-{message}");
    std::io::Write::write_all(
        &mut std::io::stdout(),
        br#"{{"artifacts":[],"envelope":1,"message":"{message}","status":"ok","tasks":[]}}"#,
    ).unwrap();
}}
"###,
        ),
    )
    .unwrap();
}

fn host_package(root: &Path, point: &str, message: &str, requires: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    let applies = if point.starts_with("slot:") {
        "\napplies_to = { packages = [\"org.alpha/*\"] }"
    } else {
        ""
    };
    fs::write(
        root.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.host"
name = "root"
kind = "tool"
version = "0.1.0"
{requires}
[[binary]]
name = "runner"
crate = "."

[[extension]]
id = "host-binary"
point = "{point}"
handler = {{ kind = "binary", name = "runner" }}{applies}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "host-binary-fixture"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "runner"
path = "src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        format!(
            r###"use std::io::Read;
fn main() {{
    let mut wire = String::new();
    std::io::stdin().read_to_string(&mut wire).unwrap();
    assert!(wire.contains("\"envelope\":1"));
    std::io::Write::write_all(
        &mut std::io::stdout(),
        br#"{{"artifacts":[],"envelope":1,"message":"{message}","status":"ok","tasks":[]}}"#,
    ).unwrap();
}}
"###,
        ),
    )
    .unwrap();
}

fn project() -> (UserScratch, tempfile::TempDir, PathBuf) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry_root = tempfile::tempdir().unwrap().keep();
    binary_package(&registry_root, "org.alpha", "tools", "alpha");
    binary_package(&registry_root, "org.beta", "tools", "beta");
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str(
        r#"
[requires.packages]
"org.alpha/tools" = "=0.1.0"
"org.beta/tools" = "=0.1.0"
"#,
    );
    fs::write(manifest_path, manifest).unwrap();
    (user, project, registry_root)
}

#[test]
fn colliding_binary_names_resolve_within_each_provider_and_keep_json_clean() {
    let (user, project, registry) = project();
    let output = user
        .vibe()
        .env("VIBEVM_PUBLISH_TOKEN_GITHUB", "sentinel-never-print")
        .args(["build", "--json", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        !output
            .stdout
            .windows(20)
            .any(|window| window == b"sentinel-never-print")
    );
    assert!(
        !output
            .stderr
            .windows(20)
            .any(|window| window == b"sentinel-never-print")
    );
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .expect("binary build/handler logs must not contaminate JSON stdout");
    assert!(
        output.stderr.is_empty(),
        "JSON transport leaked raw build/handler stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let report: LifecycleReport =
        serde_json::from_value(documents.last().unwrap().clone()).unwrap();
    let messages = report
        .contributions
        .iter()
        .map(|row| (row.provider.as_str(), row.message.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        messages,
        [
            ("org.alpha/tools", Some("alpha")),
            ("org.beta/tools", Some("beta")),
        ]
    );
    assert!(report.contributions.iter().all(|row| row.stdout.is_none()));
    assert!(report.contributions.iter().all(|row| {
        row.stderr
            .as_deref()
            .is_some_and(|text| text.contains("structured-"))
    }));
}

#[test]
fn direct_bin_build_keeps_operator_consent_while_installed_handlers_do_not_need_it() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    binary_package(registry.path(), "org.foreign", "tools", "foreign");
    user.vibe()
        .arg("install")
        .arg("org.foreign/tools@=0.1.0")
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    user.vibe()
        .current_dir(project.path())
        .args(["bin", "build", "runner"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("consent explicitly"));
    user.vibe()
        .current_dir(project.path())
        .args(["bin", "build", "runner", "--assume-yes"])
        .assert()
        .success();
}

#[test]
fn package_role_host_phase_binary_builds_at_authored_root() {
    let user = UserScratch::new();
    let host = tempfile::tempdir().unwrap();
    host_package(host.path(), "phase:build", "host-phase", "");
    let output = user
        .vibe()
        .args(["build", "--json", "--path"])
        .arg(host.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let report: LifecycleReport = serde_json::from_value(docs.last().unwrap().clone()).unwrap();
    assert_eq!(report.contributions[0].provider, "org.host/root");
    assert_eq!(
        report.contributions[0].message.as_deref(),
        Some("host-phase")
    );
}

#[test]
fn package_role_host_slot_binary_beats_dependency_name_collision() {
    let user = UserScratch::new();
    let host = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    binary_package(registry.path(), "org.alpha", "tools", "dependency");
    host_package(
        host.path(),
        "slot:pre-install",
        "host-slot",
        "\n[requires.packages]\n\"org.alpha/tools\" = \"=0.1.0\"\n\n",
    );
    let output = user
        .vibe()
        .args(["build", "--json", "--path"])
        .arg(host.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let slot = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .flat_map(|doc| doc["contributions"].as_array().into_iter().flatten())
        .find(|row| row["point"] == "slot:pre-install")
        .unwrap();
    assert_eq!(slot["provider"], "org.host/root");
    assert_eq!(slot["message"], "host-slot");
    assert_eq!(slot["slot_target"]["name"], "tools");
}
