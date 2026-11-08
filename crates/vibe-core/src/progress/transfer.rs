//! Transfers (`PROP-068`): the one wrapper every HTTP download goes
//! through. A [`Transfer`] counts the bytes of a [`Read`] it wraps, takes a
//! sample at the cadence of `#sampling`, and writes each sample to two
//! places at once — the progress stream of the running command, as a task
//! of its own, and the [`TransferJournal`] under the settings root, for
//! any tool on the machine — then writes the end to both.
//!
//! ```
//! use std::io::Read;
//! use vibe_core::progress::transfer::{Destination, Purpose, Transfer, TransferJournal, TransferRecord};
//!
//! let home = tempfile::tempdir().unwrap();
//! let journal = TransferJournal::new(home.path());
//! let record = TransferRecord::http(Purpose::Other, "https://user:pw@example.test/f?t=1", Destination::Cache);
//! assert_eq!(record.url, "https://example.test/f");
//! let mut reader = Transfer::begin(record, None, Some(&journal)).reader(&b"twelve bytes"[..]);
//! let mut body = Vec::new();
//! reader.read_to_end(&mut body).unwrap();
//! reader.finish();
//! let history = journal.history();
//! assert_eq!(history[0].bytes, 12);
//! assert!(journal.active().is_empty());
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#transfer");

mod clock;
mod disk;
mod journal;
mod limit;
mod record;
mod sampler;
mod watch;

use disk::DiskWrites;
pub use disk::{TransferIo, TransferWriter};

pub use journal::{Outcome, TransferEntry, TransferJournal};
pub use limit::TransferLimit;
pub use record::{
    Destination, Precision, Purpose, TransferKind, TransferRecord, safe_remote, safe_url,
};
pub use sampler::{Bucket, Sample};
pub use watch::{TransferEvent, TransferEventKind, TransferWatch, watch};

use std::io::{self, Read};
use std::sync::Arc;
use std::time::{Duration, Instant};

use specmark::spec;

use super::{Progress, ProgressEventKind, ProgressTask, TransferEnd};
use sampler::Sampler;

/// What the composition root gives every transfer of the invocation: the
/// journal, the shared rate limit and the command's verb. It travels with
/// the [`Progress`](super::Progress) handle, so the readers reach it
/// through the plumbing they already have.
///
/// ```
/// use vibe_core::progress::Progress;
/// use vibe_core::progress::transfer::{TransferLimit, Transfers};
///
/// let transfers = Transfers::new(None, TransferLimit::new(1_000_000), "install -g");
/// let progress = Progress::default().with_transfers(Some(transfers));
/// let task = progress.task("fetch");
/// assert_eq!(task.transfers().unwrap().command(), "install -g");
/// task.finish();
/// ```
#[derive(Debug, Clone)]
pub struct Transfers(Arc<Sinks>);

#[derive(Debug)]
struct Sinks {
    journal: Option<TransferJournal>,
    limit: Option<TransferLimit>,
    command: String,
    cancellation: crate::cancellation::CancellationToken,
}

impl Transfers {
    pub fn new(
        journal: Option<TransferJournal>,
        limit: Option<TransferLimit>,
        command: impl Into<String>,
    ) -> Self {
        Self(Arc::new(Sinks {
            journal,
            limit,
            command: command.into(),
            cancellation: Default::default(),
        }))
    }

    pub fn with_cancellation(&self, cancellation: crate::cancellation::CancellationToken) -> Self {
        Self(Arc::new(Sinks {
            journal: self.0.journal.clone(),
            limit: self.0.limit.clone(),
            command: self.0.command.clone(),
            cancellation,
        }))
    }
    pub fn journal(&self) -> Option<&TransferJournal> {
        self.0.journal.as_ref()
    }

    pub fn limit(&self) -> Option<&TransferLimit> {
        self.0.limit.as_ref()
    }

    pub fn command(&self) -> &str {
        &self.0.command
    }

    /// Begin `record` with this invocation's journal, limit and verb.
    pub fn begin(&self, record: TransferRecord, task: Option<&ProgressTask>) -> Transfer {
        self.begin_beneath(
            record,
            task.map(ProgressTask::progress).as_ref(),
            Instant::now(),
        )
    }

