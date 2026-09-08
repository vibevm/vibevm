#[test]
fn cargo_argv_is_canonical_and_tests_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    write_cargo_fixture(temp.path());
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "build"
workspace = true
locked = true
all_targets = true
tests = "required"
profile = "release"
features = ["z", "a"]
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    assert_eq!(health.checks[0].commands.len(), 2);
    assert_eq!(
        rendered(&health.checks[0].commands[0].argv),
        [
            "build",
            "--workspace",
            "--all-targets",
            "--features",
            "a,z",
            "--profile",
            "release",
            "--locked",
            "--offline",
        ]
    );
    assert_eq!(rendered(&health.checks[0].commands[1].argv)[0], "test");
}

#[test]
fn npm_uses_node_and_cli_assets_without_a_command_shell() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("package.json"),
        r#"{"scripts":{"build":"echo build","test":"echo test"}}"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("package-lock.json"), "{}").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "web"
kind = "npm"
root = "."
manager = "npm"
lockfile = "package-lock.json"
install = "ci"
build_script = "build"
tests = "required"
test_script = "test"
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let check = &health.checks[0];
    assert_eq!(check.assets.len(), 2);
    assert_eq!(
        rendered(&check.commands[0].argv),
        ["{asset:web/npm-cli}", "ci", "--offline"]
    );
    assert_eq!(
        rendered(&check.commands[1].argv),
        ["{asset:web/npm-cli}", "run", "build"]
    );
    assert_eq!(
        rendered(&check.commands[2].argv),
        ["{asset:web/npm-cli}", "run", "test"]
    );
}

#[test]
fn test_modes_refuse_required_absence_and_type_if_present_skip() {
    let temp = tempfile::tempdir().unwrap();
    write_cargo_fixture(temp.path());
    let (project, inventory) = observed(temp.path());
    let required = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "required"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let blocked = prepare(
        &project,
        &required,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert!(
        blocked
            .blockers
            .iter()
            .any(|blocker| blocker.code == "health-preparation-failed")
    );
    let optional = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "if-present"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let health = prepare(
        &project,
        &optional,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert_eq!(
        health.checks[0].tests,
        Some(TestDisposition::SkippedNotPresent)
    );
    assert_eq!(health.checks[0].commands.len(), 1);
}

#[test]
fn system_cargo_discovery_honors_disabled_auto_and_target_tests() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname='no-tests'\nversion='0.1.0'\nedition='2024'\nautotests=false\nautobins=false\n[lib]\ntest=false\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn product() {}\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "required"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = SystemHealthResolver::new(&project);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    assert!(health.blockers.iter().any(|blocker| {
        blocker.check_id.as_deref() == Some("rust") && blocker.message.contains("requires tests")
    }));
}

#[test]
fn system_maven_discovery_blocks_profile_dependent_required_tests() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("pom.xml"),
        "<project><profiles><profile><id>tests</id></profile></profiles></project>",
    )
    .unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "java"
kind = "maven"
root = "."
runner = "wrapper-first"
goal = "verify"
offline = true
tests = "required"
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let mut resolver = SystemHealthResolver::new(&project);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    assert!(health.blockers.iter().any(|blocker| {
        blocker.check_id.as_deref() == Some("java") && blocker.message.contains("indeterminate")
    }));
}

#[test]
fn projected_cargo_model_reuses_autotests_and_target_rules() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("tests")).unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname='tests'\nversion='0.1.0'\nedition='2024'\nautolib=false\nautobins=false\nautotests=true\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("tests/it.rs"), "#[test] fn it() {}\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "rust"
kind = "cargo"
root = "."
build = "check"
workspace = false
locked = false
all_targets = false
tests = "required"
profile = "dev"
features = []
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let prepared = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let projected = vec![
        crate::rewrite::ProjectedEntry {
            path: "Cargo.toml".to_owned(),
            kind: crate::model::EntryKind::File,
            bytes: Some(b"[package]\nname='tests'\nversion='0.1.0'\nedition='2024'\nautolib=false\nautobins=false\nautotests=false\n".to_vec()),
            unix_mode: None,
        },
        crate::rewrite::ProjectedEntry {
            path: "tests".to_owned(),
            kind: crate::model::EntryKind::Directory,
            bytes: None,
            unix_mode: None,
        },
        crate::rewrite::ProjectedEntry {
            path: "tests/it.rs".to_owned(),
            kind: crate::model::EntryKind::File,
            bytes: Some(b"#[test] fn it() {}\n".to_vec()),
            unix_mode: None,
        },
    ];
    let blockers = validate_projected_final(&contract, &prepared, &projected);
    assert!(
        blockers
            .iter()
            .any(|blocker| { blocker.code == "health-projected-test-applicability-changed" })
    );
}

