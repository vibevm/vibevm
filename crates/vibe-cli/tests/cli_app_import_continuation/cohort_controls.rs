//! Real cohort abandonment and exact row/attempt correlation.
use super::*;
const PARK: &str = r#"
[[installer_options.steps]]
id='finish'
title='Finish'
kind='agent'
phase='finish'
required=true
prompt='Complete the bounded cohort control fixture.'
"#;
fn resume_cohort(r: &AppRegistry, id: &str) -> std::process::Output {
    r.vibe(&[
        "app",
        "import",
        "--resume",
        id,
        "--json",
        "--offline",
        "--unattended",
    ])
}
fn answer(error: &Value) {
    fs::write(
        Path::new(error["run"].as_str().unwrap()).join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
}
#[test]
fn duplicate_package_rows_never_adopt_an_earlier_rows_attempt() {
    let r = AppRegistry::new();
    let first = package(&r, "first", PARK);
    let marker = r.root.path().join("once");
    fs::write(first.join("build.sh"),format!("set -e\necho first >> '{}'\nif [ ! -e '{}' ]; then touch '{}'; exit 17; fi\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\n",r.root.path().join("executions").display(),marker.display(),marker.display())).unwrap();
    let file = set(&r, &["first", "first"]);
    let (status, error) = refused(&hosted(&r, &file));
    assert_eq!(status, Some(75), "{error}");
    assert_eq!(executions(&r), ["first", "first"]);
    let view = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let cohort = &view["imports"][0];
    assert_eq!(cohort["cursor"], 1);
    let id = cohort["target"]["operation"].as_str().unwrap();
    let child = &view["entries"][0];
    assert_eq!(cohort["current"], child["operation"]["target"]);
    let checkpoint = Path::new(child["path"].as_str().unwrap()).join("steps.json");
    let original = fs::read(&checkpoint).unwrap();
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(changed["operation"]["parent"]["row"], 1);
    changed["operation"]["parent"]["row"] = 0.into();
    let corrupt = serde_json::to_vec_pretty(&changed).unwrap();
    fs::write(&checkpoint, &corrupt).unwrap();
    let owner_file = Path::new(cohort["path"].as_str().unwrap()).join("import.json");
    let owner_before = fs::read(&owner_file).unwrap();
    let index = fs::read(r.settings().join("applications/index.json")).unwrap();
    let (status, refusal) = refused(&resume_cohort(&r, id));
    assert_eq!(status, Some(2));
    assert_eq!(refusal["code"], "resume_mismatch");
    assert_eq!(fs::read(&owner_file).unwrap(), owner_before);
    assert_eq!(fs::read(&checkpoint).unwrap(), corrupt);
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        index
    );
    fs::write(&checkpoint, original).unwrap();
    fs::remove_file(file).unwrap();
    answer(&error);
    let output = resume_cohort(&r, id);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["installed"],
        serde_json::json!(["org.example/first@1.0.0"])
    );
    assert_eq!(report["refused"].as_array().unwrap().len(), 1);
    assert_eq!(executions(&r), ["first", "first"]);
}
#[test]
fn explicit_cohort_clean_abandons_matching_pending_refs_and_allows_fresh_repair() {
    let r = AppRegistry::new();
    package(&r, "first", "");
    package(&r, "second", PARK);
    package(&r, "unrelated", "");
    ok(&r.app(
        &["install", "-g"],
        "org.example/unrelated",
        &["--keep-work", "--json", "--offline", "--unattended"],
    ));
    let file = set(&r, &["first", "second"]);
    let (status, error) = refused(&hosted(&r, &file));
    assert_eq!(status, Some(75), "{error}");
    let view = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let cohort = &view["imports"][0];
    let id = cohort["target"]["operation"].as_str().unwrap();
    let entries = view["entries"].as_array().unwrap();
    let unrelated = entries
        .iter()
        .find(|r| r["key"] == "org.example/unrelated@1.0.0")
        .unwrap();
    let unrelated_file = Path::new(unrelated["path"].as_str().unwrap()).join("steps.json");
    let untouched = fs::read(&unrelated_file).unwrap();
    let first = entries
        .iter()
        .find(|r| r["key"] == "org.example/first@1.0.0")
        .unwrap();
    assert_eq!(first["retainedFor"]["owner"]["kind"], "import");
    assert_eq!(first["retainedFor"]["owner"]["row"], 0);
    let owner_file = Path::new(cohort["path"].as_str().unwrap()).join("import.json");
    let owner_before = fs::read(&owner_file).unwrap();
    let before = fs::read(r.settings().join("applications/index.json")).unwrap();
    let (status, error) = refused(&r.vibe(&[
        "app",
        "work",
        "org.example/first",
        "--clean",
        "-g",
        "--json",
    ]));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "resume_mismatch");
    assert_eq!(fs::read(&owner_file).unwrap(), owner_before);
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        before
    );
    ok(&r.vibe(&["app", "work", "--import", id, "--clean", "-g", "--json"]));
    assert!(!owner_file.exists());
    assert_eq!(fs::read(&unrelated_file).unwrap(), untouched);
    let record = &r.index()["installations"]["org.example/second@1.0.0"];
    assert!(record.get("pending").is_none());
    assert!(
        record["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["outcome"] == "delegated")
    );
    let output = r
        .user
        .vibe()
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            "install",
            "-g",
            "org.example/second",
            "--registry",
            r.registry().to_str().unwrap(),
            "--force",
            "--agent-mode",
            "agent",
            "--json",
            "--offline",
            "--unattended",
        ])
        .output()
        .unwrap();
    let (status, parked) = refused(&output);
    assert_eq!(status, Some(75), "{parked}");
    let after = json(&r.vibe(&["app", "work", "org.example/second", "-g", "--json"]));
    let current = &after["entries"][0];
    let checkpoint = Path::new(current["path"].as_str().unwrap()).join("steps.json");
    let current_bytes = fs::read(&checkpoint).unwrap();
    let current_index = fs::read(r.settings().join("applications/index.json")).unwrap();
    let (status, error) =
        refused(&r.vibe(&["app", "work", "--import", id, "--clean", "-g", "--json"]));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "resume_mismatch");
    assert_eq!(fs::read(&checkpoint).unwrap(), current_bytes);
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        current_index
    );
    answer(&parked);
    ok(&r.vibe(&[
        "app",
        "resume",
        "org.example/second@1.0.0",
        "--operation",
        current["operation"]["target"]["operation"]
            .as_str()
            .unwrap(),
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]));
    assert!(
        r.index()["installations"]["org.example/second@1.0.0"]
            .get("pending")
            .is_none()
    );
    assert_eq!(fs::read(&unrelated_file).unwrap(), untouched);
}

