//! Actual closed file admission for both cohort epochs; reads never migrate.
#![cfg(test)]

use super::*;
use std::collections::BTreeMap;
use std::fs;
use vibe_wire::generated::application::e1::import_work as legacy;

const ID: &str = "cohort-reader-test";
const ROOT: &str = "org.example/root@1.0.0";
const CHILD: &str = "org.example/child@1.0.0";
const PUBLIC: &str = "PUBLIC_SOURCE_VALUE_CANARY_7351";
const SECRET: &str = "SECRET_VALUE_CANARY_8416";

fn context() -> w::ApplicationImportContext {
    w::ApplicationImportContext {
        lookup: w::ApplicationPendingLookup::Caller(Box::new(w::ApplicationPendingLookupCaller {})),
        offline: true,
        hosted: false,
        assume_yes: false,
        agent_external: false,
        agent: None,
    }
}

fn binding(
    settings: &SettingsRoot,
    key: &str,
    mark: w::PortableApplicationMark,
) -> w::ApplicationPortableBinding {
    w::ApplicationPortableBinding {
        application: w::PortableApplication {
            key: key.into(),
            installer: "main".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            mark,
            held: false,
            hidden: false,
            license_accepted: false,
            configuration: w::PortableConfigurationState::LegacyEffective(Box::new(
                w::PortableConfigurationStateLegacyEffective {
                    features: vec!["selected".into()],
                    parameters: BTreeMap::from([("caption".into(), PUBLIC.into())]),
                    secret_origins: BTreeMap::new(),
                },
            )),
            requires: vec![],
        },
        // Reader admission must not open this unavailable source. This fixture
        // asserts stored-data validity, not executable source or definition proof.
        source: w::ApplicationPendingSource {
            directory: settings
                .path()
                .join("UNAVAILABLE_SOURCE_CANARY")
                .to_str()
                .unwrap()
                .into(),
            kind: w::ApplicationPendingSourceKind::Directory,
            registry: None,
            project: None,
        },
        installer: "main".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        definition: "retained-definition-identity".into(),
        configuration_definition: None,
        agent_modes: BTreeMap::new(),
        target_input: w::PortableTargetInput {
            path_bindings: BTreeMap::new(),
            installer: None,
            review_definition: None,
            configuration: None,
        },
    }
}

fn fixture() -> (tempfile::TempDir, SettingsRoot, wire::ImportWork) {
    let directory = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(directory.path());
    let mut root = binding(&settings, ROOT, w::PortableApplicationMark::Manual);
    root.application.requires.push(w::PortableRequirement {
        package: "org.example/child@^1".into(),
        key: CHILD.into(),
        scope: w::PortableRequirementScope::Runtime,
        step: None,
    });
    let child = binding(&settings, CHILD, w::PortableApplicationMark::Auto);
    let document = wire::ImportWork {
        kind: "full".into(),
        source_evidence: Default::default(),
        protocol: PROTOCOL.into(),
        target: w::ApplicationImportTarget {
            home: pending::actual_home(&settings).unwrap(),
            operation: ID.into(),
        },
        phase: w::ApplicationImportPhase::Running,
        context: context(),
        cohort: w::ApplicationFullImport {
            completed_bindings: Default::default(),
            roots: vec![ROOT.into()],
            bindings: BTreeMap::from([(ROOT.into(), root), (CHILD.into(), child)]),
            cursor: 0,
            current: None,
            results: vec![],
            skip_installed: false,
            accept_license: false,
            collections: vec![],
            pins: vec![],
            launchers: vec![],
            arrangement_done: false,
            waiting: None,
            secret_requirements: BTreeMap::new(),
        },
    };
    (directory, settings, document)
}

