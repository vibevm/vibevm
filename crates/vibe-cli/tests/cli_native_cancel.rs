//! NATIVE-STOP-LAW end to end: a stop acknowledged while a native slot hook
//! blocks inside its synchronous ABI call ends the install at once, reaps the
//! operation's helper process and restores the project like any other stop.
#![cfg(target_os = "linux")]
mod common;

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::UserScratch;

fn fixture_library() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let profile = deps.parent().unwrap();
    let exact = format!(
        "{}vibe_native_loader_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    [profile, deps]
        .into_iter()
        .flat_map(|directory| fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .unwrap_or_else(|| panic!("fixture cdylib `{exact}`"))
}

/// One published package whose own pre-install slot hook sleeps natively.
fn registry(root: &Path) -> PathBuf {
    let package = root.join("registry/org.example/natived/v0.1.0");
    fs::create_dir_all(package.join("prebuilt")).unwrap();
    let library = fixture_library();
    let name = library.file_name().unwrap().to_string_lossy().into_owned();
    fs::copy(&library, package.join("prebuilt").join(&name)).unwrap();
    let platform = match std::env::consts::ARCH {
        "x86_64" => "linux-x86_64",
        other => panic!("unsupported test architecture {other}"),
    };
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.example\"\nname = \"natived\"\nkind = \"tool\"\nversion = \"0.1.0\"\n\n\
             [[extension]]\nid = \"slot-pre-sleep\"\npoint = \"slot:pre-install\"\n\
             handler = {{ kind = \"native\", prebuilt = {{ {platform} = \"prebuilt/{name}\" }} }}\n"
        ),
    )
    .unwrap();
    root.join("registry")
}

/// Live helper processes serving sessions of the `vibe` process `parent`.
fn helpers_of(parent: u32) -> Vec<u32> {
    let marker = format!("vibe-native-helper-{parent}-");
    fs::read_dir("/proc")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|bytes| {
                let line = String::from_utf8_lossy(&bytes);
                line.contains(vibe_lifecycle::native::NATIVE_HELPER_MODE_ARG)
                    && line.contains(&marker)
            })
        })
        .collect()
}

#[test]
fn stop_during_a_blocking_native_slot_hook_ends_the_install_at_once() {
    let outer = tempfile::tempdir().unwrap();
    let registry = registry(outer.path());
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let before = fs::read(project.path().join("vibe.toml")).unwrap();
    let stop = outer.path().join("stop");
    let mut child = Command::new(assert_cmd::cargo::cargo_bin("vibe"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env(vibe_core::settings::SETTINGS_DIR_ENV, &user.settings)
        .env(vibe_test_support::REGISTRY_CACHE_ENV, &user.cache)
        .env(vibe_test_support::SEARCH_CACHE_ENV, &user.search_cache)
        .args([
            "install",
            "org.example/natived@=0.1.0",
            "--json",
            "--assume-yes",
        ])
        .arg("--stop-file")
        .arg(&stop)
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    let helper = loop {
        if let Some(pid) = helpers_of(child.id()).first() {
            break *pid;
        }
        if let Some(status) = child.try_wait().unwrap() {
            let mut stdout = String::new();
            let mut stderr = String::new();
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(&mut stdout)
                .unwrap();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            panic!("install ended ({status}) before its native hook ran\n{stdout}\n{stderr}");
        }
        assert!(Instant::now() < deadline, "no native helper started");
        std::thread::sleep(Duration::from_millis(20));
    };
    // The helper is now loading or already inside the 30 s blocking call.
    std::thread::sleep(Duration::from_millis(500));
    let parent = child.id();
    let requested = Instant::now();
    fs::write(&stop, "").unwrap();
    let output = child.wait_with_output().unwrap();
    let elapsed = requested.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(130), "{stdout}\n{stderr}");
    assert!(
        elapsed < Duration::from_secs(2),
        "stopped after {elapsed:?}"
    );
    let docs = common::hook_slot::documents(&output.stdout);
    let report = docs.last().unwrap();
    assert_eq!(report["outcome"], "cancelled");
    // Plugin effects are unknown: the restored files never claim pristine.
    assert_eq!(report["publication"], "recovery-required", "{report}");
    assert_eq!(report["external_effects"]["native"], "unknown", "{report}");
    assert!(
        report["error"]
            .as_str()
            .is_some_and(|error| error.contains("native plugin effects are unknown")),
        "{report}"
    );
    assert!(
        !Path::new(&format!("/proc/{helper}")).exists(),
        "the helper process was reaped"
    );
    assert!(
        helpers_of(parent).is_empty(),
        "no helper of this operation is left"
    );
    assert_eq!(fs::read(project.path().join("vibe.toml")).unwrap(), before);
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.example.natived", "0.1.0"))
            .exists(),
        "the cancelled slot is not published"
    );
    assert!(stop.exists(), "vibe never removes the caller's stop file");
}
