use super::*;

#[test]
fn generated_submit_direct_owner_and_cold_connections_share_one_ready_run_and_copy() {
    let fixture = Fixture::new();
    let route = fixture.route();
    let mut client = fixture.client(route.clone());
    let absent = client.call("inspect", "copy");
    assert!(matches!(absent.reply().reply, rep::Reply::Absent(a) if a.key == "copy"));
    assert_eq!(
        absent.outcome.effect_state,
        f::OutcomeEffectState::NoEffects
    );
    assert_eq!(
        absent.outcome.completeness,
        f::OutcomeCompleteness::Complete
    );
    let first = client.call("submit", "copy");
    let run = first.completed();
    let metadata = fs::metadata(fixture.output()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let lease = LifecycleLease::acquire(fixture.dir.path()).unwrap();
    let direct = submit_fixed_static_file_package(
        &lease,
        &fixture.ns,
        &fixture.scope,
        "copy",
        "2030-01-01T00:00:00Z",
    )
    .unwrap();
    assert!(
        matches!(direct, StaticFileJournalInspection::Completed { run_id, .. } if run_id == run)
    );
    drop(lease);
    assert_eq!(client.call("inspect", "copy").envelope, first.envelope);
    assert_eq!(client.call("submit", "copy").envelope, first.envelope);
    drop(client);
    // Retained route, then fully reconstructed host with the SAME issued grant.
    let mut fresh = fixture.client(route);
    assert_eq!(fresh.call("inspect", "copy").envelope, first.envelope);
    let mut cold = fixture.client(fixture.route());
    assert_eq!(cold.call("submit", "copy").envelope, first.envelope);
    assert_eq!(
        fs::metadata(fixture.output()).unwrap().modified().unwrap(),
        metadata.modified().unwrap()
    );
    assert_eq!(fs::read(fixture.output()).unwrap(), b"copy once");
    fixture.one_run(&run);
}

#[test]
fn direct_owner_first_is_inspected_and_submitted_locally_without_recopy() {
    let fixture = Fixture::new();
    let lease = LifecycleLease::acquire(fixture.dir.path()).unwrap();
    let direct = submit_fixed_static_file_package(
        &lease,
        &fixture.ns,
        &fixture.scope,
        "copy",
        "2026-10-03T00:00:00Z",
    )
    .unwrap();
    let StaticFileJournalInspection::Completed {
        run_id,
        output_sha256,
        artifact_record,
    } = direct
    else {
        panic!("completed")
    };
    drop(lease);
    let original = fs::metadata(fixture.output()).unwrap().modified().unwrap();
    let mut client = fixture.client(fixture.route());
    let inspected = client.call("inspect", "copy");
    assert_eq!(inspected.completed(), run_id);
    assert_eq!(
        inspected.reply().reply,
        rep::Reply::Completed(Box::new(rep::ReplyCompleted {
            run_id: run_id.clone(),
            output_sha256,
            artifact_record
        }))
    );
    assert_eq!(client.call("submit", "copy").envelope, inspected.envelope);
    assert_eq!(
        fs::metadata(fixture.output()).unwrap().modified().unwrap(),
        original
    );
    fixture.one_run(&run_id);
}

#[test]
fn broken_pipe_partial_write_and_flush_loss_retain_exact_response_close_and_cold_inspect() {
    struct Lost {
        remaining: usize,
        bytes: Vec<u8>,
    }
    impl Write for Lost {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.remaining == 0 {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            let n = self.remaining.min(bytes.len());
            self.remaining -= n;
            self.bytes.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }
    for remaining in [0, 7, usize::MAX] {
        let fixture = Fixture::new();
        let route = fixture.route();
        let mut client = fixture.client(route.clone());
        let frame = client.frame(envelope("submit", "copy").to_string());
        let next = client.frame(envelope("submit", "second").to_string());
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &frame).unwrap();
        let first_length = bytes.len();
        write_frame(&mut bytes, &next).unwrap();
        let mut input = Cursor::new(bytes);
        let mut lost = Lost {
            remaining,
            bytes: Vec::new(),
        };
        let failure = client.runner.step(&mut input, &mut lost).unwrap_err();
        let PackageRunnerError::Output {
            error: FrameCodecError::Io(error),
            response,
        } = failure
        else {
            panic!("output error")
        };
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        assert_eq!(response.session_id, frame.session_id);
        assert_eq!(response.request_id, frame.request_id);
        let retained = Publication::new(response.clone());
        let run = retained.completed();
        let expected = FixedPackageRoute::exchange_local(
            Some(&route),
            envelope("inspect", "copy").to_string().as_bytes(),
            fixture.dir.path(),
        )
        .unwrap();
        assert_eq!(retained.envelope, expected);
        let mut full = Vec::new();
        write_frame(&mut full, &response).unwrap();
        assert_eq!(lost.bytes, full[..lost.bytes.len()]);
        let original = fs::metadata(fixture.output()).unwrap().modified().unwrap();
        assert_eq!(
            client.runner.step(&mut input, &mut Vec::new()).unwrap(),
            RunnerStep::Closed
        );
        assert_eq!(input.position(), first_length as u64, "next request unread");
        drop(client);
        let mut fresh = fixture.client(fixture.route());
        assert_eq!(fresh.call("inspect", "copy").envelope, retained.envelope);
        assert_eq!(fresh.call("submit", "copy").envelope, retained.envelope);
        assert_eq!(
            fs::metadata(fixture.output()).unwrap().modified().unwrap(),
            original
        );
        fixture.one_run(&run);
    }
}

#[test]
fn finite_run_replies_once_per_frame_then_eof_closes_and_prevents_next_dispatch() {
    let fixture = Fixture::new();
    let mut client = fixture.client(fixture.route());
    let mut bytes = Vec::new();
    write_frame(
        &mut bytes,
        &client.frame(envelope("submit", "copy").to_string()),
    )
    .unwrap();
    write_frame(
        &mut bytes,
        &client.frame(envelope("inspect", "copy").to_string()),
    )
    .unwrap();
    let mut output = Vec::new();
    assert_eq!(
        client
            .runner
            .run(&mut Cursor::new(bytes), &mut output)
            .unwrap(),
        2
    );
    let mut output = Cursor::new(output);
    let first = Publication::new(read_frame(&mut output).unwrap().unwrap());
    let second = Publication::new(read_frame(&mut output).unwrap().unwrap());
    assert_eq!(first.envelope, second.envelope);
    assert!(read_frame(&mut output).unwrap().is_none());
    let mut next = Vec::new();
    write_frame(&mut next, &offer()).unwrap();
    let mut next = Cursor::new(next);
    assert_eq!(
        client.runner.step(&mut next, &mut Vec::new()).unwrap(),
        RunnerStep::Closed
    );
    assert_eq!(next.position(), 0);
    fixture.one_run(&first.completed());
}
