//! Linux-host end-to-end coverage for installed application alternatives.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use serde_json::Value;

const SLOT: &str = "editor";
const A1: &str = "org.example/alpha@1.0.0";
const A2: &str = "org.example/alpha@2.0.0";
const B1: &str = "org.example/beta@1.0.0";

#[test]
fn global_and_session_alternatives_keep_independent_alias_projections() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    for package in [
        "org.example/alpha@=1.0.0",
        "org.example/alpha@=2.0.0",
        "org.example/beta@=1.0.0",
    ] {
        assert_success(&fixture.output(
            &user,
            &[
                "install",
                "-g",
                package,
                "--registry",
                fixture.registry_text(),
                "--platform",
                "linux",
                "--arch",
                std::env::consts::ARCH,
                "--installer",
                "local",
            ],
        ));
    }

    // The native companion is a separate cargo binary. The CLI uses the copy
    // beside itself when publishing stable PATH aliases.
    let companion = PathBuf::from(env!("CARGO_BIN_EXE_vibe-app-proxy"));
    assert!(
        companion.is_file(),
        "missing companion {}",
        companion.display()
    );
    let aliases = user.settings.join("opt/bin");
    let vim = aliases.join("vim");
    let vim1 = aliases.join("vim1");
    let vim2 = aliases.join("vim2");
    for alias in [&vim, &vim1, &vim2] {
        assert!(alias.is_file(), "missing alias {}", alias.display());
        assert!(
            alias
                .with_file_name(format!(
                    "{}.proxy.json",
                    alias.file_name().unwrap().to_string_lossy()
                ))
                .is_file()
        );
    }
    let vim_before = fs::read(&vim).unwrap();
    let vim_inode = fs::metadata(&vim).unwrap().ino();

    let initial = fixture.json(&user, &["app", "alternatives", SLOT, "--json"]);
    assert_eq!(initial["protocol"], "vibe-app-alternatives/1");
    assert_eq!(
        initial["global_default"], A1,
        "first install must stay selected"
    );
    assert_eq!(initial["installations"].as_array().unwrap().len(), 3);
    assert_alias(&vim, "A1", "first");
    assert_alias(&vim1, "A1", "first");

    let selected = fixture.json(
        &user,
        &["app", "alternatives", "select", SLOT, A2, "--json"],
    );
    assert_eq!(selected["global_default"], A2);
    assert_eq!(fs::read(&vim).unwrap(), vim_before);
    assert_eq!(
        fs::metadata(&vim).unwrap().ino(),
        vim_inode,
        "selection rewrote the proxy executable"
    );
    assert_alias(&vim, "A2", "selected");
    assert_alias(&vim2, "A2", "selected");
    let started = fixture.output(
        &user,
        &["app", "alternatives", "start", SLOT, "--", "one-step"],
    );
    assert_success(&started);
    assert!(String::from_utf8_lossy(&started.stdout).contains("INSTALLATION=A2"));
    assert!(String::from_utf8_lossy(&started.stdout).contains("ARG=one-step"));
    let masked = Command::new(&vim1).output().unwrap();
    assert_eq!(masked.status.code(), Some(127));
    assert!(!String::from_utf8_lossy(&masked.stdout).contains("A1"));
    assert!(String::from_utf8_lossy(&masked.stderr).contains("vibe-app-proxy"));

    // `use` returns full PATH values rather than mutating this test process.
    // Both environments coexist, even after the global default changed.
    let a_env = fixture.json(
        &user,
        &[
            "app",
            "alternatives",
            "use",
            SLOT,
            A1,
            "--shell",
            "posix",
            "--json",
        ],
    );
    let b_env = fixture.json(
        &user,
        &[
            "app",
            "alternatives",
            "use",
            SLOT,
            B1,
            "--shell",
            "posix",
            "--json",
        ],
    );
    assert_eq!(a_env["protocol"], "vibe-app-session-env/1");
    assert_eq!(b_env["protocol"], "vibe-app-session-env/1");
    let a_path = a_env["set"]["PATH"].as_str().unwrap();
    let b_path = b_env["set"]["PATH"].as_str().unwrap();
    assert_ne!(a_path, b_path);
    assert_path_alias(a_path, "vim", "A1", "session-a");
    assert_path_alias(a_path, "vim", "A1", "a b;$HOME$(echo untouched)");
    assert_path_alias(a_path, "vim1", "A1", "session-a");
    assert_path_alias(b_path, "vim", "B1", "session-b");
    let direct = fixture.output(
        &user,
        &[
            "app",
            "run",
            "org.example/alpha@=1.0.0",
            "main",
            "--",
            "direct",
        ],
    );
    assert_success(&direct);
    let direct_output = String::from_utf8_lossy(&direct.stdout);
    assert!(direct_output.contains("INSTALLATION=A1"), "{direct_output}");
    assert!(direct_output.contains("ARG=direct"), "{direct_output}");
    assert!(
        direct_output.contains(&format!(
            "PATH_PREFIX={}",
            a_path.split(':').next().unwrap()
        )),
        "{direct_output}"
    );
    let b_masked = path_alias_output(b_path, "vim1", "missing");
    assert_eq!(b_masked.status.code(), Some(127));
    assert!(!String::from_utf8_lossy(&b_masked.stdout).contains("A1"));
    assert_eq!(fs::read(&vim).unwrap(), vim_before);
    assert_eq!(fs::metadata(&vim).unwrap().ino(), vim_inode);

    // Two shells keep their own PATH while a third process changes the
    // global selection. No invocation consults Vibe's central index.
    let pinned_a = a_path.to_owned();
    let pinned_b = b_path.to_owned();
    let run_pinned = |path: String, expected: &'static str| {
        std::thread::spawn(move || {
            for _ in 0..6 {
                let output = path_alias_output(&path, "vim", "concurrent");
                assert_success(&output);
                assert!(
                    String::from_utf8_lossy(&output.stdout)
                        .contains(&format!("INSTALLATION={expected}"))
                );
            }
        })
    };
    let a_thread = run_pinned(pinned_a, "A1");
    let b_thread = run_pinned(pinned_b, "B1");
    for selected in [A1, B1, A2] {
        fixture.json(
            &user,
            &["app", "alternatives", "select", SLOT, selected, "--json"],
        );
    }
    a_thread.join().unwrap();
    b_thread.join().unwrap();

    // Ordinary PATH execution needs the proxy's local projection only. It
    // must keep working while Vibe's management index is unavailable.
    let index = user.settings.join("applications/index.json");
    let hidden = user.settings.join("applications/index.hidden");
    fs::rename(&index, &hidden).unwrap();
    assert_alias(&vim, "A2", "without-index");
    assert_path_alias(a_path, "vim", "A1", "without-index");
    fs::rename(&hidden, &index).unwrap();

    let blocked = fixture.output(&user, &["uninstall", "-g", "org.example/alpha@=2.0.0"]);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("select a replacement"));
    let a2_env = fixture.json(
        &user,
        &[
            "app",
            "alternatives",
            "use",
            SLOT,
            A2,
            "--shell",
            "posix",
            "--json",
        ],
    );
    let a2_path = a2_env["set"]["PATH"].as_str().unwrap();
    fixture.json(
        &user,
        &["app", "alternatives", "select", SLOT, B1, "--json"],
    );
    assert_success(&fixture.output(&user, &["uninstall", "-g", "org.example/alpha@=2.0.0"]));
    let tombstone = path_alias_output(a2_path, "vim", "after-uninstall");
    assert_eq!(tombstone.status.code(), Some(127));
    assert!(!String::from_utf8_lossy(&tombstone.stdout).contains("INSTALLATION="));
}

