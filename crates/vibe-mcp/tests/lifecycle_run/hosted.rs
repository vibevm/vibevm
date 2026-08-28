use std::fs;

use serde_json::json;
use vibe_mcp::ToolError;
use vibe_mcp::tools::{LifecycleRunMcpTool, LifecycleTasksMcpTool, McpTool};
use vibe_wire::generated::lifecycle_state::LifecycleState;
use vibe_wire::generated::lifecycle_tasks::{LifecycleTasks, LifecycleTasksStatus};

use super::support::{
    ONE_AGENT_ROW, context, hosted_project, report, run, state_bytes, task_bytes,
};

#[test]
fn hosted_park_reparks_same_identity_then_satisfied_resume_completes() {
    let project = hosted_project(ONE_AGENT_ROW);
    let ctx = context(project.path());

    let first = run(&ctx, "create").unwrap();
    assert!(!first.is_error(), "a park is a successful tool result");
    let first_report = report(&first);
    let first_handoff = first_report.delegation.as_ref().expect("one handoff");
    assert_eq!(first_report.steps.last().unwrap().status, "delegated");
    assert_eq!(first_handoff.tasks.len(), 1);
    assert!(
        !first_report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("#after-agent"))
    );
    assert!(first.text().contains("`lifecycle_tasks`"));
    let first_state: LifecycleState = toml::from_slice(&state_bytes(project.path())).unwrap();
    let first_started = first_state.run.started.clone();
    let first_task = task_bytes(project.path(), &first_handoff.tasks[0]);

    let tasks = LifecycleTasksMcpTool.run(&json!({}), &ctx).unwrap();
    let tasks: LifecycleTasks = serde_json::from_value(tasks.structured().clone()).unwrap();
    assert_eq!(tasks.status, LifecycleTasksStatus::Parked);
    assert_eq!(tasks.tasks.len(), 1);

    let second = run(&ctx, "create").unwrap();
    let second_report = report(&second);
    let second_handoff = second_report.delegation.as_ref().unwrap();
    let second_state: LifecycleState = toml::from_slice(&state_bytes(project.path())).unwrap();
    assert_eq!(second_handoff.run_id, first_handoff.run_id);
    assert_eq!(second_handoff.tasks, first_handoff.tasks);
    assert_eq!(second_state.run.started, first_started);
    assert_eq!(
        task_bytes(project.path(), &second_handoff.tasks[0]),
        first_task,
        "an unsatisfied repeat reparks idempotently"
    );

    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "hosted body\n").unwrap();
    let completed = run(&ctx, "create").unwrap();
    assert!(!completed.is_error());
    let completed_report = report(&completed);
    assert!(completed_report.delegation.is_none());
    assert!(
        completed_report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("#after-agent") && row.status == "ok")
    );
    let tasks = LifecycleTasksMcpTool.run(&json!({}), &ctx).unwrap();
    let tasks: LifecycleTasks = serde_json::from_value(tasks.structured().clone()).unwrap();
    assert_eq!(tasks.status, LifecycleTasksStatus::Idle);
}

#[test]
fn a_different_phase_starts_fresh_and_never_adopts_the_live_park() {
    let project = hosted_project(ONE_AGENT_ROW);
    let ctx = context(project.path());
    let parked = run(&ctx, "create").unwrap();
    let handoff = report(&parked).delegation.unwrap();
    let before_task = task_bytes(project.path(), &handoff.tasks[0]);

    let next = run(&ctx, "verify").unwrap();
    let next_report = report(&next);
    let next_handoff = next_report.delegation.unwrap();
    assert_ne!(next_handoff.run_id, handoff.run_id);
    assert_eq!(next_handoff.resume, "vibe verify");
    let state: LifecycleState =
        toml::from_str(&String::from_utf8(state_bytes(project.path())).unwrap()).unwrap();
    assert_eq!(
        state.run.run_id.as_deref(),
        Some(next_handoff.run_id.as_str())
    );
    // The displaced task may remain as an orphan, but it is never adopted or
    // rewritten by the fresh run and the state no longer names it.
    assert_eq!(task_bytes(project.path(), &handoff.tasks[0]), before_task);
}

fn member_manifest(name: &str, extension: &str) -> String {
    format!("[package]\ngroup='org.demo'\nname='{name}'\nkind='flow'\nversion='0.1.0'\n{extension}")
}

#[test]
fn a_sibling_context_cannot_adopt_another_members_park() {
    let workspace = tempfile::tempdir().unwrap();
    fs::write(
        workspace.path().join("vibe.toml"),
        "[project]\nname='root'\nversion='0.1.0'\n[workspace]\nmembers=['a','b']\n",
    )
    .unwrap();
    for name in ["a", "b"] {
        fs::create_dir_all(workspace.path().join(name)).unwrap();
        let extension = if name == "a" { ONE_AGENT_ROW } else { "" };
        fs::write(
            workspace.path().join(name).join("vibe.toml"),
            member_manifest(name, extension),
        )
        .unwrap();
    }
    let specs = workspace.path().join("a/vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    // The selected member is org.demo/a, so its prompt address must name a.
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite it.\n",
    )
    .unwrap();
    let manifest = workspace.path().join("a/vibe.toml");
    let body = fs::read_to_string(&manifest)
        .unwrap()
        .replace("spec://org.demo/demo/", "spec://org.demo/a/");
    fs::write(manifest, body).unwrap();

    let a = context(&workspace.path().join("a"));
    let b = context(&workspace.path().join("b"));
    let parked = run(&a, "create").unwrap();
    let handoff = report(&parked).delegation.unwrap();
    let before = state_bytes(workspace.path());
    let task = task_bytes(&workspace.path().join("a"), &handoff.tasks[0]);
    let error = LifecycleRunMcpTool
        .run(&json!({ "phase": "create" }), &b)
        .unwrap_err();
    assert!(matches!(error, ToolError::PreExecution(_)));
    assert_eq!(state_bytes(workspace.path()), before);
    assert_eq!(
        task_bytes(&workspace.path().join("a"), &handoff.tasks[0]),
        task
    );
}