fn legacy(settings: &SettingsRoot) -> legacy::ImportWork {
    legacy::ImportWork {
        protocol: "vibe-app-import-work/1".into(),
        target: w::ApplicationImportTarget {
            home: pending::actual_home(settings).unwrap(),
            operation: ID.into(),
        },
        phase: w::ApplicationImportPhase::Running,
        context: context(),
        cohort: w::ApplicationPendingImport {
            rows: vec![w::ApplicationPendingImportRow {
                key: ROOT.into(),
                installer: "legacy-option".into(),
                features: vec!["legacy-feature".into()],
                parameters: BTreeMap::from([("caption".into(), PUBLIC.into())]),
                license_accepted: true,
                held: true,
                hidden: true,
                mark: w::ApplicationPendingImportRowMark::Manual,
                source: w::ApplicationPendingSource {
                    directory: settings
                        .path()
                        .join("UNAVAILABLE_SOURCE_CANARY")
                        .to_str()
                        .unwrap()
                        .into(),
                    kind: w::ApplicationPendingSourceKind::Directory,
                    registry: None,
                    project: None,
                },
                platform: "linux".into(),
                arch: "x86_64".into(),
                definition: "retained-legacy-definition".into(),
                agent_modes: BTreeMap::from([(
                    "phase-agent".into(),
                    w::ApplicationAgentMode::Unattended,
                )]),
            }],
            cursor: 0,
            skip_installed: false,
            accept_license: false,
            collections: vec![],
            pins: vec![],
            launchers: vec![],
            arrangement_done: false,
            registry: None,
            current: None,
            results: vec![],
            auto: vec![],
        },
    }
}

