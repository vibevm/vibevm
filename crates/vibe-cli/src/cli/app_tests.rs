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
fn global_application_selection_flags_do_not_apply_to_project_commands() {
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

    assert!(Cli::try_parse_from(["vibe", "install", "--installer", "archive"]).is_err());
    assert!(Cli::try_parse_from(["vibe", "update", "--platform", "linux"]).is_err());
    assert!(Cli::try_parse_from(["vibe", "install", "--arch", "x86_64"]).is_err());
}

#[test]
fn global_application_force_is_accepted_for_each_operation() {
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

    assert!(Cli::try_parse_from(["vibe", "update", "--force", "flow:example"]).is_err());
    assert!(Cli::try_parse_from(["vibe", "uninstall", "--force", "flow:example"]).is_err());
}
