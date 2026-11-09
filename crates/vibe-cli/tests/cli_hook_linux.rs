//! `vibe hook <shell>` run in the real shell (PROP-075 `#ACTIVATION-HOOK`):
//! the managed block, the two `cd`s that switch versions, and the cost
//! budget — zero subprocesses on an unchanged prompt and on a `cd` into a
//! directory with no candidate file, at most one `vibe` invocation on a `cd`
//! that has one.
//!
//! bash is assumed present on a Linux host. zsh and fish are run when they
//! are installed and skipped, loudly, when they are not.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::app_registry::{AppRegistry, failed, ok};

const BLOCK_BEGIN: &str = "# >>> vibe application versions (managed by vibe, do not edit) >>>";
const BLOCK_END: &str = "# <<< vibe application versions <<<";

fn runtime(r: &AppRegistry, version: &str, marker: &str) {
    let root = r.package(
        "runtime",
        version,
        "runtime",
        r#"
path_dirs = ['bin']
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='Runtime'
executable='$app_bin/rt'
kind='command'
default=true
"#,
    );
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace(
            "[application]\nid = \"runtime\"\n",
            "[application]\nid = \"runtime\"\n\n\
             [[application.version_file]]\nformat = \"plain\"\n\
             names = [\".runtime-version\"]\n",
        ),
    )
    .unwrap();
    r.script(&root, "rt", marker);
}

fn install(r: &AppRegistry, spelling: &str) {
    ok(&r.app(
        &["install", "-g"],
        spelling,
        &["--offline", "--unattended", "--json"],
    ));
}

fn project(r: &AppRegistry, name: &str, version: &str) -> PathBuf {
    let dir = r.root.path().join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!("[project]\nname = \"{name}\"\nversion = \"1.0.0\"\n"),
    )
    .unwrap();
    fs::write(dir.join(".runtime-version"), format!("{version}\n")).unwrap();
    dir
}

/// A home with two runtime versions installed and two projects pinning one
/// each, plus a plain directory with no version file.
struct Fixture {
    r: AppRegistry,
    one: PathBuf,
    two: PathBuf,
    plain: PathBuf,
    vibe_dir: PathBuf,
}

fn fixture() -> Fixture {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");
    install(&r, "org.example/runtime@1.0.0");
    install(&r, "org.example/runtime@2.0.0");
    let one = project(&r, "one", "1.0.0");
    let two = project(&r, "two", "2.0.0");
    let plain = r.root.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    let vibe_dir = vibe_binary().parent().unwrap().to_path_buf();
    Fixture {
        r,
        one,
        two,
        plain,
        vibe_dir,
    }
}

fn vibe_binary() -> PathBuf {
    // `vibe-test-support` builds the command; its program is the binary.
    let probe = vibe_test_support::vibe();
    PathBuf::from(probe.get_program())
}

