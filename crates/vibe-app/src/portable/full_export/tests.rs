//! Independent selection/closure oracles over real index and operation readers.
#![cfg(test)]

use super::*;
use crate::index::{Lifecycle, Location, Method, PackageCoordinate, Requirement};
use crate::{EventKind, HomeEvent};
use std::collections::BTreeMap;

fn record(name: &str, mark: Mark, hidden: bool) -> Installation {
    Installation {
        source_evidence: None,
        secret_origins: BTreeMap::new(),
        configuration: None,
        pending: None,
        slot: name.into(),
        package: PackageCoordinate {
            group: "org.example".into(),
            name: name.into(),
            version: "1.0.0".into(),
        },
        option: "alternate".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        method: Method::Steps,
        location: Location::Owned {
            library: "default".into(),
            directory: name.into(),
        },
        portable: true,
        launchers: vec![],
        public_commands: vec![],
        commands: Default::default(),
        features: vec!["legacy-feature".into()],
        parameters: BTreeMap::from([("answer".into(), "legacy value".into())]),
        license: Some(crate::index::AcceptedLicense {
            name: "MIT".into(),
            accepted_at: "2026-11-05T00:00:00Z".into(),
        }),
        icon: None,
        menu: Default::default(),
        steps: vec![],
        lifecycle: Lifecycle {
            mark,
            hidden,
            held: true,
            ..Default::default()
        },
    }
}
fn needs(record: &mut Installation, child: &str) {
    record.lifecycle.requires.push(Requirement {
        package: format!("org.example/{child}@^1"),
        key: format!("org.example/{child}@1.0.0"),
        scope: RequirementScope::Runtime,
        step: None,
    });
}
fn save(settings: &SettingsRoot, records: Vec<Installation>) {
    let index = Index {
        installations: records.into_iter().map(|r| (r.package.key(), r)).collect(),
        slots: BTreeMap::new(),
    };
    index
        .save(settings, &HomeEvent::new(EventKind::Updated))
        .unwrap();
    Index::load(settings).unwrap();
}
fn request(all: bool) -> PortableExport {
    PortableExport {
        all,
        arrangement: false,
    }
}
fn full(document: PortableDocument) -> full::AppsManifest {
    match document {
        PortableDocument::Full(set) => set,
        _ => panic!("expected full"),
    }
}
fn evaluated() -> crate::configuration::ConfigurationRecord {
    let definition: vibe_core::manifest::ConfigurationDecl=toml::from_str("[[features]]\nid='docs'\nname='Docs'\ndefault=true\n[[parameters]]\nid='flag'\nname='Flag'\ntype='bool'\ndefault=false\n").unwrap();
    let input = w::ConfigurationInput {
        intent: w::ConfigurationSourceInput {
            policy: w::ConfigurationPolicy::UseDeclaredDefaults,
            features: vec![],
            default_features: true,
            selections: BTreeMap::new(),
            parameters: BTreeMap::new(),
            profile: None,
        },
        disabled_features: vec![],
    };
    let result = crate::configuration::evaluate(crate::configuration::EvaluationRequest {
        history_use: crate::configuration::HistoryUse::Local,
        definition: &definition,
        input: &input,
        previous: None,
        replacement: true,
        reviewed: false,
        expected_definition: None,
        fresh_parameters: &BTreeSet::new(),
        ephemeral: &[],
        facts: crate::HostFacts::default(),
        prompt: None,
    })
    .unwrap();
    assert!(result.record.evaluation.resolved.is_some());
    result.record
}

