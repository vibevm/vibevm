//! Host capability control at the single proven pre-dispatch boundary.
//! Requested is only a bounded host-memory latch, never durable stop evidence.
use super::*;
use crate::machine_operation::stream_message::{self as validation};
use BodyControlReplyOperationEffectState as Effect;
use BodyControlReplyStatus as Status;
use vibe_wire::generated::lifecycle_outcome_marker::ReportEvidence;
use vibe_wire::generated::machine::e1::operation_stream_message::*;

#[derive(Clone)]
pub(super) enum Gate {
    Closed,
    Open,
    Requested(String),
    Stopping(String),
    Stopped(String),
    Failed(String),
    DeferredOpen,
    DeferredRequested(String),
    DeferredStopping(String),
    DeferredStopped(String, LifecycleOutcomeMarker),
    DeferredFailed(String),
}
#[derive(Debug)]
pub(in super::super) enum ControlError {
    Binding,
    Invalid(validation::StreamValidationError),
    WrongBody,
    ConflictingRequest,
    ScopeUnavailable(String),
}
impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "host control refused: {self:?}")
    }
}
impl std::error::Error for ControlError {}

pub(super) fn reply(
    binding: &Binding,
    request: &BodyControlRequest,
    status: Status,
    effect: Effect,
    reference: Option<String>,
) -> Result<OperationStreamMessage> {
    let history = binding.history.lock().unwrap();
    let message = history.message(Body::ControlReply(Box::new(BodyControlReply {
        action: match request.action {
            BodyControlRequestAction::Cancel => BodyControlReplyAction::Cancel,
            BodyControlRequestAction::Detach => BodyControlReplyAction::Detach,
        },
        operation_effect_state: effect,
        request_id: request.request_id.clone(),
        result_ref: reference,
        status,
    })));
    validation::validate(&message, history.expected())?;
    Ok(message)
}
pub(super) fn reference(binding: &Binding) -> String {
    format!("lifecycle:{}:{}", binding.run_id, binding.reservation_ref)
}
pub(super) fn receipt_name(binding: &Binding) -> Result<String> {
    ensure!(
        vibe_lifecycle::process::is_valid_run_id(&binding.run_id),
        "invalid receipt run"
    );
    Ok(format!("{}.cancel.json", binding.run_id))
}
pub(super) fn proof(lease: &LifecycleLease, binding: &Binding, canonical: &[u8]) -> Result<()> {
    binding.physical_root.check_lease(lease)?;
    let c = &binding.coordinates;
    let row = SubmissionReservations::inspect_existing_key(lease, &c.namespace, &c.scope, &c.key)?
        .context("control C1 missing")?;
    ensure!(
        row.domain_run_id == binding.run_id
            && row.journal_admission == JournalAdmission::Ready
            && row.reservation_ref.as_deref() == Some(&binding.reservation_ref)
            && row.request_sha256 == binding.request_sha256
            && row.plan_sha256 == binding.plan_sha256,
        "control C1 changed"
    );
    SubmissionReservations::inspect_existing_request(
        lease,
        &c.namespace,
        &c.scope,
        &c.key,
        canonical,
    )?;
    Ok(())
}
// Fresh source and current header are first-dispatch authority, not conditions
// for inspecting an immutable historical outcome and stop receipt.
fn dispatch_proof(lease: &LifecycleLease, binding: &Binding, canonical: &[u8]) -> Result<()> {
    proof(lease, binding, canonical)?;
    recheck_selected_manifest_lock(&binding.coordinates.root, &binding.selected)?;
    recheck_script_source_witness(&binding.source)?;
    ProtectedJournal::header(
        lease,
        &binding.run_id,
        Some(&binding.started),
        Phase::Validate,
    )?;
    Ok(())
}

