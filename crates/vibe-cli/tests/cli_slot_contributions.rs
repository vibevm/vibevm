//! Explicit `slot:` contributions at the production install boundary — the
//! same always-run engine the `[hooks]` sugar desugars into, driven WITHOUT
//! any sugar.
//!
//! Covers the slot artifact reaching a later phase envelope across a world
//! reload, a host slot declaration keeping its own target context / state /
//! scratch, and a typed soft post-install failure preserving streams while a
//! later target still runs. The sugar half is `cli_hook_lifecycle.rs`.

mod common;

use std::fs;

use common::UserScratch;
use common::hook_slot::{
    Fixture, documents, install, lifecycle_state, plain_package, post_package, setup,
    slot_outcomes, state_key,
};

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
