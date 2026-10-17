#[path = "m_05_e_2_fixture.rs"]
mod fixture;
use fixture::*;
use vibe_install::{InstallSource, evolution::*};

#[test]
fn prepared_replay_binds_leaf_capture_without_solve() {
    let f = Fixture::new();
    let (lock, plan) = f.installed();
    let prepared = f.replay(&lock, &plan);
    assert_eq!(prepared.report.decision, Decision::ExactReuse);
    assert_eq!(prepared.resolution().len(), 1);
    prepared.validate(&lock).unwrap();
    let source = f.source.bind_request(f.request.clone()).unwrap();
    let mut missing = source.catalog_inspection().unwrap().clone();
    missing.candidates.clear();
    let refused = PreparedReinstall::from_planned(&lock, &source, &plan, &missing)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("capture missing"));
    assert!(refused.contains("\"decision\":\"Unavailable\""));
    assert!(refused.contains("\"captures\""));
    let mut no_request = lock.clone();
    no_request.meta.configuration = None;
    let refused = PreparedReinstall::from_planned(
        &no_request,
        &source,
        &plan,
        source.catalog_inspection().unwrap(),
    )
    .unwrap_err()
    .to_string();
    assert!(refused.contains("\"decision\":\"Unavailable\""));
}

#[test]
fn update_keeps_explicit_choice_when_default_changes() {
    let mut f = Fixture::new();
    let (lock, _) = f.installed();
    f.request.configuration.as_mut().unwrap().definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .defaults = Some([feature("b")].into());
    let result = f.update();
    assert_eq!(result.report.decision, Decision::SemanticInputChange);
    assert!(result.report.needs.is_empty());
    let Some(vibe_install::Plan::Ready(plan)) = result.plan else {
        panic!("ready update")
    };
    let source = f.source.bind_request(f.request.clone()).unwrap();
    vibe_install::apply(
        &source,
        *plan,
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap();
    let now = vibe_core::manifest::Lockfile::read(f.root.join("vibe.lock")).unwrap();
    assert_eq!(now.packages[0].features, vec!["a"]);
    let (before, _) =
        vibe_wire::behaviour::lock_configuration::decode(lock.meta.configuration.as_ref().unwrap())
            .unwrap();
    let (after, _) =
        vibe_wire::behaviour::lock_configuration::decode(now.meta.configuration.as_ref().unwrap())
            .unwrap();
    assert_eq!(before.configuration, after.configuration);
    assert!(result.report.value().to_string().contains("later"));
}

#[test]
fn removed_option_and_new_required_group_return_all_needs_without_writes() {
    let mut f = Fixture::new();
    f.installed();
    let before = std::fs::read(f.root.join("vibe.lock")).unwrap();
    let d = &mut f.request.configuration.as_mut().unwrap().definitions[0].definition;
    d.constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .members
        .remove(&feature("a"));
    d.constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .defaults = None;
    d.program
        .features
        .insert(feature("new"), Default::default());
    d.constraints.groups.insert(
        feature("required"),
        vibe_core::manifest::feature_constraints::ConstrainedGroup {
            cardinality: vibe_core::manifest::feature_constraints::Cardinality::ExactlyOne,
            members: [feature("new")].into(),
            guards: vec![],
            defaults: None,
        },
    );
    let result = f.update();
    assert_eq!(result.report.decision, Decision::ReconfigurationRequired);
    assert!(result.plan.is_none());
    assert!(
        result
            .report
            .needs
            .iter()
            .any(|n| n.contains("removed member"))
    );
    assert!(
        result
            .report
            .needs
            .iter()
            .any(|n| n.contains("new required group required"))
    );
    assert_eq!(before, std::fs::read(f.root.join("vibe.lock")).unwrap());
}

#[test]
fn force_cannot_admit_changed_payload_or_lock() {
    let f = Fixture::new();
    let (lock, plan) = f.installed();
    let prepared = f.replay(&lock, &plan);
    let mut drift = lock.clone();
    drift.packages[0].features = vec!["b".into()];
    assert!(prepared.validate(&drift).is_err());
    std::fs::write(
        prepared.resolution()[0].content_dir.join("payload.txt"),
        "changed",
    )
    .unwrap();
    assert!(prepared.validate(&lock).is_err());
}

#[test]
fn invalid_newer_definition_does_not_discard_valid_older_candidate() {
    let mut f = Fixture::new();
    f.installed();
    let old = f.request.configuration.as_ref().unwrap().definitions[0].clone();
    let group = old.package.group.as_ref().unwrap();
    let directory = f.temp.path().join(format!("registry/{group}/app/v2.0.0"));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("vibe.toml"),
        format!("[package]\ngroup='{group}'\nname='app'\nkind='flow'\nversion='2.0.0'\n"),
    )
    .unwrap();
    let metadata = vibe_registry::CandidateMetadata::read_local_registry(&directory).unwrap();
    let mut newer = old;
    newer.version = semver::Version::new(2, 0, 0);
    newer.identity = metadata.identity;
    let choice = newer
        .definition
        .constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap();
    choice.members = [feature("b")].into();
    choice.defaults = Some([feature("b")].into());
    f.request
        .configuration
        .as_mut()
        .unwrap()
        .definitions
        .push(newer);
    let result = f.update();
    assert!(result.report.needs.is_empty());
    let Some(vibe_install::Plan::Ready(plan)) = result.plan else {
        panic!("valid older candidate")
    };
    assert_eq!(plan.resolution[0].version, semver::Version::new(1, 0, 0));
}

#[test]
fn update_retains_explicit_empty_and_dormant_intent() {
    let mut f = Fixture::new();
    let c = f.request.configuration.as_mut().unwrap();
    c.definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .cardinality = vibe_core::manifest::feature_constraints::Cardinality::ZeroOrOne;
    c.request.edges[0]
        .selections
        .get_mut("choice")
        .unwrap()
        .members = Some(vec![]);
    let (lock, _) = f.installed();
    f.request.configuration.as_mut().unwrap().definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .defaults = Some([feature("b")].into());
    let update = f.update();
    let Some(vibe_install::Plan::Ready(plan)) = update.plan else {
        panic!("explicit empty remains selected")
    };
    assert_eq!(plan.resolution.len(), 1);
    let (_, resolved) =
        vibe_wire::behaviour::lock_configuration::decode(lock.meta.configuration.as_ref().unwrap())
            .unwrap();
    assert!(resolved.packages[0].feature_closure.is_empty());
    assert!(update.report.value().to_string().contains("later"));
    let source = f.source.bind_request(f.request.clone()).unwrap();
    vibe_install::apply(
        &source,
        *plan,
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap();
    let after = vibe_core::manifest::Lockfile::read(f.root.join("vibe.lock")).unwrap();
    assert!(after.packages[0].features.is_empty());
}
