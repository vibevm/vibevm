//! Explicit fixed Validate MCP startup over the retained lifecycle host.
#[cfg(target_os = "linux")]
#[path = "cli_mcp_fixed_validate/combined_options.rs"]
mod combined_options;
mod common;
#[cfg(target_os = "linux")]
#[path = "cli_mcp_fixed_validate/partial_effect.rs"]
mod partial_effect;

use common::UserScratch;
use serde_json::{Value, json};
use std::fs;
#[cfg(target_os = "linux")]
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};
#[cfg(target_os = "linux")]
use vibe_orchestrator::machine_operation::{input_decision as d, lifecycle_view as v};

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), "[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n").unwrap();
    let quoted = root.path().to_str().unwrap().replace('\'', "'\\''");
    fs::write(
        root.path().join("run.sh"),
        format!("#!/bin/sh\nset -eu\ncd '{quoted}'\nprintf 'effect\\n' >> counter\n"),
    )
    .unwrap();
    root
}

#[cfg(target_os = "linux")]
struct LiveServer {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

#[cfg(target_os = "linux")]
impl LiveServer {
    fn start(user: &UserScratch, root: &std::path::Path, mode: &str) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env("VIBE_SETTINGS", &user.settings)
            .env("VIBE_REGISTRY_CACHE", &user.cache)
            .env("VIBEVM_SEARCH_CACHE_DIR", &user.search_cache)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .args(["--offline", "mcp", "serve", "--path"])
            .arg(root)
            .args(["--fixed-validate", mode, "--approve-captured-script"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if mode == "issue" {
            command.args(["--ttl-ms", "60000"]);
        }
        let mut child = command.spawn().unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }

    fn call(&mut self, action: Value) -> Value {
        self.input.write_all(request(action).as_bytes()).unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        assert!(
            self.output.read_line(&mut line).unwrap() > 0,
            "MCP reply before EOF"
        );
        let response: Value = serde_json::from_str(&line).unwrap();
        assert!(response["error"].is_null(), "{response}");
        response["result"].clone()
    }

    fn finish(self) {
        drop(self.input);
        let output = self.child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(target_os = "linux")]
fn typed_question(result: &Value) -> d::InputDecisionMessage {
    assert_eq!(result["isError"], false, "{result}");
    serde_json::from_str(
        result["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

#[cfg(target_os = "linux")]
fn typed_view(result: &Value) -> v::LifecycleSubmissionView {
    assert_eq!(result["isError"], false, "{result}");
    serde_json::from_str(
        result["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

#[cfg(target_os = "linux")]
#[test]
fn real_stdio_decision_submit_and_cold_inspect_retain_one_effect() {
    let user = UserScratch::new();
    let root = project();
    let mut server = LiveServer::start(&user, root.path(), "issue");
    let mut question = typed_question(
        &server.call(json!({"kind":"prepare","phase":"validate",
        "requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}})),
    );
    assert!(matches!(question.body, d::Body::SelectionQuestion(_)));
    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    question =
        typed_question(&server.call(
            json!({"kind":"answer","message_json":serde_json::to_string(&question).unwrap()}),
        ));
    assert!(matches!(question.body, d::Body::ApprovalQuestion(_)));
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = server
        .call(json!({"kind":"answer","message_json":serde_json::to_string(&question).unwrap()}));
    assert_eq!(accepted["isError"], false, "{accepted}");
    let reference = accepted["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap();
    let started = server.call(json!({"kind":"submit","accepted_ref":reference,"key":"once"}));
    let mut run = typed_view(&started).run_id;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let view = typed_view(&server.call(json!({"kind":"inspect","key":"once"})));
        if let Some(observed) = &view.run_id {
            match &run {
                Some(expected) => assert_eq!(observed, expected),
                None => run = Some(observed.clone()),
            }
        }
        if matches!(view.status, v::Status::ResultAvailable(_)) {
            assert!(
                run.is_some(),
                "terminal owner result retains its run identity"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "fixed Validate terminal deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let run = run.unwrap();
    server.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
    let mut cold = LiveServer::start(&user, root.path(), "reattach");
    let view = typed_view(&cold.call(json!({"kind":"inspect","key":"once"})));
    assert_eq!(view.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(view.status, v::Status::ResultAvailable(_)));
    cold.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
}

#[cfg(target_os = "linux")]
#[test]
fn real_stdio_explicit_trace_retains_report_and_replays_one_operation() {
    let user = UserScratch::new();
    let root = project();
    let mut server = LiveServer::start(&user, root.path(), "issue");
    let mut question = typed_question(&server.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":false,"trace":true,"deploy_profile":null,"control":"none"}
    })));
    assert!(question.requested_options.trace);
    assert!(matches!(question.body, d::Body::SelectionQuestion(_)));

    let mut premature = question.clone();
    premature.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let refused = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&premature).unwrap()
    }));
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(refused["structuredContent"]["code"], "decision_refused");
    assert!(!root.path().join("counter").exists());

    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = typed_question(&server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&question).unwrap()
    })));
    assert!(approval.requested_options.trace);
    assert!(matches!(approval.body, d::Body::ApprovalQuestion(_)));
    assert_ne!(approval.plan_id, question.plan_id);
    let accepted_ref = approval.question_id.clone().unwrap();
    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&approval).unwrap()
    }));
    assert_eq!(accepted["isError"], false, "{accepted}");
    assert_eq!(
        accepted["structuredContent"]["reply"]["accepted_ref"],
        accepted_ref
    );

