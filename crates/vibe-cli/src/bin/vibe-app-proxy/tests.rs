use super::resolve::{Launch, Vars, expand, home_of, parse_session, resolve};
use super::*;
use std::fs;
use std::path::Path;

/// A home with one owned zip installation `org.example/alpha@1.0.0` in the
/// default library and one external `org.example/beta@1.0.0`, both
/// providing `tool` for slot `tool`.
fn home() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(dir.path()).unwrap();
    let alpha = home.join("opt/apps/tool/org.example.alpha@1.0.0");
    let beta = home.join("external/beta");
    for root in [&alpha, &beta] {
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin/tool"), "#!/bin/sh\n").unwrap();
    }
    fs::write(
        home.join("opt/vibe-library.toml"),
        "id = \"default\"\ncreated = \"now\"\n",
    )
    .unwrap();
    fs::create_dir_all(home.join("opt/bin")).unwrap();
    let launcher = serde_json::json!([{
        "id": "main", "name": "Tool", "executable": "$app_bin/tool",
        "args": ["--root=${app_root}", "$$x"], "workingDirectory": ".",
        "terminal": false, "default": true
    }]);
    let record = |name: &str, location: serde_json::Value| {
        let mut value = serde_json::json!({
            "slot": "tool",
            "package": {"group": "org.example", "name": name, "version": "1.0.0"},
            "option": "linux", "platform": "linux", "arch": "x86_64",
            "method": "zip", "portable": true, "launchers": launcher,
            "publicCommands": [{"name": "tool", "launcher": "main"}],
            "commands": {}
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(location.as_object().unwrap().clone());
        value
    };
    let index = serde_json::json!({
        "protocol": "vibe-user-applications/3",
        "installations": {
            "org.example/alpha@1.0.0": record("alpha", serde_json::json!({
                "library": "default", "directory": "apps/tool/org.example.alpha@1.0.0"})),
            "org.example/beta@1.0.0": record("beta", serde_json::json!({
                "method": "shell", "portable": false, "appRoot": beta})),
        },
        "slots": {"tool": {"default": "org.example/alpha@1.0.0"}}
    });
    fs::create_dir_all(home.join("applications")).unwrap();
    fs::write(home.join("applications/index.json"), index.to_string()).unwrap();
    (dir, home)
}

#[test]
fn session_value_grammar_matches_vibe_app() {
    let choices = parse_session(" ; a = k1 ;b=k2;; a=k3 ;").unwrap();
    assert_eq!(choices.get("a").map(String::as_str), Some("k3"));
    assert_eq!(choices.len(), 2);
    for bad in ["a", "=k", "a=", "a=b=c"] {
        let error = parse_session(bad).unwrap_err();
        assert!(error.contains(&format!("`{bad}`")), "{error}");
    }
}

#[test]
fn templates_expand_once_and_refuse_a_project() {
    let vars = Vars {
        app_root: Path::new("/apps/demo"),
        vibe_home: Path::new("/home/u/.vibe"),
    };
    assert_eq!(
        expand("${app_bin}/x $vibe_home $$5", &vars, "t").unwrap(),
        "/apps/demo/bin/x /home/u/.vibe $5"
    );
    assert!(
        expand("$project_root", &vars, "t")
            .unwrap_err()
            .contains("requires a project")
    );
    assert!(expand("$home", &vars, "t").is_err());
    assert!(expand("${app_root", &vars, "t").is_err());
}

#[test]
fn the_default_and_the_session_choice_select_the_provider() {
    let (_dir, home) = home();
    let proxy = home.join("opt/bin").join("tool");
    assert_eq!(home_of(&proxy).unwrap(), home);
    let alpha = home.join("opt/apps/tool/org.example.alpha@1.0.0");
    assert_eq!(
        resolve(&proxy, "tool", None).unwrap(),
        Launch {
            executable: alpha.join("bin/tool"),
            args: vec![format!("--root={}", alpha.display()), "$x".into()],
            working_directory: Some(alpha.clone()),
        }
    );
    let chosen = resolve(&proxy, "tool", Some("tool=org.example/beta@1.0.0")).unwrap();
    assert_eq!(chosen.executable, home.join("external/beta/bin/tool"));
    let error = resolve(&proxy, "tool", Some("tool=org.example/none@1.0.0")).unwrap_err();
    assert!(error.contains("not an installed provider"), "{error}");
    let error = resolve(&proxy, "other", None).unwrap_err();
    assert!(error.contains("not reserved"), "{error}");
    let error = resolve(&proxy, "tool", Some("broken")).unwrap_err();
    assert!(error.contains("malformed"), "{error}");
}

#[test]
fn a_missing_library_or_default_is_a_clear_error() {
    let (_dir, home) = home();
    let proxy = home.join("opt/bin").join("tool");
    fs::write(
        home.join("opt/vibe-library.toml"),
        "id = \"other\"\ncreated = \"now\"\n",
    )
    .unwrap();
    let error = resolve(&proxy, "tool", None).unwrap_err();
    assert!(error.starts_with("unavailable: library marker"), "{error}");
    let path = home.join("applications/index.json");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace(
            "\"default\":\"org.example/alpha@1.0.0\"",
            "\"default\":null",
        ),
    )
    .unwrap();
    let error = resolve(&proxy, "tool", None).unwrap_err();
    assert!(error.contains("has no default provider"), "{error}");
    let error = home_of(Path::new("/nonexistent-home/opt/bin/tool")).unwrap_err();
    assert!(error.contains("no application home"), "{error}");
}

