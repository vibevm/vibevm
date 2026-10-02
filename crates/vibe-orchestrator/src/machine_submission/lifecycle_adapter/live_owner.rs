//! Host-owned fixed Validate dispatch. Observer lifetimes never own execution.
//! ContinueCurrentDispatch applies only while this host process survives.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OWNER-LIFETIME");

use super::*;
mod control;
mod deferred_stop;
pub(super) mod observation;
mod session_facade;
mod stream;
pub use session_facade::{
    BindingSummary, FixedOperationStatus, HostControlCapability, HostOwnedFixedOperation,
    ObservationHandle, Readiness,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Condvar, Mutex, Weak};
use std::time::Duration;
use vibe_safefs::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConnectionLossPolicy {
    ContinueCurrentDispatch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LiveState {
    Starting,
    Running,
    OwnerStopped,
    ResultAvailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Observation {
    Starting,
    Running {
        run_id: String,
    },
    RecoveryRequired {
        run_id: Option<String>,
        reason: String,
    },
    ResultAvailable {
        run_id: String,
        result: RetainedLifecycleResult,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cancellation {
    Unavailable,
}

/// Trusted embedding host coordinates, never a wire request authority flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HostCoordinates {
    pub root: PathBuf,
    pub namespace: SubmissionNamespace,
    pub scope: SubmissionScope,
    pub key: String,
}
/// Minted only at actual Ready; fields are inaccessible to the embedding host.
#[derive(Clone)]
pub(super) struct Binding {
    owner: Weak<Shared>,
    coordinates: HostCoordinates,
    run_id: String,
    reservation_ref: String,
    request_sha256: String,
    plan_sha256: String,
    physical_root: Arc<PhysicalRoot>,
    history: Arc<Mutex<stream::History>>,
    source: ScriptSourceWitness,
    selected: SelectedManifestLockWitness,
    started: String,
    gate: control::Gate,
}

/// Retain a no-follow root independently of the writer lease on every host.
/// Tokens detect ordinary replacement among cooperating same-host writers;
/// identity reuse can match, and this is neither hostile-UID CAS nor authority.
struct PhysicalRoot {
    held: Project,
    token: String,
}
impl PhysicalRoot {
    fn capture(lease: &LifecycleLease) -> Result<Self> {
        // The public accessor borrows the exact project pinned by this lease.
        let token = lease
            .publication_journal_project()
            .root_correlation_token()?;
        let pinned = Self {
            held: Project::open(lease.root())?,
            token,
        };
        pinned.check_lease(lease)?;
        Ok(pinned)
    }
    fn check(&self, root: &Path) -> Result<()> {
        ensure!(
            self.held.root_correlation_token()? == self.token
                && Project::open(root)?.root_correlation_token()? == self.token,
            "owner physical root changed"
        );
        Ok(())
    }
    fn check_lease(&self, lease: &LifecycleLease) -> Result<()> {
        ensure!(
            lease
                .publication_journal_project()
                .root_correlation_token()?
                == self.token,
            "owner lease physical root changed"
        );
        self.check(lease.root())
    }
}
struct Record {
    state: LiveState,
    binding: Option<Binding>,
    lease: Option<Arc<LifecycleLease>>,
    canonical_request: Vec<u8>,
    reason: String,
    subscriptions: BTreeSet<u64>,
    next_subscription: u64,
    deferred_stop: bool,
    provider_evidence: Option<deferred_stop::NativeEvidence>,
}
impl Record {
    // Only the incumbent under this record lock carries control state.
    fn gate(&self) -> &control::Gate {
        &self.binding.as_ref().expect("Ready control binding").gate
    }
    fn gate_mut(&mut self) -> &mut control::Gate {
        &mut self.binding.as_mut().expect("Ready control binding").gate
    }
}
struct Shared {
    record: Mutex<Record>,
    changed: Condvar,
}
/// Host retains this separately from Observer. The worker owns its own Arc;
/// dropping all observation handles cannot cancel or retry the dispatch.
pub(super) struct LiveOwner {
    shared: Arc<Shared>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}
pub(super) struct Observer {
    shared: Arc<Shared>,
    subscription: Option<u64>,
}
impl Observer {
    pub(super) fn detach(&mut self) -> bool {
        let Some(id) = self.subscription.take() else {
            return false;
        };
        self.shared.record.lock().unwrap().subscriptions.remove(&id)
    }
    pub(super) fn cancel(&self) -> Cancellation {
        Cancellation::Unavailable
    }
    pub(super) fn inspect(&self) -> Result<Observation> {
        ensure!(self.subscription.is_some(), "observer detached");
        inspect(&self.shared)
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.detach();
    }
}
impl LiveOwner {
    pub(super) fn policy(&self) -> ConnectionLossPolicy {
        connection_loss_policy()
    }
    pub(super) fn state(&self) -> LiveState {
        self.shared.record.lock().unwrap().state
    }
    pub(super) fn wait_ready(&self, timeout: Duration) -> Option<Binding> {
        let record = self.shared.record.lock().unwrap();
        let (record, _) = self
            .shared
            .changed
            .wait_timeout_while(record, timeout, |r| r.state == LiveState::Starting)
            .unwrap();
        record.binding.clone()
    }
    pub(super) fn inspect(&self) -> Result<Observation> {
        inspect(&self.shared)
    }
    pub(super) fn attach(&self, binding: &Binding, trusted: &HostCoordinates) -> Result<Observer> {
        ensure!(
            Weak::ptr_eq(&Arc::downgrade(&self.shared), &binding.owner),
            "foreign owner handle"
        );
        ensure!(&binding.coordinates == trusted, "foreign host coordinates");
        binding.physical_root.check(&trusted.root)?;
        {
            let record = self.shared.record.lock().unwrap();
            let incumbent = record
                .binding
                .as_ref()
                .context("owner has no Ready binding")?;
            ensure!(same_binding(incumbent, binding), "stale owner handle");
        }
        // Terminal attach must validate domain evidence before subscribing.
        inspect(&self.shared)?;
        subscribe(&self.shared).map_err(Into::into)
    }
}
fn same_binding(a: &Binding, b: &Binding) -> bool {
    a.coordinates == b.coordinates
        && a.run_id == b.run_id
        && a.reservation_ref == b.reservation_ref
        && a.request_sha256 == b.request_sha256
        && a.plan_sha256 == b.plan_sha256
        && Arc::ptr_eq(&a.physical_root, &b.physical_root)
        && Arc::ptr_eq(&a.history, &b.history)
}
fn subscribe(shared: &Arc<Shared>) -> std::result::Result<Observer, stream::SubscriptionLimit> {
    let mut record = shared.record.lock().unwrap();
    if record.subscriptions.len() >= stream::MAX_OBSERVERS {
        return Err(stream::SubscriptionLimit);
    }
    let id = record.next_subscription;
    record.next_subscription = id.checked_add(1).ok_or(stream::SubscriptionLimit)?;
    record.subscriptions.insert(id);
    Ok(Observer {
        shared: shared.clone(),
        subscription: Some(id),
    })
}

pub(super) struct ReadyObserver {
    shared: Arc<Shared>,
    physical_root: Mutex<Option<Arc<PhysicalRoot>>>,
    #[cfg(test)]
    before_effect: Option<Arc<dyn Fn(&Binding) + Send + Sync>>,
}
impl ReadyObserver {
    pub(super) fn pin_root(&self, lease: &LifecycleLease) -> Result<()> {
        let physical_root = Arc::new(PhysicalRoot::capture(lease)?);
        *self.physical_root.lock().unwrap() = Some(physical_root);
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn admitted(
        &self,
        lease: Arc<LifecycleLease>,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        run_id: &str,
        reservation_ref: &str,
        canonical_request: &[u8],
        _canonical_plan: &[u8],
        source: &ScriptSourceWitness,
        selected: &SelectedManifestLockWitness,
        started: &str,
    ) -> Result<()> {
        let physical_root = self
            .physical_root
            .lock()
            .unwrap()
            .clone()
            .context("owner root not pinned")?;
        physical_root.check_lease(&lease)?;
        let c1 = SubmissionReservations::inspect_existing_key(&lease, namespace, scope, key)?
            .context("Ready reservation disappeared")?;
        ensure!(
            c1.domain_run_id == run_id
                && c1.journal_admission == JournalAdmission::Ready
                && c1.reservation_ref.as_deref() == Some(reservation_ref),
            "Ready C1 binding differs"
        );
        SubmissionReservations::inspect_existing_request(
            &lease,
            namespace,
            scope,
            key,
            canonical_request,
        )?;
        let binding = Binding {
            gate: control::Gate::Open,
            source: source.clone(),
            selected: selected.clone(),
            started: started.to_owned(),
            owner: Arc::downgrade(&self.shared),
            coordinates: HostCoordinates {
                root: lease.root().to_owned(),
                namespace: namespace.clone(),
                scope: scope.clone(),
                key: key.to_owned(),
            },
            run_id: run_id.to_owned(),
            reservation_ref: reservation_ref.to_owned(),
            request_sha256: c1.request_sha256,
            plan_sha256: c1.plan_sha256,
            physical_root,
            history: Arc::new(Mutex::new(stream::History::new(
                scope,
                run_id,
                reservation_ref,
            ))),
        };
        binding.history.lock().unwrap().state(
            stream::Observed::JournalReady,
            stream::Effect::NoEffects,
            None,
        );
        {
            let mut record = self.shared.record.lock().unwrap();
            ensure!(record.state == LiveState::Starting, "owner Ready twice");
            record.binding = Some(binding.clone());
            record.lease = Some(lease);
            record.canonical_request = canonical_request.to_owned();
            record.state = LiveState::Running;
        }
        self.shared.changed.notify_all();
        #[cfg(test)]
        if let Some(hook) = &self.before_effect {
            hook(&binding);
        }
        Ok(())
    }
}

fn inspect(shared: &Arc<Shared>) -> Result<Observation> {
    let (state, binding, canonical_request, reason) = {
        let r = shared.record.lock().unwrap();
        (
            r.state,
            r.binding.clone(),
            r.canonical_request.clone(),
            r.reason.clone(),
        )
    };
    if state == LiveState::Starting {
        return Ok(Observation::Starting);
    }
    let Some(binding) = binding else {
        return Ok(Observation::RecoveryRequired {
            run_id: None,
            reason,
        });
    };
    binding.physical_root.check(&binding.coordinates.root)?;
    if state == LiveState::Running {
        return Ok(Observation::Running {
            run_id: binding.run_id,
        });
    }
    let c = &binding.coordinates;
    let evidence = (|| -> Result<Option<SubmissionOutcome<RetainedLifecycleResult>>> {
        let lease = LifecycleLease::acquire(&c.root)?;
        binding.physical_root.check_lease(&lease)?;
        let row =
            SubmissionReservations::inspect_existing_key(&lease, &c.namespace, &c.scope, &c.key)?
                .context("retained C1 absent")?;
        ensure!(
            row.domain_run_id == binding.run_id
                && row.journal_admission == JournalAdmission::Ready
                && row.reservation_ref.as_deref() == Some(&binding.reservation_ref)
                && row.request_sha256 == binding.request_sha256
                && row.plan_sha256 == binding.plan_sha256,
            "retained C1 changed"
        );
        attach_existing_lifecycle(&lease, &c.namespace, &c.scope, &c.key, &canonical_request)
    })();
    match evidence {
        Ok(Some(SubmissionOutcome::Replayed { run_id, outcome })) if run_id == binding.run_id => {
            Ok(Observation::ResultAvailable {
                run_id,
                result: outcome,
            })
        }
        other => Ok(Observation::RecoveryRequired {
            run_id: Some(binding.run_id),
            reason: format!("{reason}; retained inspection: {other:?}"),
        }),
    }
}

/// Available before starting or attaching any observer; no execution occurs.
pub(super) const fn connection_loss_policy() -> ConnectionLossPolicy {
    ConnectionLossPolicy::ContinueCurrentDispatch
}

/// One fixed production execution. No caller-supplied process, command or Fn.
#[allow(clippy::too_many_arguments)]
pub(super) fn start(
    path: PathBuf,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    key: String,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<(LiveOwner, Observer)> {
    launch(OwnedDispatch {
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        deferred_stop: false,
        #[cfg(test)]
        fixture: None,
        #[cfg(test)]
        before_boundary: None,
        #[cfg(test)]
        after_effect: None,
    })
}
struct OwnedDispatch {
    path: PathBuf,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    key: String,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    deferred_stop: bool,
    #[cfg(test)]
    fixture: Option<TestFixture>,
    #[cfg(test)]
    before_boundary: Option<Arc<dyn Fn(&Binding) + Send + Sync>>,
    #[cfg(test)]
    after_effect: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
}
#[cfg(test)]
struct TestFixture {
    process: Arc<dyn ProcessRunner + Send + Sync>,
    before_effect: Option<Arc<dyn Fn(&Binding) + Send + Sync>>,
}
fn launch(dispatch: OwnedDispatch) -> Result<(LiveOwner, Observer)> {
    ensure!(
        TargetOs::current() == Some(dispatch.target_os),
        "fixed script target OS differs from current host"
    );
    let shared = Arc::new(Shared {
        record: Mutex::new(Record {
            state: LiveState::Starting,
            binding: None,
            lease: None,
            canonical_request: Vec::new(),
            reason: String::new(),
            subscriptions: BTreeSet::new(),
            next_subscription: 1,
            deferred_stop: dispatch.deferred_stop,
            provider_evidence: None,
        }),
        changed: Condvar::new(),
    });
    let observer = subscribe(&shared)?;
    let worker = shared.clone();
    let thread = std::thread::Builder::new()
        .name("fixed-validate-owner".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let ready = ReadyObserver {
                    shared: worker.clone(),
                    physical_root: Mutex::new(None),
                    #[cfg(test)]
                    before_effect: dispatch
                        .fixture
                        .as_ref()
                        .and_then(|f| f.before_effect.clone()),
                };
                #[cfg(test)]
                if let Some(hook) = dispatch.before_boundary {
                    let shared = worker.clone();
                    deferred_stop::BEFORE_CLOSURE.with(|slot| {
                        *slot.borrow_mut() = Some(Box::new(move || {
                            let binding = shared.record.lock().unwrap().binding.clone().unwrap();
                            hook(&binding);
                        }));
                    });
                }
                static PROBE: SystemProbe = SystemProbe;
                let process = observation::ObservedProcess::system(&worker);
                #[cfg(test)]
                let process = dispatch.fixture.as_ref().map_or(process, |fixture| {
                    observation::ObservedProcess::fixture(fixture.process.as_ref(), &worker)
                });
                #[cfg(test)]
                let after_effect = dispatch
                    .after_effect
                    .as_ref()
                    .map(|hook| hook.as_ref() as &dyn Fn() -> Result<()>);
                #[cfg(not(test))]
                let after_effect = None;
                submit_one_script_validate_internal(
                    &dispatch.path,
                    &dispatch.namespace,
                    &dispatch.scope,
                    &dispatch.key,
                    dispatch.request,
                    dispatch.target_os,
                    dispatch.observed_at,
                    &process,
                    &PROBE,
                    None,
                    after_effect,
                    Some(&ready),
                )
            }));
            // Submission locals have unwound/dropped. Release the active proof
            // outside the mutex, before publishing terminal state or notifying.
            let lease = worker.record.lock().unwrap().lease.take();
            drop(lease);
            let mut record = worker.record.lock().unwrap();
            // Callback/error/panic exits cannot leave a post-dispatch request
            // looking pending once this owner has stopped. No receipt is minted.
            if let Some(binding) = record.binding.as_mut() {
                if let control::Gate::DeferredRequested(id) | control::Gate::DeferredStopping(id) =
                    &binding.gate
                {
                    binding.gate = control::Gate::DeferredFailed(id.clone());
                } else if matches!(binding.gate, control::Gate::DeferredOpen) {
                    binding.gate = control::Gate::Closed;
                }
            }
            match result {
                Ok(Ok(SubmissionOutcome::Invoked { .. } | SubmissionOutcome::Replayed { .. })) => {
                    if record.binding.is_some() {
                        record.state = LiveState::ResultAvailable;
                    } else {
                        record.state = LiveState::OwnerStopped;
                        record.reason =
                            "no fresh Ready binding; use existing cold domain inspection".into();
                    }
                }
                Ok(other) => {
                    record.state = LiveState::OwnerStopped;
                    record.reason = format!("submission stopped: {other:?}");
                }
                Err(_) => {
                    record.state = LiveState::OwnerStopped;
                    record.reason =
                        "owner thread panicked; host process death is not established".into();
                }
            }
            drop(record);
            stream::publish_terminal(&worker);
            worker.changed.notify_all();
        })?;
    Ok((
        LiveOwner {
            shared,
            worker: Mutex::new(Some(thread)),
        },
        observer,
    ))
}

#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "linux"))]
mod process_tests;
