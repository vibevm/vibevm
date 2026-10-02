//! Finite Linux SystemProcessRunner cases; runtime verdict belongs to parent.
use super::super::submission::profile::AdmittedProfile;
use super::tests::{Fixture, approved, facts, source};
use super::*;
use crate::machine_submission::{FixedOperationStatus, HostOwnedFixedOperation, Readiness};
use std::time::{Duration, Instant};
use vibe_wire::generated::shared::Timestamp;

const WAIT: Duration = Duration::from_secs(15);
fn now() -> Timestamp {
    "2026-10-02T00:00:00Z".parse().unwrap()
}
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !predicate() {
        assert!(Instant::now() < deadline, "finite real-process deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn terminal(job: &ReadonlyFixedJob) -> FixedOperationStatus {
    let mut result = None;
    until(|| {
        let state = job.inspect().unwrap();
        if matches!(
            state,
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. }
        ) {
            false
        } else {
            result = Some(state);
            true
        }
    });
    result.unwrap()
}

#[test]
fn real_external_update_inside_held_guard_lease_refuses_before_c1_with_unchanged_cell() {
    let f = Fixture::new(false);
    let h = f.host();
    let e = approved(&h);
    let revision = h.state.lock().unwrap().revision;
    let mut profile = AdmittedProfile::from_owned(&h, &e).unwrap();
    let owner = f.owner.clone();
    let generation = f.initial.generation().to_owned();
    let cell = h.state.clone();
    profile.under_guard = Some(Arc::new(move |lease| {
        owner
            .update_under_lease(lease, &generation, source(), facts())
            .unwrap();
        assert_eq!(cell.lock().unwrap().revision, revision);
    }));
    let owner = HostOwnedFixedOperation::start_admitted(
        f.dir.path().into(),
        f.namespace.clone(),
        f.scope.clone(),
        "once".into(),
        TargetOs::Linux,
        now(),
        profile,
    )
    .unwrap();
    until(|| {
        !matches!(
            owner.inspect().unwrap(),
            FixedOperationStatus::Starting | FixedOperationStatus::Running { .. }
        )
    });
    assert!(matches!(
        owner.inspect().unwrap(),
        FixedOperationStatus::RecoveryRequired { run_id: None, .. }
    ));
    assert_eq!(h.state.lock().unwrap().revision, revision + 1);
    assert!(h.state.lock().unwrap().sequences.is_empty());
    assert_ne!(h.state.lock().unwrap().snapshot.as_ref(), Some(&f.initial));
    f.no_effect();
}

#[test]
fn independent_actor_after_terminal_replays_same_real_c1_capsule_marker_and_result_once() {
    let f = Fixture::new(true);
    let a = f.host();
    let b = f.host();
    let ea = approved(&a);
    let eb = approved(&b);
    assert_eq!(ea.identity(), eb.identity());
    let job = a.submit_no_control(&ea, "once", now()).unwrap();
    let mut run = None;
    until(
        || match job.wait_ready(Duration::from_millis(10)).unwrap() {
            Readiness::Ready(binding) => {
                run = Some(binding.run_id().to_owned());
                true
            }
            Readiness::Starting => false,
            other => panic!("real Ready required: {other:?}"),
        },
    );
    let run = run.unwrap();
    let first = terminal(&job);
    assert!(
        matches!(&first, FixedOperationStatus::ResultAvailable { run_id, report_bytes: Some(_), .. } if run_id == &run)
    );
    let replay = b.submit_no_control(&eb, "once", now()).unwrap();
    let observed = terminal(&replay);
    assert_eq!(
        observed, first,
        "retry must expose the actual retained run/result"
    );
    assert_eq!(b.inspect_existing("once").unwrap(), Some(first));
    assert_eq!(
        std::fs::read(f.dir.path().join("counter")).unwrap(),
        b"effect\n"
    );
    let lease = vibe_lifecycle::LifecycleLease::acquire(f.dir.path()).unwrap();
    let row = SubmissionReservations::inspect_existing_key(&lease, &f.namespace, &f.scope, "once")
        .unwrap()
        .unwrap();
    assert_eq!(row.domain_run_id, run);
    assert_eq!(
        row.journal_admission,
        vibe_lifecycle::JournalAdmission::Ready
    );
    let out = f.dir.path().join(".vibe/lifecycle-outcomes");
    assert!(out.join(format!("{run}.admitted-input.json")).exists());
    assert!(out.join(format!("{run}.marker.json")).exists());
}

#[test]
fn same_shared_host_retry_while_real_provider_holds_lease_attaches_same_job() {
    let f = Fixture::new(true);
    let path = f.dir.path().join("run.sh");
    let mut script = std::fs::read_to_string(&path).unwrap();
    script.push_str("touch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); [ \"$i\" -lt 400 ] || exit 70; sleep 0.025; done\n");
    std::fs::write(&path, script).unwrap();
    let h = f.host();
    let evidence = approved(&h);
    let job = h.submit_no_control(&evidence, "once", now()).unwrap();
    until(|| f.dir.path().join("entered").exists());
    struct Release(std::path::PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, "release");
        }
    }
    let release = Release(f.dir.path().join("release"));
    let first = job.wait_ready(WAIT).unwrap();
    let run = match first {
        Readiness::Ready(binding) => binding.run_id().to_owned(),
        other => panic!("actual Ready required: {other:?}"),
    };
    let retry = h.submit_no_control(&evidence, "once", now());
    let retry = retry.expect("same admitted key must attach while provider is active");
    assert!(
        matches!(retry.wait_ready(WAIT).unwrap(), Readiness::Ready(binding) if binding.run_id() == run)
    );
    assert!(matches!(
        retry.inspect().unwrap(),
        FixedOperationStatus::Running { run_id } if run_id == run
    ));
    let mut observer = retry.attach_readonly().unwrap();
    assert_eq!(observer.summary().run_id(), run);
    assert!(observer.detach());
    assert_eq!(
        std::fs::read(f.dir.path().join("counter")).unwrap(),
        b"effect\n"
    );
    drop(release);
    let outcome = terminal(&job);
    assert_eq!(terminal(&retry), outcome);
    assert_eq!(
        std::fs::read(f.dir.path().join("counter")).unwrap(),
        b"effect\n"
    );
}
