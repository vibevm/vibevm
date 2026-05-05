//! Integration tests for `vibe init`.
//!
//! Spec: `VIBEVM-SPEC.md` §11.1 and the M0 acceptance checklist in §16.

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

#[test]
fn init_creates_expected_layout() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .assert()
        .success();

    for rel in [
        "CLAUDE.md",
        "AGENTS.md",
        "GEMINI.md",
        "spec/boot/00-core.md",
        "spec/boot/90-user.md",
        "spec/WAL.md",
        "vibe.toml",
        "vibe.lock",
        ".vibe/.gitignore",
        ".gitignore",
    ] {
        assert!(
            path.join(rel).exists(),
            "expected `{rel}` to exist after init"
        );
    }

    // CLAUDE.md / AGENTS.md / GEMINI.md have the exact same one-line body.
    let claude = fs::read_to_string(path.join("CLAUDE.md")).unwrap();
    let agents = fs::read_to_string(path.join("AGENTS.md")).unwrap();
    let gemini = fs::read_to_string(path.join("GEMINI.md")).unwrap();
    assert_eq!(claude, agents);
    assert_eq!(agents, gemini);
    assert!(claude.trim_end().ends_with("await the user's instructions."));

    // vibe.toml should parse as a valid ProjectManifest.
    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed: vibe_core::manifest::ProjectManifest = toml::from_str(&manifest_text).unwrap();
    assert_eq!(parsed.project.version, "0.0.1");
    assert!(parsed.project.name.ends_with(
        path.file_name().unwrap().to_str().unwrap()
    ) || parsed.project.name == path.file_name().unwrap().to_str().unwrap());

    // Empty lockfile parses back and carries the expected metadata.
    let lock_text = fs::read_to_string(path.join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert!(lock.packages.is_empty());
    assert!(lock.meta.generated_by.starts_with("vibe "));
}

#[test]
fn init_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    // First run.
    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .assert()
        .success();

    // Mark boot/00-core.md with a user edit, then re-init.
    let user_marker = "# EDITED BY USER\n";
    let core_path = path.join("spec/boot/00-core.md");
    fs::write(&core_path, user_marker).unwrap();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .assert()
        .success()
        .stdout(predicate::str::contains("kept"));

    // Second run must NOT overwrite the user's edit.
    let after = fs::read_to_string(&core_path).unwrap();
    assert_eq!(after, user_marker, "00-core.md must be preserved");
}

#[test]
fn init_stack_flag_sets_active_stack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .arg("--stack")
        .arg("rust-cli")
        .assert()
        .success();

    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed: vibe_core::manifest::ProjectManifest = toml::from_str(&manifest_text).unwrap();
    assert_eq!(
        parsed.active.as_ref().and_then(|a| a.stack.as_deref()),
        Some("rust-cli")
    );
}

#[test]
fn init_custom_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .arg("--name")
        .arg("my-special-project")
        .assert()
        .success();

    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed: vibe_core::manifest::ProjectManifest = toml::from_str(&manifest_text).unwrap();
    assert_eq!(parsed.project.name, "my-special-project");
}

#[test]
fn init_json_output_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    let out = vibe()
        .arg("--json")
        .arg("init")
        .arg("--path")
        .arg(path)
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "init");
    assert_eq!(v["created"], 10);
    assert_eq!(v["kept"], 0);
}

#[test]
fn init_quiet_emits_single_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    let out = vibe()
        .arg("--quiet")
        .arg("init")
        .arg("--path")
        .arg(path)
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let trimmed = stdout.trim();
    assert!(!trimmed.contains('\n'), "quiet output must be single line: {trimmed:?}");
    assert!(trimmed.contains("vibe init:"));
}

#[test]
fn init_version() {
    vibe().arg("version").assert().success();
    vibe().arg("--version").assert().success();
}

