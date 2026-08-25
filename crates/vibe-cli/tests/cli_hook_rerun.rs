//! Install-family hook scheduling at the CLI boundary.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, git_available, run_git, write_project_with_per_package_registry};

#[derive(Clone, Copy)]
enum HookFixture {
    Count,
    CountPrint(&'static str),
    Exit(i32),
}

impl HookFixture {
    fn shell(self) -> String {
        match self {
            Self::Count => r#"set -eu
counter=.hook-count
value=0
if [ -f "$counter" ]; then value=$(tr -d '\r\n' < "$counter"); fi
value=$((value + 1))
printf '%s\n' "$value" > "$counter"
"#
            .to_string(),
            Self::CountPrint(marker) => format!(
                r#"set -eu
counter=.hook-count
value=0
if [ -f "$counter" ]; then value=$(tr -d '\r\n' < "$counter"); fi
value=$((value + 1))
printf '%s\n' "$value" > "$counter"
printf '%s\n' '{marker}'
"#
            ),
            Self::Exit(code) => format!("exit {code}\n"),
        }
    }

    fn powershell(self) -> String {
        match self {
            Self::Count => r#"$counter = Join-Path (Get-Location) ".hook-count"
$value = 0
if (Test-Path -LiteralPath $counter) {
    $value = [int](Get-Content -Raw -LiteralPath $counter).Trim()
}
Set-Content -LiteralPath $counter -Value ($value + 1)
exit 0
"#
            .to_string(),
            Self::CountPrint(marker) => format!(
                r#"$counter = Join-Path (Get-Location) ".hook-count"
$value = 0
if (Test-Path -LiteralPath $counter) {{
    $value = [int](Get-Content -Raw -LiteralPath $counter).Trim()
}}
Set-Content -LiteralPath $counter -Value ($value + 1)
Write-Output "{marker}"
exit 0
"#
            ),
            Self::Exit(code) => format!("exit {code}\n"),
        }
    }
}

fn make_hook_registry(root: &Path, group: &str, versions: &[(&str, &str, HookFixture)]) -> PathBuf {
    let source = root.join("src-hooked");
    fs::create_dir_all(source.join("boot")).unwrap();
    fs::create_dir_all(source.join("hooks")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    fs::write(source.join("boot/10-hooked.md"), "hook fixture boot\n").unwrap();

    for (version, payload, hook) in versions {
        let manifest = format!(
            r#"[package]
group = "{group}"
name = "hooked"
kind = "flow"
version = "{version}"

[boot_snippet]
source = "boot/10-hooked.md"
category = "flow"

[hooks]
post-install = "hooks/count"
"#
        );
        fs::write(source.join("vibe.toml"), manifest).unwrap();
        fs::write(source.join("payload.txt"), payload).unwrap();
        fs::write(source.join("hooks/count.sh"), hook.shell()).unwrap();
        fs::write(source.join("hooks/count.ps1"), hook.powershell()).unwrap();
        run_git(&source, &["add", "-A"]);
        run_git(
            &source,
            &["commit", "-m", &format!("{group}/hooked@{version}")],
        );
        run_git(&source, &["tag", &format!("v{version}")]);
    }

    let bare = root.join(format!("{group}.hooked.git"));
    run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    root.to_path_buf()
}

fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

fn read_counter(slot: &Path) -> String {
    fs::read_to_string(slot.join(".hook-count"))
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn lifecycle_suppresses_hook_subprocess_streams_in_json_and_quiet_modes() {
    if !git_available() {
        eprintln!("skipping lifecycle hook-stdio e2e: git not on PATH");
        return;
    }

    const MARKER: &str = "HOOK-STDIO-MUST-NOT-ESCAPE";
    let outer = tempfile::tempdir().unwrap();
    let registry = make_hook_registry(
        outer.path(),
        "org.vibevm",
        &[("0.1.0", "payload\n", HookFixture::CountPrint(MARKER))],
    );
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    user.vibe()
        .arg("install")
        .arg("org.vibevm/hooked@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let slot = project
        .path()
        .join(common::slot_dir("org.vibevm.hooked", "0.1.0"));
    for mode in ["json", "quiet"] {
        user.vibe()
            .arg("clean")
            .arg("--path")
            .arg(project.path())
            .arg("--assume-yes")
            .assert()
            .success();

        let mut command = user.vibe();
        command.arg("build");
        command.arg(if mode == "json" { "--json" } else { "--quiet" });
        let output = command
            .arg("--path")
            .arg(project.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains(MARKER), "{mode}: {stdout}");
        if mode == "json" {
            let report: vibe_wire::generated::lifecycle_report::LifecycleReport =
                serde_json::from_slice(&output.stdout)
                    .expect("hook output must not corrupt the lifecycle document");
            assert_eq!(report.command, "lifecycle");
        } else {
            assert_eq!(stdout.lines().count(), 1, "{stdout}");
        }
        assert_eq!(read_counter(&slot), "1", "{mode}: hook did not run once");
    }
}

#[test]
fn lifecycle_json_refuses_untrusted_hooks_without_running_or_reporting_success() {
    if !git_available() {
        eprintln!("skipping lifecycle hook-trust e2e: git not on PATH");
        return;
    }

    const MARKER: &str = "UNTRUSTED-HOOK-MUST-NOT-RUN";
    let outer = tempfile::tempdir().unwrap();
    let registry = make_hook_registry(
        outer.path(),
        "org.example",
        &[("0.1.0", "payload\n", HookFixture::CountPrint(MARKER))],
    );
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    user.vibe()
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .arg("--allow-hooks")
        .assert()
        .success();
    user.vibe()
        .arg("clean")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let output = user
        .vibe()
        .args(["build", "--json"])
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "no lifecycle success document");
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["ok"], false);
    assert!(
        error["error"]
            .as_str()
            .is_some_and(|message| message.contains("not trusted")),
        "{error}",
    );
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.hooked", "0.1.0"))
            .exists(),
        "refusal must happen before materialisation and hook execution",
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains(MARKER));
}

#[test]
fn reinstall_runs_post_hook_once_only_for_a_nonempty_force_diff() {
    if !git_available() {
        eprintln!("skipping hook rerun e2e: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let registry = make_hook_registry(
        outer.path(),
        "org.example",
        &[("0.1.0", "pristine\n", HookFixture::Count)],
    );
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));

