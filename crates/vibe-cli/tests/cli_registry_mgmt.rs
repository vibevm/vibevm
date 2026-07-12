//! End-to-end tests for the remaining CLI surfaces: `vibe update` /
//! `outdated`, `show` / `check` / config layering, `vibe registry vendor` /
//! `set-mirror`, feature-aware installs, the `vibe mcp` provisioning and
//! stdio-serve walk, the PROP-003 r2 omnibus fixtures, conditional
//! dependencies, the help-text smoke, and `vibe reinstall`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{
    git_available, init_project, make_per_package_registry, make_wal_dir_registry, run_git, vibe,
    write_project_with_per_package_registry,
};
use predicates::prelude::*;

/// The vibevm MCP launcher argv this host writes: `cmd /c vibe mcp serve`
/// on Windows (the `.cmd` shim), plain `vibe mcp serve` elsewhere.
fn expected_vibevm_argv() -> Vec<String> {
    let parts: &[&str] = if cfg!(windows) {
        &["cmd", "/c", "vibe", "mcp", "serve"]
    } else {
        &["vibe", "mcp", "serve"]
    };
    parts.iter().map(|s| s.to_string()).collect()
}

/// Flatten a JSON `{ "command": ..., "args": [...] }` MCP entry to argv.
fn json_entry_argv(entry: &serde_json::Value) -> Vec<String> {
    let mut v = vec![entry["command"].as_str().unwrap().to_string()];
    for a in entry["args"].as_array().unwrap() {
        v.push(a.as_str().unwrap().to_string());
    }
    v
}

/// Build a per-package git registry where `flow-wal` carries TWO
/// tagged versions: `v0.1.0` (from the in-tree fixture, content
/// rewritten so the project file the test asserts on lives at a
/// stable known path) and `v0.2.0` (one file modified, one new file
/// added, one file removed, boot snippet unchanged). Used by the
/// `vibe update` end-to-end tests.
fn make_two_version_registry(root: &Path) -> PathBuf {
    let src = root.join("src-flow-wal");
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);

    // v0.1.0 layout: A.md + B.md + boot snippet, capabilities empty.
    let manifest_v1 = r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"

[boot_snippet]
source = "boot/10-flow-wal.md"
category = "flow"
"#;
    // Pin LF endings in the fixture repo — otherwise on Windows
    // git's `core.autocrlf` will rewrite text on checkout and the
    // bytes the test asserts on (`"v1 A\n"`) won't match what
    // ends up in the cache (`"v1 A\r\n"`).
    fs::write(src.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    fs::write(src.join("vibe.toml"), manifest_v1).unwrap();
    fs::create_dir_all(src.join("spec/flows/wal")).unwrap();
    fs::create_dir_all(src.join("boot")).unwrap();
    fs::write(src.join("spec/flows/wal/A.md"), "v1 A\n").unwrap();
    fs::write(src.join("spec/flows/wal/B.md"), "v1 B\n").unwrap();
    fs::write(src.join("boot/10-flow-wal.md"), "v1 boot\n").unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "org.vibevm/wal@0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);

    // v0.2.0: A modified, B removed, C added, boot unchanged.
    let manifest_v2 = r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.2.0"

[boot_snippet]
source = "boot/10-flow-wal.md"
category = "flow"
"#;
    fs::write(src.join("vibe.toml"), manifest_v2).unwrap();
    fs::write(src.join("spec/flows/wal/A.md"), "v2 A — changed!\n").unwrap();
    fs::write(src.join("spec/flows/wal/C.md"), "v2 C\n").unwrap();
    fs::remove_file(src.join("spec/flows/wal/B.md")).unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "org.vibevm/wal@0.2.0"]);
    run_git(&src, &["tag", "v0.2.0"]);

    let bare = root.join("org.vibevm_wal.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            src.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    root.to_path_buf()
}

/// `vibe update` re-resolves the project's `[requires]` afresh — the
/// depsolver picks the newest version inside each declared constraint —
/// and re-materialises the result into `vibedeps/`. When the manifest
/// constraint admits a newer release, the package's slot moves from the
/// old version directory to the new one and the lockfile entry is
/// bumped. The PROP-009 model retired the per-file added/removed/
/// modified diff output.
#[test]
fn update_bumps_to_new_version_and_remateralises() {
    if !git_available() {
        eprintln!("skipping update_bumps_to_new_version_and_remateralises: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_two_version_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    // Install pinned to v0.1.0 exactly.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/wal@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    assert_eq!(lock.packages[0].version.to_string(), "0.1.0");
    assert!(
        project
            .path()
            .join("vibedeps/flow-wal/0.1.0/spec/flows/wal/B.md")
            .is_file(),
        "the v0.1.0 tree (with B.md) is materialised into its slot"
    );

    // Widen the manifest constraint to `*` so a re-resolve admits
    // v0.2.0 — the case where the operator deliberately loosens the
    // pin and then runs `vibe update`.
    let toml_path = project.path().join("vibe.toml");
    let mut manifest =
        vibe_core::manifest::Manifest::parse_str(&fs::read_to_string(&toml_path).unwrap()).unwrap();
    manifest.requires.packages[0] = vibe_core::PackageRef::parse("org.vibevm/wal@*").unwrap();
    manifest.write(&toml_path).unwrap();

    // `vibe update` re-resolves `[requires]` afresh and re-materialises
    // — the depsolver now picks v0.2.0.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("update")
        .arg("org.vibevm/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Lockfile still records v0.2.0; its slot carries the v0.2.0
    // content (A modified, B removed, C added relative to v0.1.0).
    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    assert_eq!(lock.packages.len(), 1);
    let entry = &lock.packages[0];
    assert_eq!(entry.version.to_string(), "0.2.0");
    assert_eq!(entry.source_ref.as_deref(), Some("v0.2.0"));
    assert!(entry.content_hash.starts_with("sha256:"));
    // The footprint is the deterministic slot — `files_written` stays
    // empty under the loading model.
    assert!(entry.files_written.is_empty());

    let slot = project.path().join("vibedeps/flow-wal/0.2.0");
    assert_eq!(
        fs::read_to_string(slot.join("spec/flows/wal/A.md")).unwrap(),
        "v2 A — changed!\n"
    );
    assert!(
        !slot.join("spec/flows/wal/B.md").exists(),
        "B.md was removed in v0.2.0 — the verbatim slot does not carry it"
    );
    assert_eq!(
        fs::read_to_string(slot.join("spec/flows/wal/C.md")).unwrap(),
        "v2 C\n"
    );
}

// NOTE: `update_refuses_when_user_edited_file` was deleted with the
// PROP-009 switch-over. The old model copied a package's files into the
// project's own `spec/` tree, so `vibe update` had to detect a
// user-edited mirror file and refuse rather than clobber it. The
// loading model materialises a package into a vibe-owned `vibedeps/`
// slot that the operator never edits — `vibedeps::materialise` clears
// and rewrites the slot wholesale on every install/update. There is no
// user-edit to detect, so there is no refusal path to test.

#[test]
fn show_effective_emits_boot_files_with_provenance() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let assertion = vibe()
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    // 00-core and 90-user boot files. spec/WAL.md is NOT created by
    // default `vibe init` — it's a project convention, not part of
    // the package manager's contract — so `show effective` should
    // simply skip it when absent, not blow up.
    assert!(
        stdout.contains("spec://project/boot/00-core.md"),
        "expected 00-core spec URI; got:\n{stdout}"
    );
    assert!(
        stdout.contains("spec://project/boot/90-user.md"),
        "expected 90-user spec URI; got:\n{stdout}"
    );
    // Provenance marker for foundation (user-owned) files.
    assert!(
        stdout.contains("(user)"),
        "expected (user) provenance marker; got:\n{stdout}"
    );
}

#[test]
fn show_effective_includes_wal_when_present() {
    // When the operator (or `org.vibevm/wal` install) put `spec/WAL.md` in
    // place, `show effective` includes it with `(wal)` provenance.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::write(
        project.path().join("spec/WAL.md"),
        "# WAL\n\n## current phase\n\nTest checkpoint.\n",
    )
    .unwrap();

    let assertion = vibe()
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    assert!(
        stdout.contains("spec://project/WAL"),
        "expected WAL spec URI; got:\n{stdout}"
    );
    assert!(
        stdout.contains("(wal)"),
        "expected (wal) provenance marker; got:\n{stdout}"
    );
}

// NOTE: `show_effective_attributes_installed_package_files` was deleted
// with the PROP-009 switch-over. It asserted that `vibe show effective`
// attributes a package's `spec/flows/<...>` files and its `NN-` boot
// snippet — read from the lockfile's `files_written` / `boot_snippet`
// fields — to the package that contributed them. The loading model
// records neither: `files_written` is always empty and there is no
// `boot_snippet` filename (the footprint is the `vibedeps/` slot, and
// boot composition is driven by `spec/boot/INDEX.md`). There is no
// per-package file attribution for `show effective` to surface. The
// boot-file provenance basics are still covered by
// `show_effective_emits_boot_files_with_provenance` and
// `show_effective_includes_wal_when_present`.

#[test]
fn user_config_promotes_vibe_registry_cache_into_runtime() {
    // Smoke: a user-config file that defaults VIBE_REGISTRY_CACHE
    // must actually take effect at install time — not just surface
    // in `vibe show config`. Without the live env override, the
    // install's per-package clone must land in the user-config-
    // pointed cache.
    if !git_available() {
        eprintln!(
            "skipping user_config_promotes_vibe_registry_cache_into_runtime: git not on PATH"
        );
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    let cache = outer.path().join("user-config-cache");
    fs::create_dir_all(&cache).unwrap();
    fs::write(
        &user_cfg_path,
        format!(
            "[env]\nVIBE_REGISTRY_CACHE = \"{}\"\n",
            cache.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env_remove("VIBE_REGISTRY_CACHE")
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The user-config-pointed cache must be populated with the
    // per-package clone — proves the promotion ran end-to-end.
    let bucket_count = fs::read_dir(&cache).unwrap().count();
    assert!(
        bucket_count >= 1,
        "expected user-config-pointed cache to have at least one bucket, got {bucket_count}"
    );
    let bucket = fs::read_dir(&cache)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .expect("bucket")
        .path();
    let pkg_clone = bucket.join("packages/org.vibevm.world_wal/clone");
    assert!(
        pkg_clone.join(".git").exists(),
        "per-package clone must land in user-config cache: {}",
        pkg_clone.display()
    );
}

#[test]
fn show_config_user_layer_provides_default_for_unset_env() {
    // User-level config defaults VIBE_REGISTRY_CACHE; live env is
    // unset. Provenance must surface as `user-config`, value is the
    // user-config string.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    fs::write(
        &user_cfg_path,
        r#"[env]
VIBE_REGISTRY_CACHE = "/from-user-config"
VIBE_LOG = "vibe_registry=info"
"#,
    )
    .unwrap();

    let out = vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env_remove("VIBE_REGISTRY_CACHE")
        .env_remove("VIBE_LOG")
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    let env_arr = payload["env"].as_array().unwrap();
    let cache_entry = env_arr
        .iter()
        .find(|e| e["name"] == "VIBE_REGISTRY_CACHE")
        .unwrap();
    assert_eq!(cache_entry["provenance"], "user-config");
    assert_eq!(cache_entry["value"], "/from-user-config");
    let log_entry = env_arr.iter().find(|e| e["name"] == "VIBE_LOG").unwrap();
    assert_eq!(log_entry["provenance"], "user-config");
    assert_eq!(log_entry["value"], "vibe_registry=info");
    // The user_config block reports loaded = true and the resolved path.
    assert_eq!(payload["user_config"]["loaded"], true);
    assert!(payload["user_config"]["path"].is_string());
}

#[test]
fn show_config_live_env_overrides_user_config() {
    // Both layers set VIBE_REGISTRY_CACHE; the live env wins.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    fs::write(
        &user_cfg_path,
        "[env]\nVIBE_REGISTRY_CACHE = \"/from-user-config\"\n",
    )
    .unwrap();

    let out = vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env("VIBE_REGISTRY_CACHE", "/from-live-env")
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    let env_arr = payload["env"].as_array().unwrap();
    let cache_entry = env_arr
        .iter()
        .find(|e| e["name"] == "VIBE_REGISTRY_CACHE")
        .unwrap();
    assert_eq!(cache_entry["provenance"], "env");
    assert_eq!(cache_entry["value"], "/from-live-env");
}

#[test]
fn show_config_user_token_default_redacts_value() {
    // VIBEVM_PUBLISH_TOKEN is sensitive: even if defaulted via
    // user-config (which would be a poor operator choice — token
    // belongs in `~/.vibevm/<host>.publish.token` per PROP-000 §20
    // — but we can't refuse to load) the value must NEVER appear in
    // output.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    fs::write(
        &user_cfg_path,
        "[env]\nVIBEVM_PUBLISH_TOKEN = \"secret-do-not-leak\"\n",
    )
    .unwrap();

    let out = vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env_remove("VIBEVM_PUBLISH_TOKEN")
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("secret-do-not-leak"),
        "token bytes leaked into stdout:\n{stdout}"
    );
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let env_arr = payload["env"].as_array().unwrap();
    let token = env_arr
        .iter()
        .find(|e| e["name"] == "VIBEVM_PUBLISH_TOKEN")
        .unwrap();
    assert_eq!(token["provenance"], "redacted");
    assert!(token["value"].as_str().unwrap().contains("redacted"));
}

