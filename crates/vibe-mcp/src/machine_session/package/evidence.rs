use super::*;

#[test]
fn damaged_owner_results_and_a2_only_retain_exact_partial_truth_and_never_recopy() {
    for damage in [
        "result-corrupt",
        "result-missing",
        "capsule",
        "output",
        "a2",
        "intent",
    ] {
        let fixture = Fixture::new();
        let mut client = fixture.client(fixture.route());
        let run = client.call("submit", "copy").completed();
        let home = fixture.dir.path().join(".vibe/package-operations");
        match damage {
            "result-corrupt" => fs::write(fixture.result(&run), b"{}").unwrap(),
            "result-missing" => fs::remove_file(fixture.result(&run)).unwrap(),
            "capsule" => {
                fs::write(home.join(format!("{run}.static-file-input.json")), b"{}").unwrap()
            }
            "output" => fs::write(fixture.output(), b"drift").unwrap(),
            "a2" => fs::write(
                fixture
                    .dir
                    .path()
                    .join(".vibe/state/artifacts/launcher.bin.json"),
                b"{}",
            )
            .unwrap(),
            "intent" => {
                fs::remove_file(home.join(format!("{run}.static-file-intent.json"))).unwrap()
            }
            _ => unreachable!(),
        }
        let original = fs::metadata(fixture.output()).unwrap().modified().unwrap();
        drop(client);
        let host = fixture.host();
        let mut cold = fixture.client(Arc::new(FixedPackageRoute::new(host.clone())));
        for action in ["inspect", "submit"] {
            let expected = if action == "inspect" {
                host.inspect("copy").unwrap().unwrap()
            } else {
                host.submit("copy").unwrap()
            };
            let published = cold.call(action, "copy");
            assert_eq!(
                published.outcome.effect_state,
                f::OutcomeEffectState::Uncertain
            );
            assert_eq!(
                published.outcome.provenance_id.as_deref(),
                Some(run.as_str())
            );
            match expected {
                StaticFileJournalInspection::Uncertain { run_id, partial } => {
                    assert_eq!(
                        published.outcome.completeness,
                        f::OutcomeCompleteness::Partial
                    );
                    let rep::Reply::Uncertain(reply) = published.reply().reply else {
                        panic!("uncertain")
                    };
                    assert_eq!(reply.run_id, run_id);
                    // Compare the complete independently inspected native partial observation.
                    assert_partial(&reply.partial, &partial);
                }
                StaticFileJournalInspection::RecoveryRequired { run_id } => {
                    assert_eq!(
                        published.outcome.completeness,
                        f::OutcomeCompleteness::Unavailable
                    );
                    assert!(
                        matches!(published.reply().reply, rep::Reply::RecoveryRequired(r) if r.run_id == run_id)
                    );
                }
                other => panic!("unexpected damaged owner observation: {other:?}"),
            }
        }
        assert_eq!(
            fs::metadata(fixture.output()).unwrap().modified().unwrap(),
            original
        );
        assert_eq!(
            fs::read(fixture.output()).unwrap(),
            if damage == "output" {
                b"drift".as_slice()
            } else {
                b"copy once".as_slice()
            }
        );
    }
}

#[test]
fn source_drift_refusal_keeps_completed_evidence_and_owner_errors_never_invent_no_effects() {
    let fixture = Fixture::new();
    let mut client = fixture.client(fixture.route());
    let run = client.call("submit", "copy").completed();
    let original = fs::metadata(fixture.output()).unwrap().modified().unwrap();
    fs::write(fixture.dir.path().join("source.bin"), b"changed").unwrap();
    let refusal = client.call("submit", "copy");
    let error = refusal.error();
    assert_eq!(
        error.code,
        err::FixedPackageActionErrorCode::SubmissionRefused
    );
    assert_eq!(
        error.effect_observation,
        err::FixedPackageActionErrorEffectObservation::Landed
    );
    assert_eq!(error.known_run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(error.view, Some(err::Reply::Completed(_))));
    assert_eq!(
        refusal.outcome.effect_state,
        f::OutcomeEffectState::Completed
    );
    assert_eq!(
        refusal.outcome.completeness,
        f::OutcomeCompleteness::Complete
    );
    assert_eq!(
        fs::metadata(fixture.output()).unwrap().modified().unwrap(),
        original
    );
    fs::write(
        fixture.dir.path().join(".vibe/machine-submissions.json"),
        b"{}",
    )
    .unwrap();
    for action in ["submit", "inspect"] {
        let publication = client.call(action, "copy");
        assert_eq!(
            publication.outcome.effect_state,
            f::OutcomeEffectState::Uncertain
        );
        assert_eq!(
            publication.outcome.completeness,
            f::OutcomeCompleteness::Unavailable
        );
        assert!(publication.error().known_run_id.is_none());
        assert!(publication.error().view.is_none());
    }
    assert_eq!(
        fs::metadata(fixture.output()).unwrap().modified().unwrap(),
        original
    );
}

