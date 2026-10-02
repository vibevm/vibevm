//! Deterministic no-model global application CLI simulation.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use serde_json::Value;

const SCENARIO: &str = include_str!("application-global.simulation.json");

// Ambient native client paths are not isolated by environment overrides on
// every platform; the later injected-directory fixture supplies portable proof.
#[cfg(target_os = "linux")]
#[test]
fn global_mcp_uses_a_user_project_and_leaves_the_callers_project_untouched() {
    let registry = tempfile::tempdir().unwrap();
    let package = package_root(registry.path(), "ai.lev", "fpf-mcp", "1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\nname='fpf-mcp'\ngroup='ai.lev'\nkind='mcp'\nversion='1.0.0'\n\
         [[mcp_server]]\nname='fpf'\ntransport='streamable-http'\nurl='https://mcp.fpf.tools/mcp'\n",
    )
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    let original = "[project]\nname='untouched'\nversion='1.0.0'\n";
    fs::write(project.path().join("vibe.toml"), original).unwrap();
    let user = UserScratch::new();

    let output = isolated_vibe(&user)
        .current_dir(project.path())
        .args([
            "install",
            "-g",
            "mcp:ai.lev/fpf-mcp",
            "--agent",
            "codex",
            "--registry",
            text(registry.path()),
            "--assume-yes",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(project.path().join("vibe.toml")).unwrap(),
        original
    );
    let global_root = user.settings.join("mcp/user-project");
    assert!(
        fs::read_to_string(global_root.join("vibe.toml"))
            .unwrap()
            .contains("mcp:ai.lev/fpf-mcp")
    );
    assert!(global_root.join("vibe.lock").exists());
    assert!(
        global_root
            .join("vibevm/vibedeps/ai.lev.fpf-mcp/1.0.0/vibe.toml")
            .exists()
    );

    let native = user.settings.join("native-home/.codex/config.toml");
    let config: toml::Value = toml::from_str(&fs::read_to_string(&native).unwrap()).unwrap();
    assert_eq!(
        config["mcp_servers"]["fpf"]["url"].as_str(),
        Some("https://mcp.fpf.tools/mcp")
    );
    let receipt: Value = serde_json::from_str(
        &fs::read_to_string(native.with_file_name("config.toml.vibevm-mcp.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["entries"]["fpf"]["package"], "ai.lev/fpf-mcp");
    assert_eq!(receipt["entries"]["fpf"]["version"], "1.0.0");
    assert!(!project.path().join(".codex").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn global_mcp_requires_an_agent_without_a_terminal_before_creating_state() {
    let user = UserScratch::new();
    isolated_vibe(&user)
        .args(["install", "-g", "mcp:ai.lev/fpf-mcp", "--json"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--agent"));
    assert!(!user.settings.join("mcp/user-project").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn global_mcp_update_and_uninstall_select_one_user_package() {
    let registry = tempfile::tempdir().unwrap();
    seed_remote_mcp(registry.path(), "fpf-mcp", "1.0.0");
    seed_remote_mcp(registry.path(), "other-mcp", "1.0.0");
    let project = tempfile::tempdir().unwrap();
    let original = "[project]\nname='untouched'\nversion='1.0.0'\n";
    fs::write(project.path().join("vibe.toml"), original).unwrap();
    let user = UserScratch::new();

    for name in ["fpf-mcp", "other-mcp"] {
        let package = format!("mcp:ai.lev/{name}");
        global(
            &user,
            project.path(),
            [
                "install",
                "-g",
                &package,
                "--agent",
                "codex",
                "--registry",
                text(registry.path()),
                "--assume-yes",
                "--json",
            ],
        );
    }
    seed_remote_mcp(registry.path(), "fpf-mcp", "1.0.1");
    seed_remote_mcp(registry.path(), "other-mcp", "1.0.1");
    global(
        &user,
        project.path(),
        [
            "update",
            "-g",
            "mcp:ai.lev/fpf-mcp",
            "--registry",
            text(registry.path()),
            "--assume-yes",
            "--json",
        ],
    );
    let user_root = user.settings.join("mcp/user-project");
    assert!(
        user_root
            .join("vibevm/vibedeps/ai.lev.fpf-mcp/1.0.1/vibe.toml")
            .exists()
    );
    assert!(
        user_root
            .join("vibevm/vibedeps/ai.lev.other-mcp/1.0.0/vibe.toml")
            .exists()
    );
    assert!(
        !user_root
            .join("vibevm/vibedeps/ai.lev.other-mcp/1.0.1")
            .exists()
    );
    let native = user.settings.join("native-home/.codex/config.toml");
    let receipt: Value = serde_json::from_str(
        &fs::read_to_string(native.with_file_name("config.toml.vibevm-mcp.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["entries"]["fpf-mcp"]["version"], "1.0.1");
    assert_eq!(receipt["entries"]["other-mcp"]["version"], "1.0.0");
    let user_manifest = fs::read_to_string(user_root.join("vibe.toml")).unwrap();
    assert!(
        user_manifest.contains("\"mcp:ai.lev/fpf-mcp\" = \"=1.0.1\""),
        "{user_manifest}"
    );

    fs::remove_dir_all(registry.path()).unwrap();
    global(
        &user,
        project.path(),
        [
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf-mcp",
            "--assume-yes",
            "--json",
        ],
    );
    assert!(
        !user_root
            .join("vibevm/vibedeps/ai.lev.fpf-mcp/1.0.1")
            .exists()
    );
    assert!(
        user_root
            .join("vibevm/vibedeps/ai.lev.other-mcp/1.0.0/vibe.toml")
            .exists()
    );
    let config: toml::Value = toml::from_str(&fs::read_to_string(&native).unwrap()).unwrap();
    assert!(config["mcp_servers"].get("fpf-mcp").is_none());
    assert_eq!(
        config["mcp_servers"]["other-mcp"]["url"].as_str(),
        Some("https://mcp.fpf.tools/mcp")
    );
    assert_eq!(
        fs::read_to_string(project.path().join("vibe.toml")).unwrap(),
        original
    );
}

#[cfg(target_os = "linux")]
#[test]
fn global_mcp_update_and_uninstall_do_not_create_an_empty_user_project() {
    let user = UserScratch::new();
    for args in [
        vec!["update", "-g", "mcp:ai.lev/fpf-mcp", "--json"],
        vec!["uninstall", "-g", "mcp:ai.lev/fpf-mcp", "--json"],
    ] {
        isolated_vibe(&user).args(args).assert().failure();
    }
    assert!(!user.settings.join("mcp/user-project").exists());
}

#[test]
fn selected_remote_registration_refuses_invalid_selection_and_foreign_entries_before_writes() {
    let registry = tempfile::tempdir().unwrap();
    seed_remote_mcp(registry.path(), "fpf-mcp", "1.0.0");
    let project = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    user.init_project(project.path());
    isolated_vibe(&user)
        .args(["install", "mcp:ai.lev/fpf-mcp", "--path"])
        .arg(project.path())
        .args(["--registry", text(registry.path()), "--assume-yes"])
        .assert()
        .success();
    let config = project.path().join(".codex/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let foreign = "[mcp_servers.fpf-mcp]\nurl = 'https://operator.example/mcp'\n";
    fs::write(&config, foreign).unwrap();
    let manifest_before = fs::read(project.path().join("vibe.toml")).unwrap();
    let lock_before = fs::read(project.path().join("vibe.lock")).unwrap();
    for (selector, agent, scope) in [
        ("skill:ai.lev/fpf-mcp", "codex", "project"),
        ("mcp:ai.lev/fpf-mcp@=9.9.9", "codex", "project"),
        ("mcp:ai.lev/fpf-mcp@^1.0", "codex", "project"),
        ("mcp:ai.lev/unknown", "codex", "project"),
        ("mcp:ai.lev/fpf-mcp", "codex", "user"),
        ("mcp:ai.lev/fpf-mcp", "claude-desktop", "project"),
        ("mcp:ai.lev/fpf-mcp", "codex", "project"),
    ] {
        isolated_vibe(&user)
            .args(["mcp", "install", selector, "--path"])
            .arg(project.path())
            .args(["--agent", agent, "--scope", scope, "--yes", "--json"])
            .assert()
            .failure();
        assert_eq!(fs::read_to_string(&config).unwrap(), foreign);
        assert!(
            !config
                .with_file_name("config.toml.vibevm-mcp.json")
                .exists()
        );
        assert!(!user.settings.join("native-home/.codex").exists());
        assert!(!user.settings.join("mcp/user-project").exists());
        assert_eq!(
            fs::read(project.path().join("vibe.toml")).unwrap(),
            manifest_before
        );
        assert_eq!(
            fs::read(project.path().join("vibe.lock")).unwrap(),
            lock_before
        );
    }
    // Positive control: this inventory and target register once the foreign
    // entry is removed, so failures above are not a broken fixture.
    fs::remove_file(&config).unwrap();
    isolated_vibe(&user)
        .args(["mcp", "install", "mcp:ai.lev/fpf-mcp@=1.0.0", "--path"])
        .arg(project.path())
        .args(["--agent", "codex", "--scope", "project", "--yes"])
        .assert()
        .success();
    let registered: toml::Value = toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        registered["mcp_servers"]["fpf-mcp"]["url"].as_str(),
        Some("https://mcp.fpf.tools/mcp")
    );
}

#[test]
fn global_application_dispatch_update_and_source_independent_uninstall() {
    let scenario: Value = serde_json::from_str(SCENARIO).unwrap();
    assert_eq!(scenario["protocol"], "vibe-application-simulation/1");
    let app = &scenario["application"];
    let registry = tempfile::tempdir().unwrap();
    seed_registry(registry.path(), app, false);
    let project = tempfile::tempdir().unwrap();
    let project_manifest = "[project]\nname = \"untouched\"\nversion = \"1.0.0\"\n";
    fs::write(project.path().join("vibe.toml"), project_manifest).unwrap();
    let user = UserScratch::new();
    let coordinate = format!(
        "{}/{}",
        app["group"].as_str().unwrap(),
        app["name"].as_str().unwrap()
    );

    let installed = global(
        &user,
        project.path(),
        [
            "install",
            "-g",
            &coordinate,
            "--registry",
            text(registry.path()),
            "--json",
        ],
    );
    assert_eq!(installed["command"], "install");
    assert_eq!(
        fs::read_to_string(project.path().join("vibe.toml")).unwrap(),
        project_manifest
    );

    let updated = global(
        &user,
        project.path(),
        [
            "update",
            "-g",
            &coordinate,
            "--registry",
            text(registry.path()),
            "--json",
        ],
    );
    assert_eq!(updated["command"], "update");
    fs::remove_dir_all(registry.path()).unwrap();

    let removed = global(
        &user,
        project.path(),
        ["uninstall", "-g", &coordinate, "--json"],
    );
    assert_eq!(removed["command"], "uninstall");
    let index: Value =
        serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
            .unwrap();
    assert_eq!(index["applications"]["demo"]["status"], "undeployed");
    assert_eq!(scenario["expected"]["zeroInference"], true);
}

#[test]
fn strict_reply_and_tampered_index_paths_refuse_without_replacing_success() {
    let scenario: Value = serde_json::from_str(SCENARIO).unwrap();
    let registry = tempfile::tempdir().unwrap();
    seed_registry(registry.path(), &scenario["application"], false);
    let user = UserScratch::new();
    let coordinate = "org.example/demo-app";
    global(
        &user,
        registry.path(),
        [
            "install",
            "-g",
            coordinate,
            "--registry",
            text(registry.path()),
            "--json",
        ],
    );

    let index_path = user.settings.join("applications/index.json");
    let before = fs::read(&index_path).unwrap();
    let app_manifest =
        package_root(registry.path(), "org.example", "demo-app", "1.0.0").join("vibe.toml");
    let renamed = fs::read_to_string(&app_manifest)
        .unwrap()
        .replace("id='demo'", "id='other'");
    fs::write(&app_manifest, renamed).unwrap();
    isolated_vibe(&user)
        .current_dir(registry.path())
        .args([
            "install",
            "-g",
            coordinate,
            "--registry",
            text(registry.path()),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already bound to application id"));
    assert_eq!(fs::read(&index_path).unwrap(), before);

    seed_registry(registry.path(), &scenario["application"], true);
    isolated_vibe(&user)
        .current_dir(registry.path())
        .args([
            "update",
            "-g",
            coordinate,
            "--registry",
            text(registry.path()),
            "--json",
        ])
        .assert()
        .failure();
    assert_eq!(fs::read(&index_path).unwrap(), before);

    let mut index: Value = serde_json::from_slice(&before).unwrap();
    index["applications"]["demo"]["hostRoot"] = Value::String(text(registry.path()).into());
    fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
    isolated_vibe(&user)
        .current_dir(registry.path())
        .args(["uninstall", "-g", coordinate, "--json"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "outside its owned application host",
        ));
}

fn global<const N: usize>(user: &UserScratch, cwd: &Path, args: [&str; N]) -> Value {
    let output = isolated_vibe(&user)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn seed_registry(root: &Path, app: &Value, bad_reply: bool) {
    let app_root = package_root(
        root,
        app["group"].as_str().unwrap(),
        app["name"].as_str().unwrap(),
        app["version"].as_str().unwrap(),
    );
    let provider_root = package_root(
        root,
        app["installerGroup"].as_str().unwrap(),
        app["installerName"].as_str().unwrap(),
        app["installerVersion"].as_str().unwrap(),
    );
    fs::create_dir_all(&app_root).unwrap();
    fs::create_dir_all(provider_root.join("tooling")).unwrap();
    fs::write(
        app_root.join("vibe.toml"),
        format!(
            "[package]\nname='{}'\ngroup='{}'\nkind='tool'\nversion='{}'\n\
             [application]\nid='{}'\ninstaller_package='{}/{}@={}'\nruntime='node'\n\
             entry='tooling/application.mjs'\ncommands=['demo-client','demo-server']\n",
            app["name"].as_str().unwrap(),
            app["group"].as_str().unwrap(),
            app["version"].as_str().unwrap(),
            app["id"].as_str().unwrap(),
            app["installerGroup"].as_str().unwrap(),
            app["installerName"].as_str().unwrap(),
            app["installerVersion"].as_str().unwrap(),
        ),
    )
    .unwrap();
    fs::write(
        provider_root.join("vibe.toml"),
        format!(
            "[package]\nname='{}'\ngroup='{}'\nkind='tool'\nversion='{}'\n",
            app["installerName"].as_str().unwrap(),
            app["installerGroup"].as_str().unwrap(),
            app["installerVersion"].as_str().unwrap(),
        ),
    )
    .unwrap();
    fs::write(
        provider_root.join("tooling/application.mjs"),
        fake_installer(bad_reply),
    )
    .unwrap();
}

fn fake_installer(bad_reply: bool) -> String {
    format!(
        r#"import {{copyFile, mkdir, readFile, writeFile}} from 'node:fs/promises';
import {{dirname, join}} from 'node:path';
const context = JSON.parse(await readFile(process.env.VIBE_APPLICATION_CONTEXT, 'utf8'));
const replyPath = process.env.VIBE_APPLICATION_REPLY;
if (context.operation === 'uninstall') {{
  await writeFile(replyPath, JSON.stringify({{protocol:'vibe-application-result/1',operation:'uninstall',applicationId:context.application.id,status:'undeployed',hostRoot:context.hostRoot,management:null,commands:context.application.commands,message:'removed'}}));
}} else {{
  const entry = join(context.hostRoot, 'generations', 'fake', 'runtime', 'application.mjs');
  await mkdir(dirname(entry), {{recursive:true}});
  await copyFile(import.meta.filename, entry);
  await writeFile(replyPath, JSON.stringify({{protocol:'vibe-application-result/1',operation:context.operation,applicationId:{},status:'ready',hostRoot:context.hostRoot,management:{{runtime:'node',entry}},commands:context.application.commands,message:'ready'}}));
}}
"#,
        if bad_reply {
            "'wrong'"
        } else {
            "context.application.id"
        }
    )
}

fn package_root(root: &Path, group: &str, name: &str, version: &str) -> PathBuf {
    root.join(group).join(name).join(format!("v{version}"))
}

fn seed_remote_mcp(root: &Path, name: &str, version: &str) {
    let package = package_root(root, "ai.lev", name, version);
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\nname='{name}'\ngroup='ai.lev'\nkind='mcp'\nversion='{version}'\n\
             [[mcp_server]]\nname='{name}'\ntransport='streamable-http'\nurl='https://mcp.fpf.tools/mcp'\n"
        ),
    )
    .unwrap();
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

// Native client state needs its own isolation; VIBE_SETTINGS only isolates Vibe.
fn isolated_vibe(user: &UserScratch) -> assert_cmd::Command {
    let home = user.settings.join("native-home");
    fs::create_dir_all(home.join(".config")).unwrap();
    let mut command = user.vibe();
    command
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("APPDATA", home.join(".config"));
    command
}