#[test]
fn show_config_emits_registry_block_with_provenance() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "show:config");
    let registries = payload["registries"].as_array().unwrap();
    assert!(
        !registries.is_empty(),
        "default `vibe init` configures a registry"
    );
    assert_eq!(registries[0]["provenance"], "vibe.toml");
    let env = payload["env"].as_array().unwrap();
    assert!(
        env.iter().any(|e| e["name"] == "VIBEVM_PUBLISH_TOKEN"),
        "VIBEVM_PUBLISH_TOKEN should appear in env block"
    );
    // Token entry must surface as either `default` (unset) or
    // `redacted` (set in env). Never the raw value.
    let token_entry = env
        .iter()
        .find(|e| e["name"] == "VIBEVM_PUBLISH_TOKEN")
        .unwrap();
    let prov = token_entry["provenance"].as_str().unwrap();
    assert!(
        prov == "default" || prov == "redacted",
        "VIBEVM_PUBLISH_TOKEN provenance must be default/redacted; got `{prov}`"
    );
}

#[test]
fn check_clean_project_exits_zero_with_no_findings() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let assertion = vibe()
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    assert!(
        stdout.contains("clean"),
        "expected clean summary; got:\n{stdout}"
    );
}

// NOTE: `check_boot_prefix_collision_exits_nonzero` was deleted with
// the PROP-009 switch-over. The old model placed every boot snippet
// directly in the project's `spec/boot/` under an author-chosen `NN-`
// numeric prefix, so `vibe check` linted for two files sharing a
// prefix. The loading model retires the `NN-` prefix: boot ordering is
// computed by the engine from each contribution's `BootCategory` band,
// and a dependency's boot lives inside its own `vibedeps/<slot>/` tree.
// There is no prefix and therefore no prefix collision to lint for.

#[test]
fn check_emits_json_envelope() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let out = vibe()
        .arg("--json")
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "check");
    let summary = &payload["summary"];
    assert_eq!(summary["error"], 0);
    assert!(payload["findings"].is_array());
}

