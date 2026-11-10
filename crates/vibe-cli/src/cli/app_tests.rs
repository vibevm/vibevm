//! Application CLI grammar checks.
use clap::Parser;

use super::{AppCommand, Cli, Command};

#[test]
fn app_run_preserves_literal_extra_arguments() {
    let cli = Cli::try_parse_from([
        "vibe",
        "app",
        "run",
        "app:org.example/tool",
        "console",
        "--",
        "two words",
        "$HOME",
        "--flag",
        "x;y",
    ])
    .unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    let AppCommand::Run {
        package,
        launcher_id,
        args,
    } = app.command
    else {
        panic!("expected app run");
    };
    assert_eq!(package, "app:org.example/tool");
    assert_eq!(launcher_id, "console");
    assert_eq!(args, ["two words", "$HOME", "--flag", "x;y"]);
}

#[test]
fn app_start_keeps_explicit_launcher_and_project_path() {
    let cli = Cli::try_parse_from([
        "vibe",
        "app",
        "start",
        "app:org.example/tool",
        "--launcher",
        "editor",
        "--path",
        "/tmp/a project",
        "--",
        "literal $project_root",
    ])
    .unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert_eq!(app.path.unwrap().to_str(), Some("/tmp/a project"));
    let AppCommand::Start { launcher, args, .. } = app.command else {
        panic!("expected app start");
    };
    assert_eq!(launcher.as_deref(), Some("editor"));
    assert_eq!(args, ["literal $project_root"]);
}

#[test]
fn app_installers_accepts_registry_and_global_json() {
    let cli = Cli::try_parse_from([
        "vibe",
        "app",
        "installers",
        "app:org.example/tool",
        "--registry",
        "/tmp/registry",
        "--json",
    ])
    .unwrap();
    assert!(cli.json);
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    let AppCommand::Installers { package, registry } = app.command else {
        panic!("expected app installers");
    };
    assert_eq!(package, "app:org.example/tool");
    assert_eq!(registry.unwrap().to_str(), Some("/tmp/registry"));
}

#[test]
fn application_selection_flags_parse_with_and_without_global() {
    let install = Cli::try_parse_from([
        "vibe",
        "install",
        "-g",
        "app:org.example/tool",
        "--platform",
        "ubuntu",
        "--arch",
        "x86_64",
        "--installer",
        "archive",
    ])
    .unwrap();
    let Command::Install(install) = install.command else {
        panic!("expected install command");
    };
    assert_eq!(install.platform.as_deref(), Some("ubuntu"));
    assert_eq!(install.arch.as_deref(), Some("x86_64"));
    assert_eq!(install.installer.as_deref(), Some("archive"));

    let update = Cli::try_parse_from([
        "vibe",
        "update",
        "-g",
        "app:org.example/tool",
        "--platform",
        "windows",
        "--arch",
        "arm64",
        "--installer",
        "source",
    ])
    .unwrap();
    let Command::Update(update) = update.command else {
        panic!("expected update command");
    };
    assert_eq!(update.platform.as_deref(), Some("windows"));
    assert_eq!(update.arch.as_deref(), Some("arm64"));
    assert_eq!(update.installer.as_deref(), Some("source"));

    // Project-home materialisation takes the same selection flags
    // (spec://org.vibevm.core/vibevm/common/PROP-059#project-scope).
    let project = Cli::try_parse_from(["vibe", "install", "--installer", "archive"]).unwrap();
    let Command::Install(project) = project.command else {
        panic!("expected install command");
    };
    assert!(!project.global);
    assert_eq!(project.installer.as_deref(), Some("archive"));
    assert!(Cli::try_parse_from(["vibe", "update", "--platform", "linux"]).is_ok());
    assert!(Cli::try_parse_from(["vibe", "install", "--arch", "x86_64"]).is_ok());
}

