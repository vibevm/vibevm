//! `[hooks]` compatibility sugar at the production install boundary.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

#[derive(Clone, Copy)]
enum Fixture {
    Timing,
    PreFail,
    PostFail,
}

fn registry(root: &Path, fixture: Fixture) -> PathBuf {
    let package = root
        .join("registry")
        .join("org.example")
        .join("hooked")
        .join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::create_dir_all(package.join("boot")).unwrap();
    let hooks = match fixture {
        Fixture::Timing => {
            r#"
[hooks]
pre-install = "hooks/pre"
post-install = "hooks/post"
"#
        }
        Fixture::PreFail => {
            r#"
[hooks]
pre-install = "hooks/fail"
"#
        }
        Fixture::PostFail => {
            r#"
[hooks]
post-install = "hooks/fail"
"#
        }
    };
    fs::write(
        package.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.example"
name = "hooked"
kind = "flow"
version = "0.1.0"

[boot_snippet]
source = "boot/generated.md"
category = "flow"
{hooks}"#,
        ),
    )
    .unwrap();
    fs::write(
        package.join("hooks/pre.sh"),
        "set -eu\nprintf 'pre\\n' >> hook-order.txt\nprintf 'generated before boot\\n' > boot/generated.md\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.sh"),
        "set -eu\ntest -f \"$VIBE_PROJECT_ROOT/vibe.lock\"\nprintf 'post\\n' >> hook-order.txt\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/fail.sh"),
        "printf SOFT-STDOUT\nprintf SOFT-STDERR >&2\nexit 17\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/pre.ps1"),
        "Add-Content -LiteralPath hook-order.txt -Value pre\nSet-Content -LiteralPath boot/generated.md -Value 'generated before boot'\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.ps1"),
        "$lock = Join-Path $env:VIBE_PROJECT_ROOT 'vibe.lock'\nif (-not (Test-Path -LiteralPath $lock)) { exit 19 }\nAdd-Content -LiteralPath hook-order.txt -Value post\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/fail.ps1"),
        "Write-Output SOFT-STDOUT\n[Console]::Error.Write('SOFT-STDERR')\nexit 17\n",
    )
    .unwrap();
    root.join("registry")
}

fn plain_package(registry: &Path, name: &str) {
    let package = registry.join("org.multi").join(name).join("v0.1.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!("[package]\ngroup='org.multi'\nname='{name}'\nkind='tool'\nversion='0.1.0'\n"),
    )
    .unwrap();
}

fn post_package(registry: &Path, name: &str, fail: bool) {
    let package = registry.join("org.continue").join(name).join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.continue'\nname='{name}'\nkind='tool'\nversion='0.1.0'\n\n[hooks]\npost-install='hooks/post'\n"
        ),
    )
    .unwrap();
    let shell = if fail {
        "printf FIRST-FAIL >&2\nexit 19\n"
    } else {
        "printf later > \"$VIBE_PROJECT_ROOT/.vibe/later-post-ran\"\n"
    };
    let powershell = if fail {
        "[Console]::Error.Write('FIRST-FAIL')\nexit 19\n"
    } else {
        "Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/later-post-ran') -Value later -NoNewline\n"
    };
    fs::write(package.join("hooks/post.sh"), shell).unwrap();
    fs::write(package.join("hooks/post.ps1"), powershell).unwrap();
}