/// When the manifest constraint excludes the newer release, `vibe
/// update` re-resolves within that constraint and re-materialises the
/// same version — it never jumps past the operator's declared pin.
/// PROP-009 retired the "up-to-date" / per-file-diff report; the
/// observable contract is the lockfile staying at the pinned version.
#[test]
fn update_keeps_pinned_version_when_constraint_excludes_newer() {
    if !git_available() {
        eprintln!(
            "skipping update_keeps_pinned_version_when_constraint_excludes_newer: git not on PATH"
        );
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_two_version_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    // Install v0.1.0 with constraint `^0.1` (admits `>=0.1.0, <0.2.0`).
    // v0.2.0 exists upstream but is excluded by the constraint.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/wal@^0.1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // `vibe update` re-resolves within `^0.1` and re-materialises —
    // succeeds, but the resolved version stays v0.1.0.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("update")
        .arg("org.vibevm/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Lockfile entry stays at v0.1.0 and its slot is the v0.1.0 tree.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].version.to_string(), "0.1.0");
    assert!(
        project
            .path()
            .join("vibedeps/flow-wal/0.1.0/vibe.toml")
            .is_file()
    );
    assert!(
        !project.path().join("vibedeps/flow-wal/0.2.0").exists(),
        "the excluded v0.2.0 must not be materialised"
    );
}

#[test]
fn vendor_produces_bare_repo_per_lockfile_entry() {
    // End-to-end: install from a per-package git registry, then run
    // `vibe registry vendor`. The vendor dir should contain a bare
    // git repo per lockfile entry, ready for use as `[[mirror]] url
    // = "file:///<abs>"`. Verifies the repo is consumable by checking
    // that `git clone` succeeds against it and that the v0.2.0 tag
    // is preserved.
    if !git_available() {
        eprintln!("skipping vendor_produces_bare_repo_per_lockfile_entry: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Run `vibe registry vendor` against the freshly installed project.
    let vendor_dir = outer.path().join("vendor-output");
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("registry")
        .arg("vendor")
        .arg("--out")
        .arg(&vendor_dir)
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    // The vendor dir contains a bare repo for the package and a
    // README.md explaining how to wire it.
    let bare_repo = vendor_dir.join("org.vibevm.world_wal.git");
    assert!(
        bare_repo.is_dir(),
        "expected vendor bare repo at {}",
        bare_repo.display()
    );
    assert!(
        bare_repo.join("HEAD").is_file(),
        "expected HEAD in vendor bare repo"
    );
    // Either loose ref or packed-refs is acceptable depending on git
    // version — tag presence is verified via `git ls-remote` below.
    assert!(
        vendor_dir.join("README.md").is_file(),
        "expected vendor/README.md"
    );

    // Verify the bare repo is a usable git source. `git ls-remote
    // <vendor>/org.vibevm.world_wal.git` must list the v0.2.0 tag the
    // install pulled in.
    let ls_out = std::process::Command::new("git")
        .args(["ls-remote", "--tags", bare_repo.to_str().unwrap()])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .expect("spawn git ls-remote");
    assert!(
        ls_out.status.success(),
        "git ls-remote against vendored repo failed: {}",
        String::from_utf8_lossy(&ls_out.stderr)
    );
    let tags = String::from_utf8_lossy(&ls_out.stdout);
    assert!(
        tags.contains("refs/tags/v0.2.0"),
        "vendored repo missing v0.2.0 tag — got:\n{tags}"
    );

    // Cloning from the vendored repo into a fresh worktree must
    // produce the original package content. This is the
    // operationally-relevant promise of `vibe registry vendor`:
    // the dir is a true git source.
    let worktree = outer.path().join("clone-from-vendor");
    let clone_out = std::process::Command::new("git")
        .args([
            "clone",
            "--branch",
            "v0.2.0",
            bare_repo.to_str().unwrap(),
            worktree.to_str().unwrap(),
        ])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .expect("spawn git clone");
    assert!(
        clone_out.status.success(),
        "git clone of vendored repo failed: {}",
        String::from_utf8_lossy(&clone_out.stderr)
    );
    assert!(
        worktree.join("vibe.toml").is_file(),
        "vendored repo's v0.2.0 tag did not produce expected payload"
    );
}

#[test]
fn vendor_refuses_non_empty_out_dir_without_force() {
    if !git_available() {
        eprintln!("skipping vendor_refuses_non_empty_out_dir_without_force: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Plant operator content in the would-be vendor dir.
    let vendor_dir = outer.path().join("user-content");
    fs::create_dir_all(&vendor_dir).unwrap();
    fs::write(vendor_dir.join("important.txt"), "do not delete\n").unwrap();

    // Without --force, vendor must refuse. Operator content survives.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("registry")
        .arg("vendor")
        .arg("--out")
        .arg(&vendor_dir)
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not empty"))
        .stderr(predicate::str::contains("--force"));
    assert!(
        vendor_dir.join("important.txt").exists(),
        "user content was wiped despite --force not being passed"
    );

    // With --force, vendor proceeds and the user content is gone.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("registry")
        .arg("vendor")
        .arg("--out")
        .arg(&vendor_dir)
        .arg("--force")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    assert!(
        !vendor_dir.join("important.txt").exists(),
        "important.txt should be gone after --force vendor"
    );
    assert!(
        vendor_dir.join("org.vibevm.world_wal.git").is_dir(),
        "vendored bare repo missing after --force"
    );
}

/// Build a self-contained local fixture registry that ships a flow
/// package with `[features]` + subskills + describes for feature-aware
/// install testing. Layout:
///
/// ```text
/// registry/org.vibevm/feat-pkg/v0.1.0/
/// ├── vibe.toml      [features].default = ["base"]
/// │                          [features].base = []
/// │                          [features].with-rust = ["subskill:stack/rust"]
/// │                          [package].describes = "pkg:cargo/sqlx@0.8.0"
/// ├── boot/10-feat-pkg.md
/// ├── spec/feats/feat-pkg/CORE.md
/// └── subskills/
///     ├── stack/rust/
///     │   ├── vibe-subskill.toml   activation.if_present = ["stack:rust"]
///     │   │                        delivery = "eager"
///     │   │                        content.files_written = ["spec/feats/feat-pkg/RUST.md"]
///     │   └── spec/feats/feat-pkg/RUST.md
///     └── doc/extra/
///         ├── vibe-subskill.toml   activation.if_files = ["**/Cargo.toml"]
///         │                        delivery = "eager"
///         │                        content.files_written = ["spec/feats/feat-pkg/EXTRA.md"]
///         └── spec/feats/feat-pkg/EXTRA.md
/// ```
fn make_features_fixture_registry(root: &Path) -> PathBuf {
    let registry = root.join("registry");
    let pkg = registry.join("org.vibevm").join("feat-pkg").join("v0.1.0");
    fs::create_dir_all(pkg.join("spec/feats/feat-pkg")).unwrap();
    fs::create_dir_all(pkg.join("boot")).unwrap();
    fs::create_dir_all(pkg.join("subskills/stack/rust/spec/feats/feat-pkg")).unwrap();
    fs::create_dir_all(pkg.join("subskills/doc/extra/spec/feats/feat-pkg")).unwrap();

    fs::write(
        pkg.join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "feat-pkg"
kind = "flow"
version = "0.1.0"
describes = "pkg:cargo/sqlx@0.8.0"

[boot_snippet]
source = "boot/10-feat-pkg.md"
category = "flow"

[features]
default = ["base"]
base = []
with-rust = ["subskill:stack/rust"]
"#,
    )
    .unwrap();
    fs::write(pkg.join("spec/feats/feat-pkg/CORE.md"), "# CORE protocol").unwrap();
    fs::write(pkg.join("boot/10-feat-pkg.md"), "# boot snippet").unwrap();

    fs::write(
        pkg.join("subskills/stack/rust/vibe-subskill.toml"),
        r#"[subskill]
path = "stack/rust"
delivery = "eager"

[activation]
if_present = ["stack:rust"]

[content]
files_written = ["spec/feats/feat-pkg/RUST.md"]
"#,
    )
    .unwrap();
    fs::write(
        pkg.join("subskills/stack/rust/spec/feats/feat-pkg/RUST.md"),
        "# Rust-specific guidance",
    )
    .unwrap();

    fs::write(
        pkg.join("subskills/doc/extra/vibe-subskill.toml"),
        r#"[subskill]
path = "doc/extra"
delivery = "eager"

[activation]
if_files = ["**/Cargo.toml"]

[content]
files_written = ["spec/feats/feat-pkg/EXTRA.md"]
"#,
    )
    .unwrap();
    fs::write(
        pkg.join("subskills/doc/extra/spec/feats/feat-pkg/EXTRA.md"),
        "# extra context",
    )
    .unwrap();

    registry
}

/// `vibe install --features` expands the requested feature set and
/// records the active features (plus the package's `describes` PURL) in
/// the lockfile entry. The whole package tree — including the content
/// behind every feature — is materialised verbatim into the
/// `vibedeps/` slot regardless of which features activated; the
/// loading model no longer copies a feature-selected subset into the
/// project's own `spec/` tree, and subskill activation at install time
/// is retired (`subskills_active` is always empty).
#[test]
fn install_with_features_records_active_features_in_lockfile() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("org.vibevm/feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("with-rust")
        .arg("--assume-yes")
        .assert()
        .success();

    // The package tree is materialised verbatim into its slot.
    let slot = project.path().join("vibedeps/flow-feat-pkg/0.1.0");
    assert!(
        slot.join("spec/feats/feat-pkg/CORE.md").is_file(),
        "the package tree is materialised verbatim into vibedeps/"
    );
    assert!(slot.join("vibe.toml").is_file());

    // Lockfile records the active features and the `describes` PURL.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    let entry = &lock.packages[0];
    assert!(
        entry.features.contains(&"with-rust".to_string()),
        "expected with-rust in features; got {:?}",
        entry.features
    );
    assert!(
        entry.features.contains(&"default".to_string())
            || entry.features.contains(&"base".to_string()),
        "expected default/base activation; got {:?}",
        entry.features
    );
    assert_eq!(entry.describes.as_deref(), Some("pkg:cargo/sqlx@0.8.0"));
    // Subskill activation at install time is retired — `subskills_active`
    // stays empty under the loading model.
    assert!(entry.subskills_active.is_empty());
    // The footprint is the deterministic `vibedeps/` slot, so
    // `files_written` is empty.
    assert!(entry.files_written.is_empty());
}

/// `--no-default-features` drops the package's `default` feature from
/// the resolved set — the lockfile entry records neither `default` nor
/// the features it would have pulled in.
#[test]
fn install_no_default_features_drops_default_feature_from_lockfile() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("org.vibevm/feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--no-default-features")
        .arg("--assume-yes")
        .assert()
        .success();

    // The package tree still materialises verbatim — `--no-default-
    // features` affects the recorded feature set, not what lands in the
    // `vibedeps/` slot.
    assert!(
        project
            .path()
            .join("vibedeps/flow-feat-pkg/0.1.0/spec/feats/feat-pkg/CORE.md")
            .is_file()
    );
    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    assert!(
        !lock.packages[0]
            .features
            .iter()
            .any(|f| f == "default" || f == "base"),
        "expected no default/base activation; got {:?}",
        lock.packages[0].features
    );
    assert!(lock.packages[0].subskills_active.is_empty());
}

// ============================================================
// M1.7 vibe-mcp e2e — driving `vibe mcp serve` via stdio
// ============================================================
//
// We spawn the CLI with `vibe mcp serve --path <project>`, write a
// JSON-RPC `initialize` + `tools/call query_package` script to its
// stdin, close stdin to signal EOF, and parse the line-delimited
// JSON responses. This is the same shape Claude Code / Cursor use
// when they speak to the server.

// NOTE: `mcp_materialise_subskill_promotes_lazy_pull_into_project` was
// deleted with the PROP-009 switch-over. It drove the MCP
// `materialise_subskill` tool to copy a `delivery = lazy-pull` subskill
// into the project tree on demand. The loading model retires
// install-time subskill activation: `vibe install` no longer records
// any `subskills_active` entry, so the MCP tool — which looks the
// subskill up by its lockfile entry — has nothing to act on after a
// real install. The `materialise_subskill` tool and its lazy-pull /
// overwrite / no-op branches remain unit-tested against a hand-written
// lockfile fixture in `vibe-mcp`'s own `tools.rs` tests.

#[test]
fn mcp_install_writes_claude_mcp_json() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .assert()
        .success();

    let config = project.path().join(".mcp.json");
    assert!(config.is_file(), "expected `.mcp.json` written");
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        json_entry_argv(&v["mcpServers"]["vibevm"]),
        expected_vibevm_argv()
    );
}

#[test]
fn mcp_install_is_idempotent() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .assert()
        .success();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON envelope");
    let claude_result = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude")
        .expect("claude result present");
    assert_eq!(claude_result["status"], "unchanged");
    assert_eq!(claude_result["scope"], "project");
}

#[test]
fn mcp_install_dry_run_does_not_write() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".cursor")).unwrap();

    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("cursor")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .arg("--dry-run")
        .assert()
        .success();

    assert!(
        !project.path().join(".cursor/mcp.json").exists(),
        "dry-run must not write any file"
    );
}

#[test]
fn mcp_install_force_writes_even_without_marker() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let _ = fs::remove_file(project.path().join("CLAUDE.md"));
    let _ = fs::remove_dir_all(project.path().join(".claude"));

    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .arg("--force")
        .assert()
        .success();

    assert!(
        project.path().join(".mcp.json").is_file(),
        "expected force-written `.mcp.json`"
    );
}

#[test]
fn mcp_install_what_both_writes_skill_md_for_claude() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("both")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["what"], "both");
    assert_eq!(v["scope"], "project");
    let skill_results = v["skill_results"].as_array().unwrap();
    assert_eq!(skill_results.len(), 1);
    assert_eq!(skill_results[0]["agent"], "claude");
    assert_eq!(skill_results[0]["scope"], "project");
    assert!(matches!(
        skill_results[0]["status"].as_str(),
        Some("created" | "unchanged")
    ));

    let skill = project.path().join(".claude/skills/vibevm/SKILL.md");
    assert!(skill.is_file(), "expected SKILL.md at {}", skill.display());
    let body = fs::read_to_string(&skill).unwrap();
    assert!(body.contains("name: vibevm"));
    assert!(body.contains("--invoked-by"));
    assert!(body.contains("query_package"));
}

#[test]
fn mcp_install_what_both_writes_skill_for_opencode() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".opencode")).unwrap();

    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("opencode")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("both")
        .assert()
        .success();

    // OpenCode JSON config in project root
    let config = project.path().join("opencode.json");
    assert!(config.is_file());
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(v["mcp"]["vibevm"]["type"], "local");
    assert_eq!(v["mcp"]["vibevm"]["enabled"], true);
    assert!(v["mcp"]["vibevm"]["command"].is_array());

    // Skill at .opencode/skills/vibevm/SKILL.md
    let skill = project.path().join(".opencode/skills/vibevm/SKILL.md");
    assert!(skill.is_file(), "expected SKILL.md at {}", skill.display());
}

#[test]
fn mcp_install_what_skill_for_cursor_reports_skipped() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".cursor")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("cursor")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("skill")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let skill_results = v["skill_results"].as_array().unwrap();
    assert_eq!(skill_results.len(), 1);
    assert_eq!(skill_results[0]["agent"], "cursor");
    assert_eq!(skill_results[0]["status"], "skipped");
    assert!(skill_results[0]["path"].is_null());
}

#[test]
fn mcp_install_what_mcp_emits_empty_skill_results() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["what"], "mcp");
    assert!(v["skill_results"].as_array().unwrap().is_empty());
    assert!(
        !project
            .path()
            .join(".claude/skills/vibevm/SKILL.md")
            .exists(),
        "what=mcp must not write SKILL.md"
    );
}