#[test]
fn application_force_is_accepted_for_each_operation() {
    let install =
        Cli::try_parse_from(["vibe", "install", "-g", "app:org.example/tool", "--force"]).unwrap();
    let Command::Install(install) = install.command else {
        panic!("expected install command");
    };
    assert!(install.force);

    let update =
        Cli::try_parse_from(["vibe", "update", "-g", "app:org.example/tool", "--force"]).unwrap();
    let Command::Update(update) = update.command else {
        panic!("expected update command");
    };
    assert!(update.force);

    let uninstall =
        Cli::try_parse_from(["vibe", "uninstall", "-g", "app:org.example/tool", "--force"])
            .unwrap();
    let Command::Uninstall(uninstall) = uninstall.command else {
        panic!("expected uninstall command");
    };
    assert!(uninstall.force);

    assert!(Cli::try_parse_from(["vibe", "update", "--force", "flow:example"]).is_ok());
    assert!(Cli::try_parse_from(["vibe", "uninstall", "--force", "flow:example"]).is_ok());
}

#[test]
fn app_list_use_alternatives_and_shell_parse() {
    let cli = Cli::try_parse_from(["vibe", "app", "list", "--json"]).unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert!(matches!(app.command, AppCommand::List));
    assert!(!app.global);
    let cli = Cli::try_parse_from(["vibe", "app", "list", "-g"]).unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert!(app.global);
    let cli = Cli::try_parse_from(["vibe", "app", "alternatives", "editor"]).unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert!(matches!(app.command, AppCommand::Alternatives { slot } if slot == "editor"));
    let cli =
        Cli::try_parse_from(["vibe", "app", "use", "editor", "org.example/vim@9.1.0"]).unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert!(
        matches!(app.command, AppCommand::Use { slot, key, none, default, .. }
            if slot == "editor"
                && key.as_deref() == Some("org.example/vim@9.1.0")
                && !none
                && !default)
    );
    // The slot off switch and its counterpart (PROP-059 `#slot-off`); both
    // together is a usage error the grammar itself refuses.
    let cli = Cli::try_parse_from(["vibe", "app", "use", "node", "--none"]).unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert!(
        matches!(app.command, AppCommand::Use { slot, key, none, default, .. }
        if slot == "node" && key.is_none() && none && !default)
    );
    let cli = Cli::try_parse_from(["vibe", "app", "use", "node", "--default"]).unwrap();
    let Command::App(app) = cli.command else {
        panic!("expected app command");
    };
    assert!(
        matches!(app.command, AppCommand::Use { slot, key, none, default, .. }
        if slot == "node" && key.is_none() && !none && default)
    );
    assert!(Cli::try_parse_from(["vibe", "app", "use", "node", "--none", "--default"]).is_err());
    // `--write-version-file[=PATH]` (PROP-075 `#VERSION-FILE-WRITE`): the
    // bare flag, the flag with a path, and absent.
    let parse = |args: &[&str]| {
        let Command::App(app) = Cli::try_parse_from(args).unwrap().command else {
            panic!("expected app command");
        };
        match app.command {
            AppCommand::Use { version_file, .. } => version_file,
            other => panic!("expected app use, got {other:?}"),
        }
    };
    let bare = parse(&["vibe", "app", "use", "node", "k", "--write-version-file"]);
    assert!(bare.asked() && bare.path().is_none());
    let named = parse(&[
        "vibe",
        "app",
        "use",
        "node",
        "k",
        "--write-version-file=.sdkmanrc",
    ]);
    assert_eq!(named.path(), Some(std::path::Path::new(".sdkmanrc")));
    assert!(!parse(&["vibe", "app", "use", "node", "k"]).asked());
    // The value needs `=`, so the flag can never swallow a positional: a
    // space-separated word stays the installation key, which the slot then
    // refuses by name rather than a file being written nobody asked for.
    let Command::App(app) = Cli::try_parse_from([
        "vibe",
        "app",
        "use",
        "node",
        "--write-version-file",
        ".sdkmanrc",
    ])
    .unwrap()
    .command
    else {
        panic!("expected app command");
    };
    assert!(matches!(
        app.command,
        AppCommand::Use { key, version_file, .. }
            if key.as_deref() == Some(".sdkmanrc") && version_file.path().is_none()
    ));
    // The same flag on `vibe app env`, which changes the shell and the file
    // and no default (PROP-075 `#VERSION-FILE-WRITE`).
    let env = |args: &[&str]| {
        let Command::App(app) = Cli::try_parse_from(args).unwrap().command else {
            panic!("expected app command");
        };
        match app.command {
            AppCommand::Env(env) => env,
            other => panic!("expected app env, got {other:?}"),
        }
    };
    let saved = env(&[
        "vibe",
        "app",
        "env",
        "--use",
        "node=22.9.0",
        "--write-version-file",
    ]);
    assert!(saved.version_file.asked() && saved.version_file.path().is_none());
    assert_eq!(saved.uses, ["node=22.9.0"]);
    assert!(
        !env(&["vibe", "app", "env", "--project"])
            .version_file
            .asked()
    );
    assert_eq!(
        env(&[
            "vibe",
            "app",
            "env",
            "--use",
            "java=21",
            "--write-version-file=.sdkmanrc",
        ])
        .version_file
        .path(),
        Some(std::path::Path::new(".sdkmanrc"))
    );
    // `vibe app versions <target> [PATTERN]` — the positional filter
    // (PROP-075 `#VERSIONS-LIST-OUTPUT`).
    let versions = |args: &[&str]| {
        let Command::App(app) = Cli::try_parse_from(args).unwrap().command else {
            panic!("expected app command");
        };
        match app.command {
            AppCommand::Versions(versions) => versions,
            other => panic!("expected app versions, got {other:?}"),
        }
    };
    let narrowed = versions(&["vibe", "app", "versions", "node", "22.9"]);
    assert_eq!(narrowed.target, "node");
    assert_eq!(narrowed.pattern.as_deref(), Some("22.9"));
    assert!(
        versions(&["vibe", "app", "versions", "node"])
            .pattern
            .is_none()
    );
    // The pattern survives a flag between it and the target, and a third
    // positional is refused rather than silently dropped.
    let after = versions(&["vibe", "app", "versions", "node", "--installed", "22"]);
    assert!(after.installed && after.pattern.as_deref() == Some("22"));
    assert!(Cli::try_parse_from(["vibe", "app", "versions", "node", "22", "23"]).is_err());

    let cli = Cli::try_parse_from(["vibe", "shell", "-g", "--use", "a=x", "--use", "b=y"]).unwrap();
    let Command::Shell(shell) = cli.command else {
        panic!("expected shell command");
    };
    assert!(shell.global);
    assert_eq!(shell.uses, ["a=x", "b=y"]);
}