#[test]
fn hidden_manual_intermediate_keeps_ownership_without_becoming_a_root() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let mut root = record("root", Mark::Manual, false);
    needs(&mut root, "middle");
    let mut middle = record("middle", Mark::Manual, true);
    needs(&mut middle, "leaf");
    let mut leaf = record("leaf", Mark::Auto, true);
    leaf.configuration = Some(evaluated());
    leaf.features.clear();
    leaf.parameters.clear();
    let orphan = record("orphan", Mark::Auto, false);
    let mut superseded = record("old", Mark::Manual, false);
    superseded.lifecycle.superseded_by = Some(root.package.key());
    save(&settings, vec![root, middle, leaf, orphan, superseded]);
    let set = full(export(&settings, &request(false), false).unwrap());
    assert_eq!(set.roots, ["org.example/root@1.0.0"]);
    assert_eq!(
        set.application
            .iter()
            .map(|r| r.key.as_str())
            .collect::<Vec<_>>(),
        [
            "org.example/leaf@1.0.0",
            "org.example/middle@1.0.0",
            "org.example/root@1.0.0"
        ]
    );
    assert_eq!(set.application[1].mark, w::PortableApplicationMark::Manual);
    assert!(
        set.application[1].hidden && set.application[1].held && set.application[1].license_accepted
    );
    assert_eq!(
        set.application[1].requires[0].package,
        "org.example/leaf@^1"
    );
    assert!(matches!(
        set.application[0].configuration,
        w::PortableConfigurationState::Configured(_)
    ));
    let all = full(export(&settings, &request(true), false).unwrap());
    assert_eq!(
        all.roots,
        ["org.example/middle@1.0.0", "org.example/root@1.0.0"]
    );
    assert_eq!(all.application.len(), 4);
    assert_eq!(
        all.application
            .iter()
            .find(|r| r.key == "org.example/orphan@1.0.0")
            .unwrap()
            .mark,
        w::PortableApplicationMark::Auto
    );
}

#[test]
fn all_rich_orphan_auto_selects_full_but_has_no_job_root() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let mut orphan = record("orphan", Mark::Auto, false);
    orphan.configuration = Some(evaluated());
    orphan.features.clear();
    orphan.parameters.clear();
    save(&settings, vec![orphan]);
    let PortableDocument::Legacy(empty) = export(&settings, &request(false), false).unwrap() else {
        panic!("no selected state needs rich form")
    };
    assert!(empty.application.is_empty());
    let set = full(export(&settings, &request(true), false).unwrap());
    assert!(set.roots.is_empty());
    assert_eq!(set.application.len(), 1);
}

#[test]
fn automatic_legacy_does_not_suddenly_require_complete_edges() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let mut root = record("root", Mark::Manual, false);
    needs(&mut root, "missing");
    save(&settings, vec![root]);
    let PortableDocument::Legacy(set) = export(&settings, &request(false), false).unwrap() else {
        panic!("legacy compatibility")
    };
    assert_eq!(set.application.len(), 1);
    assert_eq!(set.application[0].installer, "alternate");
    assert_eq!(set.application[0].features, ["legacy-feature"]);
    assert_eq!(set.application[0].parameters["answer"], "legacy value");
    assert!(set.application[0].held);
    assert!(
        export(&settings, &request(false), true)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
}

#[test]
fn known_secret_origin_selects_full_and_missing_dependency_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let mut root = record("root", Mark::Manual, false);
    root.secret_origins.insert(
        "optional-token".into(),
        w::ConfigurationParameterOrigin::Explicit(Box::new(
            w::ConfigurationParameterOriginExplicit {},
        )),
    );
    save(&settings, vec![root.clone()]);
    let set = full(export(&settings, &request(false), false).unwrap());
    let w::PortableConfigurationState::LegacyEffective(history) = &set.application[0].configuration
    else {
        panic!("legacy effective")
    };
    assert_eq!(history.secret_origins, root.secret_origins);
    needs(&mut root, "absent");
    save(&settings, vec![root]);
    assert!(
        export(&settings, &request(false), false)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
}