#[test]
fn projected_maven_model_reuses_profile_indeterminacy() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("pom.xml"), "<project/>").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "java"
kind = "maven"
root = "."
runner = "explicit"
goal = "verify"
offline = true
tests = "required"
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = FakeResolver::new(TestPresence::Present);
    let prepared = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let projected = vec![crate::rewrite::ProjectedEntry {
        path: "pom.xml".to_owned(),
        kind: crate::model::EntryKind::File,
        bytes: Some(
            b"<project><profiles><profile><id>tests</id></profile></profiles></project>".to_vec(),
        ),
        unix_mode: None,
    }];
    let blockers = validate_projected_final(&contract, &prepared, &projected);
    assert!(
        blockers
            .iter()
            .any(|blocker| { blocker.code == "health-projected-test-indeterminate" })
    );
}

#[test]
fn python_and_maven_argv_are_structured() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("src")).unwrap();
    std::fs::write(temp.path().join("src/app.py"), "pass\n").unwrap();
    std::fs::write(temp.path().join("pyproject.toml"), "[build-system]").unwrap();
    std::fs::write(temp.path().join("pom.xml"), "<project/>").unwrap();
    let (project, inventory) = observed(temp.path());
    let python = contract(
        r#"[[healthcheck]]
id = "py"
kind = "python-pip"
root = "."
interpreter = "python"
source_roots = ["src"]
dependency_check = true
build = true
tests = "skip"
timeout_seconds = 10"#,
        "strict",
        "tool-offline",
    );
    let health = prepare(
        &project,
        &python,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert_eq!(health.checks[0].commands.len(), 3);
    assert_eq!(
        rendered(&health.checks[0].commands[2].argv),
        [
            "-s",
            "-m",
            "build",
            "--no-isolation",
            "--outdir",
            "{scratch}"
        ]
    );

    let maven = contract(
        r#"[[healthcheck]]
id = "java"
kind = "maven"
root = "."
runner = "explicit"
goal = "verify"
offline = true
tests = "skip"
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let health = prepare(
        &project,
        &maven,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert_eq!(
        rendered(&health.checks[0].commands[0].argv),
        [
            "--batch-mode",
            "--no-transfer-progress",
            "--offline",
            "-DskipTests",
            "verify",
        ]
    );
}

#[test]
fn custom_bundle_is_stable_and_placeholders_are_whole_arguments() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("tools")).unwrap();
    std::fs::write(temp.path().join("tools/health.py"), b"print('ok')\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let contract = contract(
        r#"[[healthcheck]]
id = "domain"
kind = "custom"
root = "."
source = "tools/health.py"
snapshot = ["tools", "tools/**"]
interpreter = "python"
argv = ["--phase", "{phase}", "--root", "{root}"]
protocol = "exit-code"
reads = ["**"]
writes = []
spawn = true
network = "inherit"
timeout_seconds = 10"#,
        "strict",
        "deny",
    );
    let mut resolver = FakeResolver::new(TestPresence::Absent);
    let health = prepare(&project, &contract, &inventory, &mut resolver).unwrap();
    let check = &health.checks[0];
    let bundle = check.custom_bundle.as_ref().unwrap();
    assert_eq!(bundle.entries.len(), 2);
    assert!(bundle.entries[1].content.is_some());
    assert_eq!(
        rendered(&check.commands[0].argv),
        [
            "{bundle:tools/health.py}",
            "--phase",
            "{phase}",
            "--root",
            "{root}",
        ]
    );
    assert!(!check.sandbox.spawn_prevention);
    assert!(!check.sandbox.network_deny);
}

#[cfg(windows)]
#[test]
fn windows_custom_profile_and_direct_bundle_launch_block_deterministically() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("tools")).unwrap();
    std::fs::write(temp.path().join("tools/health.py"), b"print('ok')\n").unwrap();
    let (project, inventory) = observed(temp.path());
    let unsupported_effects = contract(
        r#"[[healthcheck]]
id = "domain"
kind = "custom"
root = "."
source = "tools/health.py"
snapshot = ["tools/health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = ["**"]
writes = ["generated/**"]
spawn = true
network = "inherit"
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let health = prepare(
        &project,
        &unsupported_effects,
        &inventory,
        &mut FakeResolver::new(TestPresence::Absent),
    )
    .unwrap();
    assert!(health.blockers.iter().any(|blocker| {
        blocker.code == "health-custom-profile-unsupported"
            && blocker.check_id.as_deref() == Some("domain")
    }));

    let allowed = contract(
        r#"[[healthcheck]]
id = "direct"
kind = "custom"
root = "."
source = "tools/health.py"
snapshot = ["tools/health.py"]
interpreter = "direct"
argv = []
protocol = "exit-code"
reads = ["**"]
writes = []
spawn = true
network = "inherit"
timeout_seconds = 10"#,
        "strict",
        "inherit",
    );
    let mut resolver = FakeResolver::new(TestPresence::Absent);
    resolver.custom_style = CustomLaunchStyle::Direct;
    let health = prepare(&project, &allowed, &inventory, &mut resolver).unwrap();
    assert!(health.blockers.iter().any(|blocker| {
        blocker.code == "health-unsupported"
            && blocker
                .message
                .contains("direct bundled custom executables")
    }));
}
