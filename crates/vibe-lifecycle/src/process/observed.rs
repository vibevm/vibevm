//! Optional bounded observation on a dedicated blocking owner thread.
//! These values are library evidence, not a wire protocol or execution journal.
use super::{CaptureFile, ProcessOutput, ProcessSpec, STREAM_CAP, StreamMode, SystemProcessRunner};
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex, mpsc::SyncSender};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

pub const CAPTURE_CHUNK_CAP: usize = 2048;
const LOSS_RANGE_CAP: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOrigin {
    Stdout,
    Stderr,
}

/// Offset is local to this origin. Delivery means enqueued, not client-read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRecord {
    pub origin: CaptureOrigin,
    pub byte_offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ObservedCapture {
    pub post_exit_grace: Duration,
    pub spool_prefix: bool,
    pub records: Option<SyncSender<CaptureRecord>>,
}
impl Default for ObservedCapture {
    fn default() -> Self {
        Self {
            post_exit_grace: Duration::from_secs(1),
            spool_prefix: true,
            records: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFaultStage {
    Read,
    Counter,
    DrainTask,
    Stdin,
    Wait,
    Cleanup,
    OwnerPanic,
    SpoolCreate,
    SpoolWrite,
    SpoolRemove,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureFault {
    pub stage: CaptureFaultStage,
    pub origin: Option<CaptureOrigin>,
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureLossRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptureEvidence {
    pub captured: bool,
    pub drained_bytes: u64,
    pub retained_bytes: u64,
    /// Bytes enqueued to the optional observer; zero without an observer.
    pub delivered_bytes: u64,
    /// Failed enqueue bytes; zero without an observer.
    pub dropped_delivery_bytes: u64,
    /// First 32 disjoint loss ranges; further ranges are counted but omitted.
    pub delivery_loss_ranges: Vec<CaptureLossRange>,
    pub delivery_loss_ranges_truncated: bool,
    pub discarded_prefix_bytes: u64,
    pub counters_complete: bool,
    pub eof: bool,
    pub truncated: bool,
    pub spooled_bytes: u64,
    pub spool_path: Option<PathBuf>,
    pub spool_removed: bool,
    pub faults: Vec<CaptureFault>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedOutcome {
    pub output: ProcessOutput,
    /// False means wait/cleanup failed to establish the direct child's status.
    pub exit_observed: bool,
    pub stdout: CaptureEvidence,
    pub stderr: CaptureEvidence,
    pub faults: Vec<CaptureFault>,
}

/// All errors are pre-spawn refusals. Post-spawn faults stay in the outcome.
#[derive(Debug, thiserror::Error)]
pub enum ObservedRefusal {
    #[error("observed capture requires a dedicated thread outside a Tokio runtime")]
    NestedRuntime,
    #[error("post-exit grace must be between 1ms and 10s")]
    InvalidGrace,
    #[error("creating observed runtime failed: {0}")]
    Runtime(String),
    #[error("spawning observed child failed: {0}")]
    Spawn(String),
}

#[derive(Default)]
struct DrainState {
    prefix: Vec<u8>,
    evidence: CaptureEvidence,
}
type Shared = Arc<Mutex<DrainState>>;
fn state(shared: &Shared) -> std::sync::MutexGuard<'_, DrainState> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
fn fault(
    stage: CaptureFaultStage,
    origin: Option<CaptureOrigin>,
    reason: impl ToString,
) -> CaptureFault {
    CaptureFault {
        stage,
        origin,
        reason: reason.to_string(),
    }
}
fn add_counter(value: &mut u64, amount: u64) -> bool {
    match value.checked_add(amount) {
        Some(next) => {
            *value = next;
            true
        }
        None => false,
    }
}

async fn drain<R: AsyncRead + Unpin>(
    mut reader: R,
    shared: Shared,
    origin: CaptureOrigin,
    sender: Option<SyncSender<CaptureRecord>>,
) {
    let mut buffer = [0; CAPTURE_CHUNK_CAP];
    loop {
        let count = match reader.read(&mut buffer).await {
            Ok(0) => {
                state(&shared).evidence.eof = true;
                break;
            }
            Ok(count) => count,
            Err(error) => {
                state(&shared).evidence.faults.push(fault(
                    CaptureFaultStage::Read,
                    Some(origin),
                    error,
                ));
                break;
            }
        };
        let mut data = state(&shared);
        let offset = data.evidence.drained_bytes;
        let counted = add_counter(&mut data.evidence.drained_bytes, count as u64);
        let retained = count.min(STREAM_CAP - data.prefix.len());
        data.prefix.extend_from_slice(&buffer[..retained]);
        data.evidence.retained_bytes = data.prefix.len() as u64;
        let discarded_ok = add_counter(
            &mut data.evidence.discarded_prefix_bytes,
            (count - retained) as u64,
        );
        let mut delivery_ok = true;
        if let Some(sender) = &sender {
            // Never fabricate offsets once a counter has overflowed.
            if counted
                && data.evidence.counters_complete
                && sender
                    .try_send(CaptureRecord {
                        origin,
                        byte_offset: offset,
                        bytes: buffer[..count].to_vec(),
                    })
                    .is_ok()
            {
                delivery_ok = add_counter(&mut data.evidence.delivered_bytes, count as u64);
            } else {
                delivery_ok = add_counter(&mut data.evidence.dropped_delivery_bytes, count as u64);
                if counted && data.evidence.counters_complete {
                    let end = data.evidence.drained_bytes;
                    if let Some(last) = data
                        .evidence
                        .delivery_loss_ranges
                        .last_mut()
                        .filter(|last| last.end == offset)
                    {
                        last.end = end;
                    } else if data.evidence.delivery_loss_ranges.len() < LOSS_RANGE_CAP {
                        data.evidence
                            .delivery_loss_ranges
                            .push(CaptureLossRange { start: offset, end });
                    } else {
                        data.evidence.delivery_loss_ranges_truncated = true;
                    }
                } else {
                    data.evidence.delivery_loss_ranges_truncated = true;
                }
            }
        }
        if !(counted && discarded_ok && delivery_ok) && data.evidence.counters_complete {
            data.evidence.counters_complete = false;
            data.evidence.faults.push(fault(
                CaptureFaultStage::Counter,
                Some(origin),
                "u64 capture counter overflow; counting is partial",
            ));
        }
    }
}
fn stdio(mode: StreamMode) -> Stdio {
    match mode {
        StreamMode::Capture => Stdio::piped(),
        StreamMode::Inherit => Stdio::inherit(),
        StreamMode::Null => Stdio::null(),
    }
}

impl SystemProcessRunner {
    /// Caller owns a resolved spec and runs this on a dedicated blocking thread.
    /// Nested runtimes refuse before spawn rather than panicking. Observer loss
    /// never cancels the provider. At the grace deadline, capture handles close;
    /// this does not establish that descendants have stopped.
    pub fn run_observed(
        &self,
        spec: &ProcessSpec,
        capture: ObservedCapture,
    ) -> Result<ObservedOutcome, ObservedRefusal> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(ObservedRefusal::NestedRuntime);
        }
        if !(Duration::from_millis(1)..=Duration::from_secs(10)).contains(&capture.post_exit_grace)
        {
            return Err(ObservedRefusal::InvalidGrace);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ObservedRefusal::Runtime(e.to_string()))?;
        let stdout = Arc::new(Mutex::new(DrainState::default()));
        let stderr = Arc::new(Mutex::new(DrainState::default()));
        let mut child = {
            let _entered = runtime.enter();
            let mut command = tokio::process::Command::new(&spec.program);
            command
                .args(&spec.args)
                .current_dir(&spec.cwd)
                .env_clear()
                .envs(&spec.env)
                .stdin(if spec.stdin.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(stdio(spec.stdout))
                .stderr(stdio(spec.stderr))
                .kill_on_drop(true);
            let child = command
                .spawn()
                .map_err(|e| ObservedRefusal::Spawn(e.to_string()))?;
            child
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(run(spec, &capture, &mut child, &stdout, &stderr))
        })) {
            Ok(outcome) => outcome,
            Err(_) => {
                let mut faults = vec![fault(
                    CaptureFaultStage::OwnerPanic,
                    None,
                    "observed owner panicked after spawn",
                )];
                let status = runtime.block_on(async {
                    // Preserve an already available exit before cleanup.
                    if let Ok(Some(status)) = child.try_wait() {
                        return Some(status);
                    }
                    if let Err(error) = child.start_kill() {
                        faults.push(fault(CaptureFaultStage::Cleanup, None, error));
                    }
                    let status = match child.wait().await {
                        Ok(status) => Some(status),
                        Err(error) => {
                            faults.push(fault(CaptureFaultStage::Cleanup, None, error));
                            None
                        }
                    };
                    tokio::task::yield_now().await;
                    status
                });
                // Runtime shutdown drops all remaining drain futures/handles.
                drop(runtime);
                let (out, out_evidence) =
                    finish(&stdout, CaptureOrigin::Stdout, spec, &capture, false);
                let (err, err_evidence) =
                    finish(&stderr, CaptureOrigin::Stderr, spec, &capture, false);
                Ok(ObservedOutcome {
                    output: ProcessOutput {
                        code: status.and_then(|s| s.code()),
                        stdout: out,
                        stderr: err,
                        stdout_truncated: out_evidence.truncated,
                        stderr_truncated: err_evidence.truncated,
                    },
                    exit_observed: status.is_some(),
                    stdout: out_evidence,
                    stderr: err_evidence,
                    faults,
                })
            }
        }
    }
}

async fn run(
    spec: &ProcessSpec,
    capture: &ObservedCapture,
    child: &mut tokio::process::Child,
    stdout: &Shared,
    stderr: &Shared,
) -> Result<ObservedOutcome, ObservedRefusal> {
    let mut tasks = Vec::with_capacity(2);
    for (origin, shared) in [
        (CaptureOrigin::Stdout, &stdout),
        (CaptureOrigin::Stderr, &stderr),
    ] {
        let mode = if origin == CaptureOrigin::Stdout {
            spec.stdout
        } else {
            spec.stderr
        };
        state(shared).evidence.captured = mode == StreamMode::Capture;
        state(shared).evidence.counters_complete = true;
    }
    if let Some(pipe) = child.stdout.take() {
        tasks.push((
            CaptureOrigin::Stdout,
            stdout.clone(),
            tokio::spawn(drain(
                pipe,
                stdout.clone(),
                CaptureOrigin::Stdout,
                capture.records.clone(),
            )),
        ));
    }
    if let Some(pipe) = child.stderr.take() {
        tasks.push((
            CaptureOrigin::Stderr,
            stderr.clone(),
            tokio::spawn(drain(
                pipe,
                stderr.clone(),
                CaptureOrigin::Stderr,
                capture.records.clone(),
            )),
        ));
    }
    let mut faults = Vec::new();
    let mut stdin = child.stdin.take();
    let mut written_bytes = 0;
    let status = {
        let writer = async {
            if let Some(bytes) = &spec.stdin {
                match stdin.as_mut() {
                    Some(pipe) => {
                        while written_bytes < bytes.len() {
                            let count = pipe.write(&bytes[written_bytes..]).await?;
                            if count == 0 {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::WriteZero,
                                    "zero-length stdin write",
                                ));
                            }
                            written_bytes += count;
                        }
                    }
                    None => return Err(std::io::Error::other("requested stdin pipe missing")),
                }
            }
            stdin.take();
            Ok::<_, std::io::Error>(())
        };
        tokio::pin!(writer);
        tokio::select! {
            // Prefer actual exit when both exit and broken pipe are ready.
            biased;
            status = child.wait() => status,
            written = &mut writer => {
                if let Err(error) = written {
                    faults.push(fault(CaptureFaultStage::Stdin, None, error));
                    // Reap a child that already exited before choosing cleanup kill.
                    match child.try_wait() {
                        Ok(Some(status)) => Ok(status),
                        _ => {
                            if let Err(error) = child.start_kill() { faults.push(fault(CaptureFaultStage::Cleanup, None, error)); }
                            child.wait().await
                        }
                    }
                } else { child.wait().await }
            }
        }
    };
    drop(stdin); // Direct-child exit must not wait for a blocked stdin writer.
    if spec
        .stdin
        .as_ref()
        .is_some_and(|bytes| written_bytes < bytes.len())
        && !faults.iter().any(|f| f.stage == CaptureFaultStage::Stdin)
    {
        faults.push(fault(
            CaptureFaultStage::Stdin,
            None,
            format!("child exited before stdin completed; wrote {written_bytes} bytes"),
        ));
    }
    let status = match status {
        Ok(status) => Some(status),
        Err(error) => {
            faults.push(fault(CaptureFaultStage::Wait, None, error));
            if let Err(error) = child.start_kill() {
                faults.push(fault(CaptureFaultStage::Cleanup, None, error));
            }
            match child.wait().await {
                Ok(status) => Some(status),
                Err(error) => {
                    faults.push(fault(CaptureFaultStage::Cleanup, None, error));
                    None
                }
            }
        }
    };
    let deadline = tokio::time::Instant::now() + capture.post_exit_grace;
    for (origin, shared, mut task) in tasks {
        match tokio::time::timeout_at(deadline, &mut task).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => state(&shared).evidence.faults.push(fault(
                CaptureFaultStage::DrainTask,
                Some(origin),
                error,
            )),
            Err(_) => {
                task.abort();
                let _ = task.await;
            }
        }
    }
    let (stdout_bytes, stdout_evidence) = finish(
        &stdout,
        CaptureOrigin::Stdout,
        spec,
        &capture,
        status.as_ref().is_some_and(|s| s.success()),
    );
    let (stderr_bytes, stderr_evidence) = finish(
        &stderr,
        CaptureOrigin::Stderr,
        spec,
        &capture,
        status.as_ref().is_some_and(|s| s.success()),
    );
    Ok(ObservedOutcome {
        output: ProcessOutput {
            code: status.and_then(|s| s.code()),
            stdout: stdout_bytes,
            stderr: stderr_bytes,
            stdout_truncated: stdout_evidence.truncated,
            stderr_truncated: stderr_evidence.truncated,
        },
        exit_observed: status.is_some(),
        stdout: stdout_evidence,
        stderr: stderr_evidence,
        faults,
    })
}