#[test]
fn init_writes_default_registry() {
    // Default `vibe init` provisions two `[[registry]]` blocks: GitHub
    // primary (canonical publish target) + GitVerse secondary
    // (resolve-time fall-through). The block order is load-bearing —
    // primary first drives `vibe registry publish` and the resolver
    // walk order. See `resolve_registry_sections` in
    // `crates/vibe-cli/src/commands/init.rs`.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe().arg("init").arg("--path").arg(path).assert().success();

    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed: vibe_core::manifest::ProjectManifest =
        toml::from_str(&manifest_text).unwrap();
    assert_eq!(
        parsed.registries.len(),
        2,
        "default `vibe init` writes both GitHub + GitVerse registries; got: {manifest_text}"
    );

    let primary = &parsed.registries[0];
    assert_eq!(primary.name, vibe_core::manifest::DEFAULT_REGISTRY_NAME);
    assert_eq!(primary.url, vibe_core::manifest::DEFAULT_REGISTRY_URL);
    assert_eq!(primary.r#ref, vibe_core::manifest::DEFAULT_REGISTRY_REF);
    assert_eq!(primary.naming, vibe_core::manifest::NamingConvention::KindName);

    let secondary = &parsed.registries[1];
    assert_eq!(
        secondary.name,
        vibe_core::manifest::DEFAULT_REGISTRY_GITVERSE_NAME
    );
    assert_eq!(
        secondary.url,
        vibe_core::manifest::DEFAULT_REGISTRY_GITVERSE_URL
    );
    // GitVerse default uses `naming = "name"` (no kind-prefix);
    // the public `vibespecs` org on GitVerse provisions repos
    // under bare package names (`vibevm-direct-push-smoke`) and the
    // resolver must match that to find them.
    assert_eq!(
        secondary.naming,
        vibe_core::manifest::NamingConvention::Name
    );

    assert!(
        manifest_text.contains("[[registry]]"),
        "manifest must contain [[registry]]: {manifest_text}"
    );
    assert!(manifest_text.contains(vibe_core::manifest::DEFAULT_REGISTRY_URL));
    assert!(manifest_text.contains(vibe_core::manifest::DEFAULT_REGISTRY_GITVERSE_URL));
}

#[test]
fn init_no_registry_flag_omits_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .arg("--no-registry")
        .assert()
        .success();

    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed: vibe_core::manifest::ProjectManifest =
        toml::from_str(&manifest_text).unwrap();
    assert!(
        parsed.registries.is_empty(),
        "[[registry]] must be absent after --no-registry: {manifest_text}"
    );
    assert!(!manifest_text.contains("[[registry]]"));
    assert!(!manifest_text.contains("[registry]"));
}

#[test]
fn init_registry_url_override() {
    // `--registry-url` replaces both default registries with a single
    // operator-controlled one. The GitVerse fall-through default is
    // intentionally dropped — the operator who supplied a custom URL
    // is asking for an explicit, single-source layout.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .arg("--registry-url")
        .arg("git+https://example.test/registry.git")
        .arg("--registry-ref")
        .arg("develop")
        .assert()
        .success();

    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed: vibe_core::manifest::ProjectManifest =
        toml::from_str(&manifest_text).unwrap();
    assert_eq!(
        parsed.registries.len(),
        1,
        "--registry-url replaces defaults with a single entry; got: {manifest_text}"
    );
    let reg = parsed
        .primary_registry()
        .expect("[[registry]] should exist");
    assert_eq!(reg.url, "git+https://example.test/registry.git");
    assert_eq!(reg.r#ref, "develop");
    // Non-default ref must be serialized.
    assert!(manifest_text.contains("develop"));
    // GitVerse default must NOT appear when the operator supplied their own URL.
    assert!(!manifest_text.contains(vibe_core::manifest::DEFAULT_REGISTRY_GITVERSE_URL));
}

#[test]
fn init_registry_url_and_no_registry_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .arg("--registry-url")
        .arg("git+file:///whatever")
        .arg("--no-registry")
        .assert()
        .failure();
}
