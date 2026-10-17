//! Source-backed F2 stories driven through the normal planner and apply.
use std::{collections::BTreeSet, path::Path};
use vibe_core::manifest::{
    Lockfile, Manifest, ManifestMigrationOutcome, MigrationSelection, SpecFormat,
};
use vibe_install as install;
use vibe_install::{InstallSource, NullObserver, Plan};
use vibe_resolver as resolver;
#[path = "../../../vibevm/vibepacks/org.vibevm.world/redbook/v1.1.0/tests/f1_fixture.rs"]
mod fixture;
const OLD: &str = include_str!("../tests/fixtures/m_05_f_2_old.toml");

fn migrate(text: &str) -> vibe_core::manifest::ManifestMigrationPlan {
    match vibe_core::manifest::plan_manifest_migration(
        text,
        MigrationSelection {
            source_profile: Some(vibe_core::manifest::redbook::LEGACY),
            target_profile: Some(vibe_core::manifest::redbook::SELECTED),
            recipe: Some(vibe_core::manifest::redbook::RECIPE),
            source_decisions: &[],
        },
    ) {
        ManifestMigrationOutcome::Planned(p) => p,
        other => panic!("{other:?}"),
    }
}
fn request() -> install::InstallRequest {
    install::InstallRequest {
        roots: vec![],
        features: Default::default(),
        language: None,
        exact: false,
        generated_by: "F2 isolated fixture".into(),
        offline: true,
    }
}
fn source(registry: &Path, root: &Path) -> vibe_package_source::InstallResolver {
    vibe_package_source::build_install_resolver(
        &vibe_package_source::PackageSourceOptions {
            registry: Some(registry.into()),
            ..Default::default()
        },
        &Manifest::read(root.join("vibe.toml")).unwrap(),
        None,
        root,
        &Default::default(),
        true,
        &[],
    )
    .unwrap()
}
fn plan(source: &impl InstallSource, root: &Path) -> install::PlannedInstall {
    let Plan::Ready(p) = install::plan(source, root, request(), &NullObserver).unwrap() else {
        panic!("ready");
    };
    *p
}
fn names(plan: &install::PlannedInstall) -> BTreeSet<String> {
    plan.resolution.iter().map(|f| f.name.to_string()).collect()
}
fn apply(source: &impl InstallSource, plan: install::PlannedInstall) {
    install::apply(
        source,
        plan,
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap();
}

#[test]
fn m_05_f_2_normal_migration_preserves_old_graph_and_comments() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    fixture::registry(&registry);
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), OLD).unwrap();
    let rich_bytes =
        std::fs::read(registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml")).unwrap();
    std::fs::write(
        registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml"),
        include_str!("../tests/fixtures/m_05_f_2_redbook_old.toml"),
    )
    .unwrap();
    let old_source = source(&registry, &root);
    let before = plan(&old_source, &root);
    std::fs::write(
        registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml"),
        rich_bytes,
    )
    .unwrap();
    let migration = migrate(OLD);
    for comment in OLD.lines().filter(|l| l.trim_start().starts_with('#')) {
        assert!(migration.after.contains(comment));
    }
    assert_eq!(
        migration.checked_manifest.requires.packages,
        Manifest::parse_str(OLD).unwrap().requires.packages
    );
    std::fs::write(root.join("vibe.toml"), &migration.after).unwrap();
    let new_source = source(&registry, &root);
    // No fixture-constructed B2 request: ordinary planner reads the manifest.
    let after = plan(&new_source, &root);
    assert_eq!(names(&before), names(&after));
    let edges = |p: &install::PlannedInstall| {
        p.resolution
            .iter()
            .flat_map(|node| {
                node.requires.iter().map(move |(group, name)| {
                    (
                        format!("{}/{}", node.group, node.name),
                        format!("{group}/{name}"),
                    )
                })
            })
            .collect::<BTreeSet<_>>()
    };
    let mut old_edges = edges(&before);
    // New explicit Redbook continuity shares the already independently
    // required MUP node; every old selected dependency edge is retained.
    old_edges.insert((
        "org.vibevm.world/redbook".into(),
        "org.vibevm.world/multi-user-planning".into(),
    ));
    assert_eq!(old_edges, edges(&after));
    eprintln!(
        "F2 graph: before/after packages={}, old edges={}, new edges={}; added Redbook-to-existing-MUP edge",
        names(&after).len(),
        edges(&before).len(),
        edges(&after).len()
    );
    assert_eq!(
        names(&after),
        fixture::expected_names(
            "mup",
            &[
                "health-audit",
                "manual-tests",
                "comparative-research",
                "dev-runtime-docs",
                "discovery-prompt"
            ]
        )
    );
    let retained = after.clone();
    let current = new_source
        .bind_request(after.normalized_request.clone().unwrap())
        .unwrap();
    apply(&new_source, after);
    let lock = Lockfile::read(root.join("vibe.lock")).unwrap();
    let replay = install::evolution::PreparedReinstall::from_planned(
        &lock,
        &current,
        &retained,
        current.catalog_inspection().unwrap(),
    )
    .unwrap();
    assert_eq!(replay.resolution().len(), lock.packages.len());
    assert_eq!(
        replay.report.decision,
        install::evolution::Decision::ExactReuse
    );
}

