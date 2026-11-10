//! Index domain/wire round trip and atomic save proofs.
use super::*;
use vibe_core::manifest::{
    IconSet, LauncherActionDecl, LauncherKind, LauncherSource, RequirementScope, StepPhase,
};

fn sample() -> Index {
    let owned = Installation {
        source_evidence: None,
        secret_origins: Default::default(),
        configuration: None,
        pending: None,
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
            executable: "$app_bin/editor".into(),
            args: vec!["--fixed".into()],
            working_directory: Some("$project_root".into()),
            kind: Some(LauncherKind::Tui),
            terminal: Some(true),
            default: true,
            menu: Some(false),
            ..LauncherDecl::default()
        }],
        public_commands: vec![PublicCommandDecl {
            name: "edit".into(),
            launcher: "main".into(),
        }],
        path_dirs: Vec::new(),
        version_file: Vec::new(),
        template: None,
        migrations: Vec::new(),
        migrated: Vec::new(),
        extras: Vec::new(),
        commands: AuthoredCommands::default(),
        features: Vec::new(),
        parameters: Default::default(),
        license: None,
        icon: None,
        menu: ApplicationMenu::default(),
        steps: Vec::new(),
        lifecycle: Lifecycle::default(),
    };
    let external = Installation {
        source_evidence: None,
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
        public_commands: Vec::new(),
        path_dirs: Vec::new(),
        version_file: Vec::new(),
        template: None,
        migrations: Vec::new(),
        migrated: Vec::new(),
        extras: Vec::new(),
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
        icon: Some(IconDecl::Path("share/icon.png".into())),
        menu: ApplicationMenu {
            menu_group: Some("Editors".into()),
            categories: vec!["Development".into()],
            keywords: vec!["text".into()],
        },
        launchers: vec![LauncherDecl {
            id: "editor-gtk".into(),
            name: "Editor".into(),
            description: Some("Edit text".into()),
            executable: "$app_bin/editor".into(),
            args: vec!["{files}".into()],
            kind: Some(LauncherKind::Gui),
            terminal: Some(false),
            menu: Some(true),
            categories: vec!["Utility".into()],
            keywords: vec!["notes".into()],
            generic_name: Some("Text editor".into()),
            window_class: Some("editor-gtk".into()),
            mime_types: vec!["text/plain".into()],
            names: [("de".to_owned(), "Editor (de)".to_owned())].into(),
            descriptions: [("de".to_owned(), "Text bearbeiten".to_owned())].into(),
            icon: Some(IconDecl::Set(IconSet {
                svg: Some("$app_root/share/icons/hicolor/scalable/apps/editor.svg".into()),
                png: [(
                    "48".to_owned(),
                    "$app_root/share/icons/hicolor/48x48/apps/editor.png".to_owned(),
                )]
                .into(),
                maskable: true,
                ..IconSet::default()
            })),
            actions: vec![LauncherActionDecl {
                id: "new-window".into(),
                name: "New Window".into(),
                args: vec!["--new-window".into()],
                icon: None,
            }],
            source: LauncherSource::DesktopEntry,
            ..LauncherDecl::default()
        }],
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
                    weak: false,
                },
                Requirement {
                    package: "org.example/gcc@13".into(),
                    key: "org.example/gcc@13.2.0".into(),
                    scope: RequirementScope::Build,
                    step: Some("register".into()),
                    weak: false,
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
    index.slots.insert(
        "editor".into(),
        SlotState::with_default(Some(owned.package.key())),
    );
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
    sample()
        .save(
            &settings,
            &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
        )
        .unwrap();
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
            off: false,
            path_env: Vec::new(),
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

#[test]
fn a_record_without_kinds_reads_kind_from_terminal_and_no_menu() {
    let mut old = sample().to_wire();
    for record in old.installations.values_mut() {
        for launcher in &mut record.launchers {
            launcher.kind = None;
            launcher.menu = false;
            launcher.source = None;
        }
    }
    let index = Index::from_wire(old).unwrap();
    let owned = &index.installations["org.example/editor@1.2.0"].launchers[0];
    assert_eq!(owned.kind, Some(LauncherKind::Tui));
    assert!(!owned.shows_in_menu());
    let external = &index.installations["org.example/editor-apt@2.0.0"].launchers[0];
    assert_eq!(external.kind, Some(LauncherKind::Gui));
    assert_eq!(external.source, LauncherSource::Authored);
}

/// The writer's ladder, which is the whole reason a `/7` generation costs an
/// existing PATH proxy nothing (`PROP-059#proxy-index-upgrade`, PROP-075
/// `#MIGRATION-FAILURE`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#proxy-index-upgrade")]
fn the_writer_reaches_protocol_seven_only_for_a_fact_seven_carries() {
    /// Which `/7`-only fact the case marks the record with.
    enum Fact {
        Template,
        Declared,
        Carried,
    }

    // The sample retains nothing any generation above the first needs, so
    // it writes the oldest protocol — which is the ladder's whole point.
    let base = sample();
    assert_eq!(base.minimum_reader(), 3);
    assert_eq!(base.to_wire().protocol, "vibe-user-applications/3");

    for fact in [Fact::Template, Fact::Declared, Fact::Carried] {
        let mut index = sample();
        let record = index.installations.values_mut().next().unwrap();
        match fact {
            Fact::Template => {
                record.template = Some(crate::index::InstalledTemplate {
                    revision: "1.0.0".into(),
                    produced: "1.2.0".into(),
                    rendered_sha256: "a".repeat(64),
                    digest_source_sha256: None,
                    source_url: None,
                    digest_url: None,
                })
            }
            Fact::Declared => record.migrations.push(crate::index::MigrationEntryRecord {
                id: "state".into(),
                name: "State".into(),
                description: None,
                default: true,
            }),
            Fact::Carried => record.migrated.push(crate::index::MigrationOutcomeRecord {
                id: "state".into(),
                from: "org.example/editor@1.1.0".into(),
                to: "org.example/editor@1.2.0".into(),
                result: crate::index::MigrationResult::Done,
                step: None,
                work: None,
                log: None,
                at: None,
            }),
        }
        assert_eq!(index.minimum_reader(), 7);
        let document = index.to_wire();
        assert_eq!(document.protocol, "vibe-user-applications/7");
        // And it reads back as itself, through the `/7` arm.
        assert_eq!(Index::from_wire(document.clone()).unwrap(), index);
        // A `/7`-only fact in a `/6` document is refused rather than dropped.
        let mut forged = document;
        forged.protocol = "vibe-user-applications/6".into();
        assert!(
            Index::from_wire(forged)
                .unwrap_err()
                .contains("application index7")
        );
    }
}

/// The ladder's new rung: a **weak** requirer edge is the one fact `/8`
/// carries, so a home that holds no recommendation goes on writing exactly
/// the protocol it wrote before (`PROP-059#proxy-index-upgrade`,
/// `PROP-077#APP-RECOMMENDS-MECHANICS`).
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS"
)]
fn the_writer_reaches_protocol_eight_only_for_a_weak_requirer_edge() {
    // Every edge of the sample is a real requirement, so the strong-edge
    // home is not moved by this generation at all.
    let base = sample();
    assert!(
        base.installations
            .values()
            .any(|record| !record.lifecycle.requires.is_empty())
    );
    assert_eq!(base.minimum_reader(), 3);

    let mut index = sample();
    let owner = index
        .installations
        .values_mut()
        .find(|record| !record.lifecycle.requires.is_empty())
        .unwrap();
    owner.lifecycle.requires[0].weak = true;
    assert_eq!(index.minimum_reader(), 8);
    let document = index.to_wire();
    assert_eq!(document.protocol, "vibe-user-applications/8");
    // And it reads back as itself, through the `/8` arm.
    assert_eq!(Index::from_wire(document.clone()).unwrap(), index);
    // A `/8`-only fact in a `/7` document is refused rather than dropped,
    // because a dropped weak edge would silently orphan a recommendation.
    let mut forged = document;
    forged.protocol = "vibe-user-applications/7".into();
    assert!(
        Index::from_wire(forged)
            .unwrap_err()
            .contains("application index8")
    );
}