#[test]
fn mcp_install_auto_with_dry_run_previews_every_detected_agent() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();
    fs::create_dir_all(project.path().join(".cursor")).unwrap();
    fs::create_dir_all(project.path().join(".opencode")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--auto")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["mode"], "auto");
    assert_eq!(v["dry_run"], true);
    // --auto with vibe.toml present resolves scope=project; what=both.
    assert_eq!(v["scope"], "project");
    assert_eq!(v["what"], "both");
    let detected: Vec<&str> = v["detected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect();
    for a in &["claude", "cursor", "opencode"] {
        assert!(
            detected.contains(a),
            "expected `{a}` in detected={detected:?}"
        );
    }
    for r in v["results"].as_array().unwrap() {
        let s = r["status"].as_str().unwrap();
        assert!(
            s.starts_with("would-") || s == "unchanged" || s == "skipped",
            "unexpected dry-run status `{s}`"
        );
    }
    let skill_by_agent: std::collections::BTreeMap<&str, &str> = v["skill_results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["agent"].as_str().unwrap(), r["status"].as_str().unwrap()))
        .collect();
    assert_eq!(skill_by_agent.get("cursor"), Some(&"skipped"));
    assert!(matches!(
        skill_by_agent.get("claude"),
        Some(&"would-create") | Some(&"unchanged")
    ));
    assert!(matches!(
        skill_by_agent.get("opencode"),
        Some(&"would-create") | Some(&"unchanged")
    ));
}

#[test]
fn mcp_install_scope_user_works_without_vibe_toml() {
    // Bootstrap mode: directory has no vibe.toml, scope=user → no
    // resolve_project_root_required gate, install proceeds against
    // user-level paths. Use --dry-run so we don't touch real ~/.claude.
    let project = tempfile::tempdir().unwrap();
    // Deliberately no init_project — no vibe.toml.

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("user")
        .arg("--what")
        .arg("mcp")
        .arg("--dry-run")
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["scope"], "user");
    assert!(v["project"].is_null() || v["project"].as_str().is_some());
    let results = v["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["agent"], "claude");
    assert_eq!(results[0]["scope"], "user");
}

#[test]
fn mcp_install_user_scope_mcp_entry_omits_path_arg() {
    // dry-run a user-scope install and confirm the would-create
    // output's wire shape lacks `--path` in args. We can't read the
    // resulting file (would touch ~/.claude/), but the JSON envelope
    // doesn't expose the args directly — instead, rely on the unit
    // test `user_scope_mcp_entry_omits_path_arg` for that contract
    // and just assert here that the install succeeds.
    let project = tempfile::tempdir().unwrap();
    let out = vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("user")
        .arg("--what")
        .arg("mcp")
        .arg("--dry-run")
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn mcp_install_scope_project_without_vibe_toml_errors() {
    // Inverse of bootstrap-mode test: if scope=project but vibe.toml
    // is absent, the gate must fire with a helpful message.
    let project = tempfile::tempdir().unwrap();
    let out = vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure when vibe.toml missing under scope=project"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("vibe.toml") && stderr.contains("--scope user"),
        "expected hint about --scope user; got: {stderr}"
    );
}

/// `--unattended` is the human-readable shape for "I am in a
/// script, no TTY, no prompts." Provisioning recipe straight out
/// of the docs: opencode, scope both, what both, no `--yes`, no
/// `--invoked-by` placeholder. Must succeed without a vibe.toml
/// (project leg silently skipped) and stamp `unattended: true` on
/// the JSON envelope.
#[test]
fn mcp_install_unattended_replaces_yes_for_provisioning_recipe() {
    let project = tempfile::tempdir().unwrap();
    // No init_project — fresh user, no vibevm project on the machine.

    let out = vibe()
        .arg("--json")
        .arg("--unattended")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("opencode")
        .arg("--scope")
        .arg("both")
        .arg("--what")
        .arg("both")
        .arg("--dry-run")
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected --unattended to drive a successful provisioning install; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["unattended"], true,
        "expected unattended:true stamped on envelope: {v}"
    );
    assert_eq!(v["scope"], "both");
    assert!(v["project"].is_null());
    let scopes: std::collections::HashSet<&str> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["scope"].as_str().unwrap())
        .collect();
    assert!(scopes.contains("user") && !scopes.contains("project"));
}

/// `--unattended` without enough flags must fail loudly with a hint
/// pointing at the missing dimensions, instead of opening a wizard
/// that would deadlock a script.
#[test]
fn mcp_install_unattended_bails_when_wizard_dimensions_missing() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let out = vibe()
        .arg("--unattended")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("user")
        // Deliberately no --agent, no --what.
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unattended") && stderr.contains("--agent") && stderr.contains("--what"),
        "expected the bail to name the missing flags; got: {stderr}"
    );
}

/// `vibe install <pkg> --unattended` skips the apply confirm prompt,
/// same as `--assume-yes`. Lets a single global flag drive both the
/// package CLI surface and the MCP CLI surface.
#[test]
fn install_unattended_skips_confirm_like_assume_yes() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("--unattended")
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(make_wal_dir_registry(project.path()))
        .assert()
        .success();

    // The install ran to completion without a confirm prompt — the
    // package tree is materialised into its `vibedeps/` slot.
    assert!(
        project
            .path()
            .join("vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md")
            .is_file()
    );
}

#[test]
fn mcp_install_scope_both_without_vibe_toml_does_user_leg_only() {
    // First-time-user provisioning scenario: a setup script runs on
    // a fresh machine before any vibevm project exists, and asks
    // for `--scope both --agent opencode` so the user-level config
    // lands and the project-leg is silently skipped. Symmetric with
    // `vibe mcp upgrade` / `vibe mcp uninstall`. Uses --dry-run so
    // the test does not touch real user configs.
    let project = tempfile::tempdir().unwrap();
    // Deliberately no init_project — no vibe.toml.

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("opencode")
        .arg("--scope")
        .arg("both")
        .arg("--what")
        .arg("both")
        .arg("--yes")
        .arg("--dry-run")
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "expected --scope both to succeed without vibe.toml (best-effort project leg); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["scope"], "both");
    // No project resolved → envelope reports project as null.
    assert!(
        v["project"].is_null(),
        "project should be null when vibe.toml absent: {v}"
    );
    // Walker emitted only user-leg rows; project leg silently skipped.
    let results = v["results"].as_array().unwrap();
    let scopes: std::collections::HashSet<&str> = results
        .iter()
        .map(|r| r["scope"].as_str().unwrap())
        .collect();
    assert!(
        scopes.contains("user"),
        "expected at least one user-scope row in results: {results:#?}"
    );
    assert!(
        !scopes.contains("project"),
        "project-scope rows must NOT appear when vibe.toml is missing: {results:#?}"
    );
    // SKILL.md leg also walks user-only.
    let skills = v["skill_results"].as_array().unwrap();
    let skill_scopes: std::collections::HashSet<&str> = skills
        .iter()
        .map(|r| r["scope"].as_str().unwrap())
        .collect();
    assert!(
        !skill_scopes.contains("project"),
        "skill project-scope rows must NOT appear when vibe.toml is missing: {skills:#?}"
    );
}

#[test]
fn mcp_install_scope_both_writes_to_project_and_user_for_claude() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("both")
        .arg("--what")
        .arg("mcp")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["scope"], "both");
    let results = v["results"].as_array().unwrap();
    let scopes: Vec<&str> = results
        .iter()
        .map(|r| r["scope"].as_str().unwrap())
        .collect();
    assert!(scopes.contains(&"project"));
    assert!(scopes.contains(&"user"));
}

#[test]
fn mcp_install_scope_both_collapses_to_user_for_user_only_agent() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("codex")
        .arg("--scope")
        .arg("both")
        .arg("--what")
        .arg("mcp")
        .arg("--force")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let results = v["results"].as_array().unwrap();
    // Both expands to two entries — but the project one is `skipped`
    // (Codex has no project surface) and the user one is `would-create`.
    let by_scope: std::collections::BTreeMap<&str, &str> = results
        .iter()
        .map(|r| (r["scope"].as_str().unwrap(), r["status"].as_str().unwrap()))
        .collect();
    assert_eq!(by_scope.get("project"), Some(&"skipped"));
    assert!(matches!(
        by_scope.get("user"),
        Some(&"would-create") | Some(&"would-update") | Some(&"unchanged")
    ));
}

#[test]
fn mcp_install_auto_conflicts_with_explicit_agent() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let out = vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--auto")
        .arg("--agent")
        .arg("claude")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected --auto + --agent to clap-fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("conflict") || stderr.contains("cannot be used"),
        "expected clap conflict message; got: {stderr}"
    );
}

#[test]
fn mcp_install_no_args_in_non_tty_emits_error() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure in non-TTY interactive path"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--scope") || stderr.contains("--auto"),
        "expected hint pointing at --scope/--auto; got: {stderr}"
    );
}

#[test]
fn mcp_install_with_invoked_by_stamps_envelope() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--invoked-by")
        .arg("opencode")
        .arg("--json")
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("mcp")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["invoked_by"], "opencode");
}

#[test]
fn mcp_upgrade_reports_not_installed_when_block_absent() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("upgrade")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("project")
        .arg("--agent")
        .arg("claude")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["command"], "mcp:upgrade");
    let claude = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .expect("claude project entry present");
    assert_eq!(claude["status"], "not-installed");
}

#[test]
fn mcp_upgrade_reports_unchanged_after_fresh_install() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    // Install fresh.
    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("both")
        .assert()
        .success();

    // Upgrade should be a no-op (everything matches the shipped template).
    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("upgrade")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("project")
        .arg("--agent")
        .arg("claude")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let claude_mcp = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .unwrap();
    assert_eq!(claude_mcp["status"], "unchanged");
    let claude_skill = v["skill_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .unwrap();
    assert_eq!(claude_skill["status"], "unchanged");
}

