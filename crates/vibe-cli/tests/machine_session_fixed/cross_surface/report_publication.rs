//! The report path is occupied before the real owner attempts publication.

use super::*;

#[test]
fn cli_and_mcp_keep_run_uncertain_when_report_publication_fails_after_effect() {
    let fixture = Fixture::new(true);
    let mut cli = fixture.client("issue");
    let cli_reference = approve(&mut cli);
    for name in ["config.toml", "settings.toml", "registry.toml"] {
        fs::write(fixture.dir.path().join("home/.vibe").join(name), "").unwrap();
    }
    let mut mcp = ProcessMcp::reattach(&fixture);
    let mcp_reference = process_mcp_approve(&mut mcp, "none");
    let key = "blocked-report-publication";
    let submitted = cli.call(submit(&cli_reference, key));
    assert!(matches!(submitted.envelope.body, e::Body::Result(_)));
    until(|| fixture.root().join("entered").exists());
    fixture.count_one();
    let active = cli.call(inspect(key)).row();
    let run = active.run_id.expect("active C1 run identity");
    let report = fixture
        .root()
        .join(format!(".vibe/lifecycle-outcomes/{run}.report.json"));
    fs::create_dir_all(report.parent().unwrap()).unwrap();
    fs::create_dir(&report).unwrap();
    let contender = mcp.call(submit(&mcp_reference, key));
    assert!(
        contender["isError"] == false
            || contender["structuredContent"]["code"] == "inspection_unavailable",
        "{contender}"
    );
    fixture.release();
    let mut cli_error = None;
    until(|| {
        let publication = cli.call(inspect(key));
        if matches!(&publication.envelope.body, e::Body::Error(_)) {
            let error = publication.error();
            if error.known_run_id.as_deref() == Some(run.as_str()) {
                cli_error = Some(error);
                return true;
            }
        }
        false
    });
    let cli_error = cli_error.unwrap();
    assert_eq!(cli_error.code, Code::InspectionUnavailable);
    assert_eq!(cli_error.requested_phase.as_deref(), Some("validate"));
    assert_ne!(
        serde_json::to_value(cli_error.effect_observation).unwrap(),
        json!("none")
    );
    let cli_view: v::LifecycleSubmissionView =
        serde_json::from_str(cli_error.view_json.as_deref().unwrap()).unwrap();
    assert_eq!(cli_view.run_id.as_deref(), Some(run.as_str()));
    assert!(cli_view.partial);
    assert!(matches!(cli_view.status, v::Status::RecoveryRequired(_)));
    cli.finish();

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let reply = mcp.call(inspect(key));
        if reply["isError"] == true && reply["structuredContent"]["known_run_id"] == run {
            assert_eq!(reply["structuredContent"]["code"], "inspection_unavailable");
            assert_ne!(reply["structuredContent"]["effect_observation"], "none");
            let view: v::LifecycleSubmissionView =
                serde_json::from_str(reply["structuredContent"]["view_json"].as_str().unwrap())
                    .unwrap();
            assert_eq!(view.run_id.as_deref(), Some(run.as_str()));
            assert!(matches!(view.status, v::Status::RecoveryRequired(_)));
            break;
        }
        assert!(
            Instant::now() < deadline,
            "MCP publication recovery deadline: {reply}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let replay = mcp.call(submit(&mcp_reference, key));
    assert!(
        replay["isError"] == false || replay["structuredContent"]["known_run_id"] == run,
        "{replay}"
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let reply = mcp.call(inspect(key));
        if reply["isError"] == true && reply["structuredContent"]["known_run_id"] == run {
            assert_ne!(reply["structuredContent"]["effect_observation"], "none");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "MCP same-key recovery deadline: {reply}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    mcp.finish();
    fixture.count_one();
    assert!(
        report.is_dir(),
        "report was never published over the occupant"
    );
}
