use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_lifecycle::{
    LifecycleLease, LifecycleLeaseError, LifecycleStateStore, SubmissionNamespace,
    SubmissionReservations, SubmissionScope,
};
use vibe_wire::generated::lifecycle_state::{ExecutionRecord, ExecutionRecordStatus};

use super::*;

const RUN_A: &str = "00112233445566778899aabbccddeeff";
const RUN_B: &str = "ffeeddccbbaa99887766554433221100";
const EFFECT_KEY: &str = "fixture-effect";

#[derive(Clone, Copy, PartialEq, Eq)]
enum FixtureFault {
    None,
    AfterCountBeforeCheckpoint,
    AfterRetainedCheckpoint,
    AfterUncertainCheckpoint,
}

fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "ctx".into(),
        target: "target".into(),
        principal: "principal".into(),
    }
}

struct FixtureJournal {
    proposed: String,
    calls: Arc<AtomicUsize>,
    ready_probe: Option<(SubmissionNamespace, SubmissionScope, String)>,
    hide_result: bool,
    fault: FixtureFault,
    fail_initialize_after_begin: bool,
}

impl FixtureJournal {
    fn new(proposed: &str, calls: Arc<AtomicUsize>) -> Self {
        Self {
            proposed: proposed.into(),
            calls,
            ready_probe: None,
            hide_result: false,
            fault: FixtureFault::None,
            fail_initialize_after_begin: false,
        }
    }

    fn requiring_ready(mut self, namespace: &SubmissionNamespace) -> Self {
        self.ready_probe = Some((namespace.clone(), scope(), "same-key".into()));
        self
    }

    fn without_retained_result(mut self) -> Self {
        self.hide_result = true;
        self
    }

    fn failing_invoke(mut self, fault: FixtureFault) -> Self {
        self.fault = fault;
        self
    }

    fn failing_initialize_after_begin(mut self) -> Self {
        self.fail_initialize_after_begin = true;
        self
    }
}

impl SubmissionJournal for FixtureJournal {
    type Outcome = String;

    fn proposed_run_id(&self) -> String {
        self.proposed.clone()
    }

    fn inspect(
        &mut self,
        lease: &LifecycleLease,
        run_id: &str,
    ) -> Result<JournalState<String>, String> {
        let state =
            LifecycleStateStore::peek_with_lease(lease).map_err(|error| error.to_string())?;
        let Some(state) = state else {
            return Ok(JournalState::Absent);
        };
        if state.run.run_id.as_deref() != Some(run_id) {
            return Ok(JournalState::Uncertain(
                "different durable run identity".into(),
            ));
        }
        if self.hide_result {
            return Ok(JournalState::Prepared);
        }
        Ok(match state.execution.get(EFFECT_KEY) {
            Some(record)
                if record.status == ExecutionRecordStatus::Ok
                    && record.fingerprint == format!("fixture:effect:{run_id}") =>
            {
                JournalState::Complete(run_id.to_owned())
            }
            Some(_) => JournalState::Uncertain("fixture effect has nonterminal record".into()),
            None => JournalState::Prepared,
        })
    }

    fn initialize(&mut self, lease: Arc<LifecycleLease>, run_id: &str) -> Result<(), String> {
        LifecycleStateStore::begin(
            lease,
            "build".into(),
            ["validate", "install", "generate", "build"]
                .map(str::to_owned)
                .to_vec(),
            "2026-09-30T00:00:00Z".into(),
            run_id.into(),
            ".".into(),
            false,
        )
        .map_err(|error| error.to_string())?;
        if self.fail_initialize_after_begin {
            Err("fixture lost initialization response".into())
        } else {
            Ok(())
        }
    }

