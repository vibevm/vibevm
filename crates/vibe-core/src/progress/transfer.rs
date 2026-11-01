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
mod journal;
mod limit;
mod sampler;
mod watch;

pub use journal::{Outcome, TransferEntry, TransferJournal};
pub use limit::TransferLimit;
pub use sampler::{Bucket, Sample};
pub use watch::{TransferEvent, TransferEventKind, TransferWatch, watch};

use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::{ProgressEventKind, ProgressTask, TransferEnd};
use sampler::Sampler;

/// The running count behind every transfer id of this process: the one
/// process-wide state the wrapper keeps, since an id must be unique on the
/// machine and the pid alone is not.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// How the bytes are known: read and counted by vibe, or reported by git.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub enum TransferKind {
    Http,
    Git,
}

/// What the bytes are for — the closed vocabulary of `PROP-068#purpose`.
///
/// ```
/// use vibe_core::progress::transfer::Purpose;
///
/// assert_eq!(Purpose::parse("native-input"), Some(Purpose::NativeInput));
/// assert_eq!(Purpose::ReleaseBundle.as_str(), "release-bundle");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    PackageSource,
    Archive,
    NativeInput,
    PackageClone,
    ReleaseBundle,
    IndexFile,
    Other,
}

impl Purpose {
    const ALL: [Self; 7] = [
        Self::PackageSource,
        Self::Archive,
        Self::NativeInput,
        Self::PackageClone,
        Self::ReleaseBundle,
        Self::IndexFile,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PackageSource => "package-source",
            Self::Archive => "archive",
            Self::NativeInput => "native-input",
            Self::PackageClone => "package-clone",
            Self::ReleaseBundle => "release-bundle",
            Self::IndexFile => "index-file",
            Self::Other => "other",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|purpose| purpose.as_str() == text)
    }
}

/// Where the bytes land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub enum Destination {
    Store,
    Cache,
    Work,
    Home,
}

/// `counted` when vibe counted the bytes it wrote, `reported` when it
/// relayed git's figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub enum Precision {
    Counted,
    Reported,
}

/// The identity and context of one transfer (`PROP-068#transfer`), plus
/// the command's resume line, which its history entry names only when it
/// was interrupted (`#pause`).
///
/// ```
/// use vibe_core::progress::transfer::{Destination, Purpose, TransferRecord};
///
/// let record = TransferRecord::http(Purpose::Archive, "https://example.test/a.zip", Destination::Cache)
///     .package("org.example/a@1.0.0")
///     .step("unpack")
///     .batch("20261101T100000.000Z");
/// assert!(record.id.starts_with(&format!("{}:", std::process::id())));
/// assert_eq!(record.step.as_deref(), Some("unpack"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#transfer")]
pub struct TransferRecord {
    pub id: String,
    pub batch: String,
    pub kind: TransferKind,
    pub url: String,
    pub purpose: Purpose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    pub destination: Destination,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    pub started: String,
    pub pid: u32,
    pub command: String,
    pub precision: Precision,
    pub limited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_bps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remapped_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continues: Option<String>,
}

impl TransferRecord {
    /// A counted HTTP transfer from `url`, whose credentials, query and
    /// fragment are dropped here, before anything records it.
    pub fn http(purpose: Purpose, url: &str, destination: Destination) -> Self {
        let pid = std::process::id();
        Self {
            id: format!("{pid}:{}", NEXT.fetch_add(1, Ordering::Relaxed)),
            batch: pid.to_string(),
            kind: TransferKind::Http,
            url: safe_url(url),
            purpose,
            package: None,
            step: None,
            destination,
            total: None,
            started: clock::format_millis(clock::unix_millis()),
            pid,
            command: String::new(),
            precision: Precision::Counted,
            limited: false,
            limit_bps: None,
            remapped_from: None,
            resume: None,
            continues: None,
        }
    }

    pub fn package(self, key: impl Into<String>) -> Self {
        Self {
            package: Some(key.into()),
            ..self
        }
    }

    pub fn step(self, id: impl Into<String>) -> Self {
        Self {
            step: Some(id.into()),
            ..self
        }
    }