#[test]
fn forced_full_retains_legacy_closure_and_arrangement_under_one_lock() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let mut root = record("root", Mark::Manual, false);
    needs(&mut root, "child");
    save(&settings, vec![root, record("child", Mark::Auto, true)]);
    let mut layer = UserLaunchers::default();
    layer.collections.push(crate::catalog::UserCollection {
        name: "Ordered".into(),
        icon: None,
        launchers: vec!["one".into(), "two".into()],
        seeded: false,
        removed: vec![],
    });
    {
        let _lock = settings.lock().unwrap();
        layer
            .save(&settings, &HomeEvent::new(EventKind::Updated))
            .unwrap();
    }
    let set = full(
        export(
            &settings,
            &PortableExport {
                all: false,
                arrangement: true,
            },
            true,
        )
        .unwrap(),
    );
    assert_eq!(set.roots, ["org.example/root@1.0.0"]);
    assert_eq!(set.application.len(), 2);
    assert_eq!(set.collection[0].launchers, ["one", "two"]);
    assert!(
        full(export(&settings, &request(false), true).unwrap())
            .collection
            .is_empty()
    );
    let _lock = settings.lock().unwrap();
    assert!(
        export(&settings, &request(false), true).is_err(),
        "export must use existing home lock"
    );
}

#[test]
fn relevant_unfinished_arrangement_cohort_refuses_but_complete_cohort_is_ready() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    save(&settings, vec![record("root", Mark::Manual, false)]);
    let PortableDocument::Legacy(mut set) = export(&settings, &request(false), false).unwrap()
    else {
        panic!("legacy fixture");
    };
    set.collection
        .push(vibe_wire::generated::apps_manifest::Collection {
            name: "Pending arrangement".into(),
            launchers: vec![],
        });
    let prepared = crate::prepare_import(&set, &[], false, false, None, false).unwrap();
    let id = "export-cohort";
    let mut document = vibe_wire::generated::application::e3::legacy_import_work::ImportWork {
        protocol: "vibe-app-import-work/3".into(),
        kind: "legacy".into(),
        source_evidence: Default::default(),
        target: w::ApplicationImportTarget {
            home: pending::actual_home(&settings).unwrap(),
            operation: id.into(),
        },
        phase: w::ApplicationImportPhase::Arrangement,
        context: w::ApplicationImportContext {
            lookup: w::ApplicationPendingLookup::Caller(Box::new(
                w::ApplicationPendingLookupCaller {},
            )),
            offline: true,
            hosted: false,
            assume_yes: false,
            agent_external: false,
            agent: None,
        },
        cohort: prepared.state,
    };
    import::store::save(&settings, &document).unwrap();
    assert!(
        export(
            &settings,
            &PortableExport {
                all: false,
                arrangement: true
            },
            true
        )
        .is_err()
    );
    assert!(
        export(&settings, &request(false), true).is_ok(),
        "unrelated arrangement is not an installation change"
    );
    document.phase = w::ApplicationImportPhase::Complete;
    document.cohort.arrangement_done = true;
    import::store::save(&settings, &document).unwrap();
    import::load(&settings, id).unwrap();
    assert!(
        export(
            &settings,
            &PortableExport {
                all: false,
                arrangement: true
            },
            true
        )
        .is_ok()
    );
}

#[test]
fn unreadable_relevant_work_refuses_full_without_changing_legacy_mode() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let root = record("root", Mark::Manual, false);
    let path = pending::work(&settings, &root.package);
    save(&settings, vec![root]);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("steps.json"), "CANARY_INVALID_WORK").unwrap();
    assert!(matches!(
        export(&settings, &request(false), false).unwrap(),
        PortableDocument::Legacy(_)
    ));
    let error = export(&settings, &request(false), true)
        .unwrap_err()
        .to_string();
    assert!(!error.contains("CANARY_INVALID_WORK"));
    assert!(error.contains("unreadable"));
}

