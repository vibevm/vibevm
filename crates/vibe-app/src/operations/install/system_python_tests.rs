//! Selected interpreter refusal must precede archive fetch and durable work.
use super::*;
use std::cell::Cell;
use std::path::Path;

#[test]
fn system_python_refuses_before_target_effects_and_legacy_never_probes() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    let text = "[package]\ngroup='org.example'\nname='python-app'\nversion='1.0.0'\nkind='app'\n[application]\nid='python-app'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\n[installer_options.system_python]\nimplementation='cpython'\nmajor=3\nminor=14\nsoabi='cpython-314-x86_64-linux-gnu'\n[installer_options.installation]\ntype='tar_gz'\nsource='https://example.invalid/never-fetch.tgz'\nsha256='abababababababababababababababababababababababababababababababab'\n";
    std::fs::write(package.join("vibe.toml"), text).unwrap();
    let resolved = crate::from_manifest(
        vibe_core::manifest::Manifest::parse_str(text).unwrap(),
        package,
        crate::PackageCoordinate {
            group: "org.example".into(),
            name: "python-app".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let settings = crate::SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let chooser = |_: &str, _: &[crate::Choice]| Ok(Some(0));
    let calls = Cell::new(0);
    let wrong = || {
        calls.set(calls.get() + 1);
        Ok(crate::SystemPythonObservation {
            executable: "/usr/bin/python3.14".into(),
            implementation: "cpython".into(),
            major: 3,
            minor: 14,
            soabi: "wrong-abi".into(),
            prefix: "/usr".into(),
            base_prefix: "/usr".into(),
        })
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
        facts: crate::HostFacts {
            system_python: Some(&wrong),
            ..Default::default()
        },
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
    let error = install(&ctx, &resolved, &request).unwrap_err();
    assert_eq!(error.code(), "runtime_remediation_refused");
    assert_eq!(calls.get(), 1);
    assert!(!settings.index_path().exists());
    assert!(!settings.app_work_dir().exists());
    assert!(!settings.path().join("opt/bin").exists());
    let missing = OperationContext {
        facts: crate::HostFacts::default(),
        ..ctx
    };
    assert_eq!(
        install(&missing, &resolved, &request).unwrap_err().code(),
        "runtime_remediation_refused"
    );
    let mut legacy = resolved.clone();
    legacy.options[0].system_python = None;
    // Pure plan still works with an ordinary archive input; no host observation.
    assert!(crate::plan(&ctx, &legacy, &request).is_ok());
    assert_eq!(calls.get(), 1);
}