#[test]
fn overlapping_external_root_is_refused_before_shell_command_runs() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    assert_success(&fixture.output(
        &user,
        &[
            "install",
            "-g",
            "org.example/alpha@=1.0.0",
            "--registry",
            fixture.registry_text(),
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--installer",
            "local",
        ],
    ));
    let index_path = user.settings.join("applications/index.json");
    let prior_index = fs::read(&index_path).unwrap();
    let first_root = fixture._root.path().join("alpha-1.0.0-payload");
    let second_root = fixture._root.path().join("beta-1.0.0-payload");
    let marker = fixture._root.path().join("overlap-installer-ran");
    let beta_manifest = fixture.registry.join("org.example/beta/v1.0.0/vibe.toml");
    let original = fs::read_to_string(&beta_manifest).unwrap();
    let old_root = format!("app_root={}", toml_string(second_root.to_str().unwrap()));
    let new_root = format!("app_root={}", toml_string(first_root.to_str().unwrap()));
    let old_install = format!(
        "install={}",
        toml_string(&format!(
            "{} install {}",
            shell_word(&fixture.helper),
            shell_word(&second_root)
        ))
    );
    let new_install = format!(
        "install={}",
        toml_string(&format!("printf invoked > {}", shell_word(&marker)))
    );
    assert!(original.contains(&old_root));
    assert!(original.contains(&old_install));
    let overlapping =
        original
            .replacen(&old_root, &new_root, 1)
            .replacen(&old_install, &new_install, 1);
    fs::write(&beta_manifest, overlapping).unwrap();

    let rejected = fixture.output(
        &user,
        &[
            "install",
            "-g",
            "org.example/beta@=1.0.0",
            "--registry",
            fixture.registry_text(),
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--installer",
            "local",
        ],
    );
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("non_isolated_installation"),
        "{}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(!marker.exists(), "overlapping installer command ran");
    assert_eq!(fs::read(&index_path).unwrap(), prior_index);
    assert_alias(&user.settings.join("opt/bin/vim"), "A1", "still-ready");
}

