//! The feed behind `vibe transfers watch` (`PROP-068#verbs`): one
//! `started` per transfer already active when the watch connects, then,
//! read from the journal every tick, one `sample` per sample of every
//! active transfer and one line per change of the active set. It reads
//! files only: it never starts or stops a transfer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#verbs");

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use super::TransferRecord;
use super::clock::{format_millis, unix_millis};
use super::journal::{Outcome, TransferEntry, TransferJournal};

/// What one watch line says happened.
///
/// ```
/// use vibe_core::progress::transfer::TransferEventKind;
///
/// assert_eq!(TransferEventKind::Sample.as_str(), "sample");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferEventKind {
    Started,
    Sample,
    Finished,
    Failed,
    Interrupted,
}

impl TransferEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Sample => "sample",
            Self::Finished => "finished",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }
}

/// One watch line: `bytes` is cumulative in every line and `total` is the
/// record's; `record` rides on `started`, the totals on the three ends.
///
/// ```
/// use vibe_core::progress::transfer::{TransferEvent, TransferEventKind};
///
/// let event = TransferEvent::bare(TransferEventKind::Sample, "7:1", "7");
/// assert!(event.record.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferEvent {
    pub at: String,
    pub event: TransferEventKind,
    pub id: String,
    pub batch: String,
    pub bytes: Option<u64>,
    pub total: Option<u64>,
    pub rate_bps: Option<u64>,
    pub peak_bps: Option<u64>,
    pub record: Option<TransferRecord>,
    pub message: Option<String>,
}

impl TransferEvent {
    pub fn bare(event: TransferEventKind, id: &str, batch: &str) -> Self {
        Self {
            at: format_millis(unix_millis()),
            event,
            id: id.to_owned(),
            batch: batch.to_owned(),
            bytes: None,
            total: None,
            rate_bps: None,
            peak_bps: None,
            record: None,
            message: None,
        }
    }

    /// `started` with the record and the figures as of now.
    fn started(entry: &TransferEntry) -> Self {
        let last = entry.samples.last();
        Self {
            bytes: Some(entry.bytes),
            total: entry.record.total,
            rate_bps: Some(last.map_or(0, |sample| sample.rate_bps)),
            peak_bps: Some(entry.peak_bps),
            record: Some(entry.record.clone()),
            ..Self::bare(
                TransferEventKind::Started,
                &entry.record.id,
                &entry.record.batch,
            )
        }
    }

    /// The end of a transfer, from its history line.
    fn ended(entry: &TransferEntry) -> Self {
        let kind = match entry.outcome {
            Some(Outcome::Finished) => TransferEventKind::Finished,
            Some(Outcome::Failed) => TransferEventKind::Failed,
            _ => TransferEventKind::Interrupted,
        };
        Self {
            bytes: Some(entry.bytes),
            total: entry.record.total,
            peak_bps: Some(entry.peak_bps),
            message: entry.message.clone(),
            ..Self::bare(kind, &entry.record.id, &entry.record.batch)
        }
    }
}

/// The state of one watch between two reads of the journal.
///
/// ```
/// use vibe_core::progress::transfer::{TransferJournal, TransferWatch};
///
/// let home = tempfile::tempdir().unwrap();
/// let journal = TransferJournal::new(home.path());
/// let (mut watch, opening) = TransferWatch::open(&journal, &|_| false);
/// assert!(opening.is_empty(), "nothing was running");
/// assert!(watch.poll(&|_| false).is_empty());
/// ```
pub struct TransferWatch<'a> {
    journal: &'a TransferJournal,
    /// Active transfers by id: the `(at, bytes)` of the last sample
    /// emitted — both, since two samples can share a millisecond — and the
    /// batch.
    active: BTreeMap<String, ((String, u64), String)>,
    /// Ids whose history line was already read.
    ended: BTreeSet<String>,
    revision: u64,
}

