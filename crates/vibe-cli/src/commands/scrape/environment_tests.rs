use super::*;

#[cfg(windows)]
fn project_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname='scrape-env-fixture'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn value() -> u8 { 1 }\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname='scrape-env-fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    vibe_scrape::init_contract(project.path()).unwrap();
    project
}

#[test]
fn state_root_preserves_override_directory_file_and_home_precedence() {
    let root = tempfile::tempdir().unwrap();
    let override_dir = root.path().join("settings");
    std::fs::create_dir(&override_dir).unwrap();
    let home = root.path().join("home");

    let directory = ScrapeEnvironment::new(
        Some(override_dir.clone().into_os_string()),
        Some(home.clone().into_os_string()),
    );
    assert_eq!(
        scrape_state_root(&directory).unwrap(),
        override_dir.join("scrape-state")
    );

    let override_file = root.path().join("settings.toml");
    std::fs::write(&override_file, "").unwrap();
    let file = ScrapeEnvironment::new(
        Some(override_file.into_os_string()),
        Some(home.clone().into_os_string()),
    );
    assert_eq!(
        scrape_state_root(&file).unwrap(),
        root.path().join("scrape-state")
    );

    let fallback = ScrapeEnvironment::new(None, Some(home.clone().into_os_string()));
    assert_eq!(
        scrape_state_root(&fallback).unwrap(),
        home.join(".vibe/scrape")
    );
}

#[test]
fn missing_home_is_a_bounded_typed_refusal() {
    let error = scrape_state_root(&ScrapeEnvironment::new(None, None))
        .unwrap_err()
        .to_string();
    assert_eq!(error, "resolving user home for scrape transaction state");
    assert!(error.len() < 128);
}

#[cfg(windows)]
#[test]
fn planning_never_requires_the_injected_journal_root() {
    let project = project_fixture();
    let args = ScrapeArgs {
        plan: true,
        output: None,
        in_place: true,
        recover: false,
        contract: None,
        path: Some(project.path().to_path_buf()),
        assume_yes: false,
        command: None,
    };
    let context = Context::from_flags(true, false, None, false, crate::cli::AgentModeArg::Cli);
    let error = run(&context, args, ScrapeEnvironment::new(None, None))
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("scrape plan is blocked by"), "{error}");
    assert!(!error.contains("user home"), "{error}");
}

#[cfg(windows)]
#[test]
fn execute_and_recover_require_the_injected_journal_root() {
    let project = project_fixture();
    let context = Context::from_flags(true, false, None, false, crate::cli::AgentModeArg::Cli);
    for (recover, in_place) in [(false, true), (true, false)] {
        let args = ScrapeArgs {
            plan: false,
            output: None,
            in_place,
            recover,
            contract: None,
            path: Some(project.path().to_path_buf()),
            assume_yes: !recover,
            command: None,
        };
        let error = run(&context, args, ScrapeEnvironment::new(None, None))
            .unwrap_err()
            .to_string();
        assert_eq!(error, "resolving user home for scrape transaction state");
    }
}