#[test]
fn update_locks_an_existing_target_version_even_when_default_is_older() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    for package in ["org.example/alpha@=1.0.0", "org.example/alpha@=2.0.0"] {
        assert_success(&fixture.output(
            &user,
            &[
                "install",
                "-g",
                package,
                "--registry",
                fixture.registry_text(),
                "--platform",
                "linux",
                "--arch",
                std::env::consts::ARCH,
                "--installer",
                "local",
            ],
        ));
    }
    let state = fixture.json(&user, &["app", "alternatives", SLOT, "--json"]);
    assert_eq!(state["global_default"], A1);
    let session = fixture.json(
        &user,
        &[
            "app",
            "alternatives",
            "use",
            SLOT,
            A2,
            "--shell",
            "posix",
            "--json",
        ],
    );
    let view = session["set"]["PATH"]
        .as_str()
        .unwrap()
        .split(':')
        .next()
        .unwrap();
    let active = fixture._root.path().join("a2-active");
    let mut child = Command::new(Path::new(view).join("vim"))
        .arg("hold")
        .env("VIBE_TEST_ACTIVE_FILE", &active)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if active.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(active.exists(), "target version did not start");
    let attempted = fixture.output(
        &user,
        &[
            "update",
            "-g",
            "org.example/alpha",
            "--registry",
            fixture.registry_text(),
        ],
    );
    assert!(!attempted.status.success());
    assert!(
        String::from_utf8_lossy(&attempted.stderr).contains("application_in_use"),
        "{}",
        String::from_utf8_lossy(&attempted.stderr)
    );
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn external_root_overlapping_ready_archive_is_refused_before_shell_command_runs() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    assert_success(&fixture.output(
        &user,
        &[
            "install",
            "-g",
            "org.example/alpha@=1.0.0",
            "--registry",
            fixture.registry_text(),
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--installer",
            "local",
        ],
    ));

    let index_path = user.settings.join("applications/index.json");
    let mut index: Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    index["applications"][A1]["installKind"] = Value::String("archive".into());
    fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
    let prior_index = fs::read(&index_path).unwrap();

    let first_root = fixture._root.path().join("alpha-1.0.0-payload");
    let marker = fixture._root.path().join("archive-overlap-installer-ran");
    rewrite_beta_installer(
        &fixture,
        &first_root,
        &format!("printf invoked > {}", shell_word(&marker)),
    );

    let rejected = fixture.output(
        &user,
        &[
            "install",
            "-g",
            "org.example/beta@=1.0.0",
            "--registry",
            fixture.registry_text(),
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--installer",
            "local",
        ],
    );
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("non_isolated_installation"),
        "{}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(!marker.exists(), "overlapping installer command ran");
    assert_eq!(fs::read(&index_path).unwrap(), prior_index);
}

