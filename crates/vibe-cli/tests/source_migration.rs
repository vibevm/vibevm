//! Actual binaries: source choice conversation and independently granted D1 apply.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use vibe_core::manifest::{BOOT_SOURCE_INSERT, SOURCE_BOOT_CURRENT, SOURCE_BOOT_LEGACY};
const PATH: &str = "boot/native.md";
const DOCUMENT: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><chosen fact=\"true\" status=\"spec/done\">exact café</chosen></p></spec>\r\n";
fn source() -> String {
    format!(
        "# café 東京\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.0.0'\r\nformat='normal'\r\n[boot_snippet]\r\nsource='{PATH}' # keep"
    )
}
fn request() -> Value {
    json!({"schema":1,"source_text":source(),"source_profile":SOURCE_BOOT_LEGACY,"target_profile":SOURCE_BOOT_CURRENT,"recipe":BOOT_SOURCE_INSERT})
}
fn answer() -> Value {
    let mut input = request();
    input["source_decisions"] = json!([{"file":PATH,"media":"application/xml","interpretation":"vibe-spec","role":"spec","loading":"explicit"}]);
    input
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !dir.exists() {
            return;
        }
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                result.insert(path.strip_prefix(root).unwrap().into(), None);
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().into(),
                    Some(std::fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
struct Fixture {
    temp: tempfile::TempDir,
    home: PathBuf,
    cwd: PathBuf,
    project: PathBuf,
    store: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let home = root.join("home");
        let cwd = root.join("cwd");
        let project = root.join("project");
        let store = root.join("recovery");
        std::fs::create_dir_all(home.join(".vibe")).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        std::fs::create_dir_all(project.join("boot")).unwrap();
        for file in ["config.toml", "settings.toml", "registry.toml"] {
            std::fs::write(home.join(".vibe").join(file), "[poison\n").unwrap();
        }
        std::fs::write(project.join("vibe.toml"), source()).unwrap();
        std::fs::write(project.join(PATH), DOCUMENT).unwrap();
        Self {
            temp,
            home,
            cwd,
            project,
            store,
        }
    }
    fn run(&self, input: &Value, operation: bool, literal: bool, exit: i32) -> Value {
        let home_before = tree(&self.home);
        let cwd_before = tree(&self.cwd);
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("VIBE_SETTINGS", self.home.join(".vibe"))
            .env("VIBE_LOG", "invalid=???")
            .env("VIBEVM_HOME", self.temp.path().join("invalid-home"))
            .current_dir(&self.cwd)
            .args([
                "refactor",
                if operation {
                    "migration-operation"
                } else {
                    "plan-migration"
                },
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if operation {
            command
                .arg("--project-root")
                .arg(&self.project)
                .args(["--target-relative", "vibe.toml"])
                .arg("--recovery-store")
                .arg(&self.store);
        }
        let text = input.to_string();
        if literal {
            command.args(["--request-json", &text]);
        }
        let mut child = command.spawn().unwrap();
        if !literal {
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(text.as_bytes())
                .unwrap();
        }
        drop(child.stdin.take());
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(exit),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(tree(&self.home), home_before);
        assert_eq!(tree(&self.cwd), cwd_before);
        let text = std::str::from_utf8(&output.stdout).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert!(text.ends_with('\n'));
        serde_json::from_str(text).unwrap()
    }
    fn operation(&self, approved: &Value, plan: &Value) -> Value {
        json!({"schema":1,"operation":"apply","operation_id":"source-cli","authority":{
            "project_root":self.project,"target_relative":"vibe.toml","recovery_store_root":self.store,"scope":"owned_authored_file"},
            "approved_request_json":approved.to_string(),"before_content_id":plan["before_identity"],"after_content_id":plan["after_identity"],"diff_id":plan["diff_identity"]})
    }
}
#[test]
fn nonquiet_source_question_answer_stdin_literal_and_neutral_output_preserve_state() {
    let f = Fixture::new();
    let before = tree(f.temp.path());
    let questions = f.run(&request(), false, false, 2);
    assert_eq!(
        questions["outcome"],
        json!({"status":"needs_input","questions":[{"field":"source_media","resource_path":PATH,"choices":["application/xml","text/markdown"]}]})
    );
    let supplied = answer();
    let planned = f.run(&supplied, false, false, 0);
    assert_eq!(f.run(&supplied, false, true, 0), planned);
    assert_eq!(
        planned,
        serde_json::to_value(vibe_workspace::migration_planning::plan_migration_value(
            supplied
        ))
        .unwrap()
    );
    let plan = &planned["outcome"]["plan"];
    assert_eq!(plan["before"], source());
    let after = plan["after"].as_str().unwrap();
    assert!(after.starts_with(&source()));
    assert!(!after.replace("\r\n", "").contains('\n'));
    let manifest = vibe_core::manifest::Manifest::parse_str(after).unwrap();
    assert_eq!(
        manifest
            .require_package()
            .unwrap()
            .sources
            .as_ref()
            .unwrap()[0]
            .media(),
        "application/xml"
    );
    let neutral = f.run(&json!({"schema":1,"source_text":""}), false, true, 2);
    for question in neutral["outcome"]["questions"].as_array().unwrap() {
        assert!(question.get("resource_path").is_none());
        assert_ne!(question["field"], "source_media");
    }
    assert_eq!(tree(f.temp.path()), before);
    assert!(!f.store.exists());
}
#[test]
fn strict_source_decision_errors_and_unsupported_old_shapes_have_no_effects() {
    let f = Fixture::new();
    let before = tree(f.temp.path());
    let valid = answer()["source_decisions"][0].clone();
    let mut bad = vec![
        Value::Null,
        json!({}),
        json!([null]),
        json!([valid.clone(), valid.clone()]),
    ];
    for field in ["file", "media", "interpretation", "role", "loading"] {
        let mut row = valid.clone();
        row[field] = Value::Null;
        bad.push(json!([row]));
    }
    let mut unknown = valid;
    unknown["trust"] = json!("verified");
    bad.push(json!([unknown]));
    for invalid in bad {
        let mut input = request();
        input["source_decisions"] = invalid;
        let out = f.run(&input, false, true, 2);
        assert_eq!(out["stage"], "preflight");
        assert_eq!(out["error"]["code"], "invalid_request");
    }
    let mut wrong = answer();
    wrong["source_decisions"][0]["file"] = json!("boot/foreign.xml");
    let out = f.run(&wrong, false, false, 1);
    assert_eq!(out["stage"], "planner");
    assert_eq!(out["outcome"]["status"], "refused");
    let mut simple = request();
    simple["source_text"] = json!(source().replace("format='normal'", "format='simple'"));
    assert_eq!(
        f.run(&simple, false, true, 1)["outcome"]["status"],
        "refused"
    );
    assert_eq!(tree(f.temp.path()), before);
}
#[test]
fn actual_source_apply_repeat_and_stale_refusal_use_existing_d1_writer() {
    let f = Fixture::new();
    let approved = answer();
    let planned = f.run(&approved, false, false, 0);
    let plan = &planned["outcome"]["plan"];
    let operation = f.operation(&approved, plan);
    let result = f.run(&operation, true, false, 0);
    assert_eq!(result["outcome"], "completed");
    let after = std::fs::read_to_string(f.project.join("vibe.toml")).unwrap();
    assert_eq!(after, plan["after"]);
    assert_eq!(
        std::fs::read(f.project.join(PATH)).unwrap(),
        DOCUMENT.as_bytes()
    );
    let manifest = vibe_core::manifest::Manifest::parse_str(&after).unwrap();
    let observed = vibe_facts::observe_declared_document(
        manifest.require_package().unwrap(),
        PATH,
        Some(DOCUMENT.as_bytes()),
    )
    .unwrap();
    let vibe_facts::DeclaredDocumentObservation::Facts { facts, provenance } = observed else {
        panic!("selected XML must read despite .md suffix")
    };
    assert_eq!(facts.len(), 1);
    assert!(facts[0].address.ends_with("#chosen"));
    assert_eq!(provenance.reader, Some("vibe-specdoc:xml/spec"));
    let stable = tree(f.temp.path());
    assert_eq!(f.run(&operation, true, true, 0)["outcome"], "no_op");
    assert_eq!(tree(f.temp.path()), stable);
    let stale = Fixture::new();
    let operation = stale.operation(&approved, plan);
    std::fs::write(
        stale.project.join("vibe.toml"),
        format!("{}\r\n# changed", source()),
    )
    .unwrap();
    let before = tree(&stale.project);
    let result = stale.run(&operation, true, true, 1);
    assert_eq!(result["error"]["code"], "source_changed");
    assert_eq!(tree(&stale.project), before);
    assert!(
        tree(&stale.store)
            .keys()
            .all(|path| path.file_name().unwrap() != "journal.json")
    );
    assert!(
        result["effects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|effect| effect["kind"] != "target_publication")
    );
}
