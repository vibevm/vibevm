//! Pure handoff: actual answers once, protected memory, changed authority refuses.
use super::*;
use std::cell::Cell;

#[test]
fn prepared_answers_are_once_secret_safe_and_reused_with_fresh_facts() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let text = format!(
        "[package]\nname='prepared'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n[application]\nid='prepared'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\n[installer_options.license]\nname='Fixture license'\nfile='LICENSE'\n[installer_options.npm_runtime]\nstep='fetch'\ninput='npm'\npackage='test-package'\nversion='1.0.0'\nnode_range='>=22.0.0'\n[[installer_options.features]]\nid='chosen'\nname='Chosen'\n[[installer_options.parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\nrequired=true\n[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='sources'\n[[installer_options.steps.sources]]\nname='npm'\nurl='https://example.invalid/package.tgz'\nsha256='{}'\n[[installer_options.steps]]\nid='install'\ntitle='Install'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'\nneeds=['command:npm']\n",
        "ab".repeat(32)
    );
    std::fs::write(source.join("vibe.toml"), &text).unwrap();
    std::fs::write(source.join("LICENSE"), "unchanged approved terms").unwrap();
    let resolved = crate::from_manifest(
        vibe_core::manifest::Manifest::parse_str(&text).unwrap(),
        source.clone(),
        PackageCoordinate {
            group: "org.example".into(),
            name: "prepared".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let feature_calls = Cell::new(0);
    let parameter_calls = Cell::new(0);
    let license_calls = Cell::new(0);
    let features = |_: &str, _: &[Choice], _: &[bool]| {
        feature_calls.set(feature_calls.get() + 1);
        Ok(Some(vec![0]))
    };
    let parameter = |_: &vibe_core::manifest::ParameterDecl, _: Option<&str>| {
        parameter_calls.set(parameter_calls.get() + 1);
        Ok(Some("SYNTHETIC_PRIVATE_ANSWER".into()))
    };
    let license = |_: &str, _: &str| {
        license_calls.set(license_calls.get() + 1);
        Ok(true)
    };
    let request = FeatureRequest::default();
    let input = PlanInput {
        import_scope: None,
        transfer: None,
        agent_modes: None,
        configuration: None,
        resolved: &resolved,
        option: &resolved.options[0],
        request: &request,
        parameters: &[],
        accept_license: false,
        previous: None,
        settings: &settings,
        facts: HostFacts::default(),
        offline: false,
        attended: true,
        prompts: Prompts {
            chooser: None,
            features: Some(&features),
            parameter: Some(&parameter),
            license: Some(&license),
        },
        agent: AgentHooks::default(),
        requirements: None,
        required_by: None,
    };
    let prepared = match prepare_install(&input, true).unwrap() {
        PreparedInstall::NeedsNode(p) => p,
        PreparedInstall::Ready(_) => panic!("missing npm must produce only typed intent"),
    };
    assert_eq!(
        (
            feature_calls.get(),
            parameter_calls.get(),
            license_calls.get()
        ),
        (1, 1, 1)
    );
    assert_eq!(prepared.command, "npm");
    assert!(prepared.secret_provided.contains(&"token".to_owned()));
    assert!(!format!("{prepared:?}").contains("SYNTHETIC_PRIVATE_ANSWER"));
    prepared.revalidate(&resolved).unwrap();
    let mut changed = resolved.clone();
    changed.options[0].name = "different option".into();
    assert!(prepared.revalidate(&changed).is_err());
    std::fs::write(source.join("LICENSE"), "new unapproved terms").unwrap();
    assert!(prepared.revalidate(&resolved).is_err());
    std::fs::write(source.join("LICENSE"), "unchanged approved terms").unwrap();
    std::fs::write(source.join("new-input"), "changed source").unwrap();
    assert!(prepared.revalidate(&resolved).is_err());
    std::fs::remove_file(source.join("new-input")).unwrap();
    let present = |_: &str| true;
    let continued = PlanInput {
        facts: HostFacts {
            command_present: &present,
            ..HostFacts::default()
        },
        ..input.clone()
    };
    let plan = prepared.continue_plan(&continued).unwrap();
    assert_eq!(
        (
            feature_calls.get(),
            parameter_calls.get(),
            license_calls.get()
        ),
        (1, 1, 1)
    );
    assert_eq!(plan.features, vec!["chosen"]);
    assert_eq!(plan.parameters[0].value, "SYNTHETIC_PRIVATE_ANSWER");
    assert!(!settings.index_path().exists());
    assert!(!settings.app_work_dir().exists());
    // The unchanged public planner cannot emit an install-only intent.
    assert_eq!(plan_with_no_remediation(&input).code(), "need_unmet");
    let changing_license = |_: &str, _: &str| {
        std::fs::write(source.join("LICENSE"), "changed during presentation").unwrap();
        Ok(true)
    };
    let changed_terms = PlanInput {
        prompts: Prompts {
            license: Some(&changing_license),
            ..input.prompts
        },
        ..input.clone()
    };
    assert!(prepare_install(&changed_terms, true).is_err());
    assert!(!settings.index_path().exists());
}

fn plan_with_no_remediation(input: &PlanInput<'_>) -> AppError {
    plan(input).unwrap_err()
}

#[test]
fn profile_and_explicit_feature_deltas_survive_handoff() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let text = format!(
        "[package]\nname='profiled'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n[application]\nid='profiled'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\n[installer_options.npm_runtime]\nstep='fetch'\ninput='npm'\npackage='test-package'\nversion='1.0.0'\nnode_range='>=22.0.0'\n[[installer_options.features]]\nid='a'\nname='A'\n[[installer_options.features]]\nid='b'\nname='B'\n[[installer_options.profiles]]\nid='typical'\nname='Typical'\nfeatures=['a']\n[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='sources'\n[[installer_options.steps.sources]]\nname='npm'\nurl='https://example.invalid/pkg.tgz'\nsha256='{}'\n[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'\nneeds=['command:npm']\n",
        "ab".repeat(32)
    );
    std::fs::write(source.join("vibe.toml"), &text).unwrap();
    let resolved = crate::from_manifest(
        vibe_core::manifest::Manifest::parse_str(&text).unwrap(),
        source,
        PackageCoordinate {
            group: "org.example".into(),
            name: "profiled".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let request = FeatureRequest {
        enable: vec!["b".into()],
        disable: vec!["a".into()],
        profile: Some("typical".into()),
    };
    let input = PlanInput {
        import_scope: None,
        transfer: None,
        agent_modes: None,
        configuration: None,
        resolved: &resolved,
        option: &resolved.options[0],
        request: &request,
        parameters: &[],
        accept_license: false,
        previous: None,
        settings: &settings,
        facts: HostFacts::default(),
        offline: false,
        attended: false,
        prompts: Prompts::default(),
        agent: AgentHooks::default(),
        requirements: None,
        required_by: None,
    };
    let p = match prepare_install(&input, true).unwrap() {
        PreparedInstall::NeedsNode(p) => p,
        _ => panic!("missing runtime"),
    };
    assert_eq!(p.features, vec!["b"]);
    let available = |_: &str| true;
    let current = PlanInput {
        facts: HostFacts {
            command_present: &available,
            ..HostFacts::default()
        },
        ..input.clone()
    };
    let plan = p.continue_plan(&current).unwrap();
    assert_eq!(plan.features, vec!["b"]);
    assert_eq!(plan.profile.as_deref(), Some("typical"));
}