    /// Begin `record`, dated from `began`, as a child of whatever task
    /// `progress` was handed out by — the form for a reader that holds the
    /// observation's handle rather than its task.
    pub fn begin_beneath(
        &self,
        mut record: TransferRecord,
        progress: Option<&Progress>,
        began: Instant,
    ) -> Transfer {
        record.command = self.0.command.clone();
        let inherited = match progress {
            Some(progress) if progress.cancellation().has_control() => progress.clone(),
            _ => progress
                .cloned()
                .unwrap_or_default()
                .with_cancellation(self.0.cancellation.clone()),
        };
        let progress = Some(&inherited);
        Transfer::open(
            record,
            progress,
            self.0.journal.clone(),
            self.0.limit.clone(),
            began,
        )
    }
}

/// Begin `record` through whatever the task carries: the invocation's
/// [`Transfers`] when the composition root attached them, the progress
/// stream alone otherwise.
pub fn begin(record: TransferRecord, task: Option<&ProgressTask>) -> Transfer {
    match task.and_then(ProgressTask::transfers) {
        Some(transfers) => transfers.clone().begin(record, task),
        None => Transfer::begin(record, task, None),
    }
}

/// Begin `record` beneath `progress`, with the invocation's [`Transfers`]
/// that handle carries, or `fallback` when it carries none — a reader the
/// composition root reached by another road than the observation (the git
/// backend, shared by every registry of a resolver). The transfer is dated
/// from `began`, when the bytes were first asked for: git's transfer opens
/// only once git names a size, and dating it from then would draw the bytes
/// before it as one burst.
///
/// ```
/// use vibe_core::progress::Progress;
/// use vibe_core::progress::transfer::{self, Destination, Purpose, TransferJournal, TransferRecord, Transfers};
///
/// let home = tempfile::tempdir().unwrap();
/// let fallback = Transfers::new(Some(TransferJournal::new(home.path())), None, "cache add");
/// let record = TransferRecord::git(Purpose::PackageClone, "file:///srv/a.git", Destination::Store);
/// let began = std::time::Instant::now();
/// let mut transfer = transfer::begin_beneath(record, Some(&Progress::default()), Some(&fallback), began);
/// transfer.reported(2048, Some(2048));
/// transfer.written(4096);
/// transfer.finish();
/// let entry = &fallback.journal().unwrap().history()[0];
/// assert_eq!((entry.bytes, entry.written, entry.record.command.as_str()), (2048, Some(4096), "cache add"));
/// ```
pub fn begin_beneath(
    record: TransferRecord,
    progress: Option<&Progress>,
    fallback: Option<&Transfers>,
    began: Instant,
) -> Transfer {
    match progress.and_then(Progress::transfers).or(fallback) {
        Some(transfers) => transfers.begin_beneath(record, progress, began),
        None => Transfer::open(record, progress, None, None, began),
    }
}

/// One transfer in flight. Dropped without an end, it is recorded failed,
/// so an early return is never reported as a finished download.
#[must_use = "a transfer dropped without an end is recorded as failed"]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub struct Transfer {
    record: TransferRecord,
    task: Option<ProgressTask>,
    journal: Option<TransferJournal>,
    limit: Option<TransferLimit>,
    sampler: Sampler,
    started: Instant,
    bytes: u64,
    written: Option<u64>,
    disk_writes: Option<DiskWrites>,
    ended: bool,
}

impl Transfer {
    /// Begin `record`: a child task of `task` whose start carries the
    /// record, a first sample at zero bytes, and its active file.
    pub fn begin(
        record: TransferRecord,
        task: Option<&ProgressTask>,
        journal: Option<&TransferJournal>,
    ) -> Self {
        Self::open(
            record,
            task.map(ProgressTask::progress).as_ref(),
            journal.cloned(),
            None,
            Instant::now(),
        )
    }