fn write_document(
    settings: &SettingsRoot,
    document: &impl serde::Serialize,
) -> (std::path::PathBuf, Vec<u8>) {
    let path = store::path(settings, ID).unwrap().join("import.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = b" \n".to_vec();
    bytes.extend(serde_json::to_vec_pretty(document).unwrap());
    bytes.push(b'\n');
    fs::write(&path, &bytes).unwrap();
    (path, bytes)
}

fn refuses(settings: &SettingsRoot, document: &impl serde::Serialize, label: &str) {
    let (path, bytes) = write_document(settings, document);
    let error = load(settings, ID).expect_err(label);
    let diagnostic = format!("{error}\n{error:?}");
    for value in [PUBLIC, SECRET, "UNAVAILABLE_SOURCE_CANARY"] {
        assert!(
            !diagnostic.contains(value),
            "{label}: leaked private source/value"
        );
    }
    assert_eq!(
        fs::read(path).unwrap(),
        bytes,
        "{label}: reader changed input"
    );
    assert!(!settings.index_path().exists());
}

fn settled(document: &mut wire::ImportWork) {
    document.cohort.cursor = 1;
    document.cohort.results = vec![w::ApplicationPendingImportResult {
        key: ROOT.into(),
        action: w::ApplicationPendingImportResultAction::Skipped,
        option: Some("main".into()),
        code: None,
        message: "explicit skip".into(),
        target: None,
        row_index: Some(0),
    }];
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn closed_v1_and_v2_read_exact_bytes_without_migration_or_source_access() {
    let (_directory, settings, document) = fixture();
    let original = legacy(&settings);
    let (path, bytes) = write_document(&settings, &original);
    let ImportWork::Legacy(loaded) = load(&settings, ID).unwrap() else {
        panic!("version1 must stay legacy");
    };
    assert_eq!(loaded.cohort, original.cohort);
    assert_eq!(loaded.target, original.target);
    assert_eq!(loaded.context, original.context);
    assert_eq!(loaded.phase, original.phase);
    assert!(loaded.source_evidence.is_empty());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let frozen: vibe_wire::generated::application::e2::import_work::ImportWork =
        serde_json::from_slice(&encode(&document).unwrap()).unwrap();
    let (path, bytes) = write_document(&settings, &frozen);
    let ImportWork::Full(loaded) = load(&settings, ID).unwrap() else {
        panic!("version2 must stay full");
    };
    assert_eq!(loaded, document);
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(!settings.path().join("UNAVAILABLE_SOURCE_CANARY").exists());
    assert!(!settings.index_path().exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn shared_row_admission_rejects_missing_inconsistent_edges_roots_and_secret_overlap() {
    let (_directory, settings, base) = fixture();
    let mut invalid = base.clone();
    invalid.cohort.bindings.remove(CHILD);
    refuses(&settings, &invalid, "missing required binding");
    let mut invalid = base.clone();
    invalid
        .cohort
        .bindings
        .get_mut(ROOT)
        .unwrap()
        .application
        .requires[0]
        .package = "org.example/child@^2".into();
    refuses(&settings, &invalid, "range excludes exact key");
    let mut invalid = base.clone();
    invalid
        .cohort
        .bindings
        .get_mut(ROOT)
        .unwrap()
        .application
        .requires[0]
        .package = "org.example/other@^1".into();
    refuses(&settings, &invalid, "wrong requirement coordinate");
    let mut invalid = base.clone();
    invalid.cohort.roots.push(ROOT.into());
    refuses(&settings, &invalid, "duplicate root");
    let mut invalid = base.clone();
    invalid.cohort.roots = vec![CHILD.into()];
    refuses(&settings, &invalid, "Auto is not a selected job");
    let mut invalid = base.clone();
    let w::PortableConfigurationState::LegacyEffective(history) = &mut invalid
        .cohort
        .bindings
        .get_mut(ROOT)
        .unwrap()
        .application
        .configuration
    else {
        panic!("legacy fixture");
    };
    history.parameters.insert("caption".into(), SECRET.into());
    history.secret_origins.insert(
        "caption".into(),
        w::ConfigurationParameterOrigin::Explicit(Box::new(
            w::ConfigurationParameterOriginExplicit {},
        )),
    );
    refuses(
        &settings,
        &invalid,
        "public value overlaps known secret origin",
    );
    let mut invalid = base;
    let w::PortableConfigurationState::LegacyEffective(history) = &mut invalid
        .cohort
        .bindings
        .get_mut(ROOT)
        .unwrap()
        .application
        .configuration
    else {
        panic!("legacy fixture");
    };
    history.secret_origins.insert(
        "token".into(),
        w::ConfigurationParameterOrigin::Profile(Box::new(
            w::ConfigurationParameterOriginProfile {
                profile: "not-a-secret-origin".into(),
            },
        )),
    );
    refuses(
        &settings,
        &invalid,
        "secret cannot acquire a public profile origin",
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn arrangement_and_complete_require_settled_roots_without_current_or_waiting() {
    let (_directory, settings, base) = fixture();
    for phase in [
        w::ApplicationImportPhase::Arrangement,
        w::ApplicationImportPhase::Complete,
    ] {
        let mut invalid = base.clone();
        invalid.phase = phase.clone();
        refuses(&settings, &invalid, "phase skips root execution");
        settled(&mut invalid);
        invalid.cohort.arrangement_done = true;
        invalid.cohort.current = Some(w::ApplicationPendingTarget {
            home: invalid.target.home.clone(),
            key: ROOT.into(),
            operation: "child-operation".into(),
            project: None,
        });
        refuses(&settings, &invalid, "settled phase retains current child");
        invalid.cohort.current = None;
        invalid.cohort.waiting = Some(w::ApplicationImportWaitingInput {
            key: ROOT.into(),
            parameters: vec!["token".into()],
        });
        refuses(&settings, &invalid, "settled phase retains waiting input");
    }
    let mut arrangement = base;
    settled(&mut arrangement);
    arrangement.phase = w::ApplicationImportPhase::Arrangement;
    let (path, bytes) = write_document(&settings, &arrangement);
    assert!(matches!(load(&settings, ID).unwrap(), ImportWork::Full(_)));
    assert_eq!(fs::read(path).unwrap(), bytes);
    arrangement.phase = w::ApplicationImportPhase::Complete;
    refuses(&settings, &arrangement, "Complete requires arrangementDone");
    arrangement.cohort.arrangement_done = true;
    let (path, bytes) = write_document(&settings, &arrangement);
    assert!(matches!(load(&settings, ID).unwrap(), ImportWork::Full(_)));
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn private_map_identity_waiting_and_outcome_checks_remain_enforced() {
    let (_directory, settings, base) = fixture();
    let mut invalid = base.clone();
    let root = invalid.cohort.bindings.remove(ROOT).unwrap();
    invalid
        .cohort
        .bindings
        .insert("org.example/alias@1.0.0".into(), root);
    refuses(
        &settings,
        &invalid,
        "map key differs despite valid row universe",
    );
    let mut invalid = base.clone();
    invalid.target.home = "/another-home".into();
    refuses(&settings, &invalid, "foreign home");
    let mut invalid = base.clone();
    invalid.target.operation = "other-generation".into();
    refuses(&settings, &invalid, "foreign generation");
    let mut invalid = base.clone();
    invalid.cohort.cursor = 2;
    refuses(&settings, &invalid, "cursor past roots");
    let mut invalid = base.clone();
    invalid.cohort.waiting = Some(w::ApplicationImportWaitingInput {
        key: "org.example/absent@1.0.0".into(),
        parameters: vec!["token".into()],
    });
    refuses(&settings, &invalid, "waiting recipient absent");
    invalid.cohort.waiting = Some(w::ApplicationImportWaitingInput {
        key: ROOT.into(),
        parameters: vec![],
    });
    refuses(&settings, &invalid, "empty waiting obligations");
    let mut invalid = base.clone();
    invalid
        .cohort
        .secret_requirements
        .insert(ROOT.into(), vec![String::new()]);
    refuses(&settings, &invalid, "empty retained secret ID");
    let mut invalid = base;
    settled(&mut invalid);
    invalid.cohort.results[0].action = w::ApplicationPendingImportResultAction::Installed;
    refuses(&settings, &invalid, "Installed has no exact attempt");
    invalid.cohort.results[0].action = w::ApplicationPendingImportResultAction::Skipped;
    invalid.cohort.results[0].row_index = Some(1);
    refuses(&settings, &invalid, "outcome points past completed cursor");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn both_closed_readers_reject_unknown_fields_and_missing_required_nullable_members() {
    let (_directory, settings, full) = fixture();
    for base in [
        serde_json::to_value(legacy(&settings)).unwrap(),
        serde_json::to_value(full).unwrap(),
    ] {
        let mut unknown = base.clone();
        unknown["unknown"] = SECRET.into();
        refuses(&settings, &unknown, "unknown closed field");
        let mut missing = base.clone();
        missing["cohort"].as_object_mut().unwrap().remove("current");
        refuses(&settings, &missing, "required nullable current is absent");
        let mut protocol = base;
        protocol["protocol"] = "vibe-app-import-work/999".into();
        refuses(&settings, &protocol, "unknown epoch");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn invalid_full_save_cannot_replace_existing_closed_record() {
    let (_directory, settings, mut document) = fixture();
    let (path, before) = write_document(&settings, &document);
    document.phase = w::ApplicationImportPhase::Arrangement;
    assert!(save_full(&settings, &document).is_err());
    assert_eq!(fs::read(path).unwrap(), before);
    assert!(matches!(load(&settings, ID).unwrap(), ImportWork::Full(_)));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn settled_cursor_requires_exactly_one_outcome_for_every_prefix_root() {
    let (_directory, settings, mut d) = fixture();
    d.cohort.cursor = 1;
    refuses(&settings, &d, "cursor cannot erase all outcomes");
    settled(&mut d);
    d.cohort.results.push(d.cohort.results[0].clone());
    refuses(&settings, &d, "duplicate settled outcome");
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn waiting_cannot_authorize_an_unused_orphan_or_unstarted_descendant() {
    let (_directory, settings, mut d) = fixture();
    let mut orphan = d.cohort.bindings[CHILD].clone();
    orphan.application.key = "org.example/orphan@1.0.0".into();
    d.cohort
        .bindings
        .insert(orphan.application.key.clone(), orphan);
    d.cohort.waiting = Some(w::ApplicationImportWaitingInput {
        key: "org.example/orphan@1.0.0".into(),
        parameters: vec!["token".into()],
    });
    refuses(&settings, &d, "orphan cannot become active waiting input");
    d.cohort.waiting.as_mut().unwrap().key = CHILD.into();
    refuses(
        &settings,
        &d,
        "descendant waiting requires an actual current root attempt",
    );
}