#[test]
fn external_uninstall_overlapping_ready_archive_is_refused_before_command_runs() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let marker = fixture._root.path().join("archive-overlap-uninstaller-ran");
    rewrite_alpha_uninstaller(
        &fixture,
        &format!("printf invoked > {}", shell_word(&marker)),
    );
    for package in ["org.example/alpha@=1.0.0", "org.example/beta@=1.0.0"] {
        assert_success(&fixture.output(
            &user,
            &[
                "install",
                "-g",
                package,
                "--registry",
                fixture.registry_text(),
                "--platform",
                "linux",
                "--arch",
                std::env::consts::ARCH,
                "--installer",
                "local",
            ],
        ));
    }
    fixture.json(
        &user,
        &["app", "alternatives", "select", SLOT, B1, "--json"],
    );

    let index_path = user.settings.join("applications/index.json");
    let mut index: Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    let first_root = fixture._root.path().join("alpha-1.0.0-payload");
    index["applications"][B1]["appRoot"] = Value::String(first_root.to_str().unwrap().into());
    index["applications"][B1]["installKind"] = Value::String("archive".into());
    fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
    let prior_index = fs::read(&index_path).unwrap();

    let rejected = fixture.output(&user, &["uninstall", "-g", "org.example/alpha@=1.0.0"]);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("non_isolated_installation"),
        "{}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(!marker.exists(), "overlapping uninstall command ran");
    assert_eq!(fs::read(&index_path).unwrap(), prior_index);
}

#[test]
fn external_root_through_symlinked_ancestor_is_refused_with_missing_leaf() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    assert_success(&fixture.output(
        &user,
        &[
            "install",
            "-g",
            "org.example/alpha@=1.0.0",
            "--registry",
            fixture.registry_text(),
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--installer",
            "local",
        ],
    ));

    let index_path = user.settings.join("applications/index.json");
    let prior_index = fs::read(&index_path).unwrap();
    let alias = fixture._root.path().join("linked-root");
    std::os::unix::fs::symlink(fixture._root.path(), &alias).unwrap();
    let overlapping = alias.join("alpha-1.0.0-payload").join("not-created-yet");
    let marker = fixture._root.path().join("symlink-overlap-installer-ran");
    rewrite_beta_installer(
        &fixture,
        &overlapping,
        &format!("printf invoked > {}", shell_word(&marker)),
    );

    let rejected = fixture.output(
        &user,
        &[
            "install",
            "-g",
            "org.example/beta@=1.0.0",
            "--registry",
            fixture.registry_text(),
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--installer",
            "local",
        ],
    );
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("non_isolated_installation"),
        "{}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(!marker.exists(), "overlapping installer command ran");
    assert_eq!(fs::read(&index_path).unwrap(), prior_index);
}

fn rewrite_beta_installer(fixture: &Fixture, app_root: &Path, install: &str) {
    let manifest = fixture.registry.join("org.example/beta/v1.0.0/vibe.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    let old_root = fixture._root.path().join("beta-1.0.0-payload");
    let old_root = format!("app_root={}", toml_string(old_root.to_str().unwrap()));
    let new_root = format!("app_root={}", toml_string(app_root.to_str().unwrap()));
    let old_install = format!(
        "install={}",
        toml_string(&format!(
            "{} install {}",
            shell_word(&fixture.helper),
            shell_word(&fixture._root.path().join("beta-1.0.0-payload"))
        ))
    );
    let new_install = format!("install={}", toml_string(install));
    assert!(original.contains(&old_root));
    assert!(original.contains(&old_install));
    fs::write(
        manifest,
        original
            .replacen(&old_root, &new_root, 1)
            .replacen(&old_install, &new_install, 1),
    )
    .unwrap();
}

fn rewrite_alpha_uninstaller(fixture: &Fixture, uninstall: &str) {
    let manifest = fixture.registry.join("org.example/alpha/v1.0.0/vibe.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    let root = fixture._root.path().join("alpha-1.0.0-payload");
    let old_uninstall = format!(
        "uninstall={}",
        toml_string(&format!(
            "{} uninstall {}",
            shell_word(&fixture.helper),
            shell_word(&root)
        ))
    );
    let new_uninstall = format!("uninstall={}", toml_string(uninstall));
    assert!(original.contains(&old_uninstall));
    fs::write(
        manifest,
        original.replacen(&old_uninstall, &new_uninstall, 1),
    )
    .unwrap();
}

fn assert_alias(alias: &Path, expected: &str, argument: &str) {
    let output = Command::new(alias).arg(argument).output().unwrap();
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("INSTALLATION={expected}")),
        "{stdout}"
    );
    assert!(stdout.contains(&format!("ARG={argument}")), "{stdout}");
}

