//! Portable stop-file and publication-boundary integration.
mod common;
use common::UserScratch;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn copy(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let output = target.join(entry.file_name());
        if path.is_dir() {
            copy(&path, &output);
        } else {
            fs::copy(path, output).unwrap();
        }
    }
}

/// Every file under `root` except the `.vibe` operation state, byte for byte.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap();
            if rel
                .components()
                .next()
                .is_some_and(|part| part.as_os_str() == ".vibe")
            {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(rel.into(), fs::read(path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

/// Arrange the node's agent files so the boot regeneration rewrites them,
/// and return the stop file that regeneration itself creates.
///
/// `CLAUDE.md` holds co-tenant bytes with no managed block, so the
/// regeneration appends one after them. `GEMINI.md` is absent, so the
/// regeneration creates it — the node publication's last write, after every
/// unit and node lane. Watching it as the stop file requests the stop exactly
/// once boot regeneration has finished and before the lock is published, the
/// window the boot inverse exists for; a restore that removes it again is
/// part of the byte-identical check.
fn stop_after_boot_regeneration(project: &Path) -> PathBuf {
    fs::write(
        project.join("CLAUDE.md"),
        "# Owner notes

Keep this.
",
    )
    .unwrap();
    let stop = project.join("GEMINI.md");
    if stop.exists() {
        fs::remove_file(&stop).unwrap();
    }
    stop
}

/// The project's files are exactly `before`; a failure names the paths.
fn assert_unchanged(project: &Path, before: &BTreeMap<PathBuf, Vec<u8>>) {
    let after = snapshot(project);
    let differing: Vec<&PathBuf> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .collect();
    assert!(
        differing.is_empty(),
        "changed by the cancelled run: {differing:?}"
    );
}

fn assert_cancelled_and_restored(output: &std::process::Output) {
    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["publication"], "restored");
}

#[test]
fn existing_stop_file_preserves_manifest_for_install_and_update() {
    for verb in ["install", "update"] {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        user.vibe()
            .args(["init", "--no-registry", "--author", "Owner"])
            .arg("--path")
            .arg(project.path())
            .assert()
            .success();
        let before = fs::read(project.path().join("vibe.toml")).unwrap();
        let outer = tempfile::tempdir().unwrap();
        let stop = outer.path().join("stop");
        fs::write(&stop, "").unwrap();
        let output = user
            .vibe()
            .args([verb, "--json", "--assume-yes", "--stop-file"])
            .arg(&stop)
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(130));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["outcome"], "cancelled");
        assert_eq!(report["publication"], "not-started");
        assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
        assert!(stop.exists());
    }
}

#[test]
fn stop_requested_by_preinstall_hook_rolls_back_owned_project_publication() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let package = registry.join("org.example/hooked/v0.1.0");
    let before = fs::read(project.path().join("vibe.toml")).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
    fs::write(
        package.join("hooks/pre.sh"),
        format!("{shell}\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n"),
    )
    .unwrap();
    let ps = fs::read_to_string(package.join("hooks/pre.ps1")).unwrap();
    fs::write(package.join("hooks/pre.ps1"), format!("{ps}\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    let output = user
        .vibe()
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["publication"], "restored");
    assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists()
    );
}

