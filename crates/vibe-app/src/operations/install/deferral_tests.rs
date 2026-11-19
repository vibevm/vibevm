//! The caller decision observes the single preflight, never recursive or retained work.
use super::*;
use std::cell::Cell;
use std::path::Path;
use vibe_core::manifest::Manifest;

#[test]
fn missing_command_decision_is_once_and_resume_is_not_relabelled() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    let text = format!(
        "[package]\ngroup='org.example'\nname='deferral'\nversion='1.0.0'\nkind='app'\n[application]\nid='deferral'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\n[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'\nneeds=['command:synthetic-npm']\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    std::fs::write(package.join("vibe.toml"), &text).unwrap();
    let resolved = crate::from_manifest(
        Manifest::parse_str(&text).unwrap(),
        package,
        crate::PackageCoordinate {
            group: "org.example".into(),
            name: "deferral".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = crate::SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let decisions = Cell::new(0);
    let selection = Cell::new(0);
    let chooser = |_: &str, _: &[crate::Choice]| {
        selection.set(selection.get() + 1);
        Ok(Some(0))
    };
    let decide = |key: &str, step: &str, command: &str| {
        assert_eq!(
            (key, step, command),
            ("org.example/deferral@1.0.0", "copy", "synthetic-npm")
        );
        decisions.set(decisions.get() + 1);
        true
    };
    let ctx = OperationContext {
        missing_command: Some(&decide),
        http_client: None,
        download_headers: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: Some(&chooser),
        proxy_image: Path::new("unused"),
        facts: crate::HostFacts::default(),
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
        agent: crate::AgentHooks::default(),
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let error = install(&ctx, &resolved, &InstallRequest::default()).unwrap_err();
    assert_eq!(error.code(), "prerequisite_deferred");
    assert_eq!(decisions.get(), 1);
    assert_eq!(
        selection.get(),
        2,
        "one platform and one architecture question, no second plan"
    );
    assert!(!settings.index_path().exists());
    assert!(!settings.path().join("cache/app-work").exists());
    let resumed = InstallRequest {
        start: crate::Start::Resume,
        ..Default::default()
    };
    let error = install(&ctx, &resolved, &resumed).unwrap_err();
    assert_eq!(error.code(), "need_unmet");
    assert_eq!(decisions.get(), 1, "resume must not invoke remediation");
}

#[test]
fn node_handoff_releases_lock_and_missing_input_never_records_success() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    let text = format!(
        "[package]\ngroup='org.example'\nname='deferral'\nversion='1.0.0'\nkind='app'\n[application]\nid='deferral'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\n[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'\nneeds=['command:npm']\n[installer_options.npm_runtime]\nstep='fetch'\ninput='npm'\npackage='test-package'\nversion='1.0.0'\nnode_range='>=22.0.0'\n[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='sources'\n[[installer_options.steps.sources]]\nname='npm'\npath='input.tgz'\nsha256='abababababababababababababababababababababababababababababababab'\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    std::fs::write(package.join("vibe.toml"), &text).unwrap();
    let resolved = crate::from_manifest(
        Manifest::parse_str(&text).unwrap(),
        package,
        crate::PackageCoordinate {
            group: "org.example".into(),
            name: "deferral".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = crate::SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let selection = Cell::new(0);
    let chooser = |_: &str, _: &[crate::Choice]| {
        selection.set(selection.get() + 1);
        Ok(Some(0))
    };
    let ctx = OperationContext {
        missing_command: None,
        http_client: None,
        download_headers: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: Some(&chooser),
        proxy_image: Path::new("unused"),
        facts: crate::HostFacts::default(),
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
        agent: crate::AgentHooks::default(),
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let request = InstallRequest::default();
    let token = match install_preparing_node(&ctx, &resolved, &request).unwrap() {
        InstallPreparation::NeedsNode(token) => token,
        InstallPreparation::Completed(_) => panic!("missing npm must not install target"),
    };
    assert_eq!(token.missing_command(), "npm");
    assert_eq!(token.option_id(), "main");
    assert_eq!(token.runtime().package, "test-package");
    assert_eq!(selection.get(), 2);
    assert!(!settings.index_path().exists());
    assert!(!settings.app_work_dir().exists());
    // Returned token holds no lock. A failed provider can simply drop it.
    drop(settings.lock().unwrap());
    let present = |_: &str| true;
    let current = OperationContext {
        chooser: None,
        facts: crate::HostFacts {
            command_present: &present,
            ..Default::default()
        },
        ..ctx
    };
    // Only execute the already-planned copy step; the unselected source input
    // is metadata for this pure handoff test, not real npm acquisition.
    let outcome = continue_after_node(&current, &resolved, *token);
    assert!(
        outcome.is_err(),
        "the missing input must refuse rather than fake provider success"
    );
    assert!(!settings.index_path().exists());
    assert_eq!(selection.get(), 2, "continuation must not repeat selection");
}

#[test]
fn node_handoff_positive_installs_one_target_after_fresh_facts() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    let data = b"synthetic verified input";
    std::fs::write(package.join("input.tgz"), data).unwrap();
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(data));
    let text = format!(
        "[package]\ngroup='org.example'\nname='deferral'\nversion='1.0.0'\nkind='app'\n[application]\nid='deferral'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\n[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'\nneeds=['command:npm']\n[installer_options.npm_runtime]\nstep='fetch'\ninput='npm'\npackage='test-package'\nversion='1.0.0'\nnode_range='>=22.0.0'\n[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='sources'\n[[installer_options.steps.sources]]\nname='npm'\npath='input.tgz'\nsha256='{digest}'\nextract=false\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    std::fs::write(package.join("vibe.toml"), &text).unwrap();
    let resolved = crate::from_manifest(
        Manifest::parse_str(&text).unwrap(),
        package,
        crate::PackageCoordinate {
            group: "org.example".into(),
            name: "deferral".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = crate::SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let selection = Cell::new(0);
    let chooser = |_: &str, _: &[crate::Choice]| {
        selection.set(selection.get() + 1);
        Ok(Some(0))
    };
    let ctx = OperationContext {
        missing_command: None,
        http_client: None,
        download_headers: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: Some(&chooser),
        proxy_image: Path::new("unused"),
        facts: crate::HostFacts::default(),
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
        agent: crate::AgentHooks::default(),
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let request = InstallRequest::default();
    let token = match install_preparing_node(&ctx, &resolved, &request).unwrap() {
        InstallPreparation::NeedsNode(token) => token,
        InstallPreparation::Completed(_) => panic!("missing npm must not install target"),
    };
    assert_eq!(token.missing_command(), "npm");
    assert_eq!(token.option_id(), "main");
    assert_eq!(token.runtime().package, "test-package");
    assert_eq!(selection.get(), 2);
    assert!(!settings.index_path().exists());
    assert!(!settings.app_work_dir().exists());
    // Returned token holds no lock. A failed provider can simply drop it.
    drop(settings.lock().unwrap());
    let present = |_: &str| true;
    let current = OperationContext {
        chooser: None,
        facts: crate::HostFacts {
            command_present: &present,
            ..Default::default()
        },
        ..ctx
    };
    let outcome = continue_after_node(&current, &resolved, *token).unwrap();
    assert_eq!(outcome.key, "org.example/deferral@1.0.0");
    let index = Index::load(&settings).unwrap();
    assert_eq!(index.installations.len(), 1);
    let installed = &index.installations[&outcome.key];
    assert_eq!(installed.option, "main");
    let completed = installed
        .steps
        .iter()
        .filter(|step| step.id == "copy")
        .count();
    assert_eq!(completed, 1);
    assert_eq!(selection.get(), 2, "no repeated chooser on continuation");
}

#[test]
fn node_handoff_rejects_concurrently_installed_target() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    let data = b"synthetic verified input";
    std::fs::write(package.join("input.tgz"), data).unwrap();
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(data));
    let text = format!(
        "[package]\ngroup='org.example'\nname='deferral'\nversion='1.0.0'\nkind='app'\n[application]\nid='deferral'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\n[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'\nneeds=['command:npm']\n[installer_options.npm_runtime]\nstep='fetch'\ninput='npm'\npackage='test-package'\nversion='1.0.0'\nnode_range='>=22.0.0'\n[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='sources'\n[[installer_options.steps.sources]]\nname='npm'\npath='input.tgz'\nsha256='{digest}'\nextract=false\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    std::fs::write(package.join("vibe.toml"), &text).unwrap();
    let resolved = crate::from_manifest(
        Manifest::parse_str(&text).unwrap(),
        package,
        crate::PackageCoordinate {
            group: "org.example".into(),
            name: "deferral".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = crate::SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let selection = Cell::new(0);
    let chooser = |_: &str, _: &[crate::Choice]| {
        selection.set(selection.get() + 1);
        Ok(Some(0))
    };
    let ctx = OperationContext {
        missing_command: None,
        http_client: None,
        download_headers: None,
        import_scope: None,
        settings: &settings,
        json: false,
        offline: true,
        chooser: Some(&chooser),
        proxy_image: Path::new("unused"),
        facts: crate::HostFacts::default(),
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
        agent: crate::AgentHooks::default(),
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
        user_config: None,
    };
    let request = InstallRequest::default();
    let token = match install_preparing_node(&ctx, &resolved, &request).unwrap() {
        InstallPreparation::NeedsNode(token) => token,
        InstallPreparation::Completed(_) => panic!("missing npm must not install target"),
    };
    assert_eq!(token.missing_command(), "npm");
    assert_eq!(token.option_id(), "main");
    assert_eq!(token.runtime().package, "test-package");
    assert_eq!(selection.get(), 2);
    assert!(!settings.index_path().exists());
    assert!(!settings.app_work_dir().exists());
    // Returned token holds no lock. A failed provider can simply drop it.
    drop(settings.lock().unwrap());
    let present = |_: &str| true;
    let current = OperationContext {
        chooser: None,
        facts: crate::HostFacts {
            command_present: &present,
            ..Default::default()
        },
        ..ctx
    };
    let installed = install(&current, &resolved, &request).unwrap();
    let before = std::fs::read(settings.index_path()).unwrap();
    let fault = continue_after_node(&current, &resolved, *token).unwrap_err();
    assert!(
        fault
            .to_string()
            .contains("target installation or default changed")
    );
    assert_eq!(std::fs::read(settings.index_path()).unwrap(), before);
    assert_eq!(Index::load(&settings).unwrap().installations.len(), 1);
    assert_eq!(installed.key, "org.example/deferral@1.0.0");
}
