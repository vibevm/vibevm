//! Actual CLI frames and MCP tools/call sharing one retained C1 host.
use super::*;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use vibe_core::manifest::TargetOs;
use vibe_mcp::{ServerContext, dispatch_one};
use vibe_orchestrator::{
    machine_operation::fixed_lifecycle::FixedLifecycleRoute,
    machine_submission::{HostPolicyFacts, TrustedPolicySource, cli_grant::CliGrantService},
};
use vibe_safefs::Project;

fn route_for_cli_root(root: &Path) -> Arc<FixedLifecycleRoute> {
    let project = Arc::new(Project::open(root).unwrap());
    let target = format!(
        "physical-root:{}",
        project.root_correlation_token().unwrap()
    );
    let service = CliGrantService::from_trusted_host(
        project,
        target,
        "vibe-cli-project-local/e1".into(),
        TargetOs::Linux,
        TrustedPolicySource::from_host_identifier("vibe-cli/fixed-validate/e1".into()).unwrap(),
        HostPolicyFacts {
            approve_captured_script: true,
            allow_detach: true,
            allow_cancel_pre_dispatch: true,
            allow_cancel_deferred_boundary: true,
        },
    )
    .unwrap();
    Arc::new(FixedLifecycleRoute::new(Arc::new(
        service.reattach().unwrap(),
    )))
}

fn mcp_call(route: &Arc<FixedLifecycleRoute>, root: &Path, action: Value) -> Value {
    let context = ServerContext::with_store_root(root, root.join("unused-cache"))
        .with_fixed_lifecycle_route(route.clone())
        .unwrap();
    let request = json!({
        "jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":"lifecycle_operation", "arguments":{"schema":1,"action":action}}
    });
    let response: Value =
        serde_json::from_str(&dispatch_one(context, &request.to_string()).unwrap()).unwrap();
    assert!(response["error"].is_null(), "{response}");
    response["result"].clone()
}

