use std::fs;

use serde_json::json;
use vibe_wire::generated::compiler_trace_index::e1::index::CompilerTraceIndex;
use vibe_wire::generated::shared::TraceReportStatus;

use super::support::{append, context, dispatch, project, report, run};

#[test]
fn algorithmic_build_returns_the_generated_completed_report() {
    let project = project("");
    let output = run(&context(project.path()), "build").unwrap();
    assert!(!output.is_error());
    let report = report(&output);
    assert!(report.ok);
    assert_eq!(report.requested, "build");
    assert_eq!(report.chain, ["validate", "install", "generate", "build"]);
    assert_eq!(
        report
            .steps
            .iter()
            .map(|step| (step.phase.as_str(), step.status.as_str()))
            .collect::<Vec<_>>(),
        [
            ("validate", "ok"),
            ("install", "fresh"),
            ("generate", "no-op"),
            ("build", "no-op"),
        ]
    );
    assert!(report.delegation.is_none());
    assert!(output.text().contains("phase `build` completed"));
}

#[test]
fn executed_handler_failure_keeps_its_structured_prefix_when_trace_is_disabled() {
    let project = project("");
    append(
        project.path(),
        r#"
[[extension]]
id = "first"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "FIRST-RAN" }

[[extension]]
id = "stop"
point = "phase:build"
handler = { kind = "builtin", name = "unknown" }

[[extension]]
id = "never"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "NEVER-RAN" }
"#,
    );
    let output = run(&context(project.path()), "build").unwrap();
    assert!(output.is_error());
    assert!(output.text().contains("unknown builtin `unknown`"));
    let report = report(&output);
    assert!(!report.ok);
    assert_eq!(report.steps.len(), 1, "failure owns one fail step");
    assert_eq!(report.steps[0].phase, "build");
    assert_eq!(report.steps[0].status, "fail");
    assert_eq!(
        report
            .contributions
            .iter()
            .map(|row| (row.key.as_str(), row.status.as_str()))
            .collect::<Vec<_>>(),
        [
            ("org.demo/demo#first", "ok"),
            ("org.demo/demo#stop", "fail")
        ]
    );
    assert!(report.trace.is_none());

    // The real dispatcher must retain the same root despite the CLI funnel's
    // trace-disabled `emit_report = false` policy.
    let response = dispatch(context(project.path()), json!({ "phase": "build" }));
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(response["result"]["structuredContent"]["ok"], false);
}

#[test]
fn manifest_trace_activation_uses_the_shared_owner_and_returns_its_member() {
    let project = project("\n[compile]\ntrace = true\n");
    let output = run(&context(project.path()), "build").unwrap();
    let report = report(&output);
    let trace = report.trace.expect("manifest activation opens one trace");
    assert_eq!(trace.status, TraceReportStatus::Ok);
    assert!(trace.finalised);
    let path = trace.run_path.expect("a real run has a path");
    assert!(path.replace('\\', "/").contains("/.vibe/trace/"));
    assert!(fs::metadata(project.path().join(".vibe/trace")).is_ok());
    let index: CompilerTraceIndex = serde_json::from_slice(
        &fs::read(std::path::PathBuf::from(&path).join("index.json")).unwrap(),
    )
    .unwrap();
    assert!(
        index.finished.unwrap().timestamp() > 1_700_000_000,
        "production trace completion uses the real current clock, not an epoch fixture"
    );
}