fn assert_path_alias(path: &str, command: &str, expected: &str, argument: &str) {
    let output = path_alias_output(path, command, argument);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("INSTALLATION={expected}")),
        "{stdout}"
    );
    assert!(stdout.contains(&format!("ARG={argument}")), "{stdout}");
}

fn path_alias_output(path: &str, command: &str, argument: &str) -> Output {
    Command::new(command)
        .arg(argument)
        .env("PATH", path)
        .output()
        .unwrap()
}

struct Fixture {
    _root: tempfile::TempDir,
    registry: PathBuf,
    helper: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let helper = root.path().join("tiny-helper");
        let source = root.path().join("tiny-helper.rs");
        fs::write(&source, HELPER_SOURCE).unwrap();
        assert_success(
            &Command::new("rustc")
                .args([
                    "--edition=2021",
                    source.to_str().unwrap(),
                    "-o",
                    helper.to_str().unwrap(),
                ])
                .output()
                .unwrap(),
        );
        let fixture = Self {
            _root: root,
            registry,
            helper,
        };
        fixture.package("alpha", "1.0.0", "A1", &["vim", "vim1"]);
        fixture.package("alpha", "2.0.0", "A2", &["vim", "vim2"]);
        fixture.package("beta", "1.0.0", "B1", &["vim"]);
        fixture
    }

    fn package(&self, name: &str, version: &str, label: &str, aliases: &[&str]) {
        let package = self
            .registry
            .join("org.example")
            .join(name)
            .join(format!("v{version}"));
        fs::create_dir_all(&package).unwrap();
        let app_root = self._root.path().join(format!("{name}-{version}-payload"));
        let install = format!(
            "{} install {}",
            shell_word(&self.helper),
            shell_word(&app_root)
        );
        let uninstall = format!(
            "{} uninstall {}",
            shell_word(&self.helper),
            shell_word(&app_root)
        );
        let mut manifest = format!(
            "[package]\nname='{name}'\ngroup='org.example'\nkind='app'\nversion='{version}'\n\
             [application]\nid='{SLOT}'\ncommands=[]\n\
             [[installer_options]]\nid='local'\nname='Local'\nplatform='linux'\nplatform_name='Linux'\narch={arch}\n\
             [installer_options.installation]\ntype='shell'\napp_root={root}\ninstall={install}\nforce_install={install}\nuninstall={uninstall}\nforce_uninstall={uninstall}\n\
             [[installer_options.launchers]]\nid='main'\nname='Main'\nexecutable='bin/tiny-helper'\nargs=['{label}']\ndefault=true\n",
            arch = toml_string(std::env::consts::ARCH),
            root = toml_string(app_root.to_str().unwrap()),
            install = toml_string(&install),
            uninstall = toml_string(&uninstall),
        );
        for alias in aliases {
            manifest.push_str(&format!(
                "[[installer_options.public_commands]]\nname='{alias}'\nlauncher='main'\n"
            ));
        }
        fs::write(package.join("vibe.toml"), manifest).unwrap();
    }

    fn registry_text(&self) -> &str {
        self.registry.to_str().unwrap()
    }
    fn output(&self, user: &UserScratch, args: &[&str]) -> Output {
        user.vibe()
            .current_dir(self._root.path())
            .args(args)
            .output()
            .unwrap()
    }
    fn json(&self, user: &UserScratch, args: &[&str]) -> Value {
        let output = self.output(user, args);
        assert_success(&output);
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}
fn shell_word(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\"'\"'"))
}
fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const HELPER_SOURCE: &str = r#"
use std::{env, fs, path::PathBuf};
fn main() {
    let mut args = env::args_os().skip(1);
    let first = args.next().expect("mode or fixed arg");
    if first == "install" {
        let root = PathBuf::from(args.next().expect("root"));
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let target = bin.join("tiny-helper");
        fs::copy(env::current_exe().unwrap(), &target).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755)).unwrap();
    } else if first == "uninstall" {
        // External roots are intentionally retained by the fixture.
    } else {
        println!("INSTALLATION={}", first.to_string_lossy());
        println!("PATH_PREFIX={}", env::var("PATH").unwrap_or_default().split(':').next().unwrap_or(""));
        for arg in args {
            println!("ARG={}", arg.to_string_lossy());
            if arg == "hold" {
                fs::write(env::var("VIBE_TEST_ACTIVE_FILE").unwrap(), b"ready").unwrap();
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
        }
    }
}
"#;
