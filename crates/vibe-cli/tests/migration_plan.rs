//! Actual-binary text-only migration planning; every child has isolated state.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use vibe_core::manifest::{
    CURRENT_ABSENT, CURRENT_EPOCH1, IDENTITY, LEGACY_ABSENT, SNAPSHOT_TO_COPY,
};
use vibe_workspace::migration_planning::{input_read_error, plan_migration_json};

const BEFORE: &str = "[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"snapshot\"\n";
fn request() -> Value {
    json!({"schema":1,"source_text":BEFORE,"source_profile":LEGACY_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":SNAPSHOT_TO_COPY})
}

fn tree(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(root: &Path, path: &Path, result: &mut BTreeMap<String, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                result.insert(key, None);
                walk(root, &path, result);
            } else {
                result.insert(key, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn child(input: &[u8], literal: Option<&str>, expected: Value, exit: i32, poisoned: bool) -> Value {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let cwd = temp.path().join("cwd");
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(&cwd).unwrap();
    if poisoned {
        let settings = home.join(".vibe");
        std::fs::create_dir(&settings).unwrap();
        for name in ["config.toml", "settings.toml", "registry.toml"] {
            std::fs::write(settings.join(name), "[broken configuration\n").unwrap();
        }
    }
    let before = tree(temp.path());
    let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
    command
        .env_clear()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("VIBE_SETTINGS", home.join(".vibe"))
        .env("VIBE_LOG", "invalid=???")
        .env("VIBEVM_HOME", temp.path().join("stale-installed-home"))
        .current_dir(&cwd)
        .args(["--quiet", "refactor", "plan-migration"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(text) = literal {
        command.args(["--request-json", text]);
    }
    let mut process = command.spawn().unwrap();
    process.stdin.take().unwrap().write_all(input).unwrap();
    let output = process.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "startup/prose leaked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert!(text.ends_with('\n'));
    assert_eq!(text.lines().count(), 1, "one complete JSON document");
    let value: Value = serde_json::from_str(text).unwrap();
    assert_eq!(value, expected);
    assert_eq!(
        tree(temp.path()),
        before,
        "planning altered isolated HOME/cwd state"
    );
    assert!(!cwd.join(".vibe").exists());
    assert!(!home.join(".vibe/cache").exists());
    if !poisoned {
        assert!(!home.join(".vibe").exists());
    }
    value
}
fn run(value: Value, exit: i32, literal: bool, poisoned: bool) -> Value {
    let text = serde_json::to_string(&value).unwrap();
    let expected = serde_json::to_value(plan_migration_json(&text)).unwrap();
    child(
        if literal {
            b"stdin is ignored"
        } else {
            text.as_bytes()
        },
        literal.then_some(text.as_str()),
        expected,
        exit,
        poisoned,
    )
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn stdin_and_literal_match_shared_golden_plan_without_state() {
    let stdin = run(request(), 0, false, false);
    let literal = run(request(), 0, true, false);
    assert_eq!(stdin, literal);
    assert_eq!(stdin["stage"], "planner");
    let plan = &stdin["outcome"]["plan"];
    assert_eq!(plan["edits"][0]["before_start"], "86");
    assert_eq!(plan["edits"][0]["before_end"], "96");
    assert_eq!(
        plan["before_identity"],
        "905cf0b4eb7376819f47c9f7da02fecb8920d91dd9b76b4cb3454da77fa81887"
    );
    assert_eq!(
        plan["diff_identity"],
        "42faf5376728e4649c80657e6b0f93e9910a107261413a5d0c7f019bf5ab27e0"
    );
    assert!(stdin.get("invoked_by").is_none());
    assert!(stdin.get("unattended").is_none());
}

#[test]
fn typed_questions_domain_and_preflight_have_exact_roots_and_exits() {
    let missing = run(json!({"schema":1,"source_text":BEFORE}), 2, false, false);
    assert_eq!(missing["outcome"]["status"], "needs_input");
    assert_eq!(missing["outcome"]["questions"].as_array().unwrap().len(), 3);
    let mut unknown = request();
    unknown["source_profile"] = json!("unrecognized-r2");
    let domain = run(unknown, 1, true, false);
    assert_eq!(domain["outcome"]["error"]["code"], "unknown_selection");
    let mut malformed = request();
    malformed["source_text"] = json!("[package private-parser-detail");
    let domain = run(malformed, 1, false, false);
    assert_eq!(domain["outcome"]["error"]["code"], "malformed_syntax");
    for (value, code) in [
        (
            json!({"schema":1,"source_text":BEFORE,"unknown":true}),
            "invalid_request",
        ),
        (
            json!({"schema":2,"source_text":BEFORE}),
            "unsupported_schema",
        ),
        (
            json!({"schema":"1","source_text":BEFORE}),
            "invalid_request",
        ),
        (json!({"schema":1,"source_text":false}), "invalid_request"),
    ] {
        let out = run(value, 2, false, false);
        assert_eq!(out["stage"], "preflight");
        assert_eq!(out["error"]["code"], code);
    }
    for field in ["source_profile", "target_profile", "recipe"] {
        for wrong in [Value::Null, json!(7)] {
            let mut value = request();
            value[field] = wrong;
            let out = run(value, 2, true, false);
            assert_eq!(out["error"]["code"], "invalid_request");
        }
    }
    let text = "{";
    let expected = serde_json::to_value(plan_migration_json(text)).unwrap();
    child(text.as_bytes(), None, expected, 2, false);
}

#[test]
fn utf8_crlf_and_epoch_one_no_op_keep_exact_owned_identities() {
    let mut unicode = request();
    unicode["source_text"] = json!(("# café 東京\n".to_owned() + BEFORE).replace('\n', "\r\n"));
    let out = run(unicode.clone(), 0, false, false);
    assert_eq!(out["outcome"]["plan"]["before"], unicode["source_text"]);
    let mut current = request();
    current["source_text"] = json!(
        BEFORE
            .replace("[package]\n", "[package]\nepoch=1\n")
            .replace("snapshot", "copy")
    );
    current["source_profile"] = json!(CURRENT_EPOCH1);
    current["target_profile"] = json!(CURRENT_EPOCH1);
    current["recipe"] = json!(IDENTITY);
    let out = run(current, 0, true, false);
    assert_eq!(out["outcome"]["status"], "no_op");
    let plan = &out["outcome"]["plan"];
    assert_eq!(plan["before"], plan["after"]);
    assert_eq!(plan["before_identity"], plan["after_identity"]);
    assert_eq!(plan["edits"], json!([]));
    assert_eq!(plan["source_epoch"], json!({"kind":"epoch_1"}));
}

#[test]
fn failed_utf8_stdin_read_is_typed_input_read_exit_one() {
    let expected = serde_json::to_value(input_read_error()).unwrap();
    let out = child(&[0xff], None, expected, 1, false);
    assert_eq!(out["error"]["code"], "input_read");
    assert_eq!(out["stage"], "preflight");
}

#[test]
fn early_entry_bypasses_poisoned_configuration_and_preserves_all_state() {
    let out = run(request(), 0, false, true);
    assert_eq!(out["outcome"]["status"], "planned");
    let questions = run(json!({"schema":1,"source_text":BEFORE}), 2, true, true);
    assert_eq!(questions["outcome"]["status"], "needs_input");
}
