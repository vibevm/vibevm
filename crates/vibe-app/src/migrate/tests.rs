//! The decidable half of `vibe app migrate` — which installations a pair
//! names, which entries it declares, what an offer says and what the retry
//! command is — none of which runs a child (PROP-075
//! `#MIGRATION-DECLARATION`, `#MIGRATION-FAILURE`).

use std::path::Path;

use super::*;
use crate::index::{
    AuthoredCommands, Installation, Location, Method, MigrationResult, PackageCoordinate, SlotState,
};

fn record(version: &str, entries: &[(&str, bool)]) -> Installation {
    Installation {
        source_evidence: None,
        secret_origins: Default::default(),
        configuration: None,
        pending: None,
        slot: "stateful".into(),
        package: PackageCoordinate {
            group: "org.example".into(),
            name: "stateful".into(),
            version: version.into(),
        },
        option: "linux".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        method: Method::Steps,
        location: Location::External {
            app_root: Path::new("/opt/stateful").to_path_buf(),
        },
        portable: false,
        launchers: Vec::new(),
        public_commands: Vec::new(),
        path_dirs: Vec::new(),
        version_file: Vec::new(),
        template: None,
        migrations: entries
            .iter()
            .map(|(id, default)| MigrationEntryRecord {
                id: (*id).to_owned(),
                name: format!("Entry {id}"),
                description: None,
                default: *default,
            })
            .collect(),
        migrated: Vec::new(),
        commands: AuthoredCommands::default(),
        features: Vec::new(),
        parameters: Default::default(),
        license: None,
        icon: None,
        menu: Default::default(),
        steps: Vec::new(),
        lifecycle: crate::index::Lifecycle::default(),
    }
}

