use super::*;
use crate::commands::application::model::{ApplicationIdentity, ManagementEntry, PackageIdentity};

fn record(settings: &Path) -> ApplicationRecord {
    let root = settings.join("payload");
    ApplicationRecord {
        application: ApplicationIdentity {
            id: "demo".into(),
            package: PackageIdentity {
                group: "org.example".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
            installer_package: PackageIdentity {
                group: "org.example".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
            commands: Vec::new(),
        },
        host_root: settings.join("host"),
        app_root: Some(root),
        install_kind: None,
        selected_option_id: None,
        selected_platform: None,
        management: ManagementEntry {
            runtime: ManagementRuntime::Native,
            entry: settings.join("host/manage"),
        },
        status: ApplicationStatus::Ready,
        provenance: None,
        launchers: Vec::new(),
        user_launchers: vec![UserLauncher {
            id: "main".into(),
            name: "Demo".into(),
            description: None,
            executable: if cfg!(windows) {
                "$app_root/bin/demo.exe".into()
            } else {
                "$app_root/bin/demo".into()
            },
            args: vec!["--flag".into(), "hello world".into()],
            working_directory: None,
            icon: None,
            terminal: true,
            default: true,
        }],
        public_commands: Vec::new(),
    }
}

#[test]
fn public_plan_keeps_authored_args_and_inherits_caller_cwd() {
    let settings = tempfile::tempdir().unwrap();
    let record = record(settings.path());
    let executable = if cfg!(windows) {
        settings.path().join("payload/bin/demo.exe")
    } else {
        settings.path().join("payload/bin/demo")
    };
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, b"test executable").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let plan = plan_public_launcher(&record, "main", settings.path()).unwrap();
    assert_eq!(plan.target, fs::canonicalize(executable).unwrap());
    assert_eq!(plan.args, ["--flag", "hello world"]);
    assert_eq!(plan.cwd, None);
}

#[test]
fn public_plan_rejects_project_and_shared_bin_targets() {
    let settings = tempfile::tempdir().unwrap();
    let mut record = record(settings.path());
    record.user_launchers[0].executable = "$project_root/bin/demo".into();
    assert!(
        plan_public_launcher(&record, "main", settings.path())
            .unwrap_err()
            .to_string()
            .contains("$project_root")
    );
    record.user_launchers[0].executable = "$app_bin/demo".into();
    assert!(
        plan_public_launcher(&record, "main", settings.path())
            .unwrap_err()
            .to_string()
            .contains("opt/bin")
    );
}
