//! Domain-owned deferred admission, separate from the pre-dispatch NoEffects gate.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM");

use super::*;
#[cfg(test)]
thread_local! {
    pub(super) static BEFORE_CLOSURE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}
use crate::machine_operation::stream_message as validation;
use control::{Gate, domain, proof, receipt_name, reference, reply};
use vibe_lifecycle::process::ObservedOutcome;
use vibe_wire::generated::machine::e1::operation_stream_message::*;

// Only the native System path supplies this bounded evidence summary. Output,
// spool metadata and fault details remain with their existing observation owners.
#[derive(Clone, Copy)]
pub(super) struct NativeEvidence {
    exit_observed: bool,
    fault_free: bool,
    streams: [NativeDrainEvidence; 2],
}
#[derive(Clone, Copy)]
struct NativeDrainEvidence {
    captured: bool,
    eof: bool,
    counters_complete: bool,
    fault_free: bool,
}
impl NativeEvidence {
    pub(super) fn from_system(outcome: &ObservedOutcome) -> Self {
        Self {
            exit_observed: outcome.exit_observed,
            fault_free: outcome.faults.is_empty(),
            streams: [&outcome.stdout, &outcome.stderr].map(|stream| NativeDrainEvidence {
                captured: stream.captured,
                eof: stream.eof,
                counters_complete: stream.counters_complete,
                fault_free: stream.faults.is_empty(),
            }),
        }
    }
}
// Delivery loss and prefix truncation do not negate complete pipe draining.
fn quiescent(evidence: &NativeEvidence) -> bool {
    evidence.exit_observed
        && evidence.fault_free
        && evidence.streams.iter().all(|stream| {
            stream.captured && stream.eof && stream.counters_complete && stream.fault_free
        })
}
fn receipt(binding: &Binding, id: &str) -> Result<OperationStreamMessage> {
    reply(
        binding,
        &BodyControlRequest {
            action: BodyControlRequestAction::Cancel,
            after_sequence: "0".into(),
            request_id: id.into(),
        },
        BodyControlReplyStatus::ReachedSafeStop,
        BodyControlReplyOperationEffectState::Uncertain,
        Some(reference(binding)),
    )
}
pub(super) fn read_receipt(lease: &LifecycleLease, binding: &Binding, id: &str) -> Result<()> {
    let project = lease.publication_journal_project();
    let directory = project.dir(&[".vibe", "lifecycle-outcomes"], false)?;
    let bytes = project
        .read_file_bounded_in(
            &directory,
            &receipt_name(binding)?,
            validation::MAX_FRAME_BYTES,
        )?
        .context("deferred receipt missing")?;
    let message: OperationStreamMessage = serde_json::from_slice(&bytes)?;
    let history = binding.history.lock().unwrap();
    validation::validate(&message, history.expected())?;
    drop(history);
    ensure!(message == receipt(binding, id)?, "deferred receipt differs");
    Ok(())
}
impl ReadyObserver {
    /// Called after the protected callback and after-effect hook, before any
    /// normal outcome publication. The mutex selects a single winner, then is
    /// released before domain I/O. Failure never reopens deferred admission.
    pub(in super::super) fn close_deferred_gate(&self) -> Result<Option<String>> {
        #[cfg(test)]
        BEFORE_CLOSURE.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let mut record = self.shared.record.lock().unwrap();
        match record.gate() {
            Gate::DeferredOpen => {
                *record.gate_mut() = Gate::Closed;
                Ok(None)
            }
            Gate::DeferredRequested(id) => {
                let id = id.clone();
                *record.gate_mut() = Gate::DeferredStopping(id.clone());
                Ok(Some(id))
            }
            Gate::Closed => Ok(None),
            _ => bail!("invalid deferred publication boundary"),
        }
    }
    pub(in super::super) fn retain_deferred_stop(
        &self,
        lease: &LifecycleLease,
        id: &str,
        report_bytes: &[u8],
    ) -> Result<RetainedLifecycleResult> {
        let (binding, canonical, safe) = {
            let record = self.shared.record.lock().unwrap();
            (
                record.binding.clone().context("deferred binding missing")?,
                record.canonical_request.clone(),
                record.provider_evidence.as_ref().is_some_and(quiescent),
            )
        };
        let outcome = (|| {
            ensure!(safe, "native exit and complete capture not proven");
            proof(lease, &binding, &canonical)?;
            let marker = create_lifecycle_outcome_marker(
                &binding.run_id,
                &binding.reservation_ref,
                MarkerOutcome::Uncertain,
                EffectState::Uncertain,
                LifecycleReportPayload::Available(report_bytes),
            )?;
            retained(
                lease,
                &binding.run_id,
                &binding.reservation_ref,
                &marker,
                Some(report_bytes),
            )
        })();
        let result = match outcome {
            Ok(result) => result,
            Err(error) => {
                *self.shared.record.lock().unwrap().gate_mut() = Gate::DeferredFailed(id.into());
                return Err(error);
            }
        };
        // A landed valid partial marker survives receipt failure. Never fall
        // back to Completed, overwrite a receipt, repair or dispatch again.
        let published = (|| -> Result<()> {
            let bytes = serde_json::to_vec(&receipt(&binding, id)?)?;
            ensure!(
                bytes.len() <= validation::MAX_FRAME_BYTES,
                "deferred receipt over cap"
            );
            let project = lease.publication_journal_project();
            let directory = project.dir(&[".vibe", "lifecycle-outcomes"], false)?;
            project
                .publish_new_in(&directory, &receipt_name(&binding)?, &bytes)
                .map_err(|error| error.into_report())?;
            proof(lease, &binding, &canonical)?;
            read_receipt(lease, &binding, id)?;
            ensure!(
                domain(lease, &binding)?.marker == result.marker,
                "deferred domain changed"
            );
            Ok(())
        })();
        *self.shared.record.lock().unwrap().gate_mut() = if published.is_ok() {
            Gate::DeferredStopped(id.into(), result.marker.clone())
        } else {
            Gate::DeferredFailed(id.into())
        };
        Ok(result)
    }
}
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