#[test]
fn storage_subcommands_parse() {
    use super::StorageCommand;

    let storage = |args: &[&str]| {
        let cli = Cli::try_parse_from(args).unwrap();
        let Command::Storage(storage) = cli.command else {
            panic!("expected storage command");
        };
        storage.command
    };
    assert!(matches!(
        storage(&["vibe", "storage", "list"]),
        StorageCommand::List
    ));
    assert!(matches!(
        storage(&["vibe", "storage", "add", "/media/usb", "--id", "usb"]),
        StorageCommand::Add { path, id } if path.to_str() == Some("/media/usb") && id.as_deref() == Some("usb")
    ));
    assert!(matches!(
        storage(&["vibe", "storage", "attach", "/media/usb"]),
        StorageCommand::Attach { .. }
    ));
    assert!(matches!(
        storage(&["vibe", "storage", "detach", "usb"]),
        StorageCommand::Detach { id } if id == "usb"
    ));
    assert!(matches!(
        storage(&["vibe", "storage", "move", "org.example/tool@1.0.0", "--to", "default"]),
        StorageCommand::Move { key, to } if key == "org.example/tool@1.0.0" && to == "default"
    ));
    assert!(matches!(
        storage(&["vibe", "storage", "gc", "--delete"]),
        StorageCommand::Gc { delete: true }
    ));
    assert!(Cli::try_parse_from(["vibe", "storage", "move", "org.example/tool"]).is_err());
}
