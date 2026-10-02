use super::*;
use std::io::{self, Write};
struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "lost response"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn lost_submit_output_retains_typed_response_fresh_inspect_and_identical_retry() {
    let f = Fixture::new();
    let host = f.host(true);
    let mut c = f.client(host.clone());
    let reference = approve(&mut c, "none");
    let frame = c.frame(envelope(submit(&reference, "once")).to_string());
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &frame).unwrap();
    let error = c
        .runner
        .step(&mut Cursor::new(bytes), &mut Broken)
        .unwrap_err();
    let FixedRunnerError::Output { response, .. } = error else {
        panic!("typed output error");
    };
    let retained = Publication::new(response);
    assert!(!retained.refused);
    assert!(matches!(retained.reply().reply, Reply::Submitted(_)));
    assert_eq!(
        c.runner
            .step(&mut Cursor::new(Vec::<u8>::new()), &mut Vec::new())
            .unwrap(),
        RunnerStep::Closed
    );
    until(|| f.dir.path().join("entered").exists());
    f.count();
    let retained_route = c.runner.route.clone();
    let context = ServerContext::with_store_root(f.dir.path(), f.dir.path().join("unused"))
        .with_fixed_lifecycle_route(retained_route)
        .unwrap();
    let mut fresh = Client::new(FixedMachineSessionRunner::new(context).unwrap());
    assert_ne!(fresh.context, c.context);
    assert_ne!(fresh.session, c.session);
    let run = fresh.call(inspect("once")).row().run_id.unwrap();
    // The same opaque trusted route retains accepted references across EOF.
    assert_eq!(
        fresh
            .call(submit(&reference, "once"))
            .row()
            .run_id
            .as_deref(),
        Some(run.as_str())
    );
    f.count();
    f.release();
    until(|| {
        matches!(
            fresh.call(inspect("once")).row().status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = fresh.call(inspect("once"));
    assert_eq!(
        terminal.outcome.provenance_id.as_deref(),
        Some(run.as_str())
    );
    assert_eq!(
        terminal.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    f.count();
}
#[test]
fn damaged_report_or_marker_keeps_run_scope_phase_uncertainty_without_redispatch() {
    for suffix in ["report.json", "marker.json"] {
        let f = Fixture::new();
        let host = f.host(true);
        let mut c = f.client(host.clone());
        let reference = approve(&mut c, "none");
        assert!(!c.call(submit(&reference, "once")).refused);
        until(|| f.dir.path().join("entered").exists());
        f.release();
        until(|| {
            matches!(
                c.call(inspect("once")).row().status,
                v::Status::ResultAvailable(_)
            )
        });
        let run = c.call(inspect("once")).row().run_id.unwrap();
        fs::write(
            f.dir
                .path()
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{run}.{suffix}")),
            "corrupt",
        )
        .unwrap();
        let damaged = c.call(inspect("once"));
        let error = damaged.error();
        assert_eq!(error.code, Code::InspectionUnavailable);
        assert_eq!(error.known_run_id.as_deref(), Some(run.as_str()));
        assert_eq!(error.requested_phase.as_deref(), Some("validate"));
        let scope = error.requested_scope.unwrap();
        assert_eq!(scope.context, f.scope.context);
        assert_eq!(scope.target, f.scope.target);
        assert_eq!(scope.principal, f.scope.principal);
        assert!(error.view_json.is_some());
        assert_eq!(
            damaged.outcome.effect_state,
            f::OutcomeEffectState::Uncertain
        );
        assert_eq!(
            damaged.outcome.completeness,
            f::OutcomeCompleteness::Unavailable
        );
        assert_eq!(damaged.outcome.provenance_id.as_deref(), Some(run.as_str()));
        let mut fresh = f.client(host);
        let fresh_error = fresh.call(inspect("once"));
        assert_eq!(
            fresh_error.error().known_run_id.as_deref(),
            Some(run.as_str())
        );
        let _ = c.call(submit(&reference, "once"));
        f.count();
    }
}
#[test]
fn replaced_physical_root_and_foreign_embedding_root_refuse_before_provider() {
    let f = Fixture::new();
    let host = f.host(true);
    let mut c = f.client(host.clone());
    let reference = approve(&mut c, "none");
    let other = tempfile::tempdir().unwrap();
    assert!(
        ServerContext::with_store_root(other.path(), other.path().join("unused"))
            .with_fixed_lifecycle_host(host.clone())
            .is_err()
    );
    let mut replaced = f.context(host);
    replaced.project_root = other.path().into();
    assert!(matches!(
        FixedMachineSessionRunner::new(replaced),
        Err(FixedSessionBuildError::Root(_))
    ));
    let original = f.dir.path().join("original");
    // Keep the displaced physical root inside the fixture's owned temp parent.
    let parent = tempfile::tempdir().unwrap();
    let original = parent.path().join(original.file_name().unwrap());
    fs::rename(f.dir.path(), &original).unwrap();
    fs::create_dir(f.dir.path()).unwrap();
    let response = c.call(submit(&reference, "once"));
    assert_eq!(response.error().code, Code::ContextRootMismatch);
    assert_eq!(
        response.outcome.effect_state,
        f::OutcomeEffectState::NoEffects
    );
    assert!(!original.join("counter").exists());
    assert!(!f.dir.path().join("counter").exists());
    fs::remove_dir(f.dir.path()).unwrap();
    fs::rename(original, f.dir.path()).unwrap();
    f.no_provider();
}
#[test]
fn oversized_truncated_unknown_field_and_epoch_frames_stop_before_dispatch() {
    let f = Fixture::new();
    let host = f.host(true);
    let raws=vec![
        ((super::super::super::framing::MAX_FRAME_BYTES+1) as u32).to_be_bytes().to_vec(),
        vec![0,0],vec![0,0,0,12,b'{'],
        framed_raw(json!({"schema":2,"session_id":null,"request_id":null,"body":{"kind":"request","context_id":"x","target_id":null,"envelope_json":"{}"}}).to_string()),
        framed_raw(json!({"schema":1,"session_id":null,"request_id":null,"root":"/tmp","body":{"kind":"request","context_id":"x","target_id":null,"envelope_json":"{}"}}).to_string()),
    ];
    for raw in raws {
        let mut c = f.client(host.clone());
        assert!(matches!(
            c.runner.step(&mut Cursor::new(raw), &mut Vec::new()),
            Err(FixedRunnerError::Frame(_))
        ));
        assert_eq!(c.runner.session.phase(), SessionPhase::Closed);
    }
    f.no_provider();
}
fn framed_raw(raw: String) -> Vec<u8> {
    let mut bytes = (raw.len() as u32).to_be_bytes().to_vec();
    bytes.extend(raw.as_bytes());
    bytes
}