    let key = "stdio-trace-once";
    let first = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    let mut run = first.run_id;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let view = typed_view(&server.call(json!({"kind":"inspect", "key":key})));
        if let Some(observed) = &view.run_id {
            if let Some(expected) = &run {
                assert_eq!(observed, expected);
            } else {
                run = Some(observed.clone());
            }
        }
        if matches!(view.status, v::Status::ResultAvailable(_)) {
            break;
        }
        assert!(Instant::now() < deadline, "trace terminal deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    let run = run.expect("terminal result retains its run identity");
    let report: Value = serde_json::from_slice(
        &fs::read(
            root.path()
                .join(format!(".vibe/lifecycle-outcomes/{run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["run_id"], run);
    assert_eq!(report["trace"]["status"], "ok");
    assert_eq!(report["trace"]["finalised"], true);

    let replay = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    assert_eq!(replay.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(replay.status, v::Status::ResultAvailable(_)));
    server.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
}

#[cfg(target_os = "linux")]
#[test]
fn real_stdio_explicit_force_keeps_approved_intent_and_one_effect() {
    let user = UserScratch::new();
    let root = project();
    let mut server = LiveServer::start(&user, root.path(), "issue");
    let mut selection = typed_question(&server.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":true,"trace":false,"deploy_profile":null,"control":"none"}
    })));
    assert!(selection.requested_options.force);
    assert!(matches!(selection.body, d::Body::SelectionQuestion(_)));
    selection.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = typed_question(&server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&selection).unwrap()
    })));
    assert!(approval.requested_options.force);
    assert!(matches!(approval.body, d::Body::ApprovalQuestion(_)));
    let accepted_ref = approval.question_id.clone().unwrap();

    let mut changed = approval.clone();
    changed.requested_options.force = false;
    changed.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let refused = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&changed).unwrap()
    }));
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(!root.path().join("counter").exists());

    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&approval).unwrap()
    }));
    assert_eq!(accepted["isError"], false, "{accepted}");
    assert_eq!(
        accepted["structuredContent"]["reply"]["accepted_ref"],
        accepted_ref
    );

    let key = "stdio-force-once";
    let first = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    let mut run = first.run_id;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let view = typed_view(&server.call(json!({"kind":"inspect", "key":key})));
        if let Some(observed) = &view.run_id {
            if let Some(expected) = &run {
                assert_eq!(observed, expected);
            } else {
                run = Some(observed.clone());
            }
        }
        if matches!(view.status, v::Status::ResultAvailable(_)) {
            break;
        }
        assert!(Instant::now() < deadline, "force terminal deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    let run = run.expect("terminal result retains its run identity");
    let admitted: Value = serde_json::from_slice(
        &fs::read(root.path().join(format!(
            ".vibe/lifecycle-outcomes/{run}.admitted-input.json"
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(admitted["request"]["requested_options"]["force"], true);
    assert_eq!(admitted["plan"]["requested_options"]["force"], true);
    assert_eq!(admitted["plan"]["base_v4"]["force"], false);
    let replay = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    assert_eq!(replay.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(replay.status, v::Status::ResultAvailable(_)));
    server.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
}

#[cfg(target_os = "linux")]
#[test]
fn real_stdio_unsupported_phase_and_profile_refuse_before_effect() {
    let user = UserScratch::new();
    let root = project();
    let mut server = LiveServer::start(&user, root.path(), "issue");
    for (phase, profile) in [("build", Value::Null), ("validate", json!("posix"))] {
        let refused = server.call(json!({
            "kind":"prepare", "phase":phase,
            "requested_options":{
                "force":false,"trace":false,"deploy_profile":profile,"control":"none"
            }
        }));
        assert_eq!(refused["isError"], true, "{phase}: {refused}");
        assert_eq!(
            refused["structuredContent"]["code"], "implementation_gap",
            "{phase}: {refused}"
        );
        assert!(!root.path().join("counter").exists());
        assert!(!root.path().join(".vibe/lifecycle-outcomes").exists());
    }
    server.finish();
}

fn request(action: Value) -> String {
    json!({"jsonrpc":"2.0","id":71,"method":"tools/call",
        "params":{"name":"lifecycle_operation","arguments":{"schema":1,"action":action}}})
    .to_string()
        + "\n"
}
fn prepare() -> String {
    request(
        json!({"kind":"prepare","phase":"validate","requested_options":{
        "force":false,"trace":false,"deploy_profile":null,"control":"none"}}),
    )
}
fn serve(user: &UserScratch, root: &std::path::Path, mode: Option<&str>, input: String) -> Value {
    let mut command = user.vibe();
    command
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(root);
    if let Some(mode) = mode {
        command.args(["--fixed-validate", mode, "--approve-captured-script"]);
        if mode == "issue" {
            command.args(["--ttl-ms", "60000"]);
        }
    }
    let output = command.write_stdin(input).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    serde_json::from_str(text.trim()).unwrap()
}
fn question(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    serde_json::from_str(
        reply["result"]["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn issue_and_reattach_use_one_host_but_never_restore_a_process_local_question() {
    let user = UserScratch::new();
    let root = project();
    let ungranted = serve(&user, root.path(), None, prepare());
    assert_eq!(ungranted["result"]["isError"], true);
    assert_eq!(
        ungranted["result"]["structuredContent"]["code"],
        "unavailable_host"
    );
    assert!(!root.path().join(".vibe").exists());

    let first = question(&serve(&user, root.path(), Some("issue"), prepare()));
    let old_id = first["question_id"].as_str().unwrap().to_owned();
    let mut answer = first.clone();
    answer["body"] = match first["body"]["kind"].as_str().unwrap() {
        "selection_question" => json!({"kind":"selection_answer","choice_ids":["none"]}),
        "approval_question" => json!({"kind":"approval_answer","approved":true}),
        other => panic!("unexpected question: {other}"),
    };
    let stale = serve(
        &user,
        root.path(),
        Some("reattach"),
        request(json!({
            "kind":"answer","message_json":answer.to_string()
        })),
    );
    assert_eq!(stale["result"]["isError"], true, "{stale}");
    assert_eq!(
        stale["result"]["structuredContent"]["code"],
        "decision_refused"
    );
    let fresh = question(&serve(&user, root.path(), Some("reattach"), prepare()));
    assert_ne!(fresh["question_id"], old_id);
    assert!(!root.path().join("counter").exists());
    let index: Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn machine_frame_issued_validate_grant_is_reused_by_mcp() {
    let user = UserScratch::new();
    let root = project();
    let issued = user
        .vibe()
        .args([
            "machine",
            "session",
            "--fixed-validate",
            "issue",
            "--ttl-ms",
            "60000",
            "--approve-captured-script",
            "--root",
        ])
        .arg(root.path())
        .write_stdin("")
        .output()
        .unwrap();
    assert!(
        issued.status.success(),
        "{}",
        String::from_utf8_lossy(&issued.stderr)
    );
    assert!(issued.stdout.is_empty());
    let next = question(&serve(&user, root.path(), Some("reattach"), prepare()));
    assert!(next["question_id"].is_string());
    assert!(!root.path().join("counter").exists());
}

#[test]
fn validate_startup_refuses_relative_root_missing_lifetime_and_missing_pointer() {
    let user = UserScratch::new();
    let root = project();
    let relative = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-validate",
            "issue",
            "--ttl-ms",
            "60000",
        ])
        .write_stdin(prepare())
        .output()
        .unwrap();
    assert!(!relative.status.success());
    assert!(relative.stdout.is_empty());
    assert!(String::from_utf8_lossy(&relative.stderr).contains("absolute --path"));
    let missing_lifetime = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-validate",
            "issue",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(prepare())
        .output()
        .unwrap();
    assert!(!missing_lifetime.status.success());
    assert!(missing_lifetime.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing_lifetime.stderr).contains("requires --ttl-ms"));
    let missing_pointer = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-validate",
            "reattach",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(prepare())
        .output()
        .unwrap();
    assert!(!missing_pointer.status.success());
    assert!(missing_pointer.stdout.is_empty());
    assert!(!root.path().join("counter").exists());
}
