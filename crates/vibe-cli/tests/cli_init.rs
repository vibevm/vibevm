//! Integration tests for `vibe init`.
//!
//! Spec: `VIBEVM-SPEC.md` §11.1 and the M0 acceptance checklist in §16.
//!
//! Every command here is built by [`common::UserScratch`], never by a
//! local builder. `vibe init` resolves its author `--author` →
//! `<settings>/config.toml` `[init] last_author` → `git config user.name`
//! and writes the result back to that same file when it differs, so a
//! command pointed at the real home both *reads* the developer's name and
//! — on a machine where `last_author` is unset — *writes* into their
//! `~/.vibe/config.toml`. That was F-056, and it existed because this file
//! carried a builder of its own that set `VIBE_NO_DEFAULT_REGISTRY` and
//! stopped there.

use std::fs;

use predicates::prelude::*;

mod common;

use common::UserScratch;

#[test]
fn init_creates_expected_layout() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .assert()
        .success();

    for rel in [
        "CLAUDE.md".to_string(),
        "AGENTS.md".to_string(),
        "GEMINI.md".to_string(),
        common::boot_rel("00-core.md"),
        common::boot_rel("90-user.md"),
        common::index_rel(),
        "vibe.toml".to_string(),
        "vibe.lock".to_string(),
        ".vibe/.gitignore".to_string(),
        ".gitignore".to_string(),
    ] {
        assert!(
            path.join(&rel).exists(),
            "expected `{rel}` to exist after init"
        );
    }
    // The WAL file is NOT created by default — WAL discipline is a
    // project convention, not part of the package manager's contract.
    // Operators who want the WAL protocol install it explicitly via
    // `vibe install org.vibevm/wal` or write the file themselves.
    let wal_rel = vibe_core::machine_json_path(&vibe_core::layout::current_wal_md());
    assert!(
        !path.join(&wal_rel).exists(),
        "`{wal_rel}` must NOT be created by default; it's a project convention, not part of the package manager"
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
    assert!(claude.contains(&common::index_rel()));

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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    // First run.
    user.vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .assert()
        .success();

    // Mark boot/00-core.md with a user edit, then re-init.
    let user_marker = "# EDITED BY USER\n";
    let core_path = path.join(common::spec_rel("boot/00-core.md"));
    fs::write(&core_path, user_marker).unwrap();

    user.vibe()
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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    let out = user
        .vibe()
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
    // 10 files created by default: the two boot snippets, vibe.toml,
    // vibe.lock (manifest + lockfile), .vibe/.gitignore, .gitignore
    // (root), and the 4 generated boot artifacts — the boot manifest
    // plus the managed `<vibevm>` block in CLAUDE.md / AGENTS.md /
    // GEMINI.md (PROP-009 / PROP-012). The WAL is NOT created — it's a
    // project convention.
    assert_eq!(v["created"], 10);
    assert_eq!(v["kept"], 0);
}

#[test]
fn init_quiet_emits_single_line() {
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    let out = user
        .vibe()
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
    let user = UserScratch::new();
    user.vibe().arg("version").assert().success();
    user.vibe().arg("--version").assert().success();
}

#[test]
fn init_default_has_no_project_registries() {
    // Since the default pair (vibespecs GitHub + GitVerse) moved from
    // the project `vibe.toml` to the machine-global `~/.vibe/registry.toml`
    // (seeded by `ensure_default_global_registry`), a default `vibe init`
    // produces a project `vibe.toml` with NO `[[registry]]` sections.
    // The project stays clean of registry boilerplate; a project only
    // carries `[[registry]]` entries it needs *beyond* the machine default.
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
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
    let user = UserScratch::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    user.vibe()
        .arg("init")
        .arg("--path")
        .arg(path)
        .arg("--registry-url")
        .arg("git+file:///whatever")
        .arg("--no-registry")
        .assert()
        .failure();
}