/// The same rung, reached by the other phase's fact: an ecosystem
/// adapter's record of what it last applied and last observed is the
/// second thing `/8` carries, so one generation serves both and a home
/// with neither keeps writing what it wrote (`PROP-075#SLOT-EXTRAS`,
/// `AV-FMT-029`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_writer_reaches_protocol_eight_for_an_adapter_record_too_but_not_for_an_empty_one() {
    use crate::index::{ExtrasRecord, ExtrasResult};

    assert_eq!(sample().minimum_reader(), 3);

    // A record of the adapter's id and nothing else retains nothing, so it
    // is neither written nor counted: an adapter that only ever skipped
    // must not push a home onto a reader its PATH proxies may not have.
    let mut index = sample();
    let key = index.installations.keys().next().unwrap().clone();
    index.installations.get_mut(&key).unwrap().extras = vec![ExtrasRecord::new("npm")];
    assert_eq!(index.minimum_reader(), 3);
    assert!(
        index.to_wire().installations[&key].extras.is_empty(),
        "an empty record is not written"
    );

    // One that retains a list is the `/8` fact.
    index.installations.get_mut(&key).unwrap().extras = vec![ExtrasRecord {
        id: "npm".into(),
        applied: vec!["corepack@latest".into(), "; rm -rf $HOME".into()],
        applied_at: Some("2026-11-10T10:15:06Z".into()),
        applied_outcome: Some(ExtrasResult::Failed),
        observed: vec!["corepack@latest".into()],
        observed_at: Some("2026-11-10T10:15:08Z".into()),
        observed_outcome: Some(ExtrasResult::Done),
    }];
    assert_eq!(index.minimum_reader(), 8);
    let document = index.to_wire();
    assert_eq!(document.protocol, "vibe-user-applications/8");
    // The items are echoed, never parsed: the one a shell would mangle
    // survives the round trip exactly.
    assert_eq!(
        document.installations[&key].extras[0].applied[1],
        "; rm -rf $HOME"
    );
    assert_eq!(Index::from_wire(document.clone()).unwrap(), index);
    // And a `/8`-only fact in a `/7` document is refused rather than
    // dropped, exactly as the weak edge is.
    let mut forged = document;
    forged.protocol = "vibe-user-applications/7".into();
    assert!(
        Index::from_wire(forged)
            .unwrap_err()
            .contains("application index8")
    );
}

/// What each reader of the edge makes of its weakness: it keeps its target
/// out of the orphan set, and it is a dependent of nothing
/// (`PROP-077#APP-RECOMMENDS-MECHANICS`).
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS"
)]
fn a_weak_edge_is_reported_weak_and_an_owner_of_both_edges_is_not() {
    let mut index = sample();
    let owner_key = index
        .installations
        .iter()
        .find(|(_, record)| !record.lifecycle.requires.is_empty())
        .map(|(key, _)| key.clone())
        .unwrap();
    let owner = index.installations.get_mut(&owner_key).unwrap();
    let target = owner.lifecycle.requires[0].key.clone();
    owner.lifecycle.requires[0].weak = true;
    let edges = index.required_by(&target);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].owner, owner_key);
    assert!(edges[0].weak);

    // The same owner recording a real requirement of the same target is the
    // case the weakness must not survive: the strong edge decides.
    let owner = index.installations.get_mut(&owner_key).unwrap();
    let mut strong = owner.lifecycle.requires[0].clone();
    strong.weak = false;
    strong.step = Some("install".into());
    owner.lifecycle.requires.push(strong);
    let edges = index.required_by(&target);
    assert_eq!(edges.len(), 1);
    assert!(!edges[0].weak);
}