impl<'a> TransferWatch<'a> {
    /// Connect: one `started` line for every transfer already active.
    pub fn open(
        journal: &'a TransferJournal,
        alive: &dyn Fn(u32) -> bool,
    ) -> (Self, Vec<TransferEvent>) {
        let live = journal.sweep(alive);
        let mut watch = Self {
            journal,
            active: BTreeMap::new(),
            ended: journal.history().into_iter().map(|e| e.record.id).collect(),
            revision: journal.revision(),
        };
        let events = live.iter().map(|entry| watch.track(entry)).collect();
        (watch, events)
    }

    fn track(&mut self, entry: &TransferEntry) -> TransferEvent {
        let last = entry
            .samples
            .last()
            .map(|s| (s.at.clone(), s.bytes))
            .unwrap_or_default();
        self.active
            .insert(entry.record.id.clone(), (last, entry.record.batch.clone()));
        TransferEvent::started(entry)
    }

    /// Everything that happened since the previous read.
    pub fn poll(&mut self, alive: &dyn Fn(u32) -> bool) -> Vec<TransferEvent> {
        let live = self.journal.sweep(alive);
        let mut events = Vec::new();
        for entry in &live {
            let id = &entry.record.id;
            let Some((last, _)) = self.active.get_mut(id) else {
                events.push(self.track(entry));
                continue;
            };
            let since = last.clone();
            for sample in entry
                .samples
                .iter()
                .filter(|s| (s.at.as_str(), s.bytes) > (since.0.as_str(), since.1))
            {
                events.push(TransferEvent {
                    at: sample.at.clone(),
                    bytes: Some(sample.bytes),
                    total: entry.record.total,
                    rate_bps: Some(sample.rate_bps),
                    peak_bps: Some(entry.peak_bps),
                    ..TransferEvent::bare(TransferEventKind::Sample, id, &entry.record.batch)
                });
                *last = (sample.at.clone(), sample.bytes);
            }
        }
        let present: BTreeSet<&String> = live.iter().map(|entry| &entry.record.id).collect();
        let gone: Vec<String> = self
            .active
            .keys()
            .filter(|id| !present.contains(id))
            .cloned()
            .collect();
        let revision = self.journal.revision();
        if revision != self.revision || !gone.is_empty() {
            self.revision = revision;
            for entry in self.journal.history() {
                if !self.ended.insert(entry.record.id.clone()) {
                    continue;
                }
                // A transfer that began and ended between two reads still
                // gets its `started` line before its end.
                if self.active.remove(&entry.record.id).is_none() {
                    events.push(TransferEvent::started(&entry));
                }
                events.push(TransferEvent::ended(&entry));
            }
        }
        // Gone without a history line: the end was not written, which is
        // what an interrupted transfer looks like from here.
        for id in gone {
            if let Some((_, batch)) = self.active.remove(&id) {
                events.push(TransferEvent::bare(
                    TransferEventKind::Interrupted,
                    &id,
                    &batch,
                ));
            }
        }
        events
    }
}

/// Hand every watch line to `emit` — the opening ones first — until
/// `stop` answers true or `emit` refuses one (its reader went away).
///
/// ```
/// use std::time::Duration;
/// use vibe_core::progress::transfer::{TransferJournal, watch};
///
/// let home = tempfile::tempdir().unwrap();
/// let journal = TransferJournal::new(home.path());
/// let mut ticks = 0;
/// let mut seen = 0;
/// watch(&journal, &|_| false, Duration::ZERO, &mut || { ticks += 1; ticks > 2 }, &mut |_| { seen += 1; true });
/// assert_eq!(seen, 0);
/// ```
pub fn watch(
    journal: &TransferJournal,
    alive: &dyn Fn(u32) -> bool,
    tick: Duration,
    stop: &mut dyn FnMut() -> bool,
    emit: &mut dyn FnMut(&TransferEvent) -> bool,
) {
    let (mut state, opening) = TransferWatch::open(journal, alive);
    for event in &opening {
        if !emit(event) {
            return;
        }
    }
    while !stop() {
        std::thread::sleep(tick);
        for event in state.poll(alive) {
            if !emit(&event) {
                return;
            }
        }
    }
}
