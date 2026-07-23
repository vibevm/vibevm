//! Integration tests for `vibe init`.
//!
//! Spec: `VIBEVM-SPEC.md` §11.1 and the M0 acceptance checklist in §16.

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn vibe() -> Command {
    let mut cmd = Command::cargo_bin("vibe").expect("vibe binary built");
    // Suppress global-registry seeding so tests don't pollute the real
    // ~/.vibe/registry.toml or pick up real-world registries.
    cmd.env("VIBE_NO_DEFAULT_REGISTRY", "1");
    cmd
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
        "spec/boot/INDEX.md",
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
    // `spec/WAL.md` is NOT created by default — WAL discipline is a
    // project convention, not part of the package manager's contract.
    // Operators who want the WAL protocol install it explicitly via
    // `vibe install org.vibevm/wal` or write the file themselves.
    assert!(
        !path.join("spec/WAL.md").exists(),
        "spec/WAL.md must NOT be created by default; it's a project convention, not part of the package manager"
    );

    // CLAUDE.md / AGENTS.md / GEMINI.md each carry vibevm's managed
    // `<vibevm>` block (PROP-012), identical in all three.
    let claude = fs::read_to_string(path.join("CLAUDE.md")).unwrap();
    let agents = fs::read_to_string(path.join("AGENTS.md")).unwrap();
    let gemini = fs::read_to_string(path.join("GEMINI.md")).unwrap();
    assert_eq!(claude, agents);
    assert_eq!(agents, gemini);
    assert!(
        claude.contains("<vibevm>") && claude.contains("</vibevm>"),
        "CLAUDE.md must carry the managed <vibevm> block: {claude}"
    );
    assert!(claude.contains("spec/boot/INDEX.md"));

    // vibe.toml should parse as a valid Manifest.
    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
    assert_eq!(parsed.require_project().unwrap().version, "0.0.1");
    assert!(
        parsed
            .require_project()
            .unwrap()
            .name
            .ends_with(path.file_name().unwrap().to_str().unwrap())
            || parsed.require_project().unwrap().name
                == path.file_name().unwrap().to_str().unwrap()
    );

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
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
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
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
    assert_eq!(parsed.require_project().unwrap().name, "my-special-project");
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
    // 10 files created by default: spec/boot/00-core.md,
    // spec/boot/90-user.md (2 boot snippets), vibe.toml, vibe.lock
    // (manifest + lockfile), .vibe/.gitignore, .gitignore (root), and
    // the 4 generated boot artifacts — spec/boot/INDEX.md plus the
    // managed `<vibevm>` block in CLAUDE.md / AGENTS.md / GEMINI.md
    // (PROP-009 / PROP-012). spec/WAL.md is NOT created — it's a
    // project convention.
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
    assert!(
        !trimmed.contains('\n'),
        "quiet output must be single line: {trimmed:?}"
    );
    assert!(trimmed.contains("vibe init:"));
}

#[test]
fn init_version() {
    vibe().arg("version").assert().success();
    vibe().arg("--version").assert().success();
}

#[test]
fn init_default_has_no_project_registries() {
    // Since the default pair (vibespecs GitHub + GitVerse) moved from
    // the project `vibe.toml` to the machine-global `~/.vibe/registry.toml`
    // (seeded by `ensure_default_global_registry`), a default `vibe init`
    // produces a project `vibe.toml` with NO `[[registry]]` sections.
    // The project stays clean of registry boilerplate; a project only
    // carries `[[registry]]` entries it needs *beyond* the machine default.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .assert()
        .success();

    let manifest_text = fs::read_to_string(path.join("vibe.toml")).unwrap();
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
    assert_eq!(
        parsed.registries.len(),
        0,
        "default `vibe init` writes no [[registry]] blocks (they live in \
         ~/.vibe/registry.toml now); got: {manifest_text}"
    );
    assert!(
        !manifest_text.contains("[[registry]]"),
        "project vibe.toml must not contain [[registry]]: {manifest_text}"
    );
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
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
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
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
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
    // A custom registry inherits the project-wide `fqdn` default.
    assert_eq!(reg.naming, vibe_core::manifest::NamingConvention::Fqdn);
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
