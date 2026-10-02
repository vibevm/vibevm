//! Sealed ownership seam for the incumbent bounded one-script Validate profile.
//! C3 transport, session authentication and responsive pumping remain unfinished.
//! A future adapter must host-map connection-local IDs to these immutable facts;
//! string equality, DTO validation and summaries never grant authority.
use super::*;
use vibe_wire::generated::machine::e1::operation_stream_message::OperationStreamMessage;

/// Immutable data copied from the actual incumbent history, never a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingSummary {
    context_id: String,
    target_id: Option<String>,
    operation_id: String,
    run_id: String,
    stream_id: String,
}
impl BindingSummary {
    fn from_binding(binding: &Binding) -> Self {
        let history = binding.history.lock().unwrap();
        let expected = history.expected();
        Self {
            context_id: expected.context_id.to_owned(),
            target_id: expected.target_id.map(str::to_owned),
            operation_id: expected.operation_id.to_owned(),
            run_id: expected.run_id.to_owned(),
            stream_id: expected.stream_id.to_owned(),
        }
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn target_id(&self) -> Option<&str> {
        self.target_id.as_deref()
    }
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}

/// Fresh incumbent observation. Historical terminal references are not results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedOperationStatus {
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
        marker: LifecycleOutcomeMarker,
        report_bytes: Option<Vec<u8>>,
    },
}
impl From<Observation> for FixedOperationStatus {
    fn from(value: Observation) -> Self {
        match value {
            Observation::Starting => Self::Starting,
            Observation::Running { run_id } => Self::Running { run_id },
            Observation::RecoveryRequired { run_id, reason } => {
                Self::RecoveryRequired { run_id, reason }
            }
            Observation::ResultAvailable { run_id, result } => Self::ResultAvailable {
                run_id,
                marker: result.marker,
                report_bytes: result.report_bytes,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    Starting,
    Ready(BindingSummary),
    Stopped {
        run_id: Option<String>,
        reason: String,
    },
}
struct Inner {
    owner: LiveOwner,
    coordinates: HostCoordinates,
}

/// Host-retained ownership, independent of observer connections.
/// Losing a connection continues this dispatch while the host process survives.
/// This is neither a negotiated capability surface nor a public submit grammar.
///
/// The public import seam for a separate embedding crate is:
/// ```no_run
/// use std::time::Duration;
/// use vibe_orchestrator::machine_submission::{HostOwnedFixedOperation, Readiness};
/// fn observe(host: &HostOwnedFixedOperation) -> anyhow::Result<()> {
///     if let Readiness::Ready(summary) = host.wait_ready(Duration::ZERO)? {
///         let reader = host.attach()?;
///         assert_eq!(reader.summary(), &summary);
///         let status = reader.inspect()?;
///     }
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct HostOwnedFixedOperation {
    inner: Arc<Inner>,
}
impl HostOwnedFixedOperation {
    /// Starts the existing fixed one-script Validate owner thread, including its
    /// C1 admission, journal/source publication and eventual provider effects.
    /// This is an effectful factory. The embedding host supplies the root and
    /// previously issued namespace/scope/key; never pass wire-selected roots or
    /// treat a request or a summary as host authorization. Admission still checks
    /// the unchanged fixed profile, current OS, source and writer proofs.
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        root: PathBuf,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        key: String,
        request: DefaultLifecycleRequest,
        target_os: TargetOs,
        observed_at: Timestamp,
    ) -> Result<Self> {
        let coordinates = HostCoordinates {
            root,
            namespace,
            scope,
            key,
        };
        let (owner, initial_observer) = super::start(
            coordinates.root.clone(),
            coordinates.namespace.clone(),
            coordinates.scope.clone(),
            coordinates.key.clone(),
            request,
            target_os,
            observed_at,
        )?;
        // Factory subscription is not a public observation before actual Ready.
        drop(initial_observer);
        Ok(Self {
            inner: Arc::new(Inner { owner, coordinates }),
        })
    }

    pub(in crate::machine_submission) fn start_admitted(
        root: PathBuf,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        key: String,
        target_os: TargetOs,
        observed_at: Timestamp,
        profile: AdmittedProfile,
    ) -> Result<Self> {
        let coordinates = HostCoordinates {
            root,
            namespace,
            scope,
            key,
        };
        let (owner, observer) = launch(OwnedDispatch {
            path: coordinates.root.clone(),
            namespace: coordinates.namespace.clone(),
            scope: coordinates.scope.clone(),
            key: coordinates.key.clone(),
            request: DefaultLifecycleRequest {
                requested: Phase::Validate,
                force: false,
                agent_mode: RunAgentMode::Cli,
                assume_yes: true,
                trace_flag: false,
                install_inputs: Default::default(),
                policy: Default::default(),
            },
            target_os,
            observed_at,
            deferred_stop: profile.control_mode() == vibe_wire::generated::machine::e1::input_decision_message::RequestedOptionsControl::CancelDeferredBoundary,
            profile: Some(profile),
            #[cfg(test)]
            fixture: None,
            #[cfg(test)]
            before_boundary: None,
            #[cfg(test)]
            after_effect: None,
        })?;
        drop(observer);
        Ok(Self {
            inner: Arc::new(Inner { owner, coordinates }),
        })
    }

    /// Host opt-in for a deferred stop at this domain's protected callback boundary.
    /// An acknowledged request may wait indefinitely for the direct provider to
    /// exit naturally and the callback and after-effect hook to return. This
    /// never kills providers or descendants; detach is observation only. Reached
    /// means no further host work in this domain, not process-tree termination
    /// or undoing script effects. The retained disposition is Uncertain.
    #[allow(clippy::too_many_arguments)]
    pub fn start_with_deferred_stop(
        root: PathBuf,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        key: String,
        request: DefaultLifecycleRequest,
        target_os: TargetOs,
        observed_at: Timestamp,
    ) -> Result<Self> {
        let coordinates = HostCoordinates {
            root,
            namespace,
            scope,
            key,
        };
        let (owner, initial_observer) = launch(OwnedDispatch {
            path: coordinates.root.clone(),
            namespace: coordinates.namespace.clone(),
            scope: coordinates.scope.clone(),
            key: coordinates.key.clone(),
            request,
            target_os,
            observed_at,
            deferred_stop: true,
            profile: None,
            #[cfg(test)]
            fixture: None,
            #[cfg(test)]
            before_boundary: None,
            #[cfg(test)]
            after_effect: None,
        })?;
        drop(initial_observer);
        Ok(Self {
            inner: Arc::new(Inner { owner, coordinates }),
        })
    }

    #[cfg(test)]
    pub(super) fn from_test_owner(owner: LiveOwner, coordinates: HostCoordinates) -> Self {
        Self {
            inner: Arc::new(Inner { owner, coordinates }),
        }
    }

    #[cfg(test)]
    pub(in crate::machine_submission) fn after_cold_lease_for_test(
        &self,
        hook: Arc<dyn Fn() + Send + Sync>,
    ) {
        *self.inner.owner.shared.after_cold_lease.lock().unwrap() = Some(hook);
    }

    /// A single finite wait, capped at ten seconds. Zero is a readiness poll.
    /// Ready is returned only for the actual incumbent binding; never guess IDs.
    pub fn wait_ready(&self, timeout: Duration) -> Result<Readiness> {
        if let Some(binding) = self
            .inner
            .owner
            .wait_ready(timeout.min(Duration::from_secs(10)))
        {
            self.inner
                .owner
                .check_stream_binding(&binding, &self.inner.coordinates)?;
            return Ok(Readiness::Ready(BindingSummary::from_binding(&binding)));
        }
        Ok(match self.inner.owner.inspect()? {
            Observation::Starting => Readiness::Starting,
            Observation::RecoveryRequired { run_id, reason } => {
                Readiness::Stopped { run_id, reason }
            }
            // Admission may win between the finite wait and inspection. A
            // subsequent poll retrieves its actual binding; never invent a
            // stopped state or a guessed Ready summary for that race.
            Observation::Running { .. } | Observation::ResultAvailable { .. } => {
                Readiness::Starting
            }
        })
    }

    pub fn inspect(&self) -> Result<FixedOperationStatus> {
        let observed = self.inner.owner.inspect()?;
        if matches!(observed, Observation::RecoveryRequired { run_id: None, .. }) {
            let _access = self.inner.owner.shared.evidence_access.lock().unwrap();
            let c = &self.inner.coordinates;
            let lease = LifecycleLease::acquire(&c.root)?;
            if let Some(status) = inspect_fixed_existing(&lease, &c.namespace, &c.scope, &c.key)? {
                return Ok(status);
            }
        }
        Ok(observed.into())
    }

    /// Attach to this owner's actual Ready binding; capped by the incumbent 64
    /// subscriptions. No caller-supplied summary, root or ID selects an owner.
    pub fn attach(&self) -> Result<ObservationHandle> {
        let binding = self
            .inner
            .owner
            .wait_ready(Duration::ZERO)
            .context("owner is not Ready")?;
        let observer = self.inner.owner.attach(&binding, &self.inner.coordinates)?;
        Ok(ObservationHandle {
            summary: BindingSummary::from_binding(&binding),
            inner: self.inner.clone(),
            binding,
            observer,
        })
    }

    pub(in crate::machine_submission) fn scoped_control(
        &self,
        observer: &mut ObservationHandle,
        message: &OperationStreamMessage,
        mode: &vibe_wire::generated::machine::e1::input_decision_message::RequestedOptionsControl,
    ) -> Result<OperationStreamMessage> {
        ensure!(
            Arc::ptr_eq(&self.inner, &observer.inner),
            "foreign scoped owner/observer pairing"
        );
        self.inner
            .owner
            .control_scoped(
                &observer.binding,
                &self.inner.coordinates,
                &mut observer.observer,
                message,
                mode,
            )
            .map_err(Into::into)
    }

    /// Explicit host grant for this operation. Only owning handles can mint it.
    pub fn grant_control(&self) -> HostControlCapability {
        HostControlCapability {
            inner: self.inner.clone(),
        }
    }
}

/// A separate host grant with a private constructor, bound to one owner.
///
/// ```compile_fail
/// use vibe_orchestrator::machine_submission::HostControlCapability;
/// let capability = HostControlCapability {};
/// ```
pub struct HostControlCapability {
    inner: Arc<Inner>,
}
impl HostControlCapability {
    /// Checks already-held opaque objects without touching the control gate.
    pub fn validate_observer(&self, observer: &ObservationHandle) -> Result<()> {
        ensure!(
            Arc::ptr_eq(&self.inner, &observer.inner),
            "foreign capability/observer pairing"
        );
        ensure!(
            observer.observer.subscription.is_some(),
            "observer detached"
        );
        self.inner
            .owner
            .check_stream_binding(&observer.binding, &self.inner.coordinates)
            .map_err(Into::into)
    }

    /// Requires this exact capability and an active observer of the same owner.
    /// The generated value is validated by the incumbent before its control gate.
    pub fn control(
        &self,
        observer: &mut ObservationHandle,
        message: &OperationStreamMessage,
    ) -> Result<OperationStreamMessage> {
        ensure!(
            Arc::ptr_eq(&self.inner, &observer.inner),
            "foreign capability/observer pairing"
        );
        self.inner
            .owner
            .control(
                &observer.binding,
                &self.inner.coordinates,
                &mut observer.observer,
                message,
            )
            .map_err(Into::into)
    }
}

/// Read-only observation and subscription detachment, with no control maker.
/// Dropping or detaching this handle releases only its subscription.
///
/// ```compile_fail
/// use vibe_orchestrator::machine_submission::ObservationHandle;
/// fn manufacture(reader: &ObservationHandle) {
///     let capability = reader.grant_control();
/// }
/// ```
/// A copied summary cannot attach a subscription or admit an operation:
/// ```compile_fail
/// use vibe_orchestrator::machine_submission::BindingSummary;
/// fn attach_from_data(summary: &BindingSummary) {
///     let reader = summary.attach();
/// }
/// ```
pub struct ObservationHandle {
    inner: Arc<Inner>,
    binding: Binding,
    observer: Observer,
    summary: BindingSummary,
}
impl ObservationHandle {
    pub fn summary(&self) -> &BindingSummary {
        &self.summary
    }
    pub fn inspect(&self) -> Result<FixedOperationStatus> {
        self.inner
            .owner
            .check_stream_binding(&self.binding, &self.inner.coordinates)?;
        self.observer.inspect().map(Into::into)
    }
    pub fn replay(&self, message: &OperationStreamMessage) -> Result<OperationStreamMessage> {
        ensure!(self.observer.subscription.is_some(), "observer detached");
        self.inner
            .owner
            .replay(&self.binding, &self.inner.coordinates, message)
            .map_err(Into::into)
    }
    pub fn detach(&mut self) -> bool {
        self.observer.detach()
    }
}

#[cfg(test)]
mod tests;