/// Run `script` in `shell` with the fixture's isolated home and with the
/// `vibe` under test on PATH.
fn in_shell(f: &Fixture, shell: &str, script: &str) -> std::process::Output {
    let path = format!(
        "{}:{}",
        f.vibe_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut command = Command::new(shell);
    command
        .arg("-c")
        .arg(script)
        .current_dir(f.r.root.path())
        .env_clear()
        .env("HOME", f.r.home())
        .env("PATH", path)
        .env("VIBE_SETTINGS", f.r.settings())
        .env(
            "VIBE_REGISTRY_CACHE",
            f.r.root.path().join("registry-cache"),
        );
    command.output().unwrap()
}

/// `true` when `shell` is installed on this host.
fn available(shell: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {shell} >/dev/null 2>&1")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_block_is_written_into_the_startup_file_and_nothing_outside_it_moves() {
    let f = fixture();
    let rc = f.r.home().join(".bashrc");
    fs::write(&rc, "# mine\nexport MINE=1\n").unwrap();
    ok(&f.r.vibe(&["hook", "bash", "--yes"]));
    let text = fs::read_to_string(&rc).unwrap();
    assert!(text.starts_with("# mine\nexport MINE=1\n"), "{text}");
    assert_eq!(text.matches(BLOCK_BEGIN).count(), 1, "{text}");
    assert!(text.contains(".runtime-version"), "{text}");
    // Idempotent: a second run rewrites the same block in place.
    ok(&f.r.vibe(&["hook", "bash", "--yes"]));
    let again = fs::read_to_string(&rc).unwrap();
    assert_eq!(again, text);
    // `--print` writes the block to stdout and touches no file.
    fs::remove_file(&rc).unwrap();
    let printed = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    assert!(printed.contains("__vibe_versions"), "{printed}");
    assert!(
        !printed.contains(BLOCK_BEGIN),
        "the markers are the writer's"
    );
    assert!(!rc.exists(), "--print writes no file");
    // cmd has no prompt hook.
    let error = failed(&f.r.vibe(&["hook", "cmd", "--yes"]));
    assert!(error.contains("vibe shell"), "{error}");
    // An unattended run without --yes refuses and names the way out.
    let error = failed(&f.r.vibe(&["hook", "bash", "--unattended"]));
    assert!(error.contains("consent_required"), "{error}");
    assert!(error.contains("--print"), "{error}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn two_cds_switch_versions_in_a_real_bash() {
    let f = fixture();
    let block = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    let script = format!(
        "{block}\n\
         cd '{one}' && __vibe_versions && rt\n\
         cd '{two}' && __vibe_versions && rt\n\
         cd '{plain}' && __vibe_versions && (command -v rt >/dev/null && rt || echo gone)\n",
        one = f.one.display(),
        two = f.two.display(),
        plain = f.plain.display(),
    );
    let out = in_shell(&f, "bash", &script);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{out:?} {stdout}");
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert!(lines[0].starts_with("one"), "{stdout}");
    assert!(lines[1].starts_with("two"), "{stdout}");
    // Leaving every project retracts the selection, so the chosen payload
    // is no longer in front of PATH. The slot's own link still answers, so
    // `rt` may still be found — what must be gone is the session choice.
    assert!(lines.len() >= 3, "{stdout}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_hook_undoes_a_selection_when_the_project_is_left() {
    let f = fixture();
    let block = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    let script = format!(
        "{block}\n\
         cd '{one}' && __vibe_versions && echo in=$VIBE_APP_USE\n\
         cd '{plain}' && __vibe_versions && echo out=${{VIBE_APP_USE-unset}}\n",
        one = f.one.display(),
        plain = f.plain.display(),
    );
    let out = in_shell(&f, "bash", &script);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        stdout.contains("in=runtime=org.example/runtime@1.0.0"),
        "{stdout}"
    );
    assert!(stdout.contains("out=unset"), "{stdout}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn hook_spawns_nothing_in_an_unchanged_directory() {
    // The budget, as a command. A counting wrapper stands in for `vibe` on
    // PATH and appends one line per invocation; the hook must add none on a
    // prompt in an unchanged directory, none on a `cd` into a directory
    // with no candidate version file, and exactly one on a `cd` that has
    // one.
    let f = fixture();
    let block = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    let counter = f.r.root.path().join("calls");
    let shim_dir = f.r.root.path().join("shim");
    fs::create_dir_all(&shim_dir).unwrap();
    let shim = shim_dir.join("vibe");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\necho call >> '{}'\nexec '{}' \"$@\"\n",
            counter.display(),
            vibe_binary().display()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let count = |label: &str| {
        format!(
            "echo \"{label}=$(wc -l < '{0}' 2>/dev/null || echo 0)\"",
            counter.display()
        )
    };
    let script = format!(
        "export PATH='{shim}:'$PATH\n\
         {block}\n\
         cd '{plain}' && __vibe_versions\n\
         {base}\n\
         __vibe_versions\n\
         __vibe_versions\n\
         {unchanged}\n\
         cd '{one}' && __vibe_versions\n\
         {entered}\n",
        shim = shim_dir.display(),
        plain = f.plain.display(),
        one = f.one.display(),
        base = count("base"),
        unchanged = count("unchanged"),
        entered = count("entered"),
    );
    let out = in_shell(&f, "bash", &script);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{out:?} {stdout}");
    let read = |label: &str| -> i64 {
        stdout
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{label}=")))
            .unwrap_or_else(|| panic!("no `{label}` line in {stdout}"))
            .trim()
            .parse()
            .unwrap()
    };
    // A `cd` into a directory with no candidate version file anywhere up the
    // tree calls nothing.
    assert_eq!(read("base"), 0, "{stdout}");
    // Two prompts in an unchanged directory call nothing.
    assert_eq!(read("unchanged"), 0, "{stdout}");
    // A `cd` into a directory that has one calls `vibe` exactly once.
    assert_eq!(read("entered"), 1, "{stdout}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_hook_never_fails_the_prompt() {
    let f = fixture();
    let block = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    // A version that is not installed, an unreadable line, and a broken
    // index: in every case the `cd` succeeds and the shell is left as it
    // was, with one line on stderr.
    let broken = project(&f.r, "broken", "9.9.9");
    fs::write(broken.join(".runtime-version"), "not a version!\n").unwrap();
    let script = format!(
        "set -e\n{block}\n\
         cd '{broken}' && __vibe_versions && echo survived=1\n",
        broken = broken.display(),
    );
    let out = in_shell(&f, "bash", &script);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{out:?} {stdout}");
    assert!(stdout.contains("survived=1"), "{stdout}");

    // A corrupt index: the hook still must not fail the prompt.
    fs::write(f.r.settings().join("applications/index.json"), b"{").unwrap();
    let out = in_shell(&f, "bash", &script);
    assert!(
        out.status.success(),
        "a broken index must not fail a cd: {out:?}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_zsh_block_runs_in_a_real_zsh_when_one_is_installed() {
    if !available("zsh") {
        eprintln!("skipped: zsh is not installed on this host");
        return;
    }
    let f = fixture();
    let block = ok(&f.r.vibe(&["hook", "zsh", "--print"]));
    let script = format!(
        "{block}\n\
         cd '{one}' && __vibe_versions && rt\n\
         cd '{two}' && __vibe_versions && rt\n",
        one = f.one.display(),
        two = f.two.display(),
    );
    let out = in_shell(&f, "zsh", &script);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{out:?} {stdout}");
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert!(lines[0].starts_with("one"), "{stdout}");
    assert!(lines[1].starts_with("two"), "{stdout}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_fish_block_runs_in_a_real_fish_when_one_is_installed() {
    if !available("fish") {
        eprintln!("skipped: fish is not installed on this host");
        return;
    }
    let f = fixture();
    let block = ok(&f.r.vibe(&["hook", "fish", "--print"]));
    let script = format!(
        "{block}\n\
         cd '{one}'; __vibe_versions; rt\n\
         cd '{two}'; __vibe_versions; rt\n",
        one = f.one.display(),
        two = f.two.display(),
    );
    let out = in_shell(&f, "fish", &script);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{out:?} {stdout}");
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert!(lines[0].starts_with("one"), "{stdout}");
    assert!(lines[1].starts_with("two"), "{stdout}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_name_list_follows_what_the_installed_packages_declare() {
    let f = fixture();
    let before = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    assert!(before.contains(".runtime-version"), "{before}");
    // Uninstall every provider: the declared set is empty and the block's
    // walk tests nothing.
    ok(&f.r.vibe(&[
        "uninstall",
        "-g",
        "org.example/runtime@1.0.0",
        "--unattended",
    ]));
    ok(&f.r.vibe(&[
        "uninstall",
        "-g",
        "org.example/runtime@2.0.0",
        "--unattended",
    ]));
    let after = ok(&f.r.vibe(&["hook", "bash", "--print"]));
    assert!(!after.contains(".runtime-version"), "{after}");
    assert!(
        !f.r.settings()
            .join("applications/version-files.json")
            .exists(),
        "an empty declared set keeps no document"
    );
}

/// The startup file the hook writes for each shell exists where that shell
/// reads it.
#[test]
fn each_shell_gets_its_own_startup_file() {
    let f = fixture();
    for (shell, relative) in [
        ("bash", ".bashrc"),
        ("zsh", ".zshrc"),
        ("fish", ".config/fish/config.fish"),
        (
            "pwsh",
            ".config/powershell/Microsoft.PowerShell_profile.ps1",
        ),
    ] {
        ok(&f.r.vibe(&["hook", shell, "--yes"]));
        let path: &Path = &f.r.home().join(relative);
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{shell}: {} {error}", path.display()));
        assert!(text.contains(BLOCK_BEGIN), "{shell}: {text}");
        assert!(text.contains(BLOCK_END), "{shell}: {text}");
    }
}
