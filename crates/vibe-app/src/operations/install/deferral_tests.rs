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