/// Two installed versions of one slot; the newer declares `entries`.
fn home(entries: &[(&str, bool)]) -> Index {
    let mut index = Index::default();
    for (version, declared) in [("1.0.0", &[][..]), ("2.0.0", entries)] {
        let row = record(version, declared);
        index.installations.insert(row.package.key(), row);
    }
    index.slots.insert(
        "stateful".into(),
        SlotState {
            default: Some("org.example/stateful@2.0.0".into()),
            ..SlotState::default()
        },
    );
    index
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
fn a_pair_resolves_by_version_or_by_whole_key_and_from_equals_to_is_admitted() {
    let index = home(&[("state", true), ("extra", false)]);
    let plan = migration_plan(&index, "stateful", "1.0.0", "2.0.0").unwrap();
    assert_eq!(plan.from, "org.example/stateful@1.0.0");
    assert_eq!(plan.to, "org.example/stateful@2.0.0");
    assert_eq!(plan.from_version, "1.0.0");
    assert_eq!(plan.all(), ["state", "extra"]);
    assert_eq!(plan.preselected(), ["state"]);
    // The whole key spells the same pair.
    assert_eq!(
        migration_plan(
            &index,
            "stateful",
            "org.example/stateful@1.0.0",
            "org.example/stateful@2.0.0"
        )
        .unwrap(),
        plan
    );
    // `FROM == TO` is the maintenance shape and is admitted as written.
    let same = migration_plan(&index, "stateful", "2.0.0", "2.0.0").unwrap();
    assert_eq!(same.from, same.to);
    assert_eq!(same.all(), ["state", "extra"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn a_pair_that_names_nothing_installed_refuses_with_its_code() {
    let index = home(&[("state", true)]);
    assert_eq!(
        migration_plan(&index, "absent", "1.0.0", "2.0.0")
            .unwrap_err()
            .code(),
        "unknown_slot"
    );
    assert_eq!(
        migration_plan(&index, "stateful", "1.0.0", "9.9.9")
            .unwrap_err()
            .code(),
        "not_installed"
    );
    assert_eq!(
        migration_plan(&Index::default(), "stateful", "1.0.0", "2.0.0")
            .unwrap_err()
            .code(),
        "unknown_slot"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn the_retry_command_names_exactly_the_entry_and_the_kept_directory() {
    let index = home(&[("state", true), ("extra", false)]);
    let plan = migration_plan(&index, "stateful", "1.0.0", "2.0.0").unwrap();
    assert_eq!(
        plan.command(&["state".to_owned()], true),
        "vibe app migrate stateful 1.0.0 2.0.0 --only state --resume --yes"
    );
    assert_eq!(
        plan.command(&plan.all(), false),
        "vibe app migrate stateful 1.0.0 2.0.0 --only state --only extra --yes"
    );
    // The offer and the unattended refusal print the preselected set…
    assert_eq!(
        plan.offer_command(false),
        "vibe app migrate stateful 1.0.0 2.0.0 --only state --yes"
    );
    // …and `--all` when the package marks nothing `default`, because a
    // command naming no entry would run nothing.
    let none = migration_plan(&home(&[("extra", false)]), "stateful", "1.0.0", "2.0.0").unwrap();
    assert_eq!(
        none.offer_command(true),
        "vibe app migrate stateful 1.0.0 2.0.0 --all --resume --yes"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn an_offer_names_the_previous_version_and_falls_silent_once_it_was_carried() {
    let mut index = home(&[("state", true)]);
    let offer = migration_offer(&index, "stateful", "org.example/stateful@2.0.0")
        .expect("a previous version with declared entries is an offer");
    assert_eq!(offer.from, "org.example/stateful@1.0.0");
    assert_eq!(offer.preselected(), ["state"]);

    // Nothing declared: nothing to offer.
    assert!(
        migration_offer(&index, "stateful", "org.example/stateful@1.0.0").is_none(),
        "the older version declares no entry"
    );
    // Already carried: a second install of the same version is silent.
    index
        .installations
        .get_mut("org.example/stateful@2.0.0")
        .unwrap()
        .migrated
        .push(MigrationOutcomeRecord {
            id: "state".into(),
            from: "org.example/stateful@1.0.0".into(),
            to: "org.example/stateful@2.0.0".into(),
            result: MigrationResult::Done,
            step: None,
            work: None,
            log: None,
            at: None,
        });
    assert!(migration_offer(&index, "stateful", "org.example/stateful@2.0.0").is_none());
    // A failed carry is not a carry: the offer comes back.
    index
        .installations
        .get_mut("org.example/stateful@2.0.0")
        .unwrap()
        .migrated[0]
        .result = MigrationResult::ImportFailed;
    assert!(migration_offer(&index, "stateful", "org.example/stateful@2.0.0").is_some());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn resume_without_a_kept_directory_refuses_and_a_fresh_run_makes_one() {
    let dir = tempfile::tempdir().unwrap();
    let settings = crate::SettingsRoot::new(dir.path());
    let index = home(&[("state", true)]);
    let plan = migration_plan(&index, "stateful", "1.0.0", "2.0.0").unwrap();
    assert_eq!(
        record::open(&settings, &plan, true).unwrap_err().code(),
        "nothing_to_resume"
    );
    let (work, resumed) = record::open(&settings, &plan, false).unwrap();
    assert!(!resumed && work.is_dir(), "{}", work.display());
    assert!(
        work.starts_with(
            settings
                .app_migrate_dir()
                .join("stateful")
                .join("1.0.0-to-2.0.0")
        ),
        "{}",
        work.display()
    );
    assert_eq!(kept_migration_work(&settings), [work.as_path()]);
    assert_eq!(slot_migration_work(&settings, "stateful"), [work.as_path()]);
    // A kept directory is what `--resume` then finds.
    let (again, resumed) = record::open(&settings, &plan, true).unwrap();
    assert!(resumed && again == work);
    // A second fresh run inside the same second never merges its evidence
    // with the first.
    let (second, _) = record::open(&settings, &plan, false).unwrap();
    assert_ne!(second, work);
    assert_eq!(kept_migration_work(&settings).len(), 2);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn a_slot_or_version_can_never_reach_out_of_the_migration_root() {
    let dir = tempfile::tempdir().unwrap();
    let settings = crate::SettingsRoot::new(dir.path());
    let plan = MigrationPlan {
        slot: "../escape".into(),
        from: "k".into(),
        to: "k".into(),
        from_version: "../../etc".into(),
        to_version: "..".into(),
        entries: Vec::new(),
    };
    let (work, _) = record::open(&settings, &plan, false).unwrap();
    assert!(
        work.starts_with(settings.app_migrate_dir()),
        "{}",
        work.display()
    );
    assert!(
        !work
            .components()
            .any(|part| part == std::path::Component::ParentDir),
        "{}",
        work.display()
    );
    // `..` survives only as a literal name component, never as a traversal.
    assert!(work.starts_with(settings.app_migrate_dir().join(".._escape")));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
fn the_record_in_the_work_directory_is_the_report_itself() {
    let dir = tempfile::tempdir().unwrap();
    let settings = crate::SettingsRoot::new(dir.path());
    let index = home(&[("state", true)]);
    let plan = migration_plan(&index, "stateful", "1.0.0", "2.0.0").unwrap();
    let (work, _) = record::open(&settings, &plan, false).unwrap();
    assert!(record::read(&work).is_none(), "nothing written yet");
    let mut document = record::empty(&plan, &work, false);
    document.entries.push(record::skipped(
        &plan.entries[0],
        "not selected; name it with `--only`",
    ));
    record::save(&work, &document).unwrap();
    let back = record::read(&work).expect("the record reads back");
    assert_eq!(back, document);
    assert_eq!(back.protocol, MIGRATION_PROTOCOL);
    // A skipped entry is not an outcome the installation retains.
    assert!(record::outcomes(&plan, &back).is_empty());
    // A corrupt record is no record, and costs one re-run, never an error.
    std::fs::write(work.join("migration.json"), b"{").unwrap();
    assert!(record::read(&work).is_none());
}