fn install(user: &UserScratch, project: &Path, registry: &Path) -> std::process::Output {
    user.vibe()
        .arg("--json")
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
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

fn slot_outcomes(docs: &[serde_json::Value]) -> Vec<serde_json::Value> {
    docs.iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .flat_map(|doc| doc["contributions"].as_array().into_iter().flatten())
        .filter(|row| {
            row["point"]
                .as_str()
                .is_some_and(|point| point.starts_with("slot:"))
        })
        .cloned()
        .collect()
}

fn state_key(id: &str) -> String {
    format!("org.example/hooked#{id}@slot(org.example/hooked@0.1.0)")
}

fn lifecycle_state(project: &Path) -> LifecycleState {
    toml::from_str(&fs::read_to_string(project.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

fn setup(fixture: Fixture) -> (tempfile::TempDir, UserScratch, tempfile::TempDir, PathBuf) {
    let outer = tempfile::tempdir().unwrap();
    let registry = registry(outer.path(), fixture);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    (outer, user, project, registry)
}

#[test]
fn hook_sugar_runs_once_at_pre_and_post_install_timing() {
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let output = install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let slot = project
        .path()
        .join(common::slot_dir("org.example.hooked", "0.1.0"));
    assert_eq!(
        fs::read_to_string(slot.join("hook-order.txt"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["pre", "post"],
    );
    let generated_lane = project.path().join(common::index_rel());
    assert!(
        fs::read_to_string(generated_lane)
            .unwrap()
            .contains("generated.md"),
        "pre hook must create its declared boot source before boot regeneration",
    );
    let docs = documents(&output.stdout);
    let plan_index = docs
        .iter()
        .position(|doc| {
            doc["command"] == "lifecycle:plan"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["point"] == "slot:pre-install"))
        })
        .expect("slot ritual must be surfaced before execution");
    let outcome_index = docs
        .iter()
        .position(|doc| {
            doc["command"] == "lifecycle"
                && doc["contributions"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|row| row["point"] == "slot:pre-install"))
        })
        .expect("slot outcomes use the generated lifecycle report");
    let planned = docs[plan_index]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["point"] == "slot:pre-install")
        .unwrap();
    assert_eq!(
        planned["reference"],
        "org.example/hooked#@vibe/hooks/pre-install"
    );
    assert_eq!(planned["slot_target"]["name"], "hooked");
    assert!(plan_index < outcome_index && outcome_index < docs.len() - 1);
    assert_eq!(docs.last().unwrap()["command"], "install");
    assert!(docs.last().unwrap().get("lifecycle_hooks").is_none());
    let hooks = slot_outcomes(&docs);
    assert_eq!(
        hooks.len(),
        2,
        "pre/post sugar must each execute exactly once"
    );
    assert_eq!(hooks[0]["point"], "slot:pre-install");
    assert_eq!(hooks[1]["point"], "slot:post-install");
    let state = lifecycle_state(project.path());
    assert_eq!(
        state.execution[&state_key("@vibe/hooks/pre-install")].status,
        ExecutionRecordStatus::Ok,
    );
    assert_eq!(
        state.execution[&state_key("@vibe/hooks/post-install")].status,
        ExecutionRecordStatus::Ok,
    );

    let second = install(&user, project.path(), &registry);
    assert!(second.status.success());
    assert_eq!(
        fs::read_to_string(slot.join("hook-order.txt"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["pre", "post"],
    );
}

#[test]
fn pre_failure_aborts_and_rolls_back_the_slot() {
    let (_outer, user, project, registry) = setup(Fixture::PreFail);
    let lock_before = fs::read(project.path().join("vibe.lock")).unwrap();
    let output = install(&user, project.path(), &registry);
    assert!(!output.status.success());
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists(),
        "failed pre hook must roll the materialised slot back",
    );
    assert_eq!(
        fs::read(project.path().join("vibe.lock")).unwrap(),
        lock_before,
        "failed pre hook must not register the package in the lockfile",
    );
    assert_eq!(
        lifecycle_state(project.path()).execution[&state_key("@vibe/hooks/pre-install")].status,
        ExecutionRecordStatus::Fail,
    );
    let docs = documents(&output.stdout);
    assert_eq!(docs[0]["command"], "install:plan");
    let slot_plan = docs
        .iter()
        .position(|doc| doc["command"] == "lifecycle:plan")
        .unwrap();
    let failure = docs
        .iter()
        .position(|doc| doc["command"] == "lifecycle")
        .unwrap();
    assert!(slot_plan < failure);
    assert_eq!(docs[failure]["ok"], false);
    assert_eq!(docs[failure]["contributions"][0]["status"], "fail");
    assert!(
        !output.stderr.is_empty(),
        "terminal error follows failure outcome"
    );
}

#[test]
fn post_nonzero_is_flagged_after_the_install_is_durable() {
    let (_outer, user, project, registry) = setup(Fixture::PostFail);
    let output = install(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(project.path().join("vibe.lock").is_file());
    assert!(
        project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .is_dir(),
    );
    let docs = documents(&output.stdout);
    let hooks = slot_outcomes(&docs);
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["point"], "slot:post-install");
    assert_eq!(hooks[0]["status"], "fail");
    assert_eq!(hooks[0]["flagged"], true);
    assert!(hooks[0]["stdout"].as_str().unwrap().contains("SOFT-STDOUT"));
    assert!(hooks[0]["stderr"].as_str().unwrap().contains("SOFT-STDERR"));
    assert_eq!(
        lifecycle_state(project.path()).execution[&state_key("@vibe/hooks/post-install")].status,
        ExecutionRecordStatus::Fail,
    );
}

#[test]
fn slot_artifact_reaches_the_later_phase_envelope_across_world_reload() {
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let package = registry.join("org.example").join("hooked").join("v0.1.0");
    fs::write(
        package.join("hooks/pre.sh"),
        r#"set -eu
artifact="$VIBE_PROJECT_ROOT/slot-artifact.txt"
printf artifact > "$artifact"
wire=$(printf '%s' "$artifact" | sed 's#\\#/#g')
printf '{"artifacts":[{"id":"slot-artifact","kind":"file","path":"%s"}],"envelope":1,"status":"ok","tasks":[]}' "$wire" > "$VIBE_REPLY"
"#,
    )
    .unwrap();
    fs::write(
        package.join("hooks/pre.ps1"),
        r#"$artifact = Join-Path $env:VIBE_PROJECT_ROOT 'slot-artifact.txt'
Set-Content -LiteralPath $artifact -Value artifact -NoNewline
$wire = $artifact.Replace('\','/')
@{artifacts=@(@{id='slot-artifact';kind='file';path=$wire});envelope=1;status='ok';tasks=@()} | ConvertTo-Json -Compress -Depth 4 | Set-Content -LiteralPath $env:VIBE_REPLY -NoNewline
"#,
    )
    .unwrap();
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/observe.sh"),
        "set -eu\ngrep -q '\"id\":\"slot-artifact\"' \"$VIBE_CONTEXT\"\nprintf observed > .vibe/slot-artifact-observed\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/observe.ps1"),
        "$wire = Get-Content -Raw -LiteralPath $env:VIBE_CONTEXT\nif ($wire -notmatch 'slot-artifact') { exit 23 }\nSet-Content -LiteralPath .vibe/slot-artifact-observed -Value observed -NoNewline\n",
    )
    .unwrap();
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"
[requires.packages]
"org.example/hooked" = "=0.1.0"

[[extension]]
id = "observe-slot-artifact"
point = "phase:build"
handler = { kind = "script", base = "scripts/observe" }
"#,
    );
    fs::write(manifest, body).unwrap();

    let output = user
        .vibe()
        .args(["build", "--json", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(&registry)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        project
            .path()
            .join(".vibe/slot-artifact-observed")
            .is_file()
    );
    let docs = documents(&output.stdout);
    let slot = slot_outcomes(&docs);
    assert_eq!(slot[0]["point"], "slot:pre-install");
    let contributions = docs.last().unwrap()["contributions"].as_array().unwrap();
    assert!(
        contributions.last().unwrap()["key"]
            .as_str()
            .unwrap()
            .ends_with("#observe-slot-artifact")
    );
}

#[test]
fn explicit_slot_contribution_uses_the_same_always_run_engine_without_sugar() {
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let package = registry.join("org.example").join("hooked").join("v0.1.0");
    let manifest = package.join("vibe.toml");
    let body = fs::read_to_string(&manifest).unwrap().replace(
        "[hooks]\npre-install = \"hooks/pre\"\npost-install = \"hooks/post\"\n",
        r#"[[extension]]
id = "explicit-pre"
point = "slot:pre-install"
handler = { kind = "script", base = "hooks/pre" }
"#,
    );
    fs::write(manifest, body).unwrap();
    let output = install(&user, project.path(), &registry);
    assert!(output.status.success());
    let docs = documents(&output.stdout);
    let hooks = slot_outcomes(&docs);
    assert_eq!(hooks.len(), 1);
    assert_eq!(
        hooks[0]["key"],
        "org.example/hooked#explicit-pre@slot(org.example/hooked@0.1.0)"
    );
    assert_eq!(hooks[0]["reference"], "org.example/hooked#explicit-pre");
    assert_eq!(hooks[0]["slot_target"]["name"], "hooked");
    assert_eq!(hooks[0]["point"], "slot:pre-install");
    assert_eq!(hooks[0]["status"], "ok");
    assert!(
        !lifecycle_state(project.path())
            .execution
            .contains_key(&state_key("@vibe/hooks/pre-install"))
    );
}

#[test]
fn one_host_slot_declaration_has_distinct_target_context_state_and_scratch() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    plain_package(registry.path(), "one");
    plain_package(registry.path(), "two");
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/fanout.sh"),
        "set -eu\nmkdir -p \"$VIBE_PROJECT_ROOT/.vibe/contexts\"\ncp \"$VIBE_CONTEXT\" \"$VIBE_PROJECT_ROOT/.vibe/contexts/$VIBE_PACKAGE_NAME.json\"\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/fanout.ps1"),
        "$dir = Join-Path $env:VIBE_PROJECT_ROOT '.vibe/contexts'\nNew-Item -ItemType Directory -Force $dir | Out-Null\nCopy-Item -LiteralPath $env:VIBE_CONTEXT -Destination (Join-Path $dir ($env:VIBE_PACKAGE_NAME + '.json'))\n",
    )
    .unwrap();
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str(
        r#"
[requires.packages]
"org.multi/one" = "=0.1.0"
"org.multi/two" = "=0.1.0"

[[extension]]
id = "fanout"
point = "slot:pre-install"
handler = { kind = "script", base = "scripts/fanout" }
applies_to = { packages = ["org.multi/*"] }
"#,
    );
    fs::write(manifest_path, manifest).unwrap();
    let output = user
        .vibe()
        .args(["build", "--json", "--path"])
        .arg(project.path())
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
    let one: serde_json::Value =
        serde_json::from_slice(&fs::read(project.path().join(".vibe/contexts/one.json")).unwrap())
            .unwrap();
    let two: serde_json::Value =
        serde_json::from_slice(&fs::read(project.path().join(".vibe/contexts/two.json")).unwrap())
            .unwrap();
    assert_eq!(one["execution"]["package"], two["execution"]["package"]);
    assert_ne!(one["slot_target"], two["slot_target"]);
    assert_ne!(one["io"]["scratch"], two["io"]["scratch"]);
    let lock = vibe_core::manifest::Lockfile::read(project.path().join("vibe.lock")).unwrap();
    let locked_names = lock
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect::<Vec<_>>();
    for context in [&one, &two] {
        assert_eq!(
            context["world"]["packages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|package| package["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            locked_names,
        );
    }
    let state = lifecycle_state(project.path());
    let fanout = state
        .execution
        .keys()
        .filter(|key| key.contains("#fanout@slot(org.multi/"))
        .collect::<Vec<_>>();
    assert_eq!(fanout.len(), 2);
}

#[test]
fn typed_soft_post_failure_preserves_streams_and_continues_later_target() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    post_package(registry.path(), "a-fail", true);
    post_package(registry.path(), "z-later", false);
    let output = user
        .vibe()
        .arg("--json")
        .arg("install")
        .args(["org.continue/a-fail@=0.1.0", "org.continue/z-later@=0.1.0"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.path().join(".vibe/later-post-ran").is_file());
    let docs = documents(&output.stdout);
    let outcomes = slot_outcomes(&docs);
    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0]["slot_target"]["name"], "a-fail");
    assert_eq!(outcomes[0]["flagged"], true);
    assert!(
        outcomes[0]["stderr"]
            .as_str()
            .unwrap()
            .contains("FIRST-FAIL")
    );
    assert_eq!(outcomes[1]["slot_target"]["name"], "z-later");
    assert_eq!(outcomes[1]["status"], "ok");
    assert_eq!(docs.last().unwrap()["command"], "install");
}
