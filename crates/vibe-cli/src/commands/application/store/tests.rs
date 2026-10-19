//! Focused application index storage tests.

use super::*;
use crate::commands::application::model::{
    ApplicationIdentity, ApplicationLauncherOwnership, ApplicationRecord, ApplicationStatus,
    ManagementEntry, ManagementRuntime, PackageIdentity, PublicCommand, UserLauncher,
};

fn record(settings: &Path) -> ApplicationRecord {
    let host_root = settings.join("opt/apps/demo/org.demo/demo/1.0.0");
    ApplicationRecord {
        application: ApplicationIdentity {
            id: "demo".into(),
            package: PackageIdentity {
                group: "org.demo".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
            installer_package: PackageIdentity {
                group: "org.demo".into(),
                name: "installer".into(),
                version: "1.0.0".into(),
            },
            commands: Vec::new(),
        },
        host_root: host_root.clone(),
        app_root: Some(settings.join("external payload")),
        install_kind: None,
        selected_option_id: None,
        selected_platform: None,
        management: ManagementEntry {
            runtime: ManagementRuntime::Native,
            entry: host_root.join("manage"),
        },
        status: ApplicationStatus::Ready,
        provenance: None,
        launchers: Vec::new(),
        user_launchers: vec![UserLauncher {
            id: "main".into(),
            name: "Example".into(),
            description: None,
            executable: "$app_root/bin/example".into(),
            args: Vec::new(),
            working_directory: None,
            icon: None,
            terminal: false,
            default: true,
        }],
        public_commands: Vec::new(),
    }
}

fn index(settings: &Path) -> ApplicationIndex {
    let mut index = ApplicationIndex::default();
    index
        .applications
        .insert("org.demo/demo@1.0.0".into(), record(settings));
    index.alternatives.insert(
        "demo".into(),
        super::super::model::AlternativeRecord {
            global_default: Some("org.demo/demo@1.0.0".into()),
            reserved_commands: Vec::new(),
            runners: Vec::new(),
        },
    );
    index
}

#[test]
fn post_publication_failure_preserves_new_index_for_recovery() {
    let settings = tempfile::tempdir().unwrap();
    let store = ApplicationStore::open(settings.path()).unwrap();
    let mut state = index(settings.path());
    state.revision = "revision-one".into();
    store.save(&state).unwrap();
    state.revision = "revision-two".into();

    vibe_safefs::fail_after_publish(Some("index.json"));
    let result = store.save(&state);
    vibe_safefs::fail_after_publish(None);
    let error = result.unwrap_err();
    let uncertain = error.downcast_ref::<UncertainIndexCommit>().unwrap();
    assert_eq!(uncertain.intended_revision, "revision-two");
    assert_eq!(uncertain.observed_revision.as_deref(), Some("revision-two"));
    assert_eq!(store.load().unwrap().revision, "revision-two");
}

#[test]
fn validates_launcher_only_index_and_keeps_host_ownership() {
    let settings = std::env::temp_dir().join("application-index-validation");
    let mut index = index(&settings);
    validate_index(&index, &settings).unwrap();
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .host_root = settings.join("external payload");
    assert!(validate_index(&index, &settings).is_err());
}

#[test]
fn rejects_multiple_default_user_launchers_and_nonabsolute_payload() {
    let settings = std::env::temp_dir().join("application-index-validation");
    let mut index = index(&settings);
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .app_root = Some(PathBuf::from("relative"));
    assert!(validate_index(&index, &settings).is_err());
    let record = index.applications.get_mut("org.demo/demo@1.0.0").unwrap();
    record.app_root = Some(settings.join("external payload"));
    let mut second = record.user_launchers[0].clone();
    second.id = "editor".into();
    record.user_launchers.push(second);
    assert!(validate_index(&index, &settings).is_err());
}

#[test]
fn installer_option_selection_requires_a_complete_portable_pair() {
    let settings = std::env::temp_dir().join("application-index-validation");
    let mut index = index(&settings);
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .selected_option_id = Some("homebrew".into());
    assert!(validate_index(&index, &settings).is_err());
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .selected_platform = Some("macos/aarch64".into());
    validate_index(&index, &settings).unwrap();
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .selected_platform = Some("macos".into());
    validate_index(&index, &settings).unwrap();
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .selected_platform = Some("macos/aarch64/extra".into());
    assert!(validate_index(&index, &settings).is_err());
}

#[test]
fn snapshot_reads_while_mutation_lock_is_held() {
    let settings = std::env::temp_dir().join(format!(
        "application-index-snapshot-{}-{}",
        std::process::id(),
        monotonic_nonce()
    ));
    let store = ApplicationStore::open(&settings).unwrap();
    let mut index = index(&settings);
    index.revision = "snapshot-test-1".into();
    store.save(&index).unwrap();
    let snapshot = ApplicationStore::load_snapshot(&settings).unwrap();
    assert_eq!(snapshot, index);
    index.alternatives.get_mut("demo").unwrap().global_default =
        Some("missing/provider@1.0.0".into());
    assert!(store.save(&index).is_err());
    index.applications.clear();
    index.revision = "snapshot-test-2".into();
    index.alternatives.get_mut("demo").unwrap().global_default = None;
    index
        .alternatives
        .get_mut("demo")
        .unwrap()
        .reserved_commands
        .push("demo".into());
    index
        .alternatives
        .get_mut("demo")
        .unwrap()
        .runners
        .push(ApplicationLauncherOwnership {
            destination: settings.join("opt/bin/demo"),
            sha256: "a".repeat(64),
        });
    store.save(&index).unwrap();
    assert_eq!(ApplicationStore::load_snapshot(&settings).unwrap(), index);
    index.revision = "snapshot-test-3".into();
    index.alternatives.clear();
    assert!(store.save(&index).is_err());
    drop(store);
    fs::remove_dir_all(settings).unwrap();
}

#[test]
fn one_slot_can_have_multiple_providers_but_only_one_default() {
    let settings = std::env::temp_dir().join("application-index-validation");
    let mut index = index(&settings);
    let mut newer = record(&settings);
    newer.application.package.version = "2.0.0".into();
    newer.host_root = installation_host_root(&settings, "demo", &newer.application.package);
    newer.management.entry = newer.host_root.join("manage");
    index
        .applications
        .insert("org.demo/demo@2.0.0".into(), newer);
    let mut second = record(&settings);
    second.application.package.group = "com.acme".into();
    second.application.package.name = "superdemo".into();
    second.host_root = installation_host_root(&settings, "demo", &second.application.package);
    second.management.entry = second.host_root.join("manage");
    index
        .applications
        .insert("com.acme/superdemo@1.0.0".into(), second);
    validate_index(&index, &settings).unwrap();
    index.alternatives.get_mut("demo").unwrap().global_default = Some("org.demo/demo@2.0.0".into());
    validate_index(&index, &settings).unwrap();
    index.alternatives.get_mut("demo").unwrap().global_default =
        Some("com.acme/superdemo@1.0.0".into());
    validate_index(&index, &settings).unwrap();
    index.alternatives.get_mut("demo").unwrap().global_default =
        Some("missing/provider@1.0.0".into());
    assert!(validate_index(&index, &settings).is_err());
}

#[test]
fn runners_cover_commands_from_all_ready_providers_in_slot() {
    let settings = std::env::temp_dir().join("application-index-validation");
    let mut index = index(&settings);
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .public_commands
        .push(PublicCommand {
            name: "demo".into(),
            launcher: "main".into(),
        });
    let mut second = record(&settings);
    second.application.package.version = "2.0.0".into();
    second.host_root = installation_host_root(&settings, "demo", &second.application.package);
    second.management.entry = second.host_root.join("manage");
    second.public_commands.push(PublicCommand {
        name: "other".into(),
        launcher: "main".into(),
    });
    index
        .applications
        .insert("org.demo/demo@2.0.0".into(), second);
    index
        .alternatives
        .get_mut("demo")
        .unwrap()
        .reserved_commands = vec!["demo".into(), "other".into()];
    index.alternatives.get_mut("demo").unwrap().runners = ["demo", "other"]
        .into_iter()
        .map(|name| ApplicationLauncherOwnership {
            destination: settings.join("opt/bin").join(name),
            sha256: "a".repeat(64),
        })
        .collect();
    validate_index(&index, &settings).unwrap();

    let first = index.applications.get_mut("org.demo/demo@1.0.0").unwrap();
    first.application.commands.push("demo".into());
    first.launchers.push(ApplicationLauncherOwnership {
        destination: settings.join("opt/bin/demo"),
        sha256: "a".repeat(64),
    });
    assert!(validate_index(&index, &settings).is_err());
}

#[test]
fn ready_providers_in_different_slots_cannot_claim_same_command() {
    let settings = std::env::temp_dir().join("application-index-validation");
    let mut index = index(&settings);
    index
        .applications
        .get_mut("org.demo/demo@1.0.0")
        .unwrap()
        .public_commands
        .push(PublicCommand {
            name: "demo".into(),
            launcher: "main".into(),
        });
    let mut second = record(&settings);
    second.application.id = "other".into();
    second.application.package.group = "com.acme".into();
    second.host_root = installation_host_root(&settings, "other", &second.application.package);
    second.management.entry = second.host_root.join("manage");
    second.public_commands.push(PublicCommand {
        name: "demo".into(),
        launcher: "main".into(),
    });
    index
        .applications
        .insert("com.acme/demo@1.0.0".into(), second);
    index
        .alternatives
        .get_mut("demo")
        .unwrap()
        .reserved_commands
        .push("demo".into());
    index.alternatives.insert(
        "other".into(),
        super::super::model::AlternativeRecord {
            global_default: Some("com.acme/demo@1.0.0".into()),
            reserved_commands: vec!["demo".into()],
            runners: vec![ApplicationLauncherOwnership {
                destination: settings.join("opt/bin/demo"),
                sha256: "a".repeat(64),
            }],
        },
    );
    assert!(validate_index(&index, &settings).is_err());
}