#[test]
fn installed_cohort_results_cannot_drop_their_row_or_publication_attempt() {
    let r = AppRegistry::new();
    package(&r, "first", "");
    package(
        &r,
        "second",
        &format!(
            "{PARK}\n[[installer_options.steps]]\nid='suffix'\ntitle='Suffix effect'\nkind='shell'\nphase='finish'\ncommand=\"echo suffix >> '{}'\"\n",
            r.root.path().join("suffix").display()
        ),
    );
    let file = set(&r, &["first", "second"]);
    let (status, error) = refused(&hosted(&r, &file));
    assert_eq!(status, Some(75));
    let view = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let cohort = &view["imports"][0];
    let id = cohort["target"]["operation"].as_str().unwrap();
    let owner_file = Path::new(cohort["path"].as_str().unwrap()).join("import.json");
    let original: Value = serde_json::from_slice(&fs::read(&owner_file).unwrap()).unwrap();
    let child = view["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "org.example/second@1.0.0")
        .unwrap();
    let checkpoint = Path::new(child["path"].as_str().unwrap()).join("steps.json");
    ok(&r.vibe(&[
        "app",
        "hide",
        "org.example/first",
        "-g",
        "--json",
        "--offline",
        "--unattended",
    ]));
    answer(&error);
    for erase_row in [false, true] {
        let mut malformed = original.clone();
        malformed["cohort"]["results"][0]["target"] = Value::Null;
        if erase_row {
            malformed["cohort"]["results"][0]["rowIndex"] = Value::Null;
        }
        let bytes = serde_json::to_vec_pretty(&malformed).unwrap();
        fs::write(&owner_file, &bytes).unwrap();
        let before = fs::read(&checkpoint).unwrap();
        let index = fs::read(r.settings().join("applications/index.json")).unwrap();
        let output = resume_cohort(&r, id);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let (_, error) = refused(&output);
        assert_eq!(error["code"], "resume_mismatch");
        assert_eq!(fs::read(&owner_file).unwrap(), bytes);
        assert_eq!(fs::read(&checkpoint).unwrap(), before);
        assert_eq!(
            fs::read(r.settings().join("applications/index.json")).unwrap(),
            index
        );
        assert!(!r.root.path().join("suffix").exists());
    }
}

#[test]
fn ordinary_fresh_retry_retires_completed_children_before_replacing_the_old_owner() {
    let r = AppRegistry::new();
    package(&r, "dependency", "");
    let p = package(&r, "first", "");
    let manifest = p.join("vibe.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest).unwrap().replace(
            "result='owned'",
            "result='owned'\nrequires=['org.example/dependency@=1.0.0']",
        ),
    )
    .unwrap();
    let marker = r.root.path().join("once");
    fs::write(p.join("build.sh"),format!("set -e\necho first >> '{}'\nif [ ! -e '{}' ]; then touch '{}'; exit 17; fi\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\n",r.root.path().join("executions").display(),marker.display(),marker.display())).unwrap();
    let failed = r.app(
        &["install", "-g"],
        "org.example/first",
        &["--offline", "--unattended", "--json"],
    );
    assert_eq!(failed.status.code(), Some(1));
    ok(&r.app(
        &["install", "-g"],
        "org.example/first",
        &["--offline", "--unattended", "--json"],
    ));
    assert_eq!(executions(&r), ["dependency", "first", "first"]);
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    assert!(
        listing["entries"].as_array().unwrap().is_empty(),
        "{listing}"
    );
    let (status, error) = refused(&r.vibe(&[
        "app",
        "work",
        "org.example/dependency",
        "--clean",
        "-g",
        "--json",
    ]));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "work_missing");
}