#[test]
fn mcp_upgrade_detects_drift_and_rewrites_to_current() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    // Plant a stale vibevm block + stale SKILL.md by hand.
    let config_path = project.path().join(".mcp.json");
    fs::write(
        &config_path,
        r#"{ "mcpServers": { "vibevm": { "command": "old-binary", "args": [] } } }"#,
    )
    .unwrap();
    let skill_path = project.path().join(".claude/skills/vibevm/SKILL.md");
    fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
    fs::write(&skill_path, "stale content").unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("upgrade")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("project")
        .arg("--agent")
        .arg("claude")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    let claude_mcp = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .unwrap();
    assert_eq!(claude_mcp["status"], "updated");
    let claude_skill = v["skill_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .unwrap();
    assert_eq!(claude_skill["status"], "updated");

    // Verify on-disk state was actually refreshed.
    let written = fs::read_to_string(&config_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(
        json_entry_argv(&parsed["mcpServers"]["vibevm"]),
        expected_vibevm_argv()
    );
    let new_skill = fs::read_to_string(&skill_path).unwrap();
    assert!(new_skill.contains("name: vibevm"));
}

#[test]
fn mcp_upgrade_dry_run_does_not_write() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let config_path = project.path().join(".mcp.json");
    let original = r#"{ "mcpServers": { "vibevm": { "command": "old", "args": [] } } }"#;
    fs::write(&config_path, original).unwrap();

    let out = vibe()
        .arg("mcp")
        .arg("upgrade")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("project")
        .arg("--agent")
        .arg("claude")
        .arg("--config-only")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(out.status.success());
    // File untouched.
    assert_eq!(fs::read_to_string(&config_path).unwrap(), original);
}

#[test]
fn mcp_upgrade_skill_only_skips_mcp_results() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("upgrade")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("project")
        .arg("--agent")
        .arg("claude")
        .arg("--skill-only")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["what"], "skill");
    assert!(
        v["results"].as_array().unwrap().is_empty(),
        "skill-only must skip mcp"
    );
}

#[test]
fn mcp_upgrade_scope_project_without_vibe_toml_errors() {
    let project = tempfile::tempdir().unwrap();
    let out = vibe()
        .arg("mcp")
        .arg("upgrade")
        .arg("--path")
        .arg(project.path())
        .arg("--scope")
        .arg("project")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--scope user"),
        "expected hint; got {stderr}"
    );
}

#[test]
fn mcp_uninstall_removes_vibevm_block_from_claude() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    // Install first.
    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("both")
        .assert()
        .success();
    let config = project.path().join(".mcp.json");
    let skill = project.path().join(".claude/skills/vibevm/SKILL.md");
    assert!(config.is_file());
    assert!(skill.is_file());

    // Uninstall (default --what is "both" — no flag needed).
    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("uninstall")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["command"], "mcp:uninstall");

    let claude_mcp = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude")
        .unwrap();
    assert_eq!(claude_mcp["status"], "removed");
    let claude_skill = v["skill_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude")
        .unwrap();
    assert_eq!(claude_skill["status"], "removed");

    // Verify on-disk state: vibevm-block gone, .mcp.json still exists.
    assert!(
        config.is_file(),
        ".mcp.json should remain (other keys preserved)"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert!(
        parsed["mcpServers"].get("vibevm").is_none(),
        "expected vibevm key removed; got {parsed}"
    );
    // Skill file gone, parent dir cleaned up.
    assert!(!skill.exists(), "SKILL.md should be deleted");
    assert!(
        !skill.parent().unwrap().exists(),
        "parent vibevm/ skill dir should be cleaned"
    );
}

#[test]
fn mcp_uninstall_preserves_foreign_keys() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    // Plant a config with both vibevm and another server.
    let config = project.path().join(".mcp.json");
    fs::write(
        &config,
        r#"{
          "preexisting": "keep-me",
          "mcpServers": {
            "vibevm": { "command": "vibe", "args": ["mcp", "serve"] },
            "other": { "command": "other-bin", "args": [] }
          }
        }"#,
    )
    .unwrap();

    vibe()
        .arg("mcp")
        .arg("uninstall")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--config-only")
        .assert()
        .success();

    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(parsed["preexisting"], "keep-me");
    assert!(parsed["mcpServers"].get("vibevm").is_none());
    assert_eq!(parsed["mcpServers"]["other"]["command"], "other-bin");
}

#[test]
fn mcp_uninstall_dry_run_does_not_delete() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();
    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("both")
        .assert()
        .success();
    let config = project.path().join(".mcp.json");
    let skill = project.path().join(".claude/skills/vibevm/SKILL.md");
    let pre_config = fs::read_to_string(&config).unwrap();
    let pre_skill = fs::read_to_string(&skill).unwrap();

    vibe()
        .arg("mcp")
        .arg("uninstall")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--dry-run")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&config).unwrap(), pre_config);
    assert_eq!(fs::read_to_string(&skill).unwrap(), pre_skill);
    assert!(skill.exists());
}

#[test]
fn mcp_uninstall_reports_not_installed_when_block_absent() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("uninstall")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let claude_mcp = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude")
        .unwrap();
    assert_eq!(claude_mcp["status"], "not-installed");
}

#[test]
fn mcp_uninstall_skill_only_keeps_mcp_block() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();
    vibe()
        .arg("mcp")
        .arg("install")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--what")
        .arg("both")
        .assert()
        .success();

    vibe()
        .arg("mcp")
        .arg("uninstall")
        .arg("--path")
        .arg(project.path())
        .arg("--agent")
        .arg("claude")
        .arg("--scope")
        .arg("project")
        .arg("--skill-only")
        .assert()
        .success();

    let config = project.path().join(".mcp.json");
    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    // mcp block kept
    assert_eq!(
        json_entry_argv(&parsed["mcpServers"]["vibevm"]),
        expected_vibevm_argv()
    );
    // skill file removed
    assert!(
        !project
            .path()
            .join(".claude/skills/vibevm/SKILL.md")
            .exists()
    );
}

#[test]
fn mcp_status_reports_per_agent_state() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("status")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let detected: Vec<&str> = v["detected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect();
    assert!(detected.contains(&"claude"));
    let claude_status = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .unwrap();
    let s = claude_status["status"].as_str().unwrap();
    assert!(
        s == "would-create" || s == "would-update" || s == "unchanged",
        "unexpected status `{s}`"
    );
    // skill_results must be present — skill drift is reported by
    // status now, not just install/upgrade.
    assert!(
        v["skill_results"].is_array(),
        "expected skill_results in envelope"
    );
}

#[test]
fn mcp_status_reports_skill_drift() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::create_dir_all(project.path().join(".claude")).unwrap();

    // Plant a stale skill file by hand.
    let skill_path = project.path().join(".claude/skills/vibevm/SKILL.md");
    fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
    fs::write(&skill_path, "stale content").unwrap();

    let out = vibe()
        .arg("--json")
        .arg("mcp")
        .arg("status")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let claude_skill = v["skill_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .expect("claude project skill entry present");
    // Stale on-disk content → would-update.
    assert_eq!(claude_skill["status"], "would-update");
}

#[test]
fn mcp_serve_responds_to_initialize_and_query_package() {
    let registry = workspace_root().join("fixtures").join("registry");
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Install the omnibus alpha package so query_package has
    // something interesting to return.
    vibe()
        .arg("install")
        .arg("org.vibevm/integration-alpha")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("extra-discipline")
        .arg("--assume-yes")
        .assert()
        .success();

    // Drive the MCP server. We send three line-delimited JSON-RPC
    // messages: initialize, tools/list, tools/call query_package.
    let script = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"query_package","arguments":{"name":"org.vibevm/integration-alpha"}}}"#,
        "",
    ]
    .join("\n");

    // `assert_cmd::Command` lacks the `Stdio` knobs we need for stdin
    // piping; reconstruct the underlying `std::process::Command` to
    // get full control.
    let bin = env!("CARGO_BIN_EXE_vibe");
    let mut cmd = std::process::Command::new(bin);
    cmd.arg("mcp")
        .arg("serve")
        .arg("--path")
        .arg(project.path());
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn vibe mcp serve");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    drop(child.stdin.take()); // signal EOF
    let output = child.wait_with_output().expect("wait child");
    assert!(
        output.status.success(),
        "mcp serve exit non-zero — stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 response lines; got:\n{stdout}");

    let init: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init["result"]["serverInfo"]["name"], "vibe-mcp");

    let list: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let tool_names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(tool_names.contains(&"query_package"));
    assert!(tool_names.contains(&"read_subskill"));

    let call: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(call["result"]["isError"], false);
    let pkg = &call["result"]["structuredContent"];
    assert_eq!(pkg["kind"], "flow");
    assert_eq!(pkg["name"], "integration-alpha");
    // `query_package` returns the lockfile entry. Under the loading
    // model that entry still carries the package's `describes` PURL and
    // its active feature set; `subskills_active` is empty, since
    // install-time subskill activation is retired.
    assert_eq!(pkg["describes"], "pkg:cargo/sqlx@0.8.0");
    let features: Vec<&str> = pkg["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(
        features.contains(&"extra-discipline"),
        "query_package must surface the active feature set; got {features:?}"
    );
    assert!(
        pkg["subskills_active"].as_array().unwrap().is_empty(),
        "subskills_active is empty under the loading model"
    );
}

// ============================================================
// PROP-003 r2 omnibus integration e2e
// ============================================================
//
// `fixtures/registry/{flow/integration-alpha, flow/integration-beta,
// stack/integration-rust}` are committed in-tree under the same
// LocalRegistry layout the M0/M1 fixtures already use. They exercise
// every PROP-003 r2 surface in combination:
//
// - alpha declares `[package].describes = "pkg:cargo/sqlx@0.8.0"`,
//   `[i18n] available = ["en", "ru"]`, `[features]` with default +
//   `extra-discipline` feature mapping `subskill:feature/extra-discipline`,
//   `[features.exclusive]` group, and a conditional dep
//   `[target."context(stack:integration-rust)".dependencies]` →
//   `org.vibevm/integration-beta`.
// - alpha ships subskills probing every channel: `feature/extra-
//   discipline` (manual feature), `stack/rust` (if_present), `lang/ru-
//   extras` (if_language), `sqlx/v08` (if_describes_match + own PURL).
// - beta provides `interface:trace-discipline` and ships an
//   `if-cargo` subskill activated by `if_files = ["**/Cargo.toml"]`.
// - org.vibevm/integration-rust is the trigger that pulls beta in via
//   alpha's conditional dep + activates alpha's `stack/rust` subskill.