#[cfg(unix)]
#[test]
fn caller_arguments_follow_the_launcher_arguments_literally() {
    let root = tempfile::tempdir().unwrap();
    let launch = Launch {
        executable: PathBuf::from("/bin/echo"),
        args: vec!["fixed value".into()],
        working_directory: Some(root.path().to_path_buf()),
    };
    let caller = [
        OsString::from("$(touch unsafe)"),
        OsString::from("two words"),
    ];
    let output = command_for(&launch, &caller)
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "fixed value $(touch unsafe) two words\n"
    );
    assert!(!root.path().join("unsafe").exists());
    assert_eq!(execute(&launch, &[]).unwrap(), 0);
}

#[test]
fn placeholders_take_the_caller_arguments() {
    let caller = [OsString::from("a.txt"), OsString::from("b.txt")];
    let files = vec!["--new".to_owned(), "{files}".to_owned()];
    assert_eq!(
        super::resolve::operands(&files, &caller),
        ["--new", "a.txt", "b.txt"]
    );
    let file = vec!["--open={file}".to_owned()];
    assert_eq!(super::resolve::operands(&file, &caller), ["--open=a.txt"]);
    assert_eq!(super::resolve::operands(&files, &[]), ["--new"]);
    let plain = vec!["--x".to_owned()];
    assert_eq!(
        super::resolve::operands(&plain, &caller),
        ["--x", "a.txt", "b.txt"]
    );
}

#[test]
fn current_index_routing_preserves_the_legacy_default_and_session_selection() {
    let (_dir, home) = home();
    let proxy = home.join("opt/bin/tool");
    let default = resolve(&proxy, "tool", None).unwrap();
    let session = resolve(&proxy, "tool", Some("tool=org.example/beta@1.0.0")).unwrap();
    let path = home.join("applications/index.json");
    let mut index: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    index["protocol"] = "vibe-user-applications/4".into();
    index["installations"]["org.example/alpha@1.0.0"]["secretOrigins"] =
        serde_json::json!({"token":{"kind":"absent"}});
    let bytes = serde_json::to_vec(&index).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert_eq!(resolve(&proxy, "tool", None).unwrap(), default);
    assert_eq!(
        resolve(&proxy, "tool", Some("tool=org.example/beta@1.0.0")).unwrap(),
        session
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
