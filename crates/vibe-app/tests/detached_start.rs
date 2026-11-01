//! `vibe_app::start_launcher` (PROP-059 `#launcher-catalog`): the detached
//! start answers a process id, a command line and a log, and counts the
//! start before the child exists.
//!
//! It is an integration test and not a unit test of `catalog/start.rs` for
//! one mechanical reason: it spawns a child, and a child briefly inherits
//! the open mutation-lock files of every other test running in the same
//! process, which makes a sibling unit test that takes the home's lock fail
//! as `locked`. Its own test binary has no such siblings.

#![cfg(all(target_os = "linux", not(miri)))]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use vibe_app::{
    AuthoredCommands, Index, Installation, Location, Method, PackageCoordinate, SettingsRoot,
    StartRequest, Storage, Usage, create_library, start_launcher,
};
use vibe_core::manifest::{LauncherDecl, LauncherKind};

const KEY: &str = "org.example/editor@1.0.0";

/// A home whose index records one installation with a `gui` launcher `edit`,
/// its payload in the default library and a marker so the library is
/// available.
fn home(dir: &Path) -> SettingsRoot {
    let settings = SettingsRoot::new(dir);
    let package = PackageCoordinate {
        group: "org.example".into(),
        name: "editor".into(),
        version: "1.0.0".into(),
    };
    let mut index = Index::default();
    index.installations.insert(
        package.key(),
        Installation {
            slot: "editor".into(),
            package: package.clone(),
            option: "main".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            method: Method::Zip,
            location: Location::Owned {
                library: "default".into(),
                directory: "apps/editor/org.example.editor@1.0.0".into(),
            },
            portable: true,
            launchers: vec![LauncherDecl {
                id: "edit".into(),
                name: "Editor".into(),
                executable: "$app_bin/edit".into(),
                kind: Some(LauncherKind::Gui),
                default: true,
                ..LauncherDecl::default()
            }],
            public_commands: Vec::new(),
            commands: AuthoredCommands::default(),
            features: Vec::new(),
            parameters: Default::default(),
            license: None,
            icon: None,
            menu: Default::default(),
            steps: Vec::new(),
            lifecycle: Default::default(),
        },
    );
    index.slots.insert("editor".into(), Some(package.key()));
    index
        .save(
            &settings,
            &vibe_app::HomeEvent::new(vibe_app::EventKind::Installed),
        )
        .unwrap();

    let root = settings
        .default_library_path()
        .join("apps/editor/org.example.editor@1.0.0");
    fs::create_dir_all(root.join("bin")).unwrap();
    let program = root.join("bin/edit");
    fs::write(&program, "#!/bin/sh\necho \"STARTED $*\"\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    create_library(&settings.default_library_path(), "default").unwrap();
    settings
}

#[test]
fn a_detached_start_answers_a_pid_a_log_and_one_counted_start() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());
    let index = Index::load(&settings).unwrap();
    let storage = Storage::load(&settings).unwrap();
    let installation = &index.installations[KEY];
    let extra = [OsString::from("a.txt")];
    let answer = start_launcher(
        installation,
        &storage,
        &StartRequest {
            settings: &settings,
            launcher_id: "edit",
            extra_args: &extra,
            project_root: None,
            processes: None,
        },
    )
    .unwrap();

    assert_eq!(answer.protocol, "vibe-app-start/1");
    assert_eq!(answer.key, "org.example/editor@1.0.0#edit");
    assert!(answer.pid > 0);
    assert!(answer.command.ends_with("/bin/edit"), "{}", answer.command);
    assert_eq!(answer.args, ["a.txt"]);
    assert!(
        answer
            .log
            .contains("cache/app-runs/org.example.editor@1.0.0/start-edit-"),
        "{}",
        answer.log
    );
    assert!(Path::new(&answer.log).is_file(), "{}", answer.log);

    let usage = Usage::load(&settings).unwrap();
    assert_eq!(usage.of("org.example/editor@1.0.0#edit").starts, 1);
    assert!(
        usage
            .of("org.example/editor@1.0.0#edit")
            .last_started
            .is_some()
    );

    // The child writes its own output into that log; it exits at once, so a
    // short wait is enough and a slow machine only makes the check lenient.
    for _ in 0..50 {
        if fs::read_to_string(&answer.log)
            .unwrap()
            .contains("STARTED a.txt")
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("the log never caught the child's output: {}", answer.log);
}

#[test]
fn an_unknown_launcher_is_refused_before_anything_starts() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());
    let index = Index::load(&settings).unwrap();
    let storage = Storage::load(&settings).unwrap();
    let error = start_launcher(
        &index.installations[KEY],
        &storage,
        &StartRequest {
            settings: &settings,
            launcher_id: "ghost",
            extra_args: &[],
            project_root: None,
            processes: None,
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), "unknown_launcher");
    assert!(Usage::load(&settings).unwrap().launchers.is_empty());
}