#[test]
fn m_05_f_2_switch_back_restores_dormant_additions_and_shared_root() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    fixture::registry(&registry);
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let migrated = migrate(OLD);
    let mut manifest = migrated.checked_manifest;
    // A second workspace root independently needs WAL, even under MUP.
    manifest.workspace = Some(Default::default());
    manifest.workspace.as_mut().unwrap().members = vec!["member".into()];
    std::fs::create_dir(root.join("member")).unwrap();
    std::fs::write(root.join("member/vibe.toml"), "[project]\nname='shared'\nversion='1.1.0'\n[requires.packages]\n'org.vibevm.world/wal'='=1.1.0'\n").unwrap();
    let mut initial = None;
    for branch in ["mup", "wal", "mup"] {
        let row = &mut manifest.consumer_configuration[0];
        row.selections.get_mut("continuity").unwrap().members = Some(vec![branch.into()]);
        // Preserve both branch selections even while one is dormant.
        row.selections.get_mut("wal-additions").unwrap().members =
            Some(vec!["wal-specspaces".into()]);
        manifest.write(root.join("vibe.toml")).unwrap();
        let src = source(&registry, &root);
        let p = install::evolution::plan_reconfiguration(
            &src,
            &root,
            request(),
            SpecFormat::Mixed,
            &NullObserver,
        )
        .unwrap();
        eprintln!("F2 branch {branch}: complete reconfiguration plan prepared");
        let mut expected = fixture::expected_names(
            branch,
            if branch == "mup" {
                &[
                    "health-audit",
                    "manual-tests",
                    "comparative-research",
                    "dev-runtime-docs",
                    "discovery-prompt",
                ]
            } else {
                &[
                    "health-audit",
                    "manual-tests",
                    "comparative-research",
                    "dev-runtime-docs",
                    "wal-specspaces",
                ]
            },
        );
        expected.insert("multi-user-planning".into());
        expected.insert("wal".into());
        assert_eq!(names(&p), expected);
        let record = install::requested_record(p.normalized_request.as_ref().unwrap()).unwrap();
        assert!(record.defaults.is_empty());
        assert!(
            record
                .configuration
                .as_ref()
                .unwrap()
                .contains("discovery-prompt")
        );
        assert!(
            record
                .configuration
                .as_ref()
                .unwrap()
                .contains("wal-specspaces")
        );
        let configuration = p.configuration_record().unwrap().unwrap();
        let (_, selected) =
            vibe_wire::behaviour::lock_configuration::decode(&configuration).unwrap();
        let effects = &selected
            .packages
            .iter()
            .find(|p| p.package == "org.vibevm.world/redbook@1.1.0")
            .unwrap()
            .feature_closure;
        assert_eq!(
            effects.iter().any(|f| f == "discovery-prompt"),
            branch == "mup"
        );
        assert_eq!(
            effects.iter().any(|f| f == "wal-specspaces"),
            branch == "wal"
        );
        if branch == "mup" {
            if let Some(old) = &initial {
                assert_eq!(old, &record);
            } else {
                initial = Some(record);
            }
        }
        let retained = p.clone();
        let bound = src
            .bind_request(p.normalized_request.clone().unwrap())
            .unwrap();
        apply(&src, p);
        let lock = Lockfile::read(root.join("vibe.lock")).unwrap();
        let redbook = lock.packages.iter().find(|p| p.name == "redbook").unwrap();
        let slot = vibe_workspace::vibedeps::slot_abs_path(
            &root,
            &redbook.group,
            "redbook",
            &redbook.version,
        );
        assert_eq!(
            std::fs::read(slot.join("vibe.toml")).unwrap(),
            std::fs::read(registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml")).unwrap()
        );
        let (_, resolved) = vibe_wire::behaviour::lock_configuration::decode(
            lock.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        let dormant = &resolved.configurations[0].dormant;
        let dormant_json = serde_json::to_string(dormant).unwrap();
        assert!(dormant_json.contains(if branch == "mup" {
            "wal-specspaces"
        } else {
            "discovery-prompt"
        }));
        let replay = install::evolution::PreparedReinstall::from_planned(
            &lock,
            &bound,
            &retained,
            bound.catalog_inspection().unwrap(),
        )
        .unwrap();
        replay.validate(&lock).unwrap();
        assert_eq!(replay.resolution().len(), lock.packages.len());
        eprintln!("F2 branch {branch}: applied, dormant intent retained, exact replay validated");
    }
}

#[test]
fn m_05_f_2_actual_root_conversion_preserves_unrelated_settings() {
    let old = include_str!("../tests/fixtures/m_05_f_2_root_old.toml");
    let proposal = migrate(old);
    let before = Manifest::parse_str(old).unwrap();
    let after = &proposal.checked_manifest;
    assert_eq!(before.requires.packages, after.requires.packages);
    assert_eq!(before.requires.links, after.requires.links);
    assert_eq!(before.project, after.project);
    assert_eq!(before.registries, after.registries);
    assert_eq!(before.extensions, after.extensions);
    assert_eq!(before.run_commands, after.run_commands);
    for line in old.lines().filter(|l| l.trim_start().starts_with('#')) {
        assert!(proposal.after.contains(line));
    }
    assert_eq!(
        after.consumer_configuration[0].selections["additions"]
            .members
            .as_ref()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        after.consumer_configuration[0].selections["mup-additions"].members,
        Some(vec!["discovery-prompt".into()])
    );
    let actual = Manifest::parse_str(include_str!("../../../vibe.toml")).unwrap();
    assert_eq!(actual.consumer_configuration, after.consumer_configuration);
    assert_eq!(actual.requires, after.requires);
}

#[test]
fn m_05_f_2_removed_option_diagnoses_without_dropping_authored_intent() {
    let _isolation = vibe_test_support::isolated_home().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    fixture::registry(&registry);
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let migration = migrate(OLD);
    std::fs::write(root.join("vibe.toml"), &migration.after).unwrap();
    let src = source(&registry, &root);
    let p = plan(&src, &root);
    apply(&src, p);
    // Positive normal update entry, before changing the authoritative source.
    let mut unchanged_workspace = vibe_workspace::Workspace::discover(&root).unwrap();
    let mut unchanged_manifest = Manifest::read(root.join("vibe.toml")).unwrap();
    let unchanged_source = source(&registry, &root);
    let unchanged = install::evolution::plan_update(
        &unchanged_source,
        &root,
        &mut unchanged_manifest,
        &mut unchanged_workspace,
        request(),
        SpecFormat::Mixed,
        &NullObserver,
    )
    .unwrap();
    assert!(unchanged.report.needs.is_empty(), "{:?}", unchanged.report);
    assert!(unchanged.plan.is_some());
    let package = registry.join("org.vibevm.world/redbook/v1.1.0/vibe.toml");
    let changed = std::fs::read_to_string(&package)
        .unwrap()
        .replace(
            "members = [\"discovery-prompt\"]",
            "members = [\"replacement-research\"]",
        )
        .replace(
            "[features.effects]\n",
            "[features.effects]\nreplacement-research = []\n",
        );
    std::fs::write(package, changed).unwrap();
    let mut workspace = vibe_workspace::Workspace::discover(&root).unwrap();
    let mut manifest = Manifest::read(root.join("vibe.toml")).unwrap();
    let current = source(&registry, &root);
    let before_manifest = std::fs::read(root.join("vibe.toml")).unwrap();
    let before_lock = std::fs::read(root.join("vibe.lock")).unwrap();
    let update = install::evolution::plan_update(
        &current,
        &root,
        &mut manifest,
        &mut workspace,
        request(),
        SpecFormat::Mixed,
        &NullObserver,
    )
    .unwrap();
    assert!(update.plan.is_none());
    assert!(
        update
            .report
            .value()
            .to_string()
            .contains("discovery-prompt")
    );
    assert_eq!(
        std::fs::read(root.join("vibe.toml")).unwrap(),
        before_manifest
    );
    assert_eq!(std::fs::read(root.join("vibe.lock")).unwrap(), before_lock);
}
