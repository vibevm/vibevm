//! Deterministic crash cuts use the real operation/index writers and reread their files.
use super::*;
use crate::{AgentHooks, HostFacts, OperationContext};
use vibe_core::manifest::Manifest;

fn package(root: &Path, name: &str, requires: &str) -> ResolvedPackage {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    let text = format!(
        "[package]\ngroup='org.example'\nname='{name}'\nkind='app'\nversion='1.0.0'\n[application]\nid='{name}'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\nresult={{external='{}'}}\n{requires}\n[[installer_options.steps]]\nid='install'\ntitle='Install'\nkind='shell'\ncommand='true'\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        directory.display()
    );
    fs::write(directory.join("vibe.toml"), &text).unwrap();
    crate::from_manifest(
        Manifest::parse_str(&text).unwrap(),
        directory,
        PackageCoordinate {
            group: "org.example".into(),
            name: name.into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap()
}

#[test]
fn completed_child_cannot_bless_a_changed_record_at_the_first_parent_ack() {
    let root = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let child = package(root.path(), "child", "");
    let parent = package(
        root.path(),
        "parent",
        "requires=['org.example/child@=1.0.0']",
    );
    let progress = vibe_core::progress::Progress::default();
    let resolver = |name: &str| {
        assert_eq!(name, "org.example/child@=1.0.0");
        Ok(child.clone())
    };
    let ctx = OperationContext {
        http_client: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts: HostFacts::default(),
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress: &progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: AgentHooks::default(),
        resolver: Some(&resolver),
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let held = settings.lock().unwrap();
    let lock = &held;
    let request = InstallRequest::default();
    let plan = crate::plan(&ctx, &parent, &request).unwrap();
    let target = begin(
        lock,
        &ctx,
        &parent,
        &parent.options[0],
        &request,
        &plan,
        Checkpoint {
            kind: w::ApplicationPendingOperationKind::Install,
            retirements: Vec::new(),
            predecessor: None,
            parent: None,
        },
    )
    .unwrap();
    before_dependencies(&settings, &target).unwrap();
    let child_request = InstallRequest {
        mark: crate::Mark::Auto,
        ..Default::default()
    };
    // This is the real child writer, cut before the parent's normal ack call.
    super::super::install_locked(
        lock,
        &ctx,
        &child,
        &child_request,
        &mut vec![parent.package.key()],
        None,
        Some(Parent::Dependency {
            target: &target,
            requirement: "org.example/child@=1.0.0",
        }),
    )
    .unwrap();
    drop(held);
    let checkpoint = fs::read(work(&settings, &parent.package).join("steps.json")).unwrap();
    let mut index = Index::load(&settings).unwrap();
    let original = index.installations[&child.package.key()].clone();
    index
        .installations
        .get_mut(&child.package.key())
        .unwrap()
        .lifecycle
        .hidden = true;
    index
        .save(
            &settings,
            &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
        )
        .unwrap();
    // A new owner rereads both files; no retained in-memory record is the oracle.
    let reopened = SettingsRoot::new(settings.path());
    let held = reopened.lock().unwrap();
    let lock = &held;
    let result = ack_child(&reopened, &target, &child.package.key());
    assert!(
        result.is_err(),
        "a completed child receipt must not certify an unrelated changed same-key record"
    );
    assert_eq!(
        fs::read(work(&reopened, &parent.package).join("steps.json")).unwrap(),
        checkpoint
    );
    let mut index = Index::load(&reopened).unwrap();
    index.installations.insert(child.package.key(), original);
    index
        .save_locked(
            &reopened,
            &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
            lock,
        )
        .unwrap();
    ack_child(&reopened, &target, &child.package.key()).unwrap();
}

#[test]
fn saved_child_with_exact_backward_parent_binding_repairs_the_missing_forward_link() {
    let root = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let child = package(root.path(), "child", "");
    let parent = package(
        root.path(),
        "parent",
        "requires=['org.example/child@=1.0.0']",
    );
    let progress = vibe_core::progress::Progress::default();
    let resolver = |_: &str| Ok(child.clone());
    let ctx = OperationContext {
        http_client: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts: HostFacts::default(),
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress: &progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: AgentHooks::default(),
        resolver: Some(&resolver),
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let held = settings.lock().unwrap();
    let lock = &held;
    let request = InstallRequest::default();
    let parent_plan = crate::plan(&ctx, &parent, &request).unwrap();
    let target = begin(
        lock,
        &ctx,
        &parent,
        &parent.options[0],
        &request,
        &parent_plan,
        Checkpoint {
            kind: w::ApplicationPendingOperationKind::Install,
            retirements: Vec::new(),
            predecessor: None,
            parent: None,
        },
    )
    .unwrap();
    let child_request = InstallRequest {
        mark: crate::Mark::Auto,
        ..Default::default()
    };
    let child_plan = crate::plan(&ctx, &child, &child_request).unwrap();
    // Real checkpoint write; simulate loss of the caller before link_child.
    let child_target = begin(
        lock,
        &ctx,
        &child,
        &child.options[0],
        &child_request,
        &child_plan,
        Checkpoint {
            kind: w::ApplicationPendingOperationKind::Install,
            retirements: Vec::new(),
            predecessor: None,
            parent: Some(Parent::Dependency {
                target: &target,
                requirement: "org.example/child@=1.0.0",
            }),
        },
    )
    .unwrap();
    assert!(
        load(&settings, &parent.package, Some(&target.operation))
            .unwrap()
            .operation
            .unwrap()
            .children[0]
            .target
            .is_none()
    );
    assert_eq!(
        bound_child(&ctx, &target, &child.package.key()).unwrap(),
        Some(child_target.clone())
    );
    assert_eq!(
        load(&settings, &parent.package, Some(&target.operation))
            .unwrap()
            .operation
            .unwrap()
            .children[0]
            .target
            .as_ref(),
        Some(&child_target)
    );
    let mut wrong = target.clone();
    wrong.operation.push_str("-foreign");
    assert!(bound_child(&ctx, &wrong, &child.package.key()).is_err());
}

#[test]
fn retained_completed_child_is_visible_and_only_the_explicit_whole_cohort_can_be_cleaned() {
    let root = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let child = package(root.path(), "child", "");
    let parent = package(
        root.path(),
        "parent",
        "requires=['org.example/child@=1.0.0']",
    );
    let progress = vibe_core::progress::Progress::default();
    let resolver = |_: &str| Ok(child.clone());
    let ctx = OperationContext {
        http_client: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts: HostFacts::default(),
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress: &progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: AgentHooks::default(),
        resolver: Some(&resolver),
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let held = settings.lock().unwrap();
    let lock = &held;
    let request = InstallRequest::default();
    let plan = crate::plan(&ctx, &parent, &request).unwrap();
    let target = begin(
        lock,
        &ctx,
        &parent,
        &parent.options[0],
        &request,
        &plan,
        Checkpoint {
            kind: w::ApplicationPendingOperationKind::Install,
            retirements: Vec::new(),
            predecessor: None,
            parent: None,
        },
    )
    .unwrap();
    before_dependencies(&settings, &target).unwrap();
    super::super::install_locked(
        lock,
        &ctx,
        &child,
        &InstallRequest {
            mark: crate::Mark::Auto,
            ..Default::default()
        },
        &mut vec![parent.package.key()],
        None,
        Some(Parent::Dependency {
            target: &target,
            requirement: "org.example/child@=1.0.0",
        }),
    )
    .unwrap();
    drop(held);
    let view = crate::work(&settings, Some(&child.package.key())).unwrap();
    assert_eq!(
        view.entries[0].state,
        vibe_wire::generated::application_work::EntryState::Done
    );
    assert_eq!(
        view.entries[0].retained_for.as_ref().map(|r| &r.owner),
        Some(&application_reference(&target))
    );
    let parent_before = fs::read(work(&settings, &parent.package).join("steps.json")).unwrap();
    assert!(crate::clean_work(&settings, Some(&child.package.key())).is_err());
    assert_eq!(
        fs::read(work(&settings, &parent.package).join("steps.json")).unwrap(),
        parent_before
    );
    assert!(work(&settings, &child.package).exists());
    let index = fs::read(settings.index_path()).unwrap();
    let cleaned = crate::clean_work(&settings, None).unwrap();
    assert_eq!(cleaned.removed.len(), 2);
    assert!(!work(&settings, &child.package).exists());
    assert!(!work(&settings, &parent.package).exists());
    assert_eq!(fs::read(settings.index_path()).unwrap(), index);
}
