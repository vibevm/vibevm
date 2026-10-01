use std::io::Read;

use super::super::{journal, prepare, tests};
use super::*;
use crate::publication_approval::ApprovalHome;
use crate::publication_effect_plan::PublicationEffectPlanExpectations;
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::{Project, StreamOutcome};

struct Fixture {
    tiny: TinyPayloadWorld,
    root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    home: ApprovalHome,
    expected: PublicationEffectPlanExpectations<'static>,
}

impl Fixture {
    fn new() -> Self {
        let tiny = TinyPayloadWorld::new();
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = tests::scope();
        tiny.seed_approval(&lease, &scope);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let expected = tests::expected();
        let output = Project::open(tiny.output.path()).unwrap();
        let first = prepare::prepare_local_full_plan(
            &lease,
            &namespace,
            &scope,
            "session-key",
            &tiny.subject,
            &tiny.plan,
            &expected,
            prepare::trusted(
                tiny.settings.path(),
                &lease,
                &scope,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals(),
            ),
        )
        .unwrap();
        assert!(matches!(first, super::Inspection::JournalReady { .. }));
        let home = tiny.approval_home();
        Self {
            tiny,
            root,
            lease,
            namespace,
            scope,
            home,
            expected,
        }
    }

    fn open(&self) -> SessionOpen<'_> {
        ReadyRun::open(
            &self.lease,
            &self.namespace,
            &self.scope,
            "session-key",
            &self.tiny.subject,
            &self.tiny.plan,
            &self.expected,
            &self.home,
        )
        .unwrap()
    }
}

#[test]
fn one_cold_scan_session_proves_only_selected_stream_each_time() {
    let fixture = Fixture::new();
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!("complete fixture must open")
    };
    let first_name = &fixture.tiny.subject.targets[0].bundle.name;
    let mut first = session.open_selected_stream(0, first_name).unwrap();
    let mut bytes = Vec::new();
    first.reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        format!("fixture payload: {first_name}\n").into_bytes()
    );
    assert!(matches!(
        first.completion.outcome(),
        StreamOutcome::Complete { .. }
    ));
    let future_name = &fixture.tiny.subject.targets[0].bootstrap.name;
    let future = fixture.root.path().join(format!(
        ".vibe/publication-operations/{}.payloads/{future_name}",
        session.run_id,
    ));
    std::fs::write(future, b"out-of-protocol future drift").unwrap();
    // A future-file change does not trigger an all-19 rescan for this call.
    assert!(session.open_selected_stream(0, first_name).is_ok());
    // The selected-file preflight refuses it at its own frontier.
    assert!(session.open_selected_stream(1, future_name).is_err());
    drop(session);
    let cold = ReadyRun::open(
        &fixture.lease,
        &fixture.namespace,
        &fixture.scope,
        "session-key",
        &fixture.tiny.subject,
        &fixture.tiny.plan,
        &fixture.expected,
        &fixture.home,
    )
    .unwrap();
    assert!(matches!(cold, SessionOpen::RecoveryRequired { .. }));
}

#[test]
fn pending_call_and_retired_c1_never_open_next_session_stream() {
    let fixture = Fixture::new();
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!("complete fixture must open")
    };
    let retired_run = session.run_id.clone();
    SubmissionReservations::retire_namespace(&fixture.lease, &fixture.namespace, &fixture.scope)
        .unwrap();
    assert!(
        session
            .open_selected_stream(0, &fixture.tiny.subject.targets[0].bundle.name)
            .is_err()
    );
    drop(session);
    assert!(
        matches!(fixture.open(), SessionOpen::Retired { run_id: Some(seen) }
        if seen == retired_run)
    );

    let second = Fixture::new();
    let run = match second.open() {
        SessionOpen::Ready(run) => run,
        _ => panic!("second fixture must open"),
    };
    journal::publish(
        &second.lease,
        &run.run_id,
        "api-000-pending",
        &serde_json::json!({"foreign": "pending without landed"}),
    )
    .unwrap();
    drop(run);
    assert!(matches!(second.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn marker_loss_refuses_next_stream_under_same_session() {
    let fixture = Fixture::new();
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!("complete fixture must open")
    };
    let redeemed = fixture
        .tiny
        .settings
        .path()
        .join("publication-approvals/redeemed");
    let marker = std::fs::read_dir(redeemed)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::remove_file(marker).unwrap();
    assert!(
        session
            .open_selected_stream(0, &fixture.tiny.subject.targets[0].bundle.name)
            .is_err()
    );
}

#[test]
fn new_lease_reconstructs_with_full_scan_and_refuses_drift() {
    let fixture = Fixture::new();
    let Fixture {
        tiny,
        root,
        lease,
        namespace,
        scope,
        home,
        expected,
    } = fixture;
    let SessionOpen::Ready(session) = ReadyRun::open(
        &lease,
        &namespace,
        &scope,
        "session-key",
        &tiny.subject,
        &tiny.plan,
        &expected,
        &home,
    )
    .unwrap() else {
        panic!("initial session must open")
    };
    let run_id = session.run_id.clone();
    drop(session);
    drop(lease);
    let target = root.path().join(format!(
        ".vibe/publication-operations/{run_id}.payloads/{}",
        tiny.subject.targets[0].bundle.name,
    ));
    std::fs::write(target, b"changed after owner EOF").unwrap();
    let next_lease = LifecycleLease::acquire(root.path()).unwrap();
    assert!(matches!(ReadyRun::open(
        &next_lease, &namespace, &scope, "session-key", &tiny.subject, &tiny.plan,
        &expected, &home,
    ).unwrap(), SessionOpen::RecoveryRequired { run_id: seen } if seen == run_id));
}

#[cfg(unix)]
#[test]
fn vault_directory_swap_under_session_denies_next_stream() {
    let fixture = Fixture::new();
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!("session must open")
    };
    let vault = fixture.root.path().join(format!(
        ".vibe/publication-operations/{}.payloads",
        session.run_id,
    ));
    let old = vault.with_extension("old-payloads");
    std::fs::rename(&vault, &old).unwrap();
    std::fs::create_dir(&vault).unwrap();
    assert!(
        session
            .open_selected_stream(0, &fixture.tiny.subject.targets[0].bundle.name)
            .is_err()
    );
    drop(session);
    std::fs::remove_dir(&vault).unwrap();
    std::fs::rename(old, vault).unwrap();
}