    /// The run the command belongs to; the pid when there is none.
    pub fn batch(self, batch: impl Into<String>) -> Self {
        Self {
            batch: batch.into(),
            ..self
        }
    }

    pub fn total(self, total: Option<u64>) -> Self {
        Self { total, ..self }
    }

    /// The URL the manifest named, when a remap row fetched another.
    pub fn remapped_from(self, declared: &str) -> Self {
        Self {
            remapped_from: Some(safe_url(declared)),
            ..self
        }
    }

    /// The command line that resumes the command if it is interrupted.
    pub fn resume(self, line: impl Into<String>) -> Self {
        Self {
            resume: Some(line.into()),
            ..self
        }
    }

    /// `<pid>-<n>`: the id spelled for a file name.
    pub(crate) fn file_stem(&self) -> String {
        self.id.replace(':', "-")
    }

    /// The label of the transfer's task: the last part of the URL.
    fn label(&self) -> String {
        let name = self
            .url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or("data");
        format!("Downloading {name}")
    }
}

/// `user`, `password`, query and fragment removed — never a token in a
/// record, a journal or a progress line (`PROP-068#redaction`).
///
/// ```
/// use vibe_core::progress::transfer::safe_url;
///
/// assert_eq!(
///     safe_url("https://name:secret@example.test/file?token=secret#fragment"),
///     "https://example.test/file"
/// );
/// assert_eq!(safe_url("not a url"), "remote endpoint");
/// ```
pub fn safe_url(raw: &str) -> String {
    let Ok(mut url) = url::Url::parse(raw) else {
        return "remote endpoint".to_owned();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

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
    pub fn begin(&self, mut record: TransferRecord, task: Option<&ProgressTask>) -> Transfer {
        record.command = self.0.command.clone();
        Transfer::open(record, task, self.0.journal.clone(), self.0.limit.clone())
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
        Self::open(record, task, journal.cloned(), None)
    }

    fn open(
        mut record: TransferRecord,
        task: Option<&ProgressTask>,
        journal: Option<TransferJournal>,
        limit: Option<TransferLimit>,
    ) -> Self {
        let now = clock::unix_millis();
        record.started = clock::format_millis(now);
        if let Some(limit) = &limit {
            record.limited = true;
            record.limit_bps = Some(limit.bytes_per_second());
        }
        if let Some(journal) = &journal {
            record.continues = journal.predecessor(&record);
        }
        let task = task.map(|parent| {
            parent
                .progress()
                .task_transferring(record.label(), record.clone())
        });
        let mut transfer = Self {
            sampler: Sampler::new(now),
            record,
            task,
            journal,
            limit,
            started: Instant::now(),
            bytes: 0,
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

    /// Count `bytes` that just arrived, after they were written.
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

    fn sample(&mut self, since: Duration) {
        let sample = self.sampler.take(since, self.bytes);
        if let Some(state) = self.task.as_ref().and_then(|task| task.state.as_ref())
            && state.is_active()
        {
            state.emit(ProgressEventKind::Progress {
                completed: sample.bytes,
                total: self.record.total,
                unit: "bytes".to_owned(),
                transfer: Some(self.record.id.clone()),
                rate_bps: Some(sample.rate_bps),
            });
        }
    }

    fn entry(&self) -> TransferEntry {
        TransferEntry {
            bytes: self.bytes,
            peak_bps: self.sampler.peak(),
            samples: self.sampler.samples(),
            ..TransferEntry::active(self.record.clone())
        }
    }

    /// The transfer arrived whole.
    pub fn finish(mut self) {
        self.end(Outcome::Finished, None);
    }

    /// The transfer stopped on `message`.
    pub fn fail(mut self, message: impl Into<String>) {
        self.end(Outcome::Failed, Some(message.into()));
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
/// next one (or ends the transfer), which is after it wrote them: a reader
/// of the journal never sees a count ahead of the file (`#sampling`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-068#sampling")]
pub struct TransferReader<R> {
    transfer: Transfer,
    inner: R,
    pending: u64,
}

impl<R> TransferReader<R> {
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
