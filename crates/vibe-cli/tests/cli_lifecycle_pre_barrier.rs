//! Fresh-install availability barrier for cross-provider slot handlers.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;

fn package_root(registry: &Path, name: &str) -> PathBuf {
    registry.join("org.barrier").join(name).join("v0.1.0")
}

fn write_script_world(registry: &Path) {
    let target = package_root(registry, "a-script-target");
    fs::create_dir_all(&target).unwrap();
    fs::write(
        target.join("vibe.toml"),
        r#"[package]
group = "org.barrier"
name = "a-script-target"
kind = "flow"
version = "0.1.0"

[requires.packages]
"org.barrier/z-script-provider" = "=0.1.0"

[boot_snippet]
source = "boot/generated.md"
category = "flow"
"#,
    )
    .unwrap();
    fs::write(target.join("target.ready"), "target").unwrap();

    let provider = package_root(registry, "z-script-provider");
    fs::create_dir_all(provider.join("hooks")).unwrap();
    fs::write(
        provider.join("vibe.toml"),
        r#"[package]
group = "org.barrier"
name = "z-script-provider"
kind = "tool"
version = "0.1.0"

[[extension]]
id = "cross-provider-script"
point = "slot:pre-install"
handler = { kind = "script", base = "hooks/barrier" }
applies_to = { packages = ["org.barrier/a-script-target"] }
"#,
    )
    .unwrap();
    fs::write(provider.join("provider.ready"), "provider").unwrap();
    let provider_rel = common::slot_dir("org.barrier.z-script-provider", "0.1.0");
    let index_rel = common::index_rel();
    fs::write(
        provider.join("hooks/barrier.sh"),
        format!(
            r#"set -eu
test -f "$VIBE_PROJECT_ROOT/{provider_rel}/provider.ready"
test -f "$VIBE_PACKAGE_DIR/target.ready"
! grep -q 'a-script-target' "$VIBE_PROJECT_ROOT/vibe.lock"
if test -f "$VIBE_PROJECT_ROOT/{index_rel}"; then
  ! grep -q 'boot/generated.md' "$VIBE_PROJECT_ROOT/{index_rel}"
fi
mkdir -p boot
printf 'generated at pre barrier\n' > boot/generated.md
printf script > "$VIBE_PROJECT_ROOT/.vibe/pre-barrier-script"
"#,
        ),
    )
    .unwrap();
    fs::write(
        provider.join("hooks/barrier.ps1"),
        format!(
            r#"$provider = Join-Path $env:VIBE_PROJECT_ROOT '{provider_rel}/provider.ready'
if (-not (Test-Path -LiteralPath $provider)) {{ exit 31 }}
if (-not (Test-Path -LiteralPath (Join-Path $env:VIBE_PACKAGE_DIR 'target.ready'))) {{ exit 32 }}
$lock = Get-Content -Raw -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT 'vibe.lock')
if ($lock -match 'a-script-target') {{ exit 33 }}
$index = Join-Path $env:VIBE_PROJECT_ROOT '{index_rel}'
if ((Test-Path -LiteralPath $index) -and ((Get-Content -Raw -LiteralPath $index) -match 'boot/generated.md')) {{ exit 34 }}
New-Item -ItemType Directory -Force (Join-Path $env:VIBE_PACKAGE_DIR 'boot') | Out-Null
Set-Content -LiteralPath (Join-Path $env:VIBE_PACKAGE_DIR 'boot/generated.md') -Value 'generated at pre barrier'
Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/pre-barrier-script') -Value script -NoNewline
"#,
        ),
    )
    .unwrap();
}