#[test]
fn omnibus_install_exercises_every_prop003_surface() {
    let registry = workspace_root().join("fixtures").join("registry");
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("org.vibevm/integration-rust")
        .arg("org.vibevm/integration-alpha")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("extra-discipline")
        .arg("--language")
        .arg("ru")
        .arg("--assume-yes")
        .assert()
        .success();

    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    assert_eq!(lock.meta.schema_version, 5);
    assert_eq!(lock.meta.language_chain, vec!["ru", "en"]);
    // Three packages: the two CLI roots plus `org.vibevm/integration-beta`,
    // pulled in by alpha's conditional dep
    // `[target."context(stack:integration-rust)".dependencies]`. The
    // conditional-dep fixed-point loop still runs under the loading
    // model — only the materialisation half changed.
    assert_eq!(lock.packages.len(), 3);
    let alpha = lock
        .packages
        .iter()
        .find(|p| p.name == "integration-alpha")
        .expect("alpha must be in lockfile");
    let beta = lock
        .packages
        .iter()
        .find(|p| p.name == "integration-beta")
        .expect("beta must be pulled by alpha's conditional dep");
    let stack = lock
        .packages
        .iter()
        .find(|p| p.name == "integration-rust")
        .expect("stack must be in lockfile");

    // The lockfile still records each package's `describes` PURL, its
    // resolved language, and the active feature set — these survive the
    // PROP-009 switch-over.
    assert_eq!(alpha.describes.as_deref(), Some("pkg:cargo/sqlx@0.8.0"));
    assert_eq!(alpha.language.as_deref(), Some("ru"));
    assert!(alpha.features.contains(&"extra-discipline".to_string()));
    assert!(alpha.features.contains(&"default".to_string()));

    // Subskill activation at install time is retired — `subskills_active`
    // is empty for every package, and `files_written` (the old mirror
    // footprint) is empty too.
    for p in [alpha, beta, stack] {
        assert!(
            p.subskills_active.is_empty(),
            "`{}` subskills_active must be empty under the loading model",
            p.name
        );
        assert!(
            p.files_written.is_empty(),
            "`{}` files_written must be empty under the loading model",
            p.name
        );
        assert!(
            p.boot_snippet.is_none(),
            "`{}` boot_snippet filename is retired",
            p.name
        );
    }

    // Each package's whole published tree is materialised verbatim into
    // its own `vibedeps/` slot — base content, subskill directories,
    // and i18n sidecars all ride along as plain files.
    let alpha_slot = project.path().join("vibedeps/flow-integration-alpha/0.1.0");
    assert!(alpha_slot.join("vibe.toml").is_file());
    assert!(
        alpha_slot
            .join("spec/flows/integration-alpha/PROTOCOL.md")
            .is_file(),
        "alpha's base content lands in its slot"
    );
    assert!(
        alpha_slot
            .join("spec/flows/integration-alpha/PROTOCOL.ru.md")
            .is_file(),
        "the Russian i18n sidecar rides along verbatim in the slot"
    );
    assert!(
        alpha_slot
            .join("subskills/feature/extra-discipline/spec/flows/integration-alpha/EXTRA-DISCIPLINE.md")
            .is_file(),
        "subskill directories ride along inside the slot as plain content"
    );
    assert!(
        alpha_slot
            .join("subskills/sqlx/v08/spec/flows/integration-alpha/SQLX-V08.md")
            .is_file(),
        "every subskill — lazy-pull included — is part of the verbatim slot"
    );
    assert!(
        project
            .path()
            .join("vibedeps/flow-integration-beta/0.1.0/vibe.toml")
            .is_file(),
        "the conditionally-pulled beta is materialised into its own slot"
    );
    assert!(
        project
            .path()
            .join("vibedeps/stack-integration-rust/0.1.0/vibe.toml")
            .is_file()
    );

    // The OLD mirror layout — package files copied into the project's
    // own `spec/` tree, boot snippet placed at `spec/boot/NN-*.md` — is
    // retired.
    assert!(
        !project
            .path()
            .join("spec/flows/integration-alpha/PROTOCOL.md")
            .exists(),
        "the legacy [writes] mirror layout is retired"
    );
    assert!(
        !project
            .path()
            .join("spec/boot/40-flow-integration-alpha.md")
            .exists(),
        "the NN- boot-snippet placement is retired"
    );

    // The generated boot manifest exists for the project node.
    assert!(project.path().join("spec/boot/INDEX.md").is_file());
}

// NOTE: `omnibus_install_with_cargo_toml_activates_if_files_subskill`
// was deleted with the PROP-009 switch-over. It asserted that beta's
// `if_files`-activated `if-cargo` subskill materialises CARGO-NOTES.md
// into the project tree and records itself in `subskills_active`. The
// loading model retires install-time subskill activation entirely — a
// subskill directory rides along inside the package's verbatim
// `vibedeps/<slot>/` tree, and `subskills_active` is always empty.

// NOTE: `omnibus_install_without_stack_does_not_pull_conditional_dep`
// was deleted with the PROP-009 switch-over. Its assertions split into
// two: the genuine kept behaviour — a conditional dependency stays
// dormant when its predicate misses — is already fully covered by
// `conditional_dependencies_dormant_when_predicate_misses`; the rest
// asserted `subskills_active` membership (`stack/rust` absent,
// `sqlx/v08` present via `if_describes_match`), which is retired since
// the install pipeline no longer runs per-file subskill activation.

// NOTE: `omnibus_uninstall_removes_subskill_files_too` was deleted with
// the PROP-009 switch-over. It asserted that `vibe uninstall` removes
// the subskill-sourced files a package had mirrored into the project's
// `spec/` tree. The loading model materialises a package — subskills
// included — only into its `vibedeps/<slot>/`, and `vibe uninstall`
// removes that whole slot; there are no mirrored project-tree files to
// account for. The uninstall-removes-the-slot contract is covered by
// `full_install_cycle`.

