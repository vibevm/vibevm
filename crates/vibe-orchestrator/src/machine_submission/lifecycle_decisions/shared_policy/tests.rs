//! Portable real-source question identity and owned-policy refusal cases.
use super::super::submission::profile::AdmittedProfile;
use super::*;
use crate::machine_submission::{PolicySnapshot, TrustedPolicySource};
use std::fs;
use vibe_lifecycle::LifecycleLease;

pub(super) fn facts() -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: true,
        approve_exact_local_install: false,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
pub(super) fn source() -> TrustedPolicySource {
    TrustedPolicySource::from_host_identifier("trusted-shared-fixture".into()).unwrap()
}
pub(super) fn options(control: RequestedOptionsControl) -> RequestedOptions {
    RequestedOptions {
        control,
        force: false,
        trace: false,
        deploy_profile: None,
    }
}
pub(super) struct Fixture {
    pub dir: tempfile::TempDir,
    pub namespace: SubmissionNamespace,
    pub scope: SubmissionScope,
    pub owner: Arc<PolicyOwner>,
    pub initial: PolicySnapshot,
    pub os: TargetOs,
    pub effects: bool,
}
impl Fixture {
    pub fn new(effects: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"), "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n").unwrap();
        let os = TargetOs::current().expect("native OS");
        if os == TargetOs::Windows {
            fs::write(dir.path().join("run.ps1"), "Add-Content counter 'effect'\n").unwrap();
        } else {
            let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
            fs::write(
                dir.path().join("run.sh"),
                format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\n"),
            )
            .unwrap();
        }
        let scope = SubmissionScope {
            context: "shared-policy".into(),
            target: "root".into(),
            principal: "host".into(),
        };
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        let initial =
            PolicyOwner::initialize_under_lease(&lease, &scope, source(), facts()).unwrap();
        let owner = Arc::new(
            PolicyOwner::attach_readonly(
                Arc::new(Project::open(dir.path()).unwrap()),
                scope.clone(),
            )
            .unwrap(),
        );
        drop(lease);
        Self {
            dir,
            namespace,
            scope,
            owner,
            initial,
            os,
            effects,
        }
    }
    pub fn host(&self) -> FixedLifecycleDecisionHost {
        FixedLifecycleDecisionHost::from_shared_owner(
            self.dir.path().into(),
            self.namespace.clone(),
            self.scope.clone(),
            self.os,
            self.owner.clone(),
        )
        .unwrap()
    }
    pub fn update(&self) -> PolicySnapshot {
        let lease = LifecycleLease::acquire(self.dir.path()).unwrap();
        self.owner
            .update_under_lease(&lease, self.initial.generation(), source(), facts())
            .unwrap()
    }
    pub fn no_effect(&self) {
        assert!(!self.dir.path().join("counter").exists());
        assert!(!self.dir.path().join(".vibe/lifecycle.toml").exists());
        assert!(!self.dir.path().join(".vibe/lifecycle-outcomes").exists());
        assert!(!self.dir.path().join(".vibe/tmp").exists());
        let lease = LifecycleLease::acquire(self.dir.path()).unwrap();
        assert!(
            SubmissionReservations::inspect_existing_key(
                &lease,
                &self.namespace,
                &self.scope,
                "once"
            )
            .unwrap()
            .is_none()
        );
        let index: serde_json::Value = serde_json::from_slice(
            &fs::read(self.dir.path().join(".vibe/machine-submissions.json")).unwrap(),
        )
        .unwrap();
        assert!(
            index["namespaces"]
                .as_object()
                .unwrap()
                .values()
                .all(|n| n["keys"].as_object().unwrap().is_empty())
        );
    }
    pub fn policy_path(&self) -> PathBuf {
        fs::read_dir(self.dir.path().join(".vibe/machine-policy"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.effects {
            self.no_effect();
        }
    }
}
pub(super) fn approval(
    host: &FixedLifecycleDecisionHost,
    control: RequestedOptionsControl,
) -> InputDecisionMessage {
    let mut q = host.prepare_requested_options(options(control)).unwrap();
    if matches!(q.body, Body::SelectionQuestion(_)) {
        let mut a = q.clone();
        a.body = Body::SelectionAnswer(Box::new(BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        q = match host.answer_typed(&a).unwrap() {
            DecisionOutcome::Question(q) => q,
            _ => panic!("approval"),
        };
    }
    q.body = Body::ApprovalAnswer(Box::new(BodyApprovalAnswer { approved: true }));
    q
}
pub(super) fn approved(host: &FixedLifecycleDecisionHost) -> ValidatedFixedPlanEvidence {
    match host
        .answer_typed(&approval(host, RequestedOptionsControl::None))
        .unwrap()
    {
        DecisionOutcome::Validated(e) => e,
        _ => panic!("sealed local evidence"),
    }
}

#[test]
fn readonly_owner_observation_creates_nothing_and_tracks_real_updates_without_a_grant() {
    let dir = tempfile::tempdir().unwrap();
    let project = Arc::new(Project::open(dir.path()).unwrap());
    let scope = SubmissionScope {
        context: "readonly-policy".into(),
        target: "root".into(),
        principal: "host".into(),
    };
    assert!(matches!(
        PolicyOwner::attach_readonly(project.clone(), scope.clone()),
        Err(PolicyError::Unavailable)
    ));
    assert!(!dir.path().join(".vibe").exists());
    let lease = LifecycleLease::acquire(dir.path()).unwrap();
    let initial = PolicyOwner::initialize_under_lease(&lease, &scope, source(), facts()).unwrap();
    let owner = PolicyOwner::attach_readonly(project, scope).unwrap();
    let retained = owner.observe_readonly().unwrap();
    assert_eq!(retained, initial);
    // A writer is still held: observation must not acquire a nested lease.
    let next = owner
        .update_under_lease(&lease, retained.generation(), source(), facts())
        .unwrap();
    assert_eq!(owner.observe_readonly().unwrap(), next);
    assert_ne!(retained, next);
    assert!(matches!(
        owner.update_under_lease(&lease, retained.generation(), source(), facts()),
        Err(PolicyError::StaleGeneration)
    ));
    assert_eq!(owner.current_under_lease(&lease).unwrap(), next);
    assert!(!dir.path().join(".vibe/machine-submissions.json").exists());
    assert!(!dir.path().join(".vibe/lifecycle.toml").exists());
    assert!(!dir.path().join(".vibe/tmp").exists());
    let path = fs::read_dir(dir.path().join(".vibe/machine-policy"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = fs::read(&path).unwrap();
    owner.observe_readonly().unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::write(&path, "corrupt").unwrap();
    assert!(matches!(
        owner.observe_readonly(),
        Err(PolicyError::InvalidRecord(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), b"corrupt");
    fs::remove_file(&path).unwrap();
    assert!(matches!(
        owner.observe_readonly(),
        Err(PolicyError::Unavailable)
    ));
    assert!(!path.exists());
}

#[test]
fn independent_hosts_share_semantic_identity_but_never_questions_or_evidence() {
    let f = Fixture::new(false);
    let a = f.host();
    let b = f.host();
    let qa = approval(&a, RequestedOptionsControl::None);
    let qb = approval(&b, RequestedOptionsControl::None);
    assert_ne!(qa.question_id, qb.question_id);
    assert_eq!(qa.policy_generation, qb.policy_generation);
    assert_eq!(qa.plan_id, qb.plan_id);
    assert!(matches!(
        b.answer_typed(&qa),
        Err(FixedDecisionError::UnknownQuestion)
    ));
    let ea = match a.answer_typed(&qa).unwrap() {
        DecisionOutcome::Validated(e) => e,
        _ => panic!("evidence"),
    };
    let eb = match b.answer_typed(&qb).unwrap() {
        DecisionOutcome::Validated(e) => e,
        _ => panic!("evidence"),
    };
    assert_eq!(ea.identity(), eb.identity());
    let pa = AdmittedProfile::from_owned(&a, &ea).unwrap();
    let pb = AdmittedProfile::from_owned(&b, &eb).unwrap();
    assert_eq!(
        pa.request(&ea.base.request).unwrap(),
        pb.request(&eb.base.request).unwrap()
    );
    assert_eq!(
        pa.plan(&ea.base.plan).unwrap(),
        pb.plan(&eb.base.plan).unwrap()
    );
    assert!(matches!(
        b.submit_no_control(&ea, "once", "2026-10-02T00:00:00Z".parse().unwrap()),
        Err(FixedDecisionError::ForeignScope)
    ));
    let restarted = f.host();
    assert_eq!(
        restarted
            .state
            .lock()
            .unwrap()
            .snapshot
            .as_ref()
            .unwrap()
            .generation(),
        f.initial.generation()
    );
    assert!(matches!(
        restarted.answer_typed(&qa),
        Err(FixedDecisionError::UnknownQuestion)
    ));
}

#[test]
fn external_update_invalidates_answers_and_evidence_and_shared_setter_refuses() {
    let f = Fixture::new(false);
    let h = f.host();
    let e = approved(&h);
    let answer = approval(&h, RequestedOptionsControl::Detach);
    let before = fs::read(f.policy_path()).unwrap();
    assert!(
        matches!(h.set_host_policy(facts()), Err(FixedDecisionError::ImplementationGap(reason)) if reason.contains("PolicyOwner.update_under_lease"))
    );
    assert_eq!(fs::read(f.policy_path()).unwrap(), before);
    assert_eq!(h.state.lock().unwrap().sequences.len(), 2);
    let next = f.update();
    assert_ne!(next.generation(), f.initial.generation());
    assert!(matches!(
        h.answer_typed(&answer),
        Err(FixedDecisionError::StalePolicy)
    ));
    assert!(h.state.lock().unwrap().sequences.is_empty());
    assert!(matches!(
        AdmittedProfile::from_owned(&h, &e),
        Err(FixedDecisionError::StalePolicy)
    ));
    assert!(matches!(
        h.submit_no_control(&e, "once", "2026-10-02T00:00:00Z".parse().unwrap()),
        Err(FixedDecisionError::StalePolicy)
    ));
    assert!(h.jobs.lock().unwrap().is_empty());
}

#[test]
fn missing_corrupt_and_foreign_owner_refuse_without_policy_replacement() {
    for damage in ["missing", "corrupt", "foreign"] {
        let f = Fixture::new(false);
        let path = f.policy_path();
        if damage == "missing" {
            fs::remove_file(&path).unwrap();
        }
        if damage == "corrupt" {
            fs::write(&path, "invalid").unwrap();
        }
        let before = fs::read(&path).ok();
        let mut scope = f.scope.clone();
        if damage == "foreign" {
            scope.principal = "foreign".into();
        }
        assert!(
            FixedLifecycleDecisionHost::from_shared_owner(
                f.dir.path().into(),
                f.namespace.clone(),
                scope,
                f.os,
                f.owner.clone()
            )
            .is_err()
        );
        assert_eq!(fs::read(path).ok(), before);
    }
    let f = Fixture::new(false);
    let other = Fixture::new(false);
    assert!(
        FixedLifecycleDecisionHost::from_shared_owner(
            f.dir.path().into(),
            f.namespace.clone(),
            f.scope.clone(),
            f.os,
            other.owner.clone()
        )
        .is_err()
    );
}

#[test]
fn unavailable_policy_after_approval_refuses_without_changing_local_records_or_jobs() {
    for corrupt in [false, true] {
        let f = Fixture::new(false);
        let h = f.host();
        let e = approved(&h);
        let before = h.state.lock().unwrap().revision;
        if corrupt {
            fs::write(f.policy_path(), "corrupt").unwrap();
        } else {
            fs::remove_file(f.policy_path()).unwrap();
        }
        assert!(
            h.submit_no_control(&e, "once", "2026-10-02T00:00:00Z".parse().unwrap())
                .is_err()
        );
        assert_eq!(h.state.lock().unwrap().revision, before);
        assert_eq!(h.state.lock().unwrap().sequences.len(), 1);
        assert!(h.jobs.lock().unwrap().is_empty());
    }
}

#[test]
fn delayed_readonly_observation_cannot_downgrade_fresh_leased_preparation() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let started = Instant::now();
    let bound = Duration::from_secs(1);
    let f = Fixture::new(false);
    let mut host = f.host();
    let old_evidence = approved(&host);
    let (read_tx, read_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let first = AtomicBool::new(true);
    host.after_shared_read = Some(Arc::new(move |snapshot| {
        if first.swap(false, Ordering::SeqCst) {
            read_tx.send(snapshot.clone()).unwrap();
            release_rx.lock().unwrap().recv_timeout(bound).unwrap();
        }
    }));
    let (leased_tx, leased_rx) = mpsc::channel();
    let owner = f.owner.clone();
    host.before_shared_refresh_access = Some(Arc::new(move |lease| {
        // Positive control: preparation has the genuine current writer lease.
        leased_tx
            .send(owner.current_under_lease(lease).unwrap())
            .unwrap();
    }));
    let host = Arc::new(host);
    let (done_tx, done_rx) = mpsc::channel();
    let (observed_tx, observed_rx) = mpsc::channel();
    std::thread::scope(|threads| {
        let reader = host.clone();
        let reader_evidence = old_evidence.clone();
        threads.spawn(move || {
            // Actual admission-factory ingress invokes readonly observation.
            observed_tx
                .send(AdmittedProfile::from_owned(&reader, &reader_evidence).map(|_| ()))
                .unwrap();
        });
        assert_eq!(read_rx.recv_timeout(bound).unwrap(), f.initial);
        // An independent actor changes the actual file, not the host cell.
        let current = f.update();
        assert_ne!(current.generation(), f.initial.generation());
        let preparer = host.clone();
        threads.spawn(move || {
            done_tx
                .send(preparer.prepare_requested_options(options(RequestedOptionsControl::Detach)))
                .unwrap();
        });
        assert_eq!(leased_rx.recv_timeout(bound).unwrap(), current);
        // Never wait for fresh completion while the paused reader holds access.
        // With the reviewed old ordering, preparation completes here, then the
        // released reader overwrites its generation and deletes its question.
        let early = match done_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => Some(result),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(error) => panic!("preparation channel: {error}"),
        };
        release_tx.send(()).unwrap();
        // Its private record check may race the fresh apply after access is
        // released. Either an old sealed profile or a stale refusal is safe:
        // the actual guard still requires the current leased snapshot.
        assert!(matches!(
            observed_rx.recv_timeout(bound).unwrap(),
            Ok(()) | Err(FixedDecisionError::StalePolicy)
        ));
        let question = early
            .unwrap_or_else(|| done_rx.recv_timeout(bound).unwrap())
            .unwrap();
        let state = host.state.lock().unwrap();
        assert_eq!(state.snapshot.as_ref(), Some(&current));
        assert_eq!(
            question.policy_generation.as_deref(),
            Some(current.generation())
        );
        assert!(
            state.sequences.iter().any(|sequence| {
                sequence.original.question_id == question.question_id
                    && sequence.original.policy_generation == question.policy_generation
            }),
            "fresh current-policy question must remain retained"
        );
        drop(state);
        assert!(matches!(
            host.submit_no_control(
                &old_evidence,
                "once",
                "2026-10-02T00:00:00Z".parse().unwrap()
            ),
            Err(FixedDecisionError::StalePolicy)
        ));
        assert!(host.jobs.lock().unwrap().is_empty());
    });
    assert!(started.elapsed() < Duration::from_secs(5));
    f.no_effect();
}
