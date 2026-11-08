use super::*;

#[test]
fn controlled_desktop_agent_mode_parks_without_paid_completion() {
    let scratch = scratch();
    let backend = RecordingBackend::answering(r#"{"outputs":[]}"#);
    let declaration = row(TWO_OUTPUTS, PROMPT);
    let mut run = LifecycleRun::begin(
        lease(&scratch.root),
        project_fixture(&scratch.root),
        world_fixture(&scratch.root),
        metadata(RUN_ID, false),
        vec!["validate".into(), "install".into(), "create".into()],
    )
    .unwrap();
    let process = crate::process::ControlledProcessRunner::new(
        vibe_core::cancellation::CancellationToken::new(),
    );
    let mut runtime = runtime(&backend);
    runtime.process = &process;
    let result = run
        .execute_one(
            &HandlerExecution::from_row(&declaration),
            "create",
            ExecutionReuse::FreshnessAware,
            &runtime,
        )
        .unwrap();
    assert_eq!(result.status, ExecutionRecordStatus::Delegated);
    assert!(result.delegation.is_some());
    assert!(backend.completed.lock().unwrap().is_empty());
}