    fn open(
        mut record: TransferRecord,
        parent: Option<&Progress>,
        journal: Option<TransferJournal>,
        limit: Option<TransferLimit>,
        began: Instant,
    ) -> Self {
        let now = clock::unix_millis().saturating_sub(began.elapsed().as_millis() as u64);
        record.started = clock::format_millis(now);
        // Git has no portable throttle a child honours (`#limit`): its
        // transfers are never limited, whatever the machine setting says.
        let limit = limit.filter(|_| record.kind == TransferKind::Http);
        if let Some(limit) = &limit {
            record.limited = true;
            record.limit_bps = Some(limit.bytes_per_second());
        }
        if let Some(journal) = &journal {
            record.continues = journal.predecessor(&record);
        }
        let task = parent.map(|parent| parent.task_transferring(record.label(), record.clone()));
        let mut transfer = Self {
            sampler: Sampler::new(now),
            record,
            task,
            journal,
            limit,
            started: began,
            bytes: 0,
            written: None,
            disk_writes: None,
            ended: false,
        };
        transfer.sample(Duration::ZERO);
        if let Some(journal) = &transfer.journal {
            journal.opened(&transfer.entry());
        }
        transfer
    }

    pub fn id(&self) -> &str {
        &self.record.id
    }

    pub fn record(&self) -> &TransferRecord {
        &self.record
    }

    /// The bytes counted so far.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Wrap `inner`: every byte read through it is counted and limited.
    pub fn reader<R: Read>(self, inner: R) -> TransferReader<R> {
        TransferReader {
            transfer: self,
            inner,
            pending: 0,
        }
    }

    /// Count bytes returned by the input reader. This is not evidence of a disk write.
    fn arrived(&mut self, bytes: u64) {
        if let Some(limit) = &self.limit {
            limit.take(bytes);
        }
        let since = self.started.elapsed();
        self.bytes += bytes;
        self.sampler.count(since, bytes);
        if self.sampler.due(since, self.bytes) {
            self.sample(since);
            if let Some(journal) = &self.journal {
                journal.sampled(&self.entry());
            }
        }
    }

    /// Relay a cumulative figure someone else counted — git's progress
    /// line (`#git`) — and, once it knows it, the total. Every report is a
    /// sample: git already reports at its own cadence, and smoothing it
    /// would show a figure git never said.
    pub fn reported(&mut self, bytes: u64, total: Option<u64>) {
        let grew = bytes > self.bytes;
        let learned = total.is_some() && total != self.record.total;
        if !grew && !learned {
            return;
        }
        if learned {
            self.record.total = total;
        }
        let since = self.started.elapsed();
        if grew {
            self.sampler.count(since, bytes - self.bytes);
            self.bytes = bytes;
        }
        self.sample(since);
        if let Some(journal) = &self.journal {
            journal.sampled(&self.entry());
        }
    }

    /// The size on disk the transfer left behind, for the journal's entry
    /// of a transfer whose bytes were reported rather than counted.
    pub fn written(&mut self, bytes: u64) {
        self.written = Some(bytes);
    }

    fn sample(&mut self, since: Duration) {
        let sample = self.sampler.take_observed(
            since,
            self.bytes,
            self.disk_writes.as_ref().and_then(DiskWrites::bytes),
        );
        if let Some(state) = self.task.as_ref().and_then(|task| task.state.as_ref())
            && state.is_active()
        {
            state.emit(ProgressEventKind::Progress {
                completed: sample.bytes,
                total: self.record.total,
                unit: "bytes".to_owned(),
                transfer: Some(self.record.id.clone()),
                rate_bps: Some(sample.rate_bps),
                disk_written_bytes: sample.disk_written_bytes,
            });
        }
    }

    fn entry(&self) -> TransferEntry {
        TransferEntry {
            bytes: self.bytes,
            peak_bps: self.sampler.peak(),
            samples: self.sampler.samples(),
            written: self.written,
            disk_written_bytes: self.sampler.disk_written_bytes(),
            ..TransferEntry::active(self.record.clone())
        }
    }

    /// The transfer arrived whole.
    pub fn finish(mut self) {
        self.end(Outcome::Finished, None);
    }

    /// The transfer stopped on `message`.
    pub fn fail(mut self, message: impl Into<String>) {
        let stopped = self
            .task
            .as_ref()
            .is_some_and(|task| task.progress().cancellation().acknowledged());
        self.end(
            if stopped {
                Outcome::Interrupted
            } else {
                Outcome::Failed
            },
            Some(message.into()),
        );
    }

    /// The command was interrupted while the transfer ran.
    pub fn interrupted(mut self) {
        self.end(Outcome::Interrupted, Some("interrupted".to_owned()));
    }