    fn invoke(&mut self, lease: Arc<LifecycleLease>, run_id: &str) -> Result<String, String> {
        if let Some((namespace, scope, key)) = &self.ready_probe {
            let admission =
                SubmissionReservations::journal_admission(&lease, namespace, scope, key, run_id)
                    .map_err(|error| error.to_string())?;
            if admission != JournalAdmission::Ready {
                return Err("fixture effect was reached before journal-ready".into());
            }
        }
        let mut store = LifecycleStateStore::begin(
            lease,
            "build".into(),
            ["validate", "install", "generate", "build"]
                .map(str::to_owned)
                .to_vec(),
            "2026-09-30T00:00:00Z".into(),
            run_id.into(),
            ".".into(),
            false,
        )
        .map_err(|error| error.to_string())?;
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fault == FixtureFault::AfterCountBeforeCheckpoint {
            return Err("fixture effect landed before journal checkpoint".into());
        }
        store
            .checkpoint(
                EFFECT_KEY.into(),
                ExecutionRecord {
                    artifacts: Vec::new(),
                    duration_ms: 0,
                    fingerprint: if self.fault == FixtureFault::AfterUncertainCheckpoint {
                        "fixture:uncertain-effect".into()
                    } else {
                        format!("fixture:effect:{run_id}")
                    },
                    phase: "build".into(),
                    status: ExecutionRecordStatus::Ok,
                    tasks: Vec::new(),
                    scope: None,
                    input_measurement: None,
                },
            )
            .map_err(|error| error.to_string())?;
        if matches!(
            self.fault,
            FixtureFault::AfterRetainedCheckpoint | FixtureFault::AfterUncertainCheckpoint
        ) {
            return Err("fixture lost effect response".into());
        }
        Ok(run_id.to_owned())
    }
}

#[test]
fn effect_callback_observes_durable_journal_ready_before_invocation() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut journal = FixtureJournal::new(RUN_A, Arc::clone(&calls)).requiring_ready(&namespace);
    assert!(matches!(
        call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut journal,
            b"request",
        )
        .unwrap(),
        SubmissionOutcome::Invoked { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn invoke_error_after_retained_outcome_attaches_to_same_run_once() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut journal = FixtureJournal::new(RUN_A, Arc::clone(&calls))
        .failing_invoke(FixtureFault::AfterRetainedCheckpoint);
    assert_eq!(
        call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut journal,
            b"request",
        )
        .unwrap(),
        SubmissionOutcome::Invoked {
            run_id: RUN_A.into(),
            outcome: RUN_A.into(),
        }
    );
    let mut retry = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert_eq!(
        call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut retry,
            b"request",
        )
        .unwrap(),
        SubmissionOutcome::Replayed {
            run_id: RUN_A.into(),
            outcome: RUN_A.into(),
        }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn invoke_error_after_counted_effect_keeps_recovery_or_uncertain_run() {
    for fault in [
        FixtureFault::AfterCountBeforeCheckpoint,
        FixtureFault::AfterUncertainCheckpoint,
    ] {
        let root = tempfile::tempdir().unwrap();
        let namespace = issue(root.path());
        let calls = Arc::new(AtomicUsize::new(0));
        let mut journal = FixtureJournal::new(RUN_A, Arc::clone(&calls)).failing_invoke(fault);
        let result = call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut journal,
            b"request",
        );
        match fault {
            FixtureFault::AfterCountBeforeCheckpoint => assert_eq!(
                result.unwrap(),
                SubmissionOutcome::RecoveryRequired {
                    run_id: RUN_A.into()
                }
            ),
            FixtureFault::AfterUncertainCheckpoint => assert!(matches!(
                result,
                Err(SubmissionFailure::UncertainJournal { run_id, .. }) if run_id == RUN_A
            )),
            _ => unreachable!(),
        }
        let mut retry = FixtureJournal::new(RUN_B, Arc::clone(&calls));
        let retried = call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut retry,
            b"request",
        );
        match fault {
            FixtureFault::AfterCountBeforeCheckpoint => assert_eq!(
                retried.unwrap(),
                SubmissionOutcome::RecoveryRequired {
                    run_id: RUN_A.into()
                }
            ),
            FixtureFault::AfterUncertainCheckpoint => assert!(matches!(
                retried,
                Err(SubmissionFailure::UncertainJournal { run_id, .. }) if run_id == RUN_A
            )),
            _ => unreachable!(),
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn initialize_error_after_durable_prepare_retains_reserved_run_without_effect() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut journal =
        FixtureJournal::new(RUN_A, Arc::clone(&calls)).failing_initialize_after_begin();
    assert_eq!(
        call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut journal,
            b"request",
        )
        .unwrap(),
        SubmissionOutcome::RecoveryRequired {
            run_id: RUN_A.into()
        }
    );
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope(), "same-key", RUN_A)
            .unwrap(),
        JournalAdmission::Pending
    );
    let mut retry = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert_eq!(
        call(Arc::clone(&lease), &namespace, &mut retry, b"request").unwrap(),
        SubmissionOutcome::RecoveryRequired {
            run_id: RUN_A.into()
        }
    );
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope(), "same-key", RUN_A)
            .unwrap(),
        JournalAdmission::Ready
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn unavailable_retained_result_requires_recovery_without_second_effect() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut first = FixtureJournal::new(RUN_A, Arc::clone(&calls));
    call(
        Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
        &namespace,
        &mut first,
        b"request",
    )
    .unwrap();
    let lease = LifecycleLease::acquire(root.path()).unwrap();
    SubmissionReservations::retire_namespace(&lease, &namespace, &scope()).unwrap();
    drop(lease);
    let mut retry = FixtureJournal::new(RUN_B, Arc::clone(&calls)).without_retained_result();
    assert_eq!(
        call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut retry,
            b"request",
        )
        .unwrap(),
        SubmissionOutcome::RecoveryRequired {
            run_id: RUN_A.into(),
        }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