fn mcp_question(result: &Value) -> d::InputDecisionMessage {
    serde_json::from_str(
        result["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

fn mcp_approve(route: &Arc<FixedLifecycleRoute>, root: &Path, control: &str) -> String {
    let mut question = mcp_question(&mcp_call(route, root, prepare(control)));
    if matches!(question.body, d::Body::SelectionQuestion(_)) {
        question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        question = mcp_question(&mcp_call(route, root, answer(&question)));
    }
    assert!(matches!(question.body, d::Body::ApprovalQuestion(_)));
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = mcp_call(route, root, answer(&question));
    assert_eq!(accepted["isError"], false, "{accepted}");
    accepted["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap()
        .into()
}

fn mcp_row(result: &Value) -> v::LifecycleSubmissionView {
    serde_json::from_str(
        result["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap_or_else(|| panic!("MCP returned no view: {result}")),
    )
    .unwrap()
}

struct ProcessMcp {
    child: Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
}

impl ProcessMcp {
    fn reattach(fixture: &Fixture) -> Self {
        let mut command = fixture.command();
        command
            .args(["--offline", "mcp", "serve", "--path"])
            .arg(fixture.root())
            .args(["--fixed-validate", "reattach"])
            .args(FLAGS);
        let mut child = command.spawn().unwrap();
        Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }

    fn call(&mut self, action: Value) -> Value {
        let request = json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/call",
            "params":{"name":"lifecycle_operation", "arguments":{"schema":1,"action":action}}
        });
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        if self.output.read_line(&mut line).unwrap() == 0 {
            let mut stderr = String::new();
            self.child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            panic!("MCP reply before EOF: {stderr}");
        }
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

fn process_mcp_approve(mcp: &mut ProcessMcp, control: &str) -> String {
    let mut question = mcp_question(&mcp.call(prepare(control)));
    if matches!(question.body, d::Body::SelectionQuestion(_)) {
        question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        question = mcp_question(&mcp.call(answer(&question)));
    }
    assert!(matches!(question.body, d::Body::ApprovalQuestion(_)));
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = mcp.call(answer(&question));
    assert_eq!(accepted["isError"], false, "{accepted}");
    accepted["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap()
        .into()
}

fn cli_approve_combined_options(cli: &mut Client) -> String {
    let mut question = cli
        .call(json!({
            "kind":"prepare", "phase":"validate",
            "requested_options":{"force":true,"trace":true,"deploy_profile":null,"control":"none"}
        }))
        .question();
    assert!(question.requested_options.force && question.requested_options.trace);
    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    question = cli.call(answer(&question)).question();
    assert!(question.requested_options.force && question.requested_options.trace);
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(reply) = cli.call(answer(&question)).reply().reply else {
        panic!("combined options CLI approval")
    };
    reply.accepted_ref
}

fn process_mcp_approve_force_options(mcp: &mut ProcessMcp, trace: bool) -> String {
    let mut question = mcp_question(&mcp.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":true,"trace":trace,"deploy_profile":null,"control":"none"}
    })));
    assert!(question.requested_options.force);
    assert_eq!(question.requested_options.trace, trace);
    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    question = mcp_question(&mcp.call(answer(&question)));
    assert!(question.requested_options.force);
    assert_eq!(question.requested_options.trace, trace);
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = mcp.call(answer(&question));
    assert_eq!(accepted["isError"], false, "{accepted}");
    accepted["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn cli_frame_and_mcp_stdio_combined_options_share_one_run_and_trace() {
    let fixture = Fixture::new(true);
    let mut cli = fixture.client("issue");
    let cli_reference = cli_approve_combined_options(&mut cli);
    for name in ["config.toml", "settings.toml", "registry.toml"] {
        fs::write(fixture.dir.path().join("home/.vibe").join(name), "").unwrap();
    }
    let mut mcp = ProcessMcp::reattach(&fixture);
    let mcp_reference = process_mcp_approve_force_options(&mut mcp, true);
    let key = "combined-cli-mcp";
    let cli_submit = cli.call(submit(&cli_reference, key));
    assert!(matches!(cli_submit.envelope.body, e::Body::Result(_)));
    until(|| fixture.root().join("entered").exists());
    fixture.count_one();
    let mcp_submit = mcp.call(submit(&mcp_reference, key));
    assert!(
        mcp_submit["isError"] == false
            || mcp_submit["structuredContent"]["code"] == "inspection_unavailable",
        "{mcp_submit}"
    );
    fixture.release();
    let cli_terminal = terminal(&mut cli, key).row();
    let run = cli_terminal.run_id.clone().expect("terminal C1 run");
    let deadline = Instant::now() + Duration::from_secs(20);
    let mcp_terminal = loop {
        let reply = mcp.call(inspect(key));
        if reply["isError"] == false {
            let view = mcp_row(&reply);
            if matches!(view.status, v::Status::ResultAvailable(_)) {
                break view;
            }
        } else {
            assert_eq!(reply["structuredContent"]["code"], "inspection_unavailable");
        }
        assert!(
            Instant::now() < deadline,
            "MCP combined options terminal deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_same_terminal_truth(&cli_terminal, &mcp_terminal);
    let replay = mcp.call(submit(&mcp_reference, key));
    assert_eq!(replay["isError"], false, "{replay}");
    assert_eq!(mcp_row(&replay).run_id.as_deref(), Some(run.as_str()));
    let changed_ref = process_mcp_approve_force_options(&mut mcp, false);
    let changed = mcp.call(submit(&changed_ref, key));
    assert_eq!(changed["isError"], true, "{changed}");
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    cli.finish();
    mcp.finish();
    fixture.count_one();
    let admitted: Value = serde_json::from_slice(
        &fs::read(fixture.root().join(format!(
            ".vibe/lifecycle-outcomes/{run}.admitted-input.json"
        )))
        .unwrap(),
    )
    .unwrap();
    for stage in ["request", "plan"] {
        assert_eq!(admitted[stage]["requested_options"]["force"], true);
        assert_eq!(admitted[stage]["requested_options"]["trace"], true);
    }
    let report: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .root()
                .join(format!(".vibe/lifecycle-outcomes/{run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["run_id"], run);
    assert_eq!(report["trace"]["status"], "ok");
    assert_eq!(report["trace"]["finalised"], true);
}

#[test]
fn cli_frame_and_mcp_stdio_share_failed_post_effect_truth() {
    let fixture = Fixture::new(true);
    let script = fixture.root().join("run.sh");
    let mut bytes = fs::read(&script).unwrap();
    bytes.extend_from_slice(b"exit 17\n");
    fs::write(&script, bytes).unwrap();
    let mut cli = fixture.client("issue");
    let cli_reference = approve(&mut cli);
    for name in ["config.toml", "settings.toml", "registry.toml"] {
        fs::write(fixture.dir.path().join("home/.vibe").join(name), "").unwrap();
    }
    let mut mcp = ProcessMcp::reattach(&fixture);
    let mcp_reference = process_mcp_approve(&mut mcp, "none");
    let key = "failed-cli-mcp";
    let started = cli.call(submit(&cli_reference, key));
    assert!(matches!(started.envelope.body, e::Body::Result(_)));
    until(|| fixture.root().join("entered").exists());
    fixture.count_one();
    let contender = mcp.call(submit(&mcp_reference, key));
    assert!(
        contender["isError"] == false
            || contender["structuredContent"]["code"] == "inspection_unavailable",
        "{contender}"
    );
    fixture.release();
    let cli_terminal = terminal(&mut cli, key).row();
    let run = cli_terminal.run_id.clone().expect("failed C1 run identity");
    assert!(cli_terminal.partial);
    assert_eq!(
        cli_terminal
            .last_phase
            .as_ref()
            .and_then(|p| p.status.as_deref()),
        Some("fail")
    );
    assert!(
        cli_terminal
            .work_outcomes
            .iter()
            .any(|w| w.status == "fail")
    );
    assert!(matches!(
        cli_terminal.status,
        v::Status::ResultAvailable(ref result)
            if matches!(result.outcome, v::StatusResultAvailableOutcome::Failed)
                && matches!(result.effect_state, v::StatusResultAvailableEffectState::Uncertain)
    ));
    let deadline = Instant::now() + Duration::from_secs(20);
    let mcp_terminal = loop {
        let reply = mcp.call(inspect(key));
        if reply["isError"] == false {
            let view = mcp_row(&reply);
            if matches!(view.status, v::Status::ResultAvailable(_)) {
                break view;
            }
        } else {
            assert_eq!(reply["structuredContent"]["code"], "inspection_unavailable");
        }
        assert!(Instant::now() < deadline, "failed MCP terminal deadline");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_same_terminal_truth(&cli_terminal, &mcp_terminal);
    let replay = mcp.call(submit(&mcp_reference, key));
    assert_eq!(replay["isError"], false, "{replay}");
    assert_eq!(mcp_row(&replay).run_id.as_deref(), Some(run.as_str()));
    cli.finish();
    mcp.finish();
    fixture.count_one();
    let keys = fixture.index()["namespaces"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["keys"]
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[key]["domain_run_id"], run);
}

#[test]
fn real_cli_frame_and_mcp_stdio_race_one_fixed_validate_key() {
    let fixture = Fixture::new(true);
    let mut cli = fixture.client("issue");
    let cli_reference = approve(&mut cli);
    // The framed path tolerates intentionally malformed user settings in
    // this fixture; the independent MCP process loads them at startup.
    for name in ["config.toml", "settings.toml", "registry.toml"] {
        fs::write(fixture.dir.path().join("home/.vibe").join(name), "").unwrap();
    }
    let mut mcp = ProcessMcp::reattach(&fixture);
    let mcp_reference = process_mcp_approve(&mut mcp, "none");
    let barrier = std::sync::Barrier::new(3);
    let (cli_result, mcp_result) = std::thread::scope(|scope| {
        let cli_wait = &barrier;
        let mcp_wait = &barrier;
        let cli_task = scope.spawn(|| {
            cli_wait.wait();
            cli.call(submit(&cli_reference, "shared-process"))
        });
        let mcp_task = scope.spawn(|| {
            mcp_wait.wait();
            mcp.call(submit(&mcp_reference, "shared-process"))
        });
        barrier.wait();
        (cli_task.join().unwrap(), mcp_task.join().unwrap())
    });
    until(|| fixture.root().join("entered").exists());
    fixture.count_one();
    if matches!(&cli_result.envelope.body, e::Body::Error(_)) {
        let _ = cli_result.error();
        let effects = serde_json::to_value(&cli_result.outcome.effect_state).unwrap();
        assert!(
            matches!(effects.as_str(), Some("uncertain" | "unavailable")),
            "active lease contention has uncertain effect truth: {effects}"
        );
    }
    if mcp_result["isError"] == true {
        assert!(
            matches!(
                mcp_result["structuredContent"]["code"].as_str(),
                Some("inspection_unavailable" | "submission_refused")
            ),
            "{mcp_result}"
        );
        assert_eq!(
            mcp_result["structuredContent"]["effect_observation"],
            "unavailable"
        );
    }
    fixture.release();
    let mut cli_terminal = None;
    until(|| {
        let publication = cli.call(inspect("shared-process"));
        if matches!(&publication.envelope.body, e::Body::Error(_)) {
            assert_eq!(publication.error().code, Code::InspectionUnavailable);
            return false;
        }
        if matches!(publication.row().status, v::Status::ResultAvailable(_)) {
            cli_terminal = Some(publication.row());
            true
        } else {
            false
        }
    });
    let cli_terminal = cli_terminal.unwrap();
    let run = cli_terminal.run_id.clone().expect("terminal run identity");
    assert!(matches!(cli_terminal.status, v::Status::ResultAvailable(_)));
    if matches!(&cli_result.envelope.body, e::Body::Result(_)) {
        if let Some(started) = cli_result.row().run_id {
            assert_eq!(started, run);
        }
    }
    if mcp_result["isError"] == false {
        if let Some(started) = mcp_row(&mcp_result).run_id {
            assert_eq!(started, run);
        }
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    let mcp_terminal = loop {
        let response = mcp.call(inspect("shared-process"));
        if response["isError"] == false {
            let row = mcp_row(&response);
            if matches!(row.status, v::Status::ResultAvailable(_)) {
                break row;
            }
        } else {
            assert_eq!(
                response["structuredContent"]["code"], "inspection_unavailable",
                "{response}"
            );
            assert_eq!(
                response["structuredContent"]["effect_observation"],
                "unavailable"
            );
        }
        assert!(
            Instant::now() < deadline,
            "MCP terminal deadline: {response}"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_same_terminal_truth(&cli_terminal, &mcp_terminal);
    let cli_replay = cli.call(submit(&cli_reference, "shared-process"));
    assert!(matches!(&cli_replay.envelope.body, e::Body::Result(_)));
    assert_eq!(cli_replay.row().run_id.as_deref(), Some(run.as_str()));
    let replay = mcp.call(submit(&mcp_reference, "shared-process"));
    assert_eq!(replay["isError"], false, "{replay}");
    assert_eq!(mcp_row(&replay).run_id.as_deref(), Some(run.as_str()));
    let changed_reference = process_mcp_approve(&mut mcp, "detach");
    let changed = mcp.call(submit(&changed_reference, "shared-process"));
    assert_eq!(changed["isError"], true, "{changed}");
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    cli.finish();
    mcp.finish();
    fixture.count_one();
    let namespaces = fixture.index()["namespaces"].as_object().unwrap().clone();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["shared-process"]["domain_run_id"], run);
}

#[test]
fn cli_and_mcp_racing_one_scoped_key_share_the_c1_run_and_terminal_truth() {
    let fixture = Fixture::new(true);
    let root = fixture.root();
    let mut cli = fixture.client("issue");
    let cli_reference = approve(&mut cli);
    let route = route_for_cli_root(&root);
    let mcp_reference = mcp_approve(&route, &root, "none");
    let cli_result = cli.call(submit(&cli_reference, "shared"));
    assert!(matches!(cli_result.envelope.body, e::Body::Result(_)));
    until(|| root.join("entered").exists());
    // The CLI has already reserved C1 and its provider is still active when
    // MCP submits the same key from another host instance.
    let mcp_result = mcp_call(&route, &root, submit(&mcp_reference, "shared"));
    assert!(
        mcp_result["isError"] == false
            || mcp_result["structuredContent"]["code"] == "inspection_unavailable",
        "{mcp_result}"
    );
    fixture.count_one();
    let cli_active = cli.call(inspect("shared")).row();
    let mcp_active = mcp_call(&route, &root, inspect("shared"));
    if mcp_active["isError"] == false {
        assert_eq!(cli_active.run_id, mcp_row(&mcp_active).run_id);
    } else {
        // The incumbent script may still hold the lifecycle lease. Contention
        // is a truthful temporary refusal, never a claim of absent effects.
        assert_eq!(
            mcp_active["structuredContent"]["code"],
            "inspection_unavailable"
        );
        assert_eq!(
            mcp_active["structuredContent"]["effect_observation"],
            "unavailable"
        );
    }
    fixture.release();
    let cli_terminal = terminal(&mut cli, "shared");
    let cli_terminal_row = cli_terminal.row();
    cli.finish();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mcp_terminal = loop {
        let result = mcp_call(&route, &root, inspect("shared"));
        if result["isError"] == true {
            assert_eq!(
                result["structuredContent"]["code"],
                "inspection_unavailable"
            );
            if Instant::now() >= deadline {
                panic!("MCP cannot inspect completed CLI run: {result}");
            }
        } else {
            let row = mcp_row(&result);
            if matches!(row.status, v::Status::ResultAvailable(_)) {
                break row;
            }
            assert!(
                Instant::now() < deadline,
                "MCP never reached terminal: {result}"
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_same_terminal_truth(&cli_terminal_row, &mcp_terminal);
    let run = cli_terminal_row.run_id.unwrap();
    let replay = mcp_call(&route, &root, submit(&mcp_reference, "shared"));
    assert_eq!(replay["isError"], false, "{replay}");
    assert_eq!(mcp_row(&replay).run_id.as_deref(), Some(run.as_str()));
    let changed_reference = mcp_approve(&route, &root, "detach");
    let changed = mcp_call(&route, &root, submit(&changed_reference, "shared"));
    assert_eq!(changed["isError"], true, "{changed}");
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    fixture.count_one();
    let index = fixture.index();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["shared"]["domain_run_id"], run);
}

#[test]
fn mcp_contender_cannot_replay_a_cli_key_with_different_approved_intent() {
    let fixture = Fixture::new(true);
    let root = fixture.root();
    let mut cli = fixture.client("issue");
    let cli_reference = approve(&mut cli);
    let route = route_for_cli_root(&root);
    let mcp_reference = mcp_approve(&route, &root, "detach");
    cli.call(submit(&cli_reference, "shared"));
    until(|| root.join("entered").exists());
    let first = mcp_call(&route, &root, submit(&mcp_reference, "shared"));
    assert!(
        first["isError"] == false || first["structuredContent"]["code"] == "inspection_unavailable",
        "{first}"
    );
    fixture.release();
    let original = terminal(&mut cli, "shared").row();
    cli.finish();
    assert_eq!(fixture.index()["namespaces"].as_object().unwrap().len(), 1);
    let refused = mcp_call(&route, &root, submit(&mcp_reference, "shared"));
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(refused["structuredContent"]["code"], "submission_refused");
    let observed = mcp_row(&mcp_call(&route, &root, inspect("shared")));
    assert_same_terminal_truth(&observed, &original);
    fixture.count_one();
}