    let install = user
        .vibe()
        .arg("install")
        .arg("org.example/hooked@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .arg("--allow-hooks")
        .output()
        .unwrap();

    let slot = project
        .path()
        .join(common::slot_dir("org.example.hooked", "0.1.0"));
    assert!(
        slot.join("hooks/count.sh").is_file() && slot.join("hooks/count.ps1").is_file(),
        "hook scripts must materialise before execution; stderr={}",
        String::from_utf8_lossy(&install.stderr),
    );
    assert!(
        install.status.success(),
        "initial install failed: {}",
        String::from_utf8_lossy(&install.stderr),
    );
    assert_eq!(read_counter(&slot), "1");

    // Boot-only reinstall neither resolves hook trust nor runs a hook.
    user.vibe()
        .arg("reinstall")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    assert_eq!(read_counter(&slot), "1");

    // Force with byte-identical payload has no post-install plan.
    user.vibe()
        .arg("reinstall")
        .arg(project.path())
        .arg("--force")
        .arg("--assume-yes")
        .arg("--allow-hooks")
        .assert()
        .success();
    assert_eq!(read_counter(&slot), "1");

    fs::write(slot.join("payload.txt"), "corrupted\n").unwrap();
    let output = user
        .vibe()
        .arg("--json")
        .arg("reinstall")
        .arg(project.path())
        .arg("--force")
        .arg("--assume-yes")
        .arg("--allow-hooks")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(read_counter(&slot), "2", "the repaired slot runs once");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let hooks = report["hooks"].as_array().unwrap();
    assert_eq!(hooks.len(), 2, "pre and post reports are both retained");
    assert_eq!(hooks[0]["status"], "not-declared");
    assert_eq!(hooks[1]["phase"], "post-install");
    assert_eq!(hooks[1]["status"], "ran");
}

#[test]
fn scoped_update_surfaces_a_flagged_post_hook_in_every_output_mode() {
    if !git_available() {
        eprintln!("skipping hook report e2e: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let registry = make_hook_registry(
        outer.path(),
        "org.vibevm",
        &[
            ("0.1.0", "v1\n", HookFixture::Exit(0)),
            ("0.2.0", "v2\n", HookFixture::Exit(17)),
        ],
    );
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    user.vibe()
        .arg("install")
        .arg("org.vibevm/hooked@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let manifest_path = project.path().join("vibe.toml");
    let mut manifest =
        vibe_core::manifest::Manifest::read(&manifest_path).expect("read project manifest");
    manifest.requires.packages[0] = vibe_core::PackageRef::parse("org.vibevm/hooked@*").unwrap();
    manifest.write(&manifest_path).unwrap();

    let output = user
        .vibe()
        .arg("--json")
        .arg("update")
        .arg("org.vibevm/hooked")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(output.status.success());
    let reports: Vec<serde_json::Value> = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap();
    let report = reports.last().unwrap();
    assert_eq!(report["command"], "update");
    let hooks = report["hooks"].as_array().unwrap();
    assert_eq!(hooks.len(), 2, "pre and post reports are both retained");
    assert_eq!(hooks[0]["status"], "not-declared");
    assert_eq!(hooks[1]["phase"], "post-install");
    assert_eq!(hooks[1]["status"], "post-install-failed");

    let slot = project
        .path()
        .join(common::slot_dir("org.vibevm.hooked", "0.2.0"));
    fs::write(slot.join("payload.txt"), "corrupted again\n").unwrap();
    user.vibe()
        .arg("--quiet")
        .arg("update")
        .arg("org.vibevm/hooked")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success()
        .stdout(predicates::str::contains("1 hook report flagged"));

    fs::write(slot.join("payload.txt"), "corrupted once more\n").unwrap();
    user.vibe()
        .arg("update")
        .arg("org.vibevm/hooked")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success()
        .stdout(predicates::str::contains("post-install hook failed"));
}