fn finish(
    shared: &Shared,
    origin: CaptureOrigin,
    spec: &ProcessSpec,
    capture: &ObservedCapture,
    success: bool,
) -> (Vec<u8>, CaptureEvidence) {
    let mut data = state(shared);
    data.evidence.truncated = data.evidence.captured
        && (!data.evidence.eof
            || data.evidence.discarded_prefix_bytes > 0
            || !data.evidence.counters_complete);
    if capture.spool_prefix && data.evidence.captured {
        let genre = if origin == CaptureOrigin::Stdout {
            "stdout"
        } else {
            "stderr"
        };
        match super::create_unique_file(&spec.scratch, genre) {
            Err(error) => data.evidence.faults.push(fault(
                CaptureFaultStage::SpoolCreate,
                Some(origin),
                error,
            )),
            Ok((path, file)) => {
                let mut spool = CaptureFile { path, file };
                data.evidence.spool_path = Some(spool.path.clone());
                let mut offset = 0;
                while offset < data.prefix.len() {
                    match spool.file.write(&data.prefix[offset..]) {
                        Ok(0) => {
                            data.evidence.faults.push(fault(
                                CaptureFaultStage::SpoolWrite,
                                Some(origin),
                                "zero-length spool write",
                            ));
                            break;
                        }
                        Ok(count) => {
                            offset += count;
                            data.evidence.spooled_bytes = offset as u64;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) => {
                            data.evidence.faults.push(fault(
                                CaptureFaultStage::SpoolWrite,
                                Some(origin),
                                error,
                            ));
                            break;
                        }
                    }
                }
                drop(spool.file);
                if success && data.evidence.faults.is_empty() {
                    match std::fs::remove_file(&spool.path) {
                        Ok(()) => data.evidence.spool_removed = true,
                        Err(error) => data.evidence.faults.push(fault(
                            CaptureFaultStage::SpoolRemove,
                            Some(origin),
                            error,
                        )),
                    }
                }
            }
        }
    }
    (
        std::mem::take(&mut data.prefix),
        std::mem::take(&mut data.evidence),
    )
}

#[cfg(test)]
mod tests;
