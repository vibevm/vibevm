//! Deterministic no-model global application CLI simulation.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use serde_json::Value;

const SCENARIO: &str = include_str!("application-global.simulation.json");

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
    user.vibe()
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
    user.vibe()
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
    user.vibe()
        .current_dir(registry.path())
        .args(["uninstall", "-g", coordinate, "--json"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "outside its owned application host",
        ));
}

fn global<const N: usize>(user: &UserScratch, cwd: &Path, args: [&str; N]) -> Value {
    let output = user.vibe().current_dir(cwd).args(args).output().unwrap();
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

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