#[cfg(unix)]
fn context<'a>(
    settings: &'a SettingsRoot,
    progress: &'a vibe_core::progress::Progress,
) -> crate::OperationContext<'a> {
    crate::OperationContext {
        import_scope: None,
        settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts: crate::HostFacts::default(),
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: crate::AgentHooks::default(),
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
    }
}
#[cfg(unix)]
fn run_install(
    settings: &SettingsRoot,
    root: &Path,
    version: &str,
    optional: bool,
) -> Result<crate::Outcome, AppError> {
    let source = root.join(format!("source-{version}"));
    let payload = root.join(format!("payload-{version}"));
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&payload).unwrap();
    let text = format!(
        "[package]\ngroup='org.example'\nname='ready'\nkind='app'\nversion='{version}'\n[application]\nid='ready'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\nresult={{external='{}'}}\n[[installer_options.steps]]\nid='optional'\ntitle='Optional failure'\nkind='shell'\ncommand='false'\noptional={optional}\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        payload.display()
    );
    fs::write(source.join("vibe.toml"), &text).unwrap();
    let package = crate::from_manifest(
        vibe_core::manifest::Manifest::parse_str(&text).unwrap(),
        source,
        PackageCoordinate {
            group: "org.example".into(),
            name: "ready".into(),
            version: version.into(),
        },
    )
    .unwrap();
    let progress = vibe_core::progress::Progress::default();
    crate::install(
        &context(settings, &progress),
        &package,
        &crate::InstallRequest {
            keep_work: true,
            ..Default::default()
        },
    )
}

#[cfg(unix)]
#[test]
fn completed_old_version_work_after_actual_uninstall_is_not_pending_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(&dir.path().join("home"));
    run_install(&settings, dir.path(), "1.0.0", true).unwrap();
    run_install(&settings, dir.path(), "2.0.0", true).unwrap();
    let progress = vibe_core::progress::Progress::default();
    crate::uninstall(
        &context(&settings, &progress),
        "org.example/ready@1.0.0",
        &crate::RemoveRequest::default(),
    )
    .unwrap();
    let old = PackageCoordinate {
        group: "org.example".into(),
        name: "ready".into(),
        version: "1.0.0".into(),
    };
    assert!(
        !Index::load(&settings)
            .unwrap()
            .installations
            .contains_key(&old.key())
    );
    let retained = StepsFile::load(&pending::work(&settings, &old))
        .unwrap()
        .unwrap();
    assert_eq!(
        retained.pending(&settings).unwrap().phase,
        w::ApplicationPendingOperationPhase::Complete
    );
    let set = full(export(&settings, &request(false), true).unwrap());
    assert_eq!(set.roots, ["org.example/ready@2.0.0"]);
}

#[cfg(unix)]
#[test]
fn complete_optional_failure_is_ready_but_unindexed_replacement_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(&dir.path().join("home"));
    run_install(&settings, dir.path(), "1.0.0", true).unwrap();
    let coordinate = PackageCoordinate {
        group: "org.example".into(),
        name: "ready".into(),
        version: "1.0.0".into(),
    };
    let state = StepsFile::load(&pending::work(&settings, &coordinate))
        .unwrap()
        .unwrap();
    assert!(state.steps.iter().any(|s| s.outcome == StepOutcome::Failed));
    assert_eq!(
        state.pending(&settings).unwrap().phase,
        w::ApplicationPendingOperationPhase::Complete
    );
    assert_eq!(
        full(export(&settings, &request(false), true).unwrap())
            .application
            .len(),
        1
    );
    assert!(run_install(&settings, dir.path(), "2.0.0", false).is_err());
    let index = Index::load(&settings).unwrap();
    assert!(index.installations.contains_key("org.example/ready@1.0.0"));
    assert!(!index.installations.contains_key("org.example/ready@2.0.0"));
    assert!(export(&settings, &request(false), true).is_err());
    assert!(matches!(
        export(&settings, &request(false), false).unwrap(),
        PortableDocument::Legacy(_)
    ));
}