#[test]
fn failed_owner_projection_returns_original_typed_evidence_and_closes_connection() {
    let fixture = Fixture::new();
    let host = fixture.host();
    let route = Arc::new(FixedPackageRoute::new(host.clone()));
    let mut client = fixture.client(route.clone());
    let run = client.call("submit", "copy").completed();
    // Damage retained correlation with a C1-valid token that cannot be a package
    // run ID. The real owner retains it in uncertainty; projection must refuse.
    let original = fs::metadata(fixture.output()).unwrap().modified().unwrap();
    let index_path = fixture.dir.path().join(".vibe/machine-submissions.json");
    let mut index: Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    let damaged_run = "damaged-package-run";
    assert_eq!(
        index["namespaces"][fixture.ns.id()]["keys"]["copy"]["domain_run_id"],
        run
    );
    index["namespaces"][fixture.ns.id()]["keys"]["copy"]["domain_run_id"] = json!(damaged_run);
    fs::write(index_path, serde_json::to_vec(&index).unwrap()).unwrap();
    let expected_owner = host.inspect("copy").unwrap().unwrap();
    let request = envelope("inspect", "copy").to_string();
    let expected_error =
        FixedPackageRoute::exchange_local(Some(&route), request.as_bytes(), fixture.dir.path())
            .unwrap_err();
    let frame = client.frame(request);
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &frame).unwrap();
    let mut output = Vec::new();
    let error = client
        .runner
        .step(&mut Cursor::new(bytes), &mut output)
        .unwrap_err();
    let PackageRunnerError::Publication(error) = error else {
        panic!("owning publication error")
    };
    assert_eq!(error.error, expected_error.error);
    assert_eq!(
        error.effects_may_have_begun,
        expected_error.effects_may_have_begun
    );
    assert_eq!(error.owner_observation, Some(expected_owner));
    assert_eq!(error.owner_observation, expected_error.owner_observation);
    assert!(error.effects_may_have_begun);
    assert_eq!(error.error.known_run_id.as_deref(), Some(damaged_run));
    assert_eq!(
        error.error.code,
        err::FixedPackageActionErrorCode::PublicationUnavailable
    );
    assert!(output.is_empty());
    let mut next = Vec::new();
    write_frame(&mut next, &offer()).unwrap();
    let mut next = Cursor::new(next);
    assert_eq!(
        client.runner.step(&mut next, &mut output).unwrap(),
        RunnerStep::Closed
    );
    assert_eq!(next.position(), 0);
    drop(client);
    let mut cold = fixture.client(fixture.route());
    let frame = cold.frame(envelope("inspect", "copy").to_string());
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &frame).unwrap();
    let PackageRunnerError::Publication(cold_error) = cold
        .runner
        .step(&mut Cursor::new(bytes), &mut output)
        .unwrap_err()
    else {
        panic!("cold evidence")
    };
    assert_eq!(cold_error.owner_observation, error.owner_observation);
    assert_eq!(
        fs::metadata(fixture.output()).unwrap().modified().unwrap(),
        original
    );
    assert_eq!(fs::read(fixture.output()).unwrap(), b"copy once");
}

fn assert_partial(wire: &rep::Partial, owner: &vibe_lifecycle::StaticFilePartialEvidence) {
    use vibe_lifecycle::{StaticFileObservedFile as F, StaticFileObservedPresence as P};
    match (&wire.output_directory, &owner.output_directory) {
        (rep::ObservedPresence::Absent(_), P::Absent)
        | (rep::ObservedPresence::Present(_), P::Present) => {}
        (rep::ObservedPresence::Unavailable(w), P::Unavailable(reason)) => {
            assert_eq!(&w.reason, reason)
        }
        other => panic!("directory evidence differs: {other:?}"),
    }
    for (wire, owner) in [
        (&wire.output_file, &owner.output_file),
        (&wire.artifact_record, &owner.artifact_record),
    ] {
        match (wire, owner) {
            (rep::ObservedFile::Absent(_), F::Absent) => {}
            (rep::ObservedFile::Present(w), F::Present { sha256, bytes }) => {
                assert_eq!(&w.sha256, sha256);
                assert_eq!(&w.bytes, bytes);
            }
            (rep::ObservedFile::Unavailable(w), F::Unavailable(reason)) => {
                assert_eq!(&w.reason, reason)
            }
            other => panic!("file evidence differs: {other:?}"),
        }
    }
}