fn issue(root: &std::path::Path) -> SubmissionNamespace {
    let lease = LifecycleLease::acquire(root).unwrap();
    SubmissionReservations::issue_namespace(&lease, &scope(), 60_000).unwrap()
}

fn call(
    lease: Arc<LifecycleLease>,
    namespace: &SubmissionNamespace,
    journal: &mut FixtureJournal,
    request: &[u8],
) -> Result<SubmissionOutcome<String>, SubmissionFailure> {
    submit(
        lease,
        namespace,
        &scope(),
        "same-key",
        request,
        b"canonical-plan",
        journal,
    )
}

#[test]
fn eight_same_key_submitters_invoke_one_fixture_effect() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let invoked = AtomicUsize::new(0);
    std::thread::scope(|threads| {
        for _ in 0..8 {
            let root = root.path().to_path_buf();
            let namespace = namespace.clone();
            let calls = Arc::clone(&calls);
            let invoked = &invoked;
            threads.spawn(move || {
                let lease = loop {
                    match LifecycleLease::acquire(&root) {
                        Ok(lease) => break Arc::new(lease),
                        Err(LifecycleLeaseError::Busy { .. }) => std::thread::yield_now(),
                        Err(error) => panic!("lease: {error}"),
                    }
                };
                let mut journal = FixtureJournal::new(RUN_A, calls);
                match call(lease, &namespace, &mut journal, b"canonical-request").unwrap() {
                    SubmissionOutcome::Invoked { .. } => {
                        invoked.fetch_add(1, Ordering::SeqCst);
                    }
                    SubmissionOutcome::Replayed { .. } => {}
                    other => panic!("unexpected submission outcome {other:?}"),
                }
            });
        }
    });
    assert_eq!(invoked.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn different_payload_refuses_and_lost_response_replays_journal() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut journal = FixtureJournal::new(RUN_A, Arc::clone(&calls));
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    assert!(matches!(
        call(lease, &namespace, &mut journal, b"request").unwrap(),
        SubmissionOutcome::Invoked { .. }
    ));
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    let mut retry_owner = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert_eq!(
        call(Arc::clone(&lease), &namespace, &mut retry_owner, b"request").unwrap(),
        SubmissionOutcome::Replayed {
            run_id: RUN_A.into(),
            outcome: RUN_A.into()
        }
    );
    assert!(matches!(
        call(lease, &namespace, &mut retry_owner, b"different"),
        Err(SubmissionFailure::Reservation(
            vibe_lifecycle::SubmissionError::KeyConflict
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn reservation_without_journal_recovers_exact_identity_but_does_not_invoke() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    assert_eq!(
        SubmissionReservations::reserve(
            &lease,
            &namespace,
            &scope(),
            "same-key",
            b"request",
            b"canonical-plan",
            RUN_A
        )
        .unwrap(),
        ReservationDecision::Created {
            domain_run_id: RUN_A.into()
        }
    );
    drop(lease);
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    let mut retry_owner = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert_eq!(
        call(Arc::clone(&lease), &namespace, &mut retry_owner, b"request").unwrap(),
        SubmissionOutcome::RecoveryRequired {
            run_id: RUN_A.into()
        }
    );
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope(), "same-key", RUN_A)
            .unwrap(),
        JournalAdmission::Ready
    );
    assert!(matches!(
        retry_owner.inspect(&lease, RUN_A).unwrap(),
        JournalState::Prepared
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn prepared_journal_crash_window_reconciles_without_invocation() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    SubmissionReservations::reserve(
        &lease,
        &namespace,
        &scope(),
        "same-key",
        b"request",
        b"canonical-plan",
        RUN_A,
    )
    .unwrap();
    let mut owner = FixtureJournal::new(RUN_A, Arc::clone(&calls));
    owner.initialize(Arc::clone(&lease), RUN_A).unwrap();
    drop(lease);
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    let mut retry = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert_eq!(
        call(Arc::clone(&lease), &namespace, &mut retry, b"request").unwrap(),
        SubmissionOutcome::RecoveryRequired {
            run_id: RUN_A.into()
        }
    );
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &scope(), "same-key", RUN_A)
            .unwrap(),
        JournalAdmission::Ready
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn created_reservation_refuses_preexisting_journal_identity() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    let mut owner = FixtureJournal::new(RUN_A, Arc::clone(&calls));
    owner.initialize(Arc::clone(&lease), RUN_A).unwrap();
    assert!(matches!(
        call(lease, &namespace, &mut owner, b"request"),
        Err(SubmissionFailure::ConflictingJournal { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn missing_or_conflicting_journal_after_ready_refuses_without_rerun() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut journal = FixtureJournal::new(RUN_A, Arc::clone(&calls));
    call(
        Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
        &namespace,
        &mut journal,
        b"request",
    )
    .unwrap();
    std::fs::remove_file(root.path().join(LifecycleStateStore::FILE)).unwrap();
    let mut retry_owner = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert!(matches!(
        call(
            Arc::new(LifecycleLease::acquire(root.path()).unwrap()),
            &namespace,
            &mut retry_owner,
            b"request"
        ),
        Err(SubmissionFailure::MissingJournal { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    retry_owner.initialize(Arc::clone(&lease), RUN_B).unwrap();
    assert!(matches!(
        call(Arc::clone(&lease), &namespace, &mut retry_owner, b"request"),
        Err(SubmissionFailure::UncertainJournal { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn retired_namespace_replays_existing_outcome_but_refuses_new_key() {
    let root = tempfile::tempdir().unwrap();
    let namespace = issue(root.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let mut owner = FixtureJournal::new(RUN_A, Arc::clone(&calls));
    let lease = Arc::new(LifecycleLease::acquire(root.path()).unwrap());
    call(Arc::clone(&lease), &namespace, &mut owner, b"request").unwrap();
    SubmissionReservations::retire_namespace(&lease, &namespace, &scope()).unwrap();
    let mut retry = FixtureJournal::new(RUN_B, Arc::clone(&calls));
    assert_eq!(
        call(Arc::clone(&lease), &namespace, &mut retry, b"request").unwrap(),
        SubmissionOutcome::Replayed {
            run_id: RUN_A.into(),
            outcome: RUN_A.into()
        }
    );
    assert_eq!(
        submit(
            lease,
            &namespace,
            &scope(),
            "new-key",
            b"request",
            b"canonical-plan",
            &mut retry
        )
        .unwrap(),
        SubmissionOutcome::Retired { run_id: None }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
