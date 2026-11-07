//! Actual isolated CLI operation transport; authored grants are temporary only.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES");
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use vibe_core::manifest::{CURRENT_ABSENT, LEGACY_ABSENT, SNAPSHOT_TO_COPY};
use vibe_wire::generated::migration::e1::operation_report::*;
use vibe_workspace::{
    migration_application::validate_operation_report, migration_planning::plan_migration_json,
};

const BEFORE: &str = "# café and quoted value stay exact\n[package]\nname=\"demo\"\ngroup=\"org.example\"\nkind=\"flow\"\nversion=\"1.1.0\"\nmaterialization=\"snapshot\" # keep\n";
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            if path.is_symlink() {
                out.insert(
                    key,
                    std::fs::read_link(&path)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                );
            } else if path.is_dir() {
                out.insert(key, Vec::new());
                walk(root, &path, out);
            } else {
                out.insert(key, std::fs::read(path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
struct Fixture {
    temp: tempfile::TempDir,
    home: std::path::PathBuf,
    cwd: std::path::PathBuf,
    project: std::path::PathBuf,
    store: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let cwd = temp.path().join("cwd");
        let project = temp.path().join("project");
        let store = temp.path().join("recovery");
        for p in [&home, &cwd, &project] {
            std::fs::create_dir(p).unwrap();
        }
        std::fs::create_dir(home.join(".vibe")).unwrap();
        for name in ["config.toml", "settings.toml", "registry.toml"] {
            std::fs::write(home.join(".vibe").join(name), "[broken config\n").unwrap();
        }
        std::fs::write(project.join("vibe.toml"), BEFORE).unwrap();
        Self {
            temp,
            home,
            cwd,
            project,
            store,
        }
    }
    fn authority(&self) -> Value {
        json!({"project_root":self.project,"target_relative":"vibe.toml","recovery_store_root":self.store,"scope":"owned_authored_file"})
    }
    fn inspect(&self) -> Value {
        json!({"schema":1,"operation":"inspect","operation_id":"owned-example","authority":self.authority()})
    }
    fn apply(&self) -> Value {
        let plan_request = json!({"schema":1,"source_text":BEFORE,"source_profile":LEGACY_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":SNAPSHOT_TO_COPY});
        let text = plan_request.to_string();
        let planned = serde_json::to_value(plan_migration_json(&text)).unwrap();
        let plan = &planned["outcome"]["plan"];
        assert_eq!(planned["outcome"]["status"], "planned");
        json!({"schema":1,"operation":"apply","operation_id":"owned-example","authority":self.authority(),"approved_request_json":text,"before_content_id":plan["before_identity"],"after_content_id":plan["after_identity"],"diff_id":plan["diff_identity"]})
    }
    fn recover(&self, intent: &Value, action: &str) -> Value {
        json!({"schema":1,"operation":"recover","operation_id":"owned-example","authority":self.authority(),"expected_intent_id":intent,"action":action})
    }
    fn child(&self, bytes: &[u8], literal: Option<&str>, exit: i32) -> Value {
        self.child_grant(bytes, literal, exit, true)
    }
    fn child_grant(&self, bytes: &[u8], literal: Option<&str>, exit: i32, grant: bool) -> Value {
        let home_before = tree(&self.home);
        let cwd_before = tree(&self.cwd);
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("VIBE_SETTINGS", self.home.join(".vibe"))
            .env("VIBE_LOG", "invalid=???")
            .env("VIBEVM_HOME", self.temp.path().join("stale-vvm"))
            .current_dir(&self.cwd)
            .args(["refactor", "migration-operation"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if grant {
            command
                .arg("--project-root")
                .arg(&self.project)
                .args(["--target-relative", "vibe.toml"])
                .arg("--recovery-store")
                .arg(&self.store);
        }
        if let Some(text) = literal {
            command.args(["--request-json", text]);
        }
        let mut child = command.spawn().unwrap();
        child.stdin.take().unwrap().write_all(bytes).unwrap();
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
            "unexpected diagnostic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = std::str::from_utf8(&output.stdout).unwrap();
        assert!(text.ends_with('\n'));
        assert_eq!(text.lines().count(), 1);
        let report: OperationReport = serde_json::from_str(text).unwrap();
        validate_operation_report(&report).unwrap();
        assert_eq!(tree(&self.home), home_before);
        assert_eq!(tree(&self.cwd), cwd_before);
        let value = serde_json::to_value(report).unwrap();
        assert!(value.get("invoked_by").is_none());
        value
    }
    fn run(&self, value: &Value, exit: i32, literal: bool) -> Value {
        let text = value.to_string();
        self.child(
            if literal {
                b"ignored stdin"
            } else {
                text.as_bytes()
            },
            literal.then_some(text.as_str()),
            exit,
        )
    }
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
fn malformed_schema_null_and_input_read_preflight_preserve_poisoned_state() {
    let f = Fixture::new();
    let before = tree(f.temp.path());
    for input in [b"{".as_slice(), b"null"] {
        let report = f.child(input, None, 2);
        assert_eq!(report["error"]["code"], "invalid_request");
    }
    for (value, code) in [
        (json!({"schema":1,"operation":"inspect"}), "invalid_request"),
        (
            {
                let mut r = f.inspect();
                r["authority"]["extra"] = json!(true);
                r
            },
            "invalid_request",
        ),
        (
            {
                let mut r = f.inspect();
                r["operation_id"] = Value::Null;
                r
            },
            "invalid_request",
        ),
        (
            {
                let mut r = f.inspect();
                r["schema"] = json!(2);
                r["authority"]["project_root"] = json!("/nonexistent/project");
                r
            },
            "unsupported_schema",
        ),
    ] {
        let out = f.run(&value, 2, true);
        assert_eq!(out["error"]["code"], code);
        assert_eq!(out["effects"], json!([]));
    }
    for embedded in [
        "{",
        "{\"schema\":2,\"source_text\":\"x\"}",
        "{\"schema\":1,\"source_text\":\"x\",\"recipe\":null}",
    ] {
        let mut request = f.apply();
        request["authority"]["project_root"] = json!(f.temp.path().join("missing-project"));
        request["approved_request_json"] = json!(embedded);
        let report = f.run(&request, 2, false);
        assert_eq!(report["error"]["field"], "approved_request_json");
        assert_eq!(report["error"]["code"], "invalid_request");
        assert_eq!(report["effects"], json!([]));
    }
    let read = f.child(&[0xff], None, 1);
    assert_eq!(read["error"]["field"], "request.input");
    assert_eq!(tree(f.temp.path()), before);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
fn absent_store_inspection_stdin_literal_parity_creates_nothing() {
    let f = Fixture::new();
    let before = tree(f.temp.path());
    let stdin = f.run(&f.inspect(), 1, false);
    let literal = f.run(&f.inspect(), 1, true);
    assert_eq!(stdin, literal);
    assert_eq!(stdin["outcome"], "refused");
    assert_eq!(stdin["error"]["code"], "journal_invalid");
    assert!(!f.store.exists());
    assert_eq!(tree(f.temp.path()), before);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
fn apply_inspect_restore_terminal_replay_preserve_reader_bytes_and_mode() {
    let f = Fixture::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            f.project.join("vibe.toml"),
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
    }
    let request = f.apply();
    let first = f.run(&request, 0, false);
    assert_eq!(first["outcome"], "completed");
    let after = BEFORE.replace("\"snapshot\"", "\"copy\"");
    assert_eq!(
        std::fs::read_to_string(f.project.join("vibe.toml")).unwrap(),
        after
    );
    // Existing planner/current-reader validation is independently replayed against exact output.
    let plan=serde_json::to_value(plan_migration_json(&json!({"schema":1,"source_text":after,"source_profile":CURRENT_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":vibe_core::manifest::IDENTITY}).to_string())).unwrap();
    assert_eq!(plan["outcome"]["status"], "no_op");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(f.project.join("vibe.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }
    let snapshot = tree(f.temp.path());
    let replay = f.run(&request, 0, true);
    assert_eq!(replay["outcome"], "no_op");
    assert_eq!(tree(f.temp.path()), snapshot);
    let inspected = f.run(&f.inspect(), 0, false);
    assert_eq!(inspected["outcome"], "completed");
    assert_eq!(inspected["intent_id"], first["intent_id"]);
    assert_eq!(tree(f.temp.path()), snapshot);
    // Test-created foreign stage blocks a real restore after its Restoring
    // checkpoint. Inspect must expose that actual pending prefix without writes.
    let stage = f.project.join(
        vibe_workspace::migration_application::identity::transaction_stage_name(
            first["intent_id"].as_str().unwrap(),
            vibe_workspace::migration_application::identity::StageAction::Restore,
        ),
    );
    std::fs::write(&stage, b"test-owned foreign stage bytes").unwrap();
    let refused = f.run(&f.recover(&first["intent_id"], "restore"), 1, false);
    assert_eq!(refused["outcome"], "refused");
    assert_eq!(
        std::fs::read(&stage).unwrap(),
        b"test-owned foreign stage bytes"
    );
    assert_eq!(
        std::fs::read_to_string(f.project.join("vibe.toml")).unwrap(),
        after
    );
    let slots = std::fs::read_dir(f.store.join("migrations"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 1);
    let journal: vibe_wire::generated::migration::e1::application_journal::ApplicationJournal =
        serde_json::from_slice(&std::fs::read(slots[0].join("journal.json")).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&journal.phase).unwrap(), "restoring");
    let pending_before = tree(f.temp.path());
    let pending = f.run(&f.inspect(), 2, true);
    assert_eq!(pending["outcome"], "pending");
    assert_eq!(pending["intent_id"], first["intent_id"]);
    assert_eq!(tree(f.temp.path()), pending_before);
    // Only this fixture-created occupant is removed by the test, never runtime.
    std::fs::remove_file(&stage).unwrap();
    let restored = f.run(&f.recover(&first["intent_id"], "restore"), 0, true);
    assert_eq!(restored["outcome"], "completed");
    assert_eq!(
        std::fs::read_to_string(f.project.join("vibe.toml")).unwrap(),
        BEFORE
    );
    let snapshot = tree(f.temp.path());
    let terminal = f.run(&f.recover(&first["intent_id"], "restore"), 0, false);
    assert_eq!(terminal["outcome"], "no_op");
    assert_eq!(tree(f.temp.path()), snapshot);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
fn mismatched_plan_ids_and_changed_source_refuse_preserving_target() {
    let f = Fixture::new();
    let mut forged = f.apply();
    forged["before_content_id"] = json!(format!("sha256:{}", "0".repeat(64)));
    let out = f.run(&forged, 1, false);
    assert_eq!(out["error"]["code"], "intent_mismatch");
    assert!(!f.store.exists());
    let request = f.apply();
    std::fs::write(f.project.join("vibe.toml"), "# third state\n").unwrap();
    let out = f.run(&request, 1, true);
    assert_eq!(out["error"]["code"], "source_changed");
    assert_eq!(
        std::fs::read_to_string(f.project.join("vibe.toml")).unwrap(),
        "# third state\n"
    );
    assert!(
        out["effects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "barrier" && e["state"] == "completed")
    );
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
fn third_state_recovery_and_foreign_intent_retain_journal_and_effects() {
    let f = Fixture::new();
    let applied = f.run(&f.apply(), 0, false);
    let foreign = f.recover(&json!(format!("sha256:{}", "0".repeat(64))), "complete");
    let before = tree(f.temp.path());
    let out = f.run(&foreign, 1, true);
    assert_eq!(out["error"]["code"], "intent_mismatch");
    assert_eq!(tree(f.temp.path()), before);
    std::fs::write(f.project.join("vibe.toml"), "# external third state\n").unwrap();
    let before = tree(f.temp.path());
    let out = f.run(&f.recover(&applied["intent_id"], "restore"), 1, false);
    assert_eq!(out["target_state"], "third_state");
    assert_eq!(tree(f.temp.path()), before);
    assert!(
        out["effects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "journal" && e["state"] == "completed")
    );
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 1
)]
fn frozen_source_escape_and_link_refuse_without_target_write() {
    let f = Fixture::new();
    let mut frozen = f.apply();
    let mut plan: Value =
        serde_json::from_str(frozen["approved_request_json"].as_str().unwrap()).unwrap();
    plan["source_text"] = json!(BEFORE.replace("[package]", "[package]\nepoch=99"));
    frozen["approved_request_json"] = json!(plan.to_string());
    let before = tree(f.temp.path());
    let out = f.run(&frozen, 1, false);
    assert_eq!(out["error"]["code"], "plan_refused");
    assert_eq!(tree(f.temp.path()), before);
    let mut escape = f.inspect();
    escape["authority"]["target_relative"] = json!("../vibe.toml");
    let out = f.run(&escape, 1, true);
    assert_eq!(out["error"]["code"], "authority_refused");
    assert!(!f.store.exists());
    #[cfg(unix)]
    {
        std::fs::rename(f.project.join("vibe.toml"), f.project.join("actual.toml")).unwrap();
        std::os::unix::fs::symlink("actual.toml", f.project.join("vibe.toml")).unwrap();
        let before = tree(f.temp.path());
        let out = f.run(&f.inspect(), 1, false);
        assert_eq!(out["outcome"], "refused");
        assert_eq!(tree(f.temp.path()), before);
    }
}

#[test]
fn independent_missing_wrong_and_protected_grants_have_no_effects() {
    let mut f = Fixture::new();
    let before = tree(f.temp.path());
    let request = f.apply();
    let out = f.child_grant(b"", Some(&request.to_string()), 1, false);
    assert_eq!(out["error"]["code"], "authority_refused");
    assert_eq!(out["effects"], json!([]));
    for (key, value) in [
        ("project_root", json!(f.temp.path().join("absent"))),
        ("target_relative", json!("member/vibe.toml")),
        ("recovery_store_root", json!(f.temp.path().join("other"))),
    ] {
        let mut request = f.apply();
        request["authority"][key] = value;
        let out = f.run(&request, 1, true);
        assert_eq!(out["error"]["code"], "authority_refused");
        assert_eq!(out["effects"], json!([]));
    }
    f.store = f.home.join(".vibe/cache/recovery");
    let out = f.run(&f.apply(), 1, false);
    assert_eq!(out["error"]["code"], "authority_refused");
    assert_eq!(out["effects"], json!([]));
    assert_eq!(tree(f.temp.path()), before);
}

#[test]
fn nonquiet_mcp_startup_composes_only_explicit_finite_authority() {
    let f = Fixture::new();
    // The normal server startup owns user configuration; use an isolated clean home.
    for name in ["config.toml", "settings.toml", "registry.toml"] {
        std::fs::remove_file(f.home.join(".vibe").join(name)).unwrap();
    }
    for grant_case in 0..3 {
        let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":"manifest_migration_operation", "arguments":f.inspect()}});
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", &f.home)
            .env("USERPROFILE", &f.home)
            .env("VIBE_SETTINGS", f.home.join(".vibe"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .current_dir(&f.cwd)
            .args(["mcp", "serve", "--path"])
            .arg(&f.project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if grant_case != 0 {
            command.arg("--migration-recovery-store").arg(&f.store);
        }
        if grant_case == 2 {
            command.args(["--migration-file", "member/vibe.toml"]);
        }
        let mut child = command.spawn().unwrap();
        writeln!(child.stdin.take().unwrap(), "{request}").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            response["result"]["structuredContent"]["error"]["code"],
            if grant_case == 1 {
                "journal_invalid"
            } else {
                "authority_refused"
            }
        );
        assert!(!f.store.exists());
    }
}
