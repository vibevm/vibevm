#[path = "../../../vibe-orchestrator/src/install/m_05_e_2_fixture.rs"]
mod fixture;
use fixture::*;

fn context() -> crate::output::Context {
    crate::output::Context::from_flags(true, false, None, false, crate::cli::AgentModeArg::Cli)
}
fn args(f: &Fixture, force: bool, assume_yes: bool) -> crate::cli::ReinstallArgs {
    crate::cli::ReinstallArgs {
        path: f.root.clone(),
        force,
        assume_yes,
        trace_compile: false,
    }
}

#[test]
fn prepared_offline_reinstall_reproduces_lock_without_prompts() {
    let f = Fixture::new();
    let (lock, plan) = f.installed();
    let before = std::fs::read(f.root.join("vibe.lock")).unwrap();
    let prepared = f.replay(&lock, &plan);
    let report = super::reinstall::run_with_capture(
        &context(),
        args(&f, false, false),
        None,
        true,
        Some(prepared),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        report.decision,
        vibe_install::evolution::Decision::ExactReuse
    );
    assert_eq!(before, std::fs::read(f.root.join("vibe.lock")).unwrap());
}

#[test]
fn prepared_force_repairs_same_payload_and_branch_without_prompt() {
    let f = Fixture::new();
    let (lock, plan) = f.installed();
    let before = std::fs::read(f.root.join("vibe.lock")).unwrap();
    let prepared = f.replay(&lock, &plan);
    let p = &lock.packages[0];
    let slot = vibe_workspace::vibedeps::slot_abs_path(&f.root, &p.group, &p.name, &p.version);
    std::fs::write(slot.join("payload.txt"), "damaged").unwrap();
    let refused = super::reinstall::run_with_capture(
        &context(),
        args(&f, false, false),
        None,
        true,
        Some(f.replay(&lock, &plan)),
    )
    .unwrap_err();
    assert!(refused.to_string().contains("installed slot"));
    super::reinstall::run_with_capture(
        &context(),
        args(&f, true, false),
        None,
        true,
        Some(prepared),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(slot.join("payload.txt")).unwrap(),
        b"exact source payload\n"
    );
    assert_eq!(before, std::fs::read(f.root.join("vibe.lock")).unwrap());
}

#[test]
fn force_and_assume_yes_do_not_replace_missing_authority() {
    let f = Fixture::new();
    f.installed();
    let before = std::fs::read(f.root.join("vibe.lock")).unwrap();
    let e = super::reinstall::run(&context(), args(&f, true, true), None, true).unwrap_err();
    assert!(format!("{e:#}").contains("authority"));
    assert_eq!(before, std::fs::read(f.root.join("vibe.lock")).unwrap());
}

#[test]
fn public_update_uses_standard_source_and_retains_choice() {
    let mut f = Fixture::new();
    f.installed();
    f.request.configuration.as_mut().unwrap().definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .defaults = Some([feature("b")].into());
    let args = crate::cli::UpdateArgs {
        packages: vec![],
        global: false,
        from_source: false,
        local_source: false,
        binary: false,
        registry: None,
        all: true,
        path: f.root.clone(),
        assume_yes: true,
        exact: false,
        auth_required: false,
        trace_compile: false,
    };
    let report = super::update::run_with_intent(
        &context(),
        args,
        Some(f.temp.path().join("registry")),
        true,
        Some(f.request.clone()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        report.decision,
        vibe_install::evolution::Decision::SemanticInputChange
    );
    let lock = vibe_core::manifest::Lockfile::read(f.root.join("vibe.lock")).unwrap();
    assert_eq!(lock.packages[0].features, vec!["a"]);
}

#[test]
fn legacy_unconfigured_update_keeps_public_route() {
    let mut f = Fixture::new();
    f.request.configuration = None;
    f.installed();
    let args = crate::cli::UpdateArgs {
        packages: vec![],
        global: false,
        from_source: false,
        local_source: false,
        binary: false,
        registry: None,
        all: true,
        path: f.root.clone(),
        assume_yes: true,
        exact: false,
        auth_required: false,
        trace_compile: false,
    };
    super::update::run(&context(), args, Some(f.temp.path().join("registry")), true).unwrap();
    let lock = vibe_core::manifest::Lockfile::read(f.root.join("vibe.lock")).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert!(lock.packages[0].features.is_empty());
}

#[test]
fn prepared_update_still_requires_named_package_to_be_installed() {
    let f = Fixture::new();
    f.installed();
    let before = std::fs::read(f.root.join("vibe.lock")).unwrap();
    let args = crate::cli::UpdateArgs {
        packages: vec!["org.e2fixture/notinstalled".into()],
        global: false,
        from_source: false,
        local_source: false,
        binary: false,
        registry: None,
        all: false,
        path: f.root.clone(),
        assume_yes: true,
        exact: false,
        auth_required: false,
        trace_compile: false,
    };
    let e = super::update::run_with_intent(
        &context(),
        args,
        Some(f.temp.path().join("registry")),
        true,
        Some(f.request.clone()),
    )
    .unwrap_err();
    assert!(format!("{e:#}").contains("not installed"));
    assert_eq!(before, std::fs::read(f.root.join("vibe.lock")).unwrap());
}