    fn end(&mut self, outcome: Outcome, message: Option<String>) {
        if std::mem::replace(&mut self.ended, true) {
            return;
        }
        let since = self.started.elapsed();
        self.sample(since);
        let figures = TransferEnd {
            id: self.record.id.clone(),
            bytes: self.bytes,
            total: self.record.total,
            peak_bps: self.sampler.peak(),
            disk_written_bytes: self.disk_writes.as_ref().and_then(DiskWrites::bytes),
        };
        if let Some(state) = self.task.as_ref().and_then(|task| task.state.as_ref()) {
            state.terminate(match (outcome, &message) {
                (Outcome::Finished, _) => ProgressEventKind::Finished {
                    transfer: Some(figures),
                },
                (_, message) => ProgressEventKind::Failed {
                    message: message.clone().unwrap_or_default(),
                    transfer: Some(figures),
                },
            });
        }
        if let Some(journal) = &self.journal {
            let mut entry = self.entry();
            entry.message = message;
            entry.close(
                outcome,
                since.as_millis() as u64,
                &self.sampler.series(since),
            );
            journal.ended(&entry);
        }
    }
}

impl Drop for Transfer {
    fn drop(&mut self) {
        self.end(
            Outcome::Failed,
            Some("stopped without an outcome".to_owned()),
        );
    }
}

/// A [`Read`] whose bytes a [`Transfer`] counts; ended by [`Self::finish`],
/// [`Self::fail`] or [`Self::interrupted`], or, dropped, as failed.
///
/// The bytes of one read are counted when the caller comes back for the
/// next one (or ends the transfer). This reader cannot prove filesystem writes;
/// use [`Self::with_disk_writer`] at an owned writer boundary for that metric.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#sampling")]
pub struct TransferReader<R> {
    transfer: Transfer,
    inner: R,
    pending: u64,
}

impl<R> TransferReader<R> {
    /// Bind an owned, unbuffered logical disk sink to this transfer. Only
    /// successful `Write` return values count; cleanup never subtracts them.
    /// The pair owns both halves, so its terminal methods consume the writer.
    /// Uninstrumented readers retain an absent disk counter.
    ///
    /// ```
    /// use std::io::{Read, Write};
    /// use vibe_core::progress::transfer::{Destination, Purpose, Transfer, TransferRecord};
    /// let file = tempfile::tempfile().unwrap();
    /// let transfer = Transfer::begin(TransferRecord::http(Purpose::Other,
    ///     "https://example.test/body", Destination::Cache), None, None);
    /// let mut io = transfer.reader(&b"abc"[..]).with_disk_writer(file);
    /// let (reader, writer) = io.parts();
    /// let mut bytes = [0; 3];
    /// reader.read_exact(&mut bytes).unwrap();
    /// writer.write_all(&bytes).unwrap();
    /// io.sync_all().unwrap();
    /// io.finish();
    /// ```
    pub fn with_disk_writer<W: io::Write>(mut self, writer: W) -> TransferIo<R, W> {
        let writes = DiskWrites::new();
        self.transfer.disk_writes = Some(writes.clone());
        self.transfer.sample(self.transfer.started.elapsed());
        if let Some(journal) = &self.transfer.journal {
            journal.sampled(&self.transfer.entry());
        }
        TransferIo::new(self, writer, writes)
    }

    pub fn transfer(&self) -> &Transfer {
        &self.transfer
    }

    fn settle(mut self) -> Transfer {
        let pending = std::mem::take(&mut self.pending);
        if pending > 0 {
            self.transfer.arrived(pending);
        }
        self.transfer
    }

    pub fn finish(self) {
        self.settle().finish();
    }

    pub fn fail(self, message: impl Into<String>) {
        self.settle().fail(message);
    }

    pub fn interrupted(self) {
        self.settle().interrupted();
    }
}

impl<R: Read> Read for TransferReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some(task) = &self.transfer.task {
            task.progress().checkpoint().map_err(io::Error::other)?;
        }
        let pending = std::mem::take(&mut self.pending);
        if pending > 0 {
            self.transfer.arrived(pending);
        }
        let read = self.inner.read(buffer)?;
        self.pending = read as u64;
        Ok(read)
    }
}

#[cfg(test)]
mod tests;