fn write_binary_world(registry: &Path) {
    let target = package_root(registry, "a-binary-target");
    fs::create_dir_all(target.join("boot")).unwrap();
    fs::write(
        target.join("vibe.toml"),
        r#"[package]
group = "org.barrier"
name = "a-binary-target"
kind = "flow"
version = "0.1.0"

[requires.packages]
"org.barrier/z-binary-provider" = "=0.1.0"

[boot_snippet]
source = "boot/target.md"
category = "flow"
"#,
    )
    .unwrap();
    fs::write(target.join("boot/target.md"), "target boot").unwrap();
    fs::write(target.join("target.ready"), "target").unwrap();

    let provider = package_root(registry, "z-binary-provider");
    fs::create_dir_all(provider.join("src")).unwrap();
    fs::write(
        provider.join("vibe.toml"),
        r#"[package]
group = "org.barrier"
name = "z-binary-provider"
kind = "tool"
version = "0.1.0"

[[binary]]
name = "barrier"
crate = "."

[[extension]]
id = "cross-provider-binary"
point = "slot:pre-install"
handler = { kind = "binary", name = "barrier" }
applies_to = { packages = ["org.barrier/a-binary-target"] }
"#,
    )
    .unwrap();
    fs::write(provider.join("provider.ready"), "provider").unwrap();
    fs::write(
        provider.join("Cargo.toml"),
        r#"[package]
name = "pre-barrier-binary-fixture"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "barrier"
path = "src/main.rs"
"#,
    )
    .unwrap();
    let provider_rel = common::slot_dir("org.barrier.z-binary-provider", "0.1.0");
    fs::write(
        provider.join("src/main.rs"),
        format!(
            r###"use std::io::Read;
fn main() {{
    let target = std::env::current_dir().unwrap();
    assert!(target.join("target.ready").is_file());
    let project = target.ancestors().nth(4).unwrap();
    assert!(project.join("{provider_rel}/provider.ready").is_file());
    let lock = std::fs::read_to_string(project.join("vibe.lock")).unwrap();
    assert!(!lock.contains("a-binary-target"));
    let mut context = String::new();
    std::io::stdin().read_to_string(&mut context).unwrap();
    assert!(context.contains("a-binary-target"));
    std::fs::write(project.join(".vibe/pre-barrier-binary"), "binary").unwrap();
    std::io::Write::write_all(
        &mut std::io::stdout(),
        br#"{{"artifacts":[],"envelope":1,"message":"binary barrier ok","status":"ok","tasks":[]}}"#,
    ).unwrap();
}}
"###,
        ),
    )
    .unwrap();
}

fn fresh_install(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
    target: &str,
) -> std::process::Output {
    user.vibe()
        .args(["--json", "install", target])
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

fn documents(output: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(output)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn assert_plan_precedes_outcome(docs: &[serde_json::Value], id: &str) {
    let plan = docs
        .iter()
        .position(|doc| {
            doc["command"] == "lifecycle:plan"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["reference"] == id))
        })
        .unwrap();
    let outcome = docs
        .iter()
        .position(|doc| {
            doc["command"] == "lifecycle"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["reference"] == id))
        })
        .unwrap();
    assert!(plan < outcome && outcome < docs.len() - 1);
    assert_eq!(docs.last().unwrap()["command"], "install");
}

#[test]
fn fresh_script_provider_is_available_before_target_pre_and_boot_lock() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    write_script_world(registry.path());

    let output = fresh_install(
        &user,
        project.path(),
        registry.path(),
        "org.barrier/a-script-target@=0.1.0",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.path().join(".vibe/pre-barrier-script").is_file());
    assert!(
        fs::read_to_string(project.path().join(common::index_rel()))
            .unwrap()
            .contains("boot/generated.md")
    );
    let docs = documents(&output.stdout);
    assert_plan_precedes_outcome(&docs, "org.barrier/z-script-provider#cross-provider-script");
}

#[test]
fn fresh_binary_provider_builds_only_after_its_slot_exists() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    write_binary_world(registry.path());

    let output = fresh_install(
        &user,
        project.path(),
        registry.path(),
        "org.barrier/a-binary-target@=0.1.0",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.path().join(".vibe/pre-barrier-binary").is_file());
    let docs = documents(&output.stdout);
    assert_plan_precedes_outcome(&docs, "org.barrier/z-binary-provider#cross-provider-binary");
    let outcome = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .flat_map(|doc| doc["contributions"].as_array().into_iter().flatten())
        .find(|row| row["reference"] == "org.barrier/z-binary-provider#cross-provider-binary")
        .unwrap();
    assert_eq!(outcome["message"], "binary barrier ok");
}