#[test]
fn cancel_after_hook_replaces_manifest_preserves_the_foreign_file() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let package = registry.join("org.example/hooked/v0.1.0");
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
    fs::write(package.join("hooks/pre.sh"),format!("{shell}\nrm \"$VIBE_PROJECT_ROOT/vibe.toml\"\nprintf foreign > \"$VIBE_PROJECT_ROOT/vibe.toml\"\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n")).unwrap();
    let ps = fs::read_to_string(package.join("hooks/pre.ps1")).unwrap();
    fs::write(package.join("hooks/pre.ps1"),format!("{ps}\nRemove-Item -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT 'vibe.toml')\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT 'vibe.toml') -Value foreign -NoNewline\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    let output = user
        .vibe()
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = common::hook_slot::documents(&output.stdout);
    assert_eq!(
        documents.last().unwrap()["publication"],
        "recovery-required"
    );
    assert_eq!(
        fs::read(project.path().join("vibe.toml")).unwrap(),
        b"foreign"
    );
}

#[test]
fn update_cancel_restores_old_manifest_lock_and_installed_slot() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='file:///{}'\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    user.vibe()
        .args(["install", "org.example/hooked", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let manifest = fs::read(project.path().join("vibe.toml")).unwrap();
    let lock = fs::read(project.path().join("vibe.lock")).unwrap();
    let old = registry.join("org.example/hooked/v0.1.0");
    let next = registry.join("org.example/hooked/v0.1.1");
    copy(&old, &next);
    let declaration = fs::read_to_string(next.join("vibe.toml"))
        .unwrap()
        .replace("0.1.0", "0.1.1");
    fs::write(next.join("vibe.toml"), declaration).unwrap();
    let shell = fs::read_to_string(next.join("hooks/pre.sh")).unwrap();
    fs::write(
        next.join("hooks/pre.sh"),
        format!("{shell}\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n"),
    )
    .unwrap();
    let ps = fs::read_to_string(next.join("hooks/pre.ps1")).unwrap();
    fs::write(next.join("hooks/pre.ps1"),format!("{ps}\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    let output = user
        .vibe()
        .args([
            "update",
            // A named update really moves the version; `--all` has not bumped
            // versions since 1.0.11 (BACKLOG), which would make the stop untested.
            "org.example/hooked",
            "--exact",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(stop)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.last().unwrap()["outcome"], "cancelled");
    assert_eq!(docs.last().unwrap()["publication"], "restored");
    assert_eq!(
        fs::read(project.path().join("vibe.toml")).unwrap(),
        manifest
    );
    assert_eq!(fs::read(project.path().join("vibe.lock")).unwrap(), lock);
    assert!(
        project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .is_dir()
    );
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.1"))
            .exists()
    );
}

/// Desktop ApplyManifest is install with no coordinates, --offline and a
/// saved authored manifest. Cancelling must keep that saved file, not undo
/// an earlier editor save which is outside this invocation.
#[test]
fn apply_saved_manifest_cancel_keeps_saved_bytes_and_prior_installed_world() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='file:///{}'\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    let package = registry.join("org.example/hooked/v0.1.0");
    let shell = fs::read_to_string(package.join("hooks/pre.sh")).unwrap();
    fs::write(
        package.join("hooks/pre.sh"),
        format!("{shell}\nprintf stop > \"$VIBE_PROJECT_ROOT/.vibe/cancel-request\"\n"),
    )
    .unwrap();
    let ps = fs::read_to_string(package.join("hooks/pre.ps1")).unwrap();
    fs::write(package.join("hooks/pre.ps1"),format!("{ps}\nSet-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/cancel-request') -Value stop\n")).unwrap();
    user.vibe()
        .args(["cache", "add", "org.example/hooked", "--path"])
        .arg(project.path())
        .assert()
        .success();
    // The UI's saved authored input exists before the operation's boundary.
    let manifest_path = project.path().join("vibe.toml");
    let mut authored: toml::Value =
        toml::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    let requires = authored
        .as_table_mut()
        .unwrap()
        .entry("requires")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    let packages = requires
        .as_table_mut()
        .unwrap()
        .entry("packages")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    packages.as_table_mut().unwrap().insert(
        "org.example/hooked".into(),
        toml::Value::String("^0.1.0".into()),
    );
    fs::write(&manifest_path, toml::to_string(&authored).unwrap()).unwrap();
    let before = snapshot(project.path());
    let saved = fs::read(&manifest_path).unwrap();
    let stop = project.path().join(".vibe/cancel-request");
    fs::create_dir_all(stop.parent().unwrap()).unwrap();
    let output = user
        .vibe()
        .args([
            "install",
            "--offline",
            "--json",
            "--assume-yes",
            "--agent-mode",
            "agent",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = common::hook_slot::documents(&output.stdout);
    assert_eq!(documents.last().unwrap()["outcome"], "cancelled");
    assert_eq!(documents.last().unwrap()["publication"], "restored");
    assert_eq!(fs::read(&manifest_path).unwrap(), saved);
    assert_eq!(snapshot(project.path()), before);
}

#[test]
fn install_stop_after_boot_regeneration_restores_boot_lanes_and_agent_files() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    let stop = stop_after_boot_regeneration(project.path());
    let before = snapshot(project.path());
    let output = user
        .vibe()
        .args([
            "install",
            "org.example/hooked@=0.1.0",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_cancelled_and_restored(&output);
    assert_unchanged(project.path(), &before);
}

#[test]
fn update_stop_after_boot_regeneration_restores_boot_lanes_and_agent_files() {
    use common::hook_slot::{Fixture, setup};
    let (_outer, user, project, registry) = setup(Fixture::Timing);
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='fixture'\nurl='file:///{}'\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    user.vibe()
        .args(["install", "org.example/hooked", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let next = registry.join("org.example/hooked/v0.1.1");
    copy(&registry.join("org.example/hooked/v0.1.0"), &next);
    let declaration = fs::read_to_string(next.join("vibe.toml"))
        .unwrap()
        .replace("0.1.0", "0.1.1");
    fs::write(next.join("vibe.toml"), declaration).unwrap();
    let stop = stop_after_boot_regeneration(project.path());
    let before = snapshot(project.path());
    let output = user
        .vibe()
        .args([
            "update",
            "org.example/hooked",
            "--exact",
            "--json",
            "--assume-yes",
            "--stop-file",
        ])
        .arg(&stop)
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert_cancelled_and_restored(&output);
    assert_unchanged(project.path(), &before);
}
