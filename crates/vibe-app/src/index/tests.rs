//! Index domain/wire round trip and atomic save proofs.
use super::*;
use vibe_core::manifest::{RequirementScope, StepPhase};

fn sample() -> Index {
    let owned = Installation {
        slot: "editor".into(),
        package: PackageCoordinate {
            group: "org.example".into(),
            name: "editor".into(),
            version: "1.2.0".into(),
        },
        option: "linux-zip".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        method: Method::Zip,
        location: Location::Owned {
            library: "default".into(),
            directory: "apps/editor/org.example.editor@1.2.0".into(),
        },
        portable: true,
        launchers: vec![LauncherDecl {
            id: "main".into(),
            name: "Editor".into(),
            description: None,
            executable: "$app_bin/editor".into(),
            args: vec!["--fixed".into()],
            working_directory: Some("$project_root".into()),
            icon: None,
            terminal: true,
            default: true,
        }],
        public_commands: vec![PublicCommandDecl {
            name: "edit".into(),
            launcher: "main".into(),
        }],
        commands: AuthoredCommands::default(),
        features: Vec::new(),
        parameters: Default::default(),
        license: None,
        icon: None,
        steps: Vec::new(),
        lifecycle: Lifecycle::default(),
    };
    let external = Installation {
        package: PackageCoordinate {
            group: "org.example".into(),
            name: "editor-apt".into(),
            version: "2.0.0".into(),
        },
        option: "ubuntu-apt".into(),
        platform: "ubuntu".into(),
        method: Method::Shell,
        location: Location::External {
            app_root: PathBuf::from("/usr/lib/editor"),
        },
        portable: false,
        launchers: Vec::new(),
        public_commands: Vec::new(),
        commands: AuthoredCommands {
            uninstall: Some("sudo apt-get remove editor".into()),
            force_uninstall: Some("sudo apt-get purge editor".into()),
            update: Some("sudo apt-get install --only-upgrade editor".into()),
            interpreter: Some(ShellInterpreter::PosixSh),
        },
        features: vec!["docs".into()],
        parameters: [("port".to_owned(), "8080".to_owned())].into(),
        license: Some(crate::index::AcceptedLicense {
            name: "Demo".into(),
            accepted_at: "2026-10-31T00:00:00Z".into(),
        }),
        icon: Some("share/icon.png".into()),
        steps: vec![
            StepRecord {
                id: "install".into(),
                kind: "shell".into(),
                phase: StepPhase::Prepare,
                outcome: StepOutcome::Done,
                uninstall: AuthoredCommands::default(),
            },
            StepRecord {
                id: "register".into(),
                kind: "shell".into(),
                phase: StepPhase::Prepare,
                outcome: StepOutcome::Done,
                uninstall: AuthoredCommands {
                    uninstall: Some("editor-register --remove".into()),
                    interpreter: Some(ShellInterpreter::PosixSh),
                    ..AuthoredCommands::default()
                },
            },
            StepRecord {
                id: "fix".into(),
                kind: "agent".into(),
                phase: StepPhase::Finish,
                outcome: StepOutcome::Skipped,
                uninstall: AuthoredCommands::default(),
            },
        ],
        lifecycle: Lifecycle {
            hidden: true,
            mark: Mark::Auto,
            requires: vec![
                Requirement {
                    package: "org.example/editor@1".into(),
                    key: "org.example/editor@1.2.0".into(),
                    scope: RequirementScope::Runtime,
                    step: None,
                },
                Requirement {
                    package: "org.example/gcc@13".into(),
                    key: "org.example/gcc@13.2.0".into(),
                    scope: RequirementScope::Build,
                    step: Some("register".into()),
                },
            ],
            env: vec![
                EnvEntry {
                    step: "fix".into(),
                    op: EnvOp::Set,
                    name: "EDITOR_HOME".into(),
                    value: Some("/usr/lib/editor".into()),
                    previous: None,
                    previous_kind: None,
                },
                EnvEntry {
                    step: "fix".into(),
                    op: EnvOp::PathPrepend,
                    name: "PATH".into(),
                    value: Some("/usr/lib/editor/bin".into()),
                    previous: Some(r"%USERPROFILE%\bin".into()),
                    previous_kind: Some(ValueKind::ExpandString),
                },
            ],
            held: true,
            replaced: Some("org.example/editor-apt@1.9.0".into()),
            superseded_by: None,
        },
        ..owned.clone()
    };
    let mut index = Index::default();
    index
        .installations
        .insert(owned.package.key(), owned.clone());
    index.installations.insert(external.package.key(), external);
    index
        .slots
        .insert("editor".into(), Some(owned.package.key()));
    index
}

#[test]
fn domain_and_wire_round_trip_the_corpus() {
    let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/application/e2/index.json");
    let document: wire::ApplicationIndex =
        serde_json::from_slice(&fs::read(corpus).unwrap()).unwrap();
    let index = Index::from_wire(document.clone()).unwrap();
    assert_eq!(index, sample());
    assert_eq!(index.to_wire(), document);
}

#[test]
fn save_is_atomic_and_load_rejects_inconsistent_records() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    sample().save(&settings).unwrap();
    assert_eq!(Index::load(&settings).unwrap(), sample());
    let leftovers: Vec<_> = fs::read_dir(settings.applications_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty());

    let mut broken = sample().to_wire();
    broken.slots.insert(
        "other".into(),
        wire::Slot {
            default: Some("org.example/editor@1.2.0".into()),
        },
    );
    assert!(
        Index::from_wire(broken)
            .unwrap_err()
            .contains("not an installation")
    );
    let mut wrong = sample().to_wire();
    for record in wrong.installations.values_mut() {
        record.app_root = Some("/also".into());
    }
    assert!(Index::from_wire(wrong).is_err());
    let mut old = sample().to_wire();
    old.protocol = "vibe-user-applications/2".into();
    assert!(Index::from_wire(old).unwrap_err().contains("protocol"));
}
