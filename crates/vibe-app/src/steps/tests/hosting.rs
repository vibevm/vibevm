//! Synthetic held observations exercise real report projections, not acquisition.
use super::*;

#[test]
fn hosting_follows_current_evidence_in_both_plan_and_describe_views() {
    let dir = tempfile::tempdir().unwrap();
    let mut resolved = package(dir.path(), BUILD);
    resolved.source_evidence = Some(serde_json::from_str(r#"{"source_url":"https://github.com/a","fetched_from":"git@gitverse.ru:a","acquisition":"git_repository"}"#).unwrap());
    let expected = vibe_registry::source_hosting(resolved.source_evidence.as_ref());
    let planned = run(dir.path(), &resolved, &host(), None).unwrap();
    let legacy = planned.to_wire().unwrap();
    assert_eq!(legacy.source_hosting, expected);
    assert_eq!(legacy.source_evidence, resolved.source_evidence);
    let description = crate::describe(&resolved).unwrap();
    assert_eq!(description.source_hosting, expected);
    assert_eq!(description.installed_hosting, None);
    assert_eq!(description.installed_source, None);
    let option = &mut resolved.options[0];
    option.features.clear();
    option.configuration = Some(Default::default());
    let planned = run(dir.path(), &resolved, &host(), None).unwrap();
    let common = planned.to_configuration_wire().unwrap();
    assert_eq!(common.source_hosting, expected);
    assert_eq!(common.source_evidence, resolved.source_evidence);
    let description = crate::configuration::describe(&resolved);
    assert_eq!(description.source_hosting, expected);
    assert_eq!(description.installed_hosting, None);
    assert_eq!(description.installed_source, None);
    resolved.source_evidence = None;
    let planned = run(dir.path(), &resolved, &host(), None).unwrap();
    assert_eq!(
        planned.to_configuration_wire().unwrap().source_hosting,
        None
    );
    assert_eq!(
        crate::configuration::describe(&resolved).source_hosting,
        None
    );
}

#[test]
fn unresolved_configuration_report_keeps_the_same_hosting_without_execution() {
    let dir = tempfile::tempdir().unwrap();
    let mut resolved = package(dir.path(), BUILD);
    resolved.options[0].platform = std::env::consts::OS.into();
    resolved.options[0].arch = std::env::consts::ARCH.into();
    resolved.options[0].features.clear();
    resolved.options[0].configuration = Some(toml::from_str("[[features]]\nid='a'\nname='A'\n[[features]]\nid='b'\nname='B'\n[[groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='exactly-one'\nmembers=['a','b']\n").unwrap());
    resolved.source_evidence = Some(
        serde_json::from_str(
            r#"{"source_url":"https://github.com/a","fetched_from":"https://gitverse.ru/a"}"#,
        )
        .unwrap(),
    );
    let settings = SettingsRoot::new(&dir.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let ctx = crate::OperationContext {
        import_scope: None,
        settings: &settings,
        json: true,
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
        resolver: None,
        resolver_scope: crate::ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
    };
    let request = crate::InstallRequest {
        configuration: Some(crate::configuration::parse_submission(br#"{"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","features":[],"defaultFeatures":false,"selections":{},"parameters":{}},"disabledFeatures":[]}}"#).unwrap()),
        ..Default::default()
    };
    let report = crate::configuration_plan(&ctx, &resolved, &request).unwrap();
    assert!(report.execution.is_none());
    assert_eq!(report.source_evidence, resolved.source_evidence);
    assert_eq!(
        report.source_hosting,
        vibe_registry::source_hosting(resolved.source_evidence.as_ref())
    );
}
