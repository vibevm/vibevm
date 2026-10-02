//! Optional observation of the incumbent fixed owner, never a dispatch authority.
use super::*;
use vibe_lifecycle::process::{
    CaptureEvidence, CaptureOrigin, CaptureRecord, ObservedCapture, ProcessError, ProcessOutput,
    ProcessSpec, StreamMode,
};
use vibe_wire::generated::machine::e1::operation_stream_message::*;

fn publish(shared: &Arc<Shared>, payload: Payload) {
    let binding = shared.record.lock().unwrap().binding.clone();
    if let Some(binding) = binding {
        binding.history.lock().unwrap().publish(payload);
    }
}
fn diagnostic(shared: &Arc<Shared>, severity: PayloadDiagnosticSeverity, message: &str) {
    // An oversized incumbent message becomes an explicit history loss. Never
    // clone its unbounded contents into the observation queue.
    let message = if message.len() > 4096 {
        "x".repeat(4097)
    } else {
        message.to_owned()
    };
    publish(
        shared,
        Payload::Diagnostic(Box::new(PayloadDiagnostic { severity, message })),
    );
}

pub(in super::super) struct LiveRun {
    shared: Arc<Shared>,
    progress: Mutex<(u64, Option<u64>)>,
}
impl LiveRun {
    pub(in super::super) fn new(ready: &ReadyObserver) -> Self {
        Self {
            shared: ready.shared.clone(),
            progress: Mutex::new((0, None)),
        }
    }
    fn progress(&self, current: u64, total: Option<u64>) {
        publish(
            &self.shared,
            Payload::Progress(Box::new(PayloadProgress {
                current: current.to_string(),
                total: total.map(|value| value.to_string()),
                unit: PayloadProgressUnit::WorkUnits,
            })),
        );
    }
}
impl RunObserver for LiveRun {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Capture
    }
    fn binary_quiet(&self) -> bool {
        true
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn observe_plan(&self, plan: &RitualPlan, _: &RunMetadata, _: bool) -> Result<()> {
        // Borrow the incumbent canonical contributions; never plan a second run.
        let total = u64::try_from(plan.executions().len())
            .context("observed contribution count exceeds u64")?;
        *self.progress.lock().unwrap() = (0, Some(total));
        self.progress(0, Some(total));
        Ok(())
    }
    fn observe_phase_started(&self, phase: &str) {
        publish(
            &self.shared,
            Payload::Phase(Box::new(PayloadPhase {
                phase: phase.into(),
                transition: PayloadPhaseTransition::Started,
                status: None,
            })),
        );
    }
    fn observe_phase_finished(&self, phase: &str, status: &str) {
        publish(
            &self.shared,
            Payload::Phase(Box::new(PayloadPhase {
                phase: phase.into(),
                transition: PayloadPhaseTransition::Finished,
                status: Some(status.into()),
            })),
        );
    }
    fn observe_contribution_started(&self, _: &str, _: &str) {
        let (current, total) = *self.progress.lock().unwrap();
        self.progress(current, total);
    }
    fn observe_contribution_terminal(&self, report: &LifecycleContributionReport) {
        let mut progress = self.progress.lock().unwrap();
        if let Some(next) = progress.0.checked_add(1)
            && progress.1.is_none_or(|total| next <= total)
        {
            progress.0 = next;
        }
        self.progress(progress.0, progress.1);
        drop(progress);
        if let Some(message) = report.message.as_deref() {
            diagnostic(
                &self.shared,
                if report.status == "fail" {
                    PayloadDiagnosticSeverity::Error
                } else {
                    PayloadDiagnosticSeverity::Info
                },
                message,
            );
        } else if report.status == "fail" {
            diagnostic(
                &self.shared,
                PayloadDiagnosticSeverity::Error,
                &report.status,
            );
        }
    }
    fn observe_contribution(&self, _: &LifecycleContributionReport) {}
    fn observe_untracked_failure(
        &self,
        _: &RunMetadata,
        _: &str,
        contributions: &[LifecycleContributionReport],
    ) -> Result<()> {
        for row in contributions {
            if let Some(message) = row.message.as_deref() {
                diagnostic(&self.shared, PayloadDiagnosticSeverity::Error, message);
            }
        }
        Ok(())
    }
}