pub(super) fn domain(lease: &LifecycleLease, binding: &Binding) -> Result<RetainedLifecycleResult> {
    match inspect_retained_lifecycle_outcome(lease, &binding.run_id, &binding.reservation_ref)? {
        LifecycleOutcomeInspection::Validated {
            marker,
            report_bytes,
        } => Ok(RetainedLifecycleResult {
            marker,
            report_bytes,
        }),
        other => bail!("control domain recovery required: {other:?}"),
    }
}
fn stopped_marker(result: &RetainedLifecycleResult) -> bool {
    result.marker.outcome == MarkerOutcome::Failed
        && result.marker.effect_state == EffectState::None
        && matches!(&result.marker.report, ReportEvidence::Unavailable(row)
            if row.reason == ReportEvidenceUnavailableReason::NotProduced)
        && result.report_bytes.is_none()
}
fn reached_receipt(binding: &Binding, id: &str) -> Result<OperationStreamMessage> {
    reply(
        binding,
        &BodyControlRequest {
            action: BodyControlRequestAction::Cancel,
            after_sequence: "0".into(),
            request_id: id.to_owned(),
        },
        Status::ReachedSafeStop,
        Effect::NoEffects,
        Some(reference(binding)),
    )
}
fn read_receipt(lease: &LifecycleLease, binding: &Binding, id: &str) -> Result<()> {
    let project = lease.publication_journal_project();
    let directory = project.dir(&[".vibe", "lifecycle-outcomes"], false)?;
    let bytes = project
        .read_file_bounded_in(
            &directory,
            &receipt_name(binding)?,
            validation::MAX_FRAME_BYTES,
        )?
        .context("control receipt missing")?;
    let message: OperationStreamMessage = serde_json::from_slice(&bytes)?;
    let history = binding.history.lock().unwrap();
    validation::validate(&message, history.expected())?;
    drop(history);
    ensure!(
        message == reached_receipt(binding, id)?,
        "control receipt differs"
    );
    Ok(())
}
fn effect(result: &RetainedLifecycleResult) -> Effect {
    match result.marker.effect_state {
        EffectState::None => Effect::NoEffects,
        EffectState::Landed => Effect::Completed,
        EffectState::Uncertain => Effect::Uncertain,
    }
}
impl LiveOwner {
    /// Only a host holding this owner and its own active observer may control.
    pub(in super::super) fn control(
        &self,
        binding: &Binding,
        trusted: &HostCoordinates,
        observer: &mut Observer,
        message: &OperationStreamMessage,
    ) -> std::result::Result<OperationStreamMessage, ControlError> {
        {
            let history = binding.history.lock().unwrap();
            validation::validate(message, history.expected()).map_err(ControlError::Invalid)?;
        }
        let Body::ControlRequest(request) = &message.body else {
            return Err(ControlError::WrongBody);
        };
        self.check_stream_binding(binding, trusted)
            .map_err(|e| ControlError::ScopeUnavailable(e.to_string()))?;
        let (lease, canonical, stop_id, failed, pending, open, deferred, expected) = {
            let mut r = self.shared.record.lock().unwrap();
            if !Arc::ptr_eq(&observer.shared, &self.shared)
                || !observer
                    .subscription
                    .is_some_and(|id| r.subscriptions.contains(&id))
            {
                return Err(ControlError::Binding);
            }
            let detach = request.action == BodyControlRequestAction::Detach;
            let deferred = matches!(
                r.gate(),
                Gate::DeferredOpen
                    | Gate::DeferredRequested(_)
                    | Gate::DeferredStopping(_)
                    | Gate::DeferredStopped(..)
                    | Gate::DeferredFailed(_)
            );
            let expected = match r.gate() {
                Gate::DeferredStopped(_, result) => Some(result.clone()),
                _ => None,
            };
            let (id, failed, pending) = match r.gate() {
                Gate::DeferredRequested(id) => {
                    (Some(id.clone()), false, r.state == LiveState::Running)
                }
                Gate::DeferredStopping(id) | Gate::DeferredFailed(id) => {
                    (Some(id.clone()), true, false)
                }
                Gate::DeferredStopped(id, _) => (Some(id.clone()), false, false),
                Gate::Requested(id) => (Some(id.clone()), false, r.state == LiveState::Running),
                Gate::Stopping(id) => (Some(id.clone()), true, false),
                Gate::Stopped(id) => (Some(id.clone()), false, false),
                Gate::Failed(id) => (Some(id.clone()), true, false),
                _ => (None, false, false),
            };
            if !detach
                && r.state == LiveState::Running
                && id.as_ref().is_some_and(|id| id != &request.request_id)
            {
                return Err(ControlError::ConflictingRequest);
            }
            let open = r.state == LiveState::Running
                && matches!(r.gate(), Gate::Open | Gate::DeferredOpen);
            if !detach && open {
                *r.gate_mut() = if deferred {
                    Gate::DeferredRequested(request.request_id.clone())
                } else {
                    Gate::Requested(request.request_id.clone())
                };
            }
            if detach {
                r.subscriptions
                    .remove(&observer.subscription.take().unwrap());
            }
            (
                r.lease.clone(),
                r.canonical_request.clone(),
                // Only the original request can prove its cancellation receipt.
                // A new late request inspects terminal domain truth independently.
                id.filter(|id| id == &request.request_id),
                failed,
                pending,
                open,
                deferred,
                expected,
            )
        };
        let detach = request.action == BodyControlRequestAction::Detach;
        let active = lease.is_some();
        let snapshot = (|| -> Result<_> {
            let lease = match lease {
                Some(lease) => lease,
                None => Arc::new(LifecycleLease::acquire(&trusted.root)?),
            };
            proof(&lease, binding, &canonical)?;
            let result = domain(&lease, binding)?;
            if let Some(id) = stop_id.as_ref().filter(|_| !detach) {
                ensure!(!failed, "stop publication failed");
                if deferred {
                    ensure!(
                        expected.as_ref() == Some(&result.marker),
                        "deferred domain proof changed"
                    );
                    super::deferred_stop::read_receipt(&lease, binding, id)?;
                } else {
                    ensure!(stopped_marker(&result), "pre-dispatch stop not proven");
                    read_receipt(&lease, binding, id)?;
                }
            }
            Ok(result)
        })();
        let (status, effect, reference) = if detach {
            match snapshot {
                Ok(result) => (Status::Detached, effect(&result), Some(reference(binding))),
                Err(_) => (Status::Detached, Effect::Uncertain, None),
            }
        } else if open || pending {
            // Admission is memory only. The worker rechecks all proof before closing.
            (
                Status::Requested,
                if deferred {
                    Effect::Uncertain
                } else {
                    Effect::NoEffects
                },
                None,
            )
        } else {
            match snapshot {
                Ok(_) if stop_id.is_some() => (
                    Status::ReachedSafeStop,
                    if deferred {
                        Effect::Uncertain
                    } else {
                        Effect::NoEffects
                    },
                    Some(reference(binding)),
                ),
                Ok(result) if !active => (
                    Status::AlreadyTerminal,
                    effect(&result),
                    Some(reference(binding)),
                ),
                _ => (Status::Unavailable, Effect::Uncertain, None),
            }
        };
        reply(binding, request, status, effect, reference)
            .map_err(|e| ControlError::ScopeUnavailable(e.to_string()))
    }
}
impl ReadyObserver {
    /// The same Record mutex chooses request-wins versus dispatch-wins.
    /// No mutex is held during retained-domain or receipt I/O.
    pub(in super::super) fn close_gate(
        &self,
        lease: &LifecycleLease,
    ) -> Result<Option<RetainedLifecycleResult>> {
        let (binding, canonical) = {
            let r = self.shared.record.lock().unwrap();
            (
                r.binding.clone().context("gate lacks binding")?,
                r.canonical_request.clone(),
            )
        };
        // The Ready notification may have allowed source/header/C1 drift.
        dispatch_proof(lease, &binding, &canonical)?;
        let id = {
            let mut r = self.shared.record.lock().unwrap();
            let id = match r.gate() {
                Gate::Open => {
                    *r.gate_mut() = if r.deferred_stop {
                        Gate::DeferredOpen
                    } else {
                        Gate::Closed
                    };
                    return Ok(None);
                }
                Gate::Requested(id) => id.clone(),
                _ => bail!("pre-dispatch gate already closed"),
            };
            *r.gate_mut() = Gate::Stopping(id.clone());
            id
        };
        let outcome = (|| -> Result<_> {
            proof(lease, &binding, &canonical)?;
            let marker = create_lifecycle_outcome_marker(
                &binding.run_id,
                &binding.reservation_ref,
                MarkerOutcome::Failed,
                EffectState::None,
                LifecycleReportPayload::Unavailable(ReportEvidenceUnavailableReason::NotProduced),
            )?;
            let result = retained(
                lease,
                &binding.run_id,
                &binding.reservation_ref,
                &marker,
                None,
            )?;
            ensure!(stopped_marker(&result), "stop marker not read back");
            let message = reached_receipt(&binding, &id)?;
            let bytes = serde_json::to_vec(&message)?;
            ensure!(
                bytes.len() <= validation::MAX_FRAME_BYTES,
                "control receipt over cap"
            );
            let project = lease.publication_journal_project();
            let directory = project.dir(&[".vibe", "lifecycle-outcomes"], false)?;
            project
                .publish_new_in(&directory, &receipt_name(&binding)?, &bytes)
                .map_err(|e| e.into_report())?;
            // Publication failures are not repaired or promoted by a later query.
            proof(lease, &binding, &canonical)?;
            read_receipt(lease, &binding, &id)?;
            domain(lease, &binding).and_then(|fresh| {
                ensure!(stopped_marker(&fresh), "stop marker changed");
                Ok(fresh)
            })
        })();
        *self.shared.record.lock().unwrap().gate_mut() = if outcome.is_ok() {
            Gate::Stopped(id)
        } else {
            Gate::Failed(id)
        };
        outcome.map(Some)
    }
}
#[cfg(test)]
mod tests;