#[test]
fn explicit_clean_all_abandons_unreadable_package_work_without_touching_another_home() {
    let r = AppRegistry::new();
    package(&r, "first", "");
    package(&r, "second", PARK);
    let file = set(&r, &["first", "second"]);
    let (status, error) = refused(&hosted(&r, &file));
    assert_eq!(status, Some(75), "{error}");
    let work = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let child = work["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "org.example/second@1.0.0")
        .unwrap();
    let child_path = Path::new(child["path"].as_str().unwrap());
    fs::write(child_path.join("steps.json"), b"unreadable old work").unwrap();
    let foreign = AppRegistry::new();
    package(&foreign, "second", PARK);
    let other_file = set(&foreign, &["second"]);
    assert_eq!(hosted(&foreign, &other_file).status.code(), Some(75));
    let foreign_index = fs::read(foreign.settings().join("applications/index.json")).unwrap();
    let before = fs::read(r.settings().join("applications/index.json")).unwrap();
    let (status, error) = refused(&r.vibe(&[
        "app",
        "work",
        "org.example/second",
        "--clean",
        "-g",
        "--json",
    ]));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "resume_mismatch");
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        before
    );
    ok(&r.vibe(&["app", "work", "--clean", "--all", "-g", "--json"]));
    assert!(
        r.index()["installations"]["org.example/second@1.0.0"]
            .get("pending")
            .is_none()
    );
    assert!(r.index()["installations"]["org.example/first@1.0.0"].is_object());
    assert_eq!(
        fs::read(foreign.settings().join("applications/index.json")).unwrap(),
        foreign_index
    );
    let output = r
        .user
        .vibe()
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            "install",
            "-g",
            "org.example/second",
            "--registry",
            r.registry().to_str().unwrap(),
            "--force",
            "--agent-mode",
            "agent",
            "--json",
            "--offline",
            "--unattended",
        ])
        .output()
        .unwrap();
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75));
    answer(&error);
    let view = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let row = &view["entries"][0];
    ok(&r.vibe(&[
        "app",
        "resume",
        "org.example/second@1.0.0",
        "--operation",
        row["operation"]["target"]["operation"].as_str().unwrap(),
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]));
    assert_eq!(
        fs::read(foreign.settings().join("applications/index.json")).unwrap(),
        foreign_index
    );
}

#[test]
fn missing_work_does_not_turn_an_indexed_pending_row_into_import_success() {
    let r = AppRegistry::new();
    package(&r, "second", PARK);
    let file = set(&r, &["second"]);
    let (status, error) = refused(&hosted(&r, &file));
    assert_eq!(status, Some(75), "{error}");
    let view = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let child = &view["entries"][0];
    fs::remove_dir_all(child["path"].as_str().unwrap()).unwrap();
    let original = fs::read(r.settings().join("applications/index.json")).unwrap();
    let registry = r.registry();
    for dry in [true, false] {
        let mut args = vec![
            "app",
            "import",
            file.to_str().unwrap(),
            "--registry",
            registry.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ];
        if dry {
            args.push("--dry-run");
        }
        let output = r.vibe(&args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(report["installed"].as_array().unwrap().is_empty());
        assert!(report["skipped"].as_array().unwrap().is_empty());
        assert_eq!(report["refused"][0]["code"], "resume_mismatch");
        assert_eq!(
            fs::read(r.settings().join("applications/index.json")).unwrap(),
            original
        );
    }
}