/// `vibe show features` and `vibe show purls` read their data from the
/// lockfile, which under the loading model still records each package's
/// active feature set and its package-level `describes` PURL. (The
/// `show subskills` surface and subskill-level PURL bindings are no
/// longer populated by `vibe install` — install-time subskill
/// activation is retired — so this test no longer exercises them.)
#[test]
fn omnibus_show_features_and_purls_after_install() {
    let registry = workspace_root().join("fixtures").join("registry");
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("org.vibevm/integration-rust")
        .arg("org.vibevm/integration-alpha")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("extra-discipline")
        .arg("--language")
        .arg("ru")
        .arg("--assume-yes")
        .assert()
        .success();

    // `show features` — alpha's active feature set surfaces.
    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("features")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(v["command"], "show:features");
    let pkgs: Vec<&str> = v["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["package"].as_str().unwrap())
        .collect();
    assert!(pkgs.contains(&"org.vibevm/integration-alpha"));
    let alpha_features: Vec<&str> = v["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["package"] == "org.vibevm/integration-alpha")
        .unwrap()["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(alpha_features.contains(&"extra-discipline"));

    // `show purls` — alpha's package-level `describes` PURL surfaces.
    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("purls")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(v["command"], "show:purls");
    let bindings = v["bindings"].as_array().unwrap();
    assert!(
        bindings.iter().any(|b| b["purl"] == "pkg:cargo/sqlx@0.8.0"
            && b["package"] == "org.vibevm/integration-alpha"),
        "expected alpha→cargo/sqlx@0.8.0 binding; got {:?}",
        bindings
    );
}

#[test]
fn omnibus_install_no_default_features_drops_default_subskill() {
    let registry = workspace_root().join("fixtures").join("registry");
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("org.vibevm/integration-alpha")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--no-default-features")
        .arg("--features")
        .arg("extra-discipline")
        .arg("--assume-yes")
        .assert()
        .success();

    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    let alpha = lock
        .packages
        .iter()
        .find(|p| p.name == "integration-alpha")
        .unwrap();
    assert!(
        !alpha.features.contains(&"default".to_string()),
        "expected `default` excluded with --no-default-features; got {:?}",
        alpha.features
    );
    assert!(
        !alpha.features.contains(&"base-discipline".to_string()),
        "expected `base-discipline` excluded too; got {:?}",
        alpha.features
    );
    assert!(alpha.features.contains(&"extra-discipline".to_string()));
}

fn workspace_root() -> std::path::PathBuf {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Build a per-package git registry hosting three flow packages:
///
/// - `org.vibevm/dispatcher` v0.1.0 with a `[target."context(stack:rust)"]`
///   conditional dep on `org.vibevm/rust-helper@^0.1`.
/// - `org.vibevm/rust-helper` v0.1.0 (the conditional target).
/// - `org.vibevm/rust-cli` v0.1.0 (a stack package that the project will
///   install to make the predicate match).
///
/// Returns the org root path and a `git+file://...` URL pointing at it.
fn make_conditional_deps_registry(root: &Path) -> (PathBuf, String) {
    let org = root.join("org");
    fs::create_dir_all(&org).unwrap();

    fn make_pkg(
        out_root: &Path,
        org_dir: &Path,
        kind: &str,
        name: &str,
        version: &str,
        manifest_extras: &str,
        files: &[(&str, &str)],
    ) {
        let src = out_root.join(format!("src-{}-{}", kind, name));
        fs::create_dir_all(&src).unwrap();
        run_git(&src, &["init", "--initial-branch=main"]);
        run_git(&src, &["config", "user.email", "t@example.com"]);
        run_git(&src, &["config", "user.name", "Test"]);
        let manifest = format!(
            r#"[package]
group = "org.vibevm"
name = "{name}"
kind = "{kind}"
version = "{version}"
{manifest_extras}
"#
        );
        fs::write(src.join("vibe.toml"), manifest).unwrap();
        for (path, content) in files {
            let target = src.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(target, content).unwrap();
        }
        run_git(&src, &["add", "-A"]);
        run_git(&src, &["commit", "-m", &format!("v{version}")]);
        run_git(&src, &["tag", &format!("v{version}")]);

        let bare = org_dir.join(format!("org.vibevm_{name}.git"));
        run_git(
            out_root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    }

    make_pkg(
        root,
        &org,
        "flow",
        "dispatcher",
        "0.1.0",
        r#"
[target."context(stack:rust-cli)".dependencies]
packages = { "org.vibevm/rust-helper" = "^0.1" }
"#,
        &[("spec/flows/dispatcher/CORE.md", "# dispatcher core")],
    );
    make_pkg(
        root,
        &org,
        "flow",
        "rust-helper",
        "0.1.0",
        "",
        &[("spec/flows/rust-helper/HINT.md", "# rust hint")],
    );
    make_pkg(
        root,
        &org,
        "stack",
        "rust-cli",
        "0.1.0",
        "",
        &[("spec/stacks/rust-cli/STACK.md", "# rust-cli stack")],
    );

    let abs_org = org
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    let url = format!("git+file:///{abs_org}");
    (org, url)
}

#[test]
fn install_expands_conditional_dependencies_when_predicate_matches() {
    if !git_available() {
        eprintln!("skipping install_expands_conditional_dependencies: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_conditional_deps_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Install org.vibevm/rust-cli first to make `org.vibevm/rust` present in the
    // graph before dispatcher's conditional predicate evaluates.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/rust-cli")
        .arg("org.vibevm/dispatcher")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The conditional dependency `org.vibevm/rust-helper` should have been
    // pulled in as well.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    let names: Vec<_> = lock
        .packages
        .iter()
        .map(|p| format!("{}/{}", p.group, p.name))
        .collect();
    assert!(
        names.iter().any(|n| n == "org.vibevm/rust-cli"),
        "expected org.vibevm/rust-cli; got {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "org.vibevm/dispatcher"),
        "expected org.vibevm/dispatcher; got {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "org.vibevm/rust-helper"),
        "expected org.vibevm/rust-helper to be pulled in via conditional dep; got {:?}",
        names
    );
}

/// Build a registry that exercises **cascading** conditional deps:
///
/// - `org.vibevm/cascade-root` depends conditionally on `org.vibevm/cascade-mid`
///   when `org.vibevm/rust-cli` is present.
/// - `org.vibevm/cascade-mid` depends conditionally on `org.vibevm/cascade-leaf`
///   when `org.vibevm/cascade-root` is present.
/// - `org.vibevm/cascade-leaf` has no further conditional deps.
/// - `org.vibevm/rust-cli` is the trigger package.
///
/// Project installs `org.vibevm/rust-cli` + `org.vibevm/cascade-root`. Iteration 1
/// of the fixed-point loop pulls in `org.vibevm/cascade-mid`; iteration 2
/// pulls in `org.vibevm/cascade-leaf`; iteration 3 finds no new extras and
/// breaks. Verifies the runtime supports cascading predicates the
/// single-pass shape would have missed.
fn make_cascading_conditional_registry(root: &Path) -> (PathBuf, String) {
    let org = root.join("org");
    fs::create_dir_all(&org).unwrap();

    fn make_pkg(
        out_root: &Path,
        org_dir: &Path,
        kind: &str,
        name: &str,
        version: &str,
        manifest_extras: &str,
        files: &[(&str, &str)],
    ) {
        let src = out_root.join(format!("src-{}-{}", kind, name));
        fs::create_dir_all(&src).unwrap();
        run_git(&src, &["init", "--initial-branch=main"]);
        run_git(&src, &["config", "user.email", "t@example.com"]);
        run_git(&src, &["config", "user.name", "Test"]);
        let manifest = format!(
            r#"[package]
group = "org.vibevm"
name = "{name}"
kind = "{kind}"
version = "{version}"
{manifest_extras}
"#
        );
        fs::write(src.join("vibe.toml"), manifest).unwrap();
        for (path, content) in files {
            let target = src.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(target, content).unwrap();
        }
        run_git(&src, &["add", "-A"]);
        run_git(&src, &["commit", "-m", &format!("v{version}")]);
        run_git(&src, &["tag", &format!("v{version}")]);

        let bare = org_dir.join(format!("org.vibevm_{name}.git"));
        run_git(
            out_root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    }

    make_pkg(
        root,
        &org,
        "stack",
        "rust-cli",
        "0.1.0",
        "",
        &[("spec/stacks/rust-cli/STACK.md", "# rust-cli")],
    );
    make_pkg(
        root,
        &org,
        "flow",
        "cascade-root",
        "0.1.0",
        r#"
[target."context(stack:rust-cli)".dependencies]
packages = { "org.vibevm/cascade-mid" = "^0.1" }
"#,
        &[("spec/flows/cascade-root/CORE.md", "# root")],
    );
    make_pkg(
        root,
        &org,
        "flow",
        "cascade-mid",
        "0.1.0",
        r#"
[target."context(flow:cascade-root)".dependencies]
packages = { "org.vibevm/cascade-leaf" = "^0.1" }
"#,
        &[("spec/flows/cascade-mid/MID.md", "# mid")],
    );
    make_pkg(
        root,
        &org,
        "flow",
        "cascade-leaf",
        "0.1.0",
        "",
        &[("spec/flows/cascade-leaf/LEAF.md", "# leaf")],
    );

    let abs_org = org
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    let url = format!("git+file:///{abs_org}");
    (org, url)
}

#[test]
fn install_expands_cascading_conditional_dependencies() {
    if !git_available() {
        eprintln!("skipping install_expands_cascading_conditional_dependencies: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_cascading_conditional_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/rust-cli")
        .arg("org.vibevm/cascade-root")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    let names: Vec<_> = lock
        .packages
        .iter()
        .map(|p| format!("{}/{}", p.group, p.name))
        .collect();
    for expected in [
        "org.vibevm/rust-cli",
        "org.vibevm/cascade-root",
        "org.vibevm/cascade-mid",
        "org.vibevm/cascade-leaf",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "cascade expected {expected}; got {:?}",
            names
        );
    }
}

#[test]
fn conditional_dependencies_dormant_when_predicate_misses() {
    if !git_available() {
        eprintln!("skipping conditional_dependencies_dormant: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_conditional_deps_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Install dispatcher WITHOUT org.vibevm/rust-cli. The conditional
    // predicate `context(stack:rust)` doesn't match → rust-helper
    // stays dormant.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/dispatcher")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    let names: Vec<_> = lock
        .packages
        .iter()
        .map(|p| format!("{}/{}", p.group, p.name))
        .collect();
    assert!(
        names.iter().any(|n| n == "org.vibevm/dispatcher"),
        "got {:?}",
        names
    );
    assert!(
        !names.iter().any(|n| n == "org.vibevm/rust-helper"),
        "rust-helper should NOT be installed; got {:?}",
        names
    );
}

/// Build a per-package git registry with two tagged versions of
/// `org.vibevm/test-multi`: v0.1.0 (the older release) and v0.2.0 (newer).
/// Returns `(registry_org_root, file_url_for_org)` so tests can wire
/// the registry into a project's `vibe.toml`.
fn make_two_version_per_package_registry(root: &Path) -> (PathBuf, String) {
    let org = root.join("org");
    fs::create_dir_all(&org).unwrap();
    let src = root.join("src-flow-test-multi");
    fs::create_dir_all(src.join("spec/flows/test-multi")).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);
    fs::write(
        src.join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "test-multi"
kind = "flow"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(src.join("spec/flows/test-multi/PROTOCOL.md"), "# v0.1.0").unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "v0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);

    // Bump to 0.2.0.
    fs::write(
        src.join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "test-multi"
kind = "flow"
version = "0.2.0"
"#,
    )
    .unwrap();
    fs::write(src.join("spec/flows/test-multi/PROTOCOL.md"), "# v0.2.0").unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "v0.2.0"]);
    run_git(&src, &["tag", "v0.2.0"]);

    let bare = org.join("org.vibevm_test-multi.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            src.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    let abs_org = org
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    let url = format!("git+file:///{abs_org}");
    (org, url)
}

#[test]
fn outdated_reports_newer_version_available() {
    if !git_available() {
        eprintln!("skipping outdated_reports_newer_version_available: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_two_version_per_package_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);

    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Install pinned to v0.1.0 explicitly.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm/test-multi@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let out = vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("--json")
        .arg("outdated")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout must be JSON");
    assert_eq!(v["command"], "outdated");
    assert_eq!(v["update_available"], 1);
    let pkg = &v["packages"][0];
    assert_eq!(pkg["group"], "org.vibevm");
    assert_eq!(pkg["name"], "test-multi");
    assert_eq!(pkg["installed"], "0.1.0");
    assert_eq!(pkg["latest"], "0.2.0");
    assert_eq!(pkg["status"], "update available");
}

#[test]
fn show_features_lists_active_features_after_install() {
    // After installing with --features, `vibe show features --json`
    // surfaces the activation set per package.
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    vibe()
        .arg("install")
        .arg("org.vibevm/feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("with-rust")
        .arg("--assume-yes")
        .assert()
        .success();
    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("features")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout must be JSON");
    assert_eq!(v["command"], "show:features");
    let pkgs = v["packages"].as_array().unwrap();
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0]["package"], "org.vibevm/feat-pkg");
    let features: Vec<&str> = pkgs[0]["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(features.contains(&"with-rust"));
}

// NOTE: `show_subskills_and_purls_after_install` was deleted with the
// PROP-009 switch-over. Its `show subskills` half asserted that an
// `if_files`-activated subskill (`doc/extra`) is recorded in the
// lockfile and surfaced by `vibe show subskills` — retired, since
// `vibe install` no longer runs install-time subskill activation and
// `subskills_active` is always empty. Its `show purls` half (the
// package-level `describes` PURL surfacing) is genuine kept behaviour
// and is now covered by `omnibus_show_features_and_purls_after_install`.

// NOTE: `install_subskill_activates_via_if_files_glob` was deleted with
// the PROP-009 switch-over. It asserted that an `if_files`-activated
// subskill materialises its content into the project's `spec/` tree at
// install time and records itself in the lockfile's `subskills_active`.
// The loading model materialises a package's published tree verbatim
// into its `vibedeps/<slot>/` — subskill directories ride along inside
// that slot as plain content — and the install pipeline no longer runs
// per-file activation (`subskills_active` is always empty). There is no
// install-time subskill activation to test. The subskill *manifest*
// surface remains parsed and unit-tested in `vibe-core`.

// NOTE: `install_with_language_flag_picks_localised_content` and
// `install_with_language_falls_back_to_canonical_when_translation_missing`
// were deleted with the PROP-009 switch-over (their `make_i18n_fixture_
// registry` helper went with them). They asserted that `vibe install
// --language ru` resolves each `*.ru.md` sidecar and writes the
// localised bytes to the canonical project path (and that a missing
// translation falls back to the canonical content). The loading model
// materialises a package's published tree *verbatim* into its
// `vibedeps/<slot>/` — both `PROTOCOL.md` and `PROTOCOL.ru.md` simply
// ride along as plain files — so there is no install-time sidecar
// resolution to test. The resolved language chain is still recorded in
// the lockfile (`meta.language_chain`, the per-package `language`
// field); sidecar *selection* moves to the computed-view / boot layer.

#[test]
fn set_mirror_accepts_file_url_with_no_org_segment() {
    // Regression defence for the 2026-05-04 walk of
    // manual-tests/M1.6-mirror-vendor-smoke.md Scenario A4. Earlier
    // `run_set_mirror` ran `extract_host_segment` + `extract_org_segment`
    // on the mirror URL — the same gate that `[[registry]]` URLs go
    // through. That refused `file:///<dir>` URLs because they have no
    // host or org segment, even though `vibe registry vendor` produces
    // exactly that URL shape and recommends it as a `[[mirror]]`. This
    // test pins the post-fix shape: a `file:///` URL is accepted, the
    // manifest learns a new `[[mirror]]` block, and the URL is recorded
    // verbatim.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let vendor_dir = tempfile::tempdir().unwrap();
    let abs_vendor = vendor_dir.path().to_string_lossy().replace('\\', "/");
    let mirror_url = format!("file:///{abs_vendor}");

    vibe()
        .arg("registry")
        .arg("set-mirror")
        .arg("vibespecs")
        .arg(&mirror_url)
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    let manifest = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
    assert!(
        manifest.contains("[[mirror]]"),
        "expected [[mirror]] block in manifest; got:\n{manifest}"
    );
    assert!(
        manifest.contains(&mirror_url),
        "expected mirror URL `{mirror_url}` in manifest; got:\n{manifest}"
    );
}

#[test]
fn set_mirror_rejects_empty_url() {
    // The pre-fix gate accidentally caught empty URLs as a side effect
    // of `extract_host_segment` failing on them. The post-fix gate is
    // intentionally narrow: non-empty after trim. Pin both the empty
    // and the whitespace-only case so a refactor that loosens the
    // check doesn't sneak through unnoticed.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    for bad in ["", "   "] {
        vibe()
            .arg("registry")
            .arg("set-mirror")
            .arg("vibespecs")
            .arg(bad)
            .arg("--path")
            .arg(project.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("mirror URL must be non-empty"));
    }
}

// ---------------------------------------------------------------------------
// Help-text smoke — every CLI subcommand renders `--help`
// ---------------------------------------------------------------------------
//
// Regression defence for the clap derive on `crates/vibe-cli/src/cli.rs`. A
// silently-empty help text on a subcommand has happened in other Rust CLIs
// when a `#[command]` attribute drifts (missing `about` on a fresh
// subcommand, mistyped `subcommand` attribute on a parent, etc.) — the CLI
// still parses and runs, but `--help` returns blank and confuses users.
// This test catches that the next time someone adds a subcommand without
// also adding its docstring.
//
// Each entry is a path to a help target. `--help` is appended; the subcommand
// itself never runs (clap short-circuits on `--help`), so no project setup
// or network access is needed.

#[test]
fn every_subcommand_renders_help() {
    // Path lists for every help surface clap exposes today. When a new
    // subcommand lands in `crates/vibe-cli/src/cli.rs`, add it here too —
    // the help-text contract is part of the user-facing surface.
    let subcommand_paths: &[&[&str]] = &[
        &[], // top-level: `vibe --help`
        &["init"],
        &["install"],
        &["list"],
        &["outdated"],
        &["search"],
        &["mcp"],
        &["mcp", "serve"],
        &["mcp", "install"],
        &["mcp", "status"],
        &["uninstall"],
        &["update"],
        &["check"],
        &["show"],
        &["show", "effective"],
        &["show", "config"],
        &["show", "features"],
        &["show", "subskills"],
        &["show", "purls"],
        &["registry"], // shows the registry subcommand enum
        &["registry", "sync"],
        &["registry", "publish"],
        &["registry", "list"],
        &["registry", "add"],
        &["registry", "set-mirror"],
        &["registry", "remove"],
        &["registry", "vendor"],
        &["version"],
    ];

    for path in subcommand_paths {
        let mut cmd = vibe();
        for seg in *path {
            cmd.arg(seg);
        }
        cmd.arg("--help");
        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("spawning `vibe {} --help` failed: {e}", path.join(" ")));

        let label = if path.is_empty() {
            "vibe --help".to_string()
        } else {
            format!("vibe {} --help", path.join(" "))
        };

        assert!(
            out.status.success(),
            "`{label}` should exit 0 — got {:?}, stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.trim().is_empty(),
            "`{label}` produced empty stdout — clap derive likely lost the docstring"
        );
        // Cheap sanity: every help screen mentions usage. Catches the
        // "wrong binary got invoked" scenario without coupling to wording.
        assert!(
            stdout.to_lowercase().contains("usage"),
            "`{label}` stdout did not contain `Usage` — got:\n{stdout}"
        );
    }
}

#[test]
fn version_subcommand_matches_version_flag() {
    // `vibe version` and `vibe --version` are documented as identical
    // (see docs/commands/version.md). Drift between the two would confuse
    // any tooling that scrapes the version string.
    let sub = vibe().arg("version").output().expect("spawn vibe version");
    let flag = vibe()
        .arg("--version")
        .output()
        .expect("spawn vibe --version");
    assert!(sub.status.success() && flag.status.success());
    let sub_out = String::from_utf8_lossy(&sub.stdout).trim().to_string();
    let flag_out = String::from_utf8_lossy(&flag.stdout).trim().to_string();
    assert_eq!(
        sub_out, flag_out,
        "`vibe version` and `vibe --version` must produce identical output"
    );
    assert!(
        sub_out.starts_with("vibe "),
        "version output should start with `vibe `, got: {sub_out}"
    );
}

// ---------------------------------------------------------------------------
// vibe reinstall — PROP-009 §2.10
// ---------------------------------------------------------------------------

#[test]
fn reinstall_regenerates_deleted_boot_artifacts() {
    // `vibe reinstall` (no `--force`) recomputes a node's boot artifacts
    // from the materialised `vibedeps/` tree already on disk — the fix
    // for a deleted or hand-edited `INDEX.md` / redirect (PROP-009 §2.10).
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    vibe()
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(make_wal_dir_registry(project.path()))
        .arg("--assume-yes")
        .assert()
        .success();

    // Delete the generated boot artifacts — simulate a botched edit or a
    // wrong previous generation pass.
    let index = project.path().join("spec/boot/INDEX.md");
    let claude = project.path().join("CLAUDE.md");
    fs::remove_file(&index).unwrap();
    fs::remove_file(&claude).unwrap();

    // `vibe reinstall` regenerates them from the intact `vibedeps/` slot.
    vibe()
        .arg("reinstall")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    assert!(index.is_file(), "INDEX.md must be regenerated");
    assert!(
        claude.is_file(),
        "the CLAUDE.md redirect must be regenerated"
    );
    // The regenerated INDEX.md still names the installed dependency's boot
    // snippet — boot is recomputed from the materialised tree, not lost.
    let index_body = fs::read_to_string(&index).unwrap();
    assert!(
        index_body.contains("vibedeps/flow-wal/0.2.0/spec/boot/10-flow-wal.md"),
        "regenerated INDEX.md must name the materialised dependency boot:\n{index_body}"
    );
}

#[test]
fn reinstall_succeeds_on_a_project_with_no_dependencies() {
    // `vibe reinstall` of a project that has only authored boot and no
    // installed packages regenerates its boot artifacts from the
    // `spec/boot/` tree — an absent or empty `vibe.lock` is not an error.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    // Remove any boot artifacts a prior step produced, so the assertion
    // below proves `vibe reinstall` regenerated them.
    let index = project.path().join("spec/boot/INDEX.md");
    let claude = project.path().join("CLAUDE.md");
    let _ = fs::remove_file(&index);
    let _ = fs::remove_file(&claude);

    vibe()
        .arg("reinstall")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    assert!(index.is_file(), "INDEX.md must be generated");
    let claude_body = fs::read_to_string(&claude).unwrap();
    assert!(
        claude_body.contains("Generated by vibe"),
        "CLAUDE.md must be a generated redirect"
    );
}

#[test]
fn reinstall_non_force_bails_when_vibedeps_slot_missing() {
    // Without `--force`, `vibe reinstall` regenerates boot FROM the
    // materialised tree — it cannot recover a deleted `vibedeps/` slot,
    // so it stops and points at `--force` (PROP-009 §2.10).
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    vibe()
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(make_wal_dir_registry(project.path()))
        .arg("--assume-yes")
        .assert()
        .success();

    // Delete the materialised slot — the lockfile still records org.vibevm.world/wal.
    fs::remove_dir_all(project.path().join("vibedeps/flow-wal")).unwrap();

    vibe()
        .arg("reinstall")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--force"));
}

#[test]
fn reinstall_reports_json() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    vibe()
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(make_wal_dir_registry(project.path()))
        .arg("--assume-yes")
        .assert()
        .success();

    let out = vibe()
        .arg("--json")
        .arg("reinstall")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    // `vibe reinstall` emits exactly one JSON document — its report.
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("reinstall emits one JSON document");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "reinstall");
    assert_eq!(payload["forced"], false);
    // The standalone project is one node — its boot was regenerated.
    assert_eq!(payload["nodes_regenerated"].as_array().unwrap().len(), 1);
}

#[test]
fn reinstall_force_refetches_corrupted_vibedeps() {
    // `vibe reinstall --force` re-fetches every locked package from
    // source and re-materialises `vibedeps/` — the escape hatch for a
    // corrupted materialised subtree (PROP-009 §2.10).
    if !git_available() {
        eprintln!("skipping reinstall_force_refetches_corrupted_vibedeps: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Corrupt a content file inside the materialised `vibedeps/` slot.
    let corrupted = project
        .path()
        .join("vibedeps/flow-wal/0.2.0/spec/flows/wal/WAL-PROTOCOL.md");
    assert!(
        corrupted.is_file(),
        "org.vibevm.world/wal ships WAL-PROTOCOL.md"
    );
    fs::write(&corrupted, "CORRUPTED — hand-edited garbage").unwrap();

    // `vibe reinstall --force` re-fetches from source and overwrites the
    // slot wholesale; the lockfile-pinned version is unchanged.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("reinstall")
        .arg(project.path())
        .arg("--force")
        .arg("--assume-yes")
        .assert()
        .success();

    let restored = fs::read_to_string(&corrupted).unwrap();
    assert!(
        !restored.contains("CORRUPTED"),
        "the corrupted file must be overwritten by the fresh fetch: {restored}"
    );
    // The boot artifacts are intact, and the version did not move.
    assert!(project.path().join("spec/boot/INDEX.md").is_file());
    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].version.to_string(), "0.2.0");
}