enum Provider<'a> {
    System(std::marker::PhantomData<&'a ()>),
    #[cfg(test)]
    Fixture(&'a dyn ProcessRunner),
}
pub(super) struct ObservedProcess<'a> {
    provider: Provider<'a>,
    shared: &'a Arc<Shared>,
}
impl<'a> ObservedProcess<'a> {
    pub(super) fn system(shared: &'a Arc<Shared>) -> Self {
        Self {
            provider: Provider::System(std::marker::PhantomData),
            shared,
        }
    }
    #[cfg(test)]
    pub(super) fn fixture(process: &'a dyn ProcessRunner, shared: &'a Arc<Shared>) -> Self {
        Self {
            provider: Provider::Fixture(process),
            shared,
        }
    }
}
fn origin_index(origin: CaptureOrigin) -> usize {
    match origin {
        CaptureOrigin::Stdout => 0,
        CaptureOrigin::Stderr => 1,
    }
}
fn raw(shared: &Arc<Shared>, origin: CaptureOrigin, offset: u64, bytes: &[u8], truncated: bool) {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut hex_bytes = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex_bytes.push(HEX[(byte >> 4) as usize] as char);
        hex_bytes.push(HEX[(byte & 15) as usize] as char);
    }
    publish(
        shared,
        Payload::RawOutput(Box::new(PayloadRawOutput {
            origin: match origin {
                CaptureOrigin::Stdout => PayloadRawOutputOrigin::Stdout,
                CaptureOrigin::Stderr => PayloadRawOutputOrigin::Stderr,
            },
            byte_offset: offset.to_string(),
            hex_bytes,
            byte_count: bytes.len() as u32,
            truncated,
        })),
    );
}
fn receive(shared: Arc<Shared>, records: std::sync::mpsc::Receiver<CaptureRecord>) -> [u64; 2] {
    let mut next = [0; 2];
    for record in records {
        let index = origin_index(record.origin);
        if record.byte_offset != next[index] {
            raw(&shared, record.origin, next[index], &[], true);
        }
        raw(
            &shared,
            record.origin,
            record.byte_offset,
            &record.bytes,
            false,
        );
        next[index] = record
            .byte_offset
            .checked_add(record.bytes.len() as u64)
            .expect("capture producer checks byte counters");
    }
    next
}
// Closing all producer handles makes this join finite: receiver owns only the
// bounded history and short locks, never a transport writer or external callback.
struct ReceiverOwner(Option<std::thread::JoinHandle<[u64; 2]>>);
impl ReceiverOwner {
    fn finish(&mut self) -> Option<[u64; 2]> {
        self.0.take()?.join().ok()
    }
}
impl Drop for ReceiverOwner {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
fn summarize(shared: &Arc<Shared>, origin: CaptureOrigin, next: u64, evidence: &CaptureEvidence) {
    if evidence.captured
        && (next != evidence.drained_bytes
            || evidence.dropped_delivery_bytes > 0
            || evidence.truncated
            || !evidence.eof
            || !evidence.counters_complete
            || !evidence.faults.is_empty())
    {
        raw(shared, origin, next, &[], true);
    }
    if evidence.captured
        && (evidence.truncated
            || evidence.dropped_delivery_bytes > 0
            || !evidence.eof
            || !evidence.counters_complete)
    {
        diagnostic(
            shared,
            PayloadDiagnosticSeverity::Warning,
            &format!(
                "{origin:?} capture partial: next_delivered_offset={next}, drained={}, retained={}, delivery_lost={}, prefix_discarded={}, eof={}, counters_complete={}",
                evidence.drained_bytes,
                evidence.retained_bytes,
                evidence.dropped_delivery_bytes,
                evidence.discarded_prefix_bytes,
                evidence.eof,
                evidence.counters_complete
            ),
        );
    }
    for fault in &evidence.faults {
        diagnostic(shared, PayloadDiagnosticSeverity::Warning, &fault.reason);
    }
}
impl ProcessRunner for ObservedProcess<'_> {
    fn run(&self, spec: &ProcessSpec) -> std::result::Result<ProcessOutput, ProcessError> {
        let binding = self.shared.record.lock().unwrap().binding.clone();
        if let Some(binding) = binding {
            binding.history.lock().unwrap().state(
                stream::Observed::Running,
                stream::Effect::Uncertain,
                None,
            );
        }
        #[cfg(test)]
        if let Provider::Fixture(process) = self.provider {
            return process.run(spec);
        }
        let (sender, records) = std::sync::mpsc::sync_channel(32);
        let shared = self.shared.clone();
        let mut receiver = ReceiverOwner(Some(
            std::thread::Builder::new()
                .name("fixed-validate-raw".into())
                .spawn(move || receive(shared, records))
                .map_err(|error| ProcessError::Spawn {
                    program: spec.program.display().to_string(),
                    reason: error.to_string(),
                })?,
        ));
        // Live owner's worker is the dedicated blocking thread. This never runs
        // the legacy provider a second time or changes the owner-resolved spec.
        let outcome = SystemProcessRunner.run_observed(
            spec,
            ObservedCapture {
                records: Some(sender),
                ..Default::default()
            },
        );
        let next = receiver.finish();
        match outcome {
            Ok(outcome) => {
                if next.is_none() {
                    diagnostic(
                        self.shared,
                        PayloadDiagnosticSeverity::Warning,
                        "raw receiver panicked; delivery partial",
                    );
                }
                let next = next.unwrap_or([0; 2]);
                summarize(self.shared, CaptureOrigin::Stdout, next[0], &outcome.stdout);
                summarize(self.shared, CaptureOrigin::Stderr, next[1], &outcome.stderr);
                for fault in &outcome.faults {
                    diagnostic(
                        self.shared,
                        PayloadDiagnosticSeverity::Warning,
                        &fault.reason,
                    );
                }
                self.shared.record.lock().unwrap().provider_evidence =
                    Some(super::deferred_stop::NativeEvidence::from_system(&outcome));
                // Post-spawn faults retain the actual output and exit code.
                Ok(outcome.output)
            }
            Err(error) => Err(ProcessError::Spawn {
                program: spec.program.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }
}
#[cfg(test)]
mod tests;
