//! The one mutable run: dense counters, the event publication law, and the
//! index model every write projects.
//!
//! Every scope of a lifecycle run shares exactly one of these behind one
//! mutex, because the two numbers the epoch legislates are global facts, not
//! per-artifact ones: the event `sequence` is dense across the WHOLE run, and
//! the `(scope, pass)` invocation ordinal must stay dense while two artifact
//! compilations interleave. Two states would mint two zeroes.
//!
//! The publication law is deliberately ordered so that no half-truth can be
//! written down. The referenced scope must still be `pending` before anything
//! is allocated or published — a sink held past its scope's terminal word
//! records nothing at all. Counters are then allocated and CHECKED, the
//! generated members are copied rather than re-derived, the snapshot is
//! published before it is named (a filename in the index always refers to a
//! file that landed), the aggregate table is rebuilt through the wire cell the
//! validator itself compares against, and the whole index is validated before
//! every atomic update. If that validation ever refuses, the event is rolled
//! back rather than written: an index that stops validating would poison every
//! later update.
//!
//! **Bytes on disk are always charged.** A rolled-back event and a publication
//! that failed *after* the irreversible step can both leave a real file behind.
//! Each one is charged to the run's budget and its name reserved against
//! reuse, then reported as residue — so repeated faults can never put more than
//! the budget on disk, and no later event can retry a name that is taken.
//!
//! Nothing here can fail a compile. Every refusal on this path becomes a
//! [`TraceWarning`] or a `snapshot-failed` event, and the compiler's own result
//! is untouched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::collections::{BTreeMap, BTreeSet};

use vibe_safefs::LockGuard;
use vibe_spec::{PassTraceEvent, SnapshotDecision};
use vibe_wire::behaviour::compiler_trace_index::{SnapshotName, build_aggregates};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    CompilerTraceIndex, PassEvent, PassStatus, RunStatus, Scope, ScopeStatus, Timestamp,
};

use super::sequence::NextSequence;
use super::store::{IndexUpdate, TraceStore};
use super::{RunOutcome, ScopeDescriptor, TraceError, TraceLimits, TraceWarning, bounded};

/// One snapshot that is physically on disk under a known name.
struct Landed {
    name: String,
    bytes: u64,
}

/// A snapshot publication that produced no name the index may carry.
struct Refused {
    reason: String,
    /// Bytes that nevertheless reached the disk and must be charged.
    landed: Option<Landed>,
}

/// The whole mutable state of one trace run.
#[derive(Debug)]
pub(super) struct RunState {
    /// The cooperative project lock. Intentionally live and never read: every
    /// `TraceRun`/`TraceScope` clone shares this state, so the guard is
    /// released exactly when the last of them drops — which is the whole
    /// serialization contract.
    _lock: LockGuard,
    store: TraceStore,
    index: CompilerTraceIndex,
    next_sequence: NextSequence,
    ordinals: BTreeMap<(String, String), u32>,
    published: BTreeSet<String>,
    spent: u64,
    limits: TraceLimits,
    warnings: Vec<TraceWarning>,
    finalised: bool,
}

impl RunState {
    /// Adopt an index — fresh or reopened — and restore every counter FROM it,
    /// so a resumed run continues the same numbering rather than starting a
    /// second one.
    pub(super) fn adopt(
        lock: LockGuard,
        store: TraceStore,
        index: CompilerTraceIndex,
        spent: u64,
        limits: TraceLimits,
        warnings: Vec<TraceWarning>,
    ) -> Self {
        let mut ordinals: BTreeMap<(String, String), u32> = BTreeMap::new();
        let mut published = BTreeSet::new();
        for event in &index.events {
            let key = (event.scope.clone(), event.pass.clone());
            let next = event.invocation.saturating_add(1);
            let slot = ordinals.entry(key).or_insert(0);
            *slot = (*slot).max(next);
            if let Some(name) = &event.snapshot {
                published.insert(name.clone());
            }
        }
        let next_sequence = NextSequence::restored_from(index.events.len());
        Self {
            _lock: lock,
            store,
            index,
            next_sequence,
            ordinals,
            published,
            spent,
            limits,
            warnings,
            finalised: false,
        }
    }

    pub(super) fn index(&self) -> &CompilerTraceIndex {
        &self.index
    }

    pub(super) const fn spent(&self) -> u64 {
        self.spent
    }

    pub(super) const fn finalised(&self) -> bool {
        self.finalised
    }

    /// Whether this run has already published everything its budget allows.
    ///
    /// Asked BEFORE any encode clock starts, so a stood-down run costs the
    /// compiler nothing at all. The event that crosses the ceiling finishes
    /// atomically — the check is `reached`, not `would exceed` — and every
    /// later one stands down.
    pub(super) const fn budget_exhausted(&self) -> bool {
        self.spent >= self.limits.snapshot_budget_bytes
    }

    /// The pre-encode answer for one scope. A closed/finalised sink stands
    /// down exactly like an exhausted one: the resulting budget event is
    /// dropped by `record`, but the expensive encoder is never invoked first.
    pub(super) fn snapshot_decision(&self, scope_id: &str) -> SnapshotDecision {
        let pending = !self.finalised
            && self
                .index
                .scopes
                .iter()
                .any(|scope| scope.id == scope_id && scope.status == ScopeStatus::Pending);
        if pending && !self.budget_exhausted() {
            SnapshotDecision::Encode
        } else {
            SnapshotDecision::SkipBudget
        }
    }

    pub(super) fn take_warnings(&self) -> Vec<TraceWarning> {
        self.warnings.clone()
    }

    fn warn(&mut self, warning: TraceWarning) {
        self.warnings.push(warning);
    }

    /// The very first index of a fresh run, written so the directory is
    /// readable the moment it exists. Unlike every later update, ANY failure
    /// here propagates: a run whose index never landed is not a run a caller
    /// should keep feeding, and refusing lets it compile untraced instead.
    pub(super) fn open_index(&mut self) -> Result<(), String> {
        match self.store.write_index(&self.index) {
            IndexUpdate::Written(anomaly) => {
                self.note_anomaly(anomaly);
                Ok(())
            }
            IndexUpdate::Deferred(reason) | IndexUpdate::Refused(reason) => Err(reason),
        }
    }

    /// Keep a publication fault that the destination check then cleared. The
    /// outcome is sound and the fault is real; erasing it would hide a disk
    /// or filesystem problem behind a green run.
    fn note_anomaly(&mut self, anomaly: Option<String>) {
        if let Some(reason) = anomaly {
            self.warn(TraceWarning::IndexAnomaly { reason });
        }
    }

    /// An intermediate index update. `Err` means the epoch REFUSED the model —
    /// the caller must roll its change back. A pure I/O failure is `Ok` with a
    /// warning: the in-memory model is already the truth, the previous whole
    /// index is still readable, and the next update retries.
    fn publish_index(&mut self) -> Result<(), String> {
        match self.store.write_index(&self.index) {
            IndexUpdate::Written(anomaly) => {
                self.note_anomaly(anomaly);
                Ok(())
            }
            IndexUpdate::Deferred(reason) => {
                self.warn(TraceWarning::IndexWrite { reason });
                Ok(())
            }
            IndexUpdate::Refused(reason) => Err(reason),
        }
    }

    /// Declare one artifact scope, or reacquire an identical pending one.
    ///
    /// A reopened run may already carry this scope. It is reacquirable only
    /// while it is still `pending` and only under the EXACT same descriptor: a
    /// compiled, failed or skipped scope id is never silently reset, and a
    /// descriptor that differs is a different scope wearing a taken name.
    pub(super) fn declare_scope(&mut self, descriptor: &ScopeDescriptor) -> Result<(), TraceError> {
        if self.finalised {
            return Err(TraceError::Finalised);
        }
        if let Some(existing) = self.index.scopes.iter().find(|s| s.id == descriptor.id) {
            if existing.status != ScopeStatus::Pending {
                return Err(TraceError::ScopeAlreadyResolved {
                    id: bounded::preview(&descriptor.id),
                });
            }
            if !descriptor.matches(existing) {
                return Err(TraceError::ScopeConflict {
                    id: bounded::preview(&descriptor.id),
                });
            }
            return Ok(());
        }
        self.index.scopes.push(Scope {
            artifact: descriptor.artifact.clone(),
            id: descriptor.id.clone(),
            kind: descriptor.kind.clone(),
            label: descriptor.label.clone(),
            status: ScopeStatus::Pending,
            target: descriptor.target.clone(),
            failure: None,
            fingerprint: None,
        });
        match self.publish_index() {
            Ok(()) => Ok(()),
            Err(reason) => {
                self.index.scopes.pop();
                Err(TraceError::IndexRefused { reason })
            }
        }
    }

    /// Move one pending scope to its terminal word.
    ///
    /// A `failure` is a DIAGNOSTIC and is bounded to the epoch's cap. A
    /// `fingerprint` is an IDENTITY and is copied byte-for-byte: silently
    /// shortening one would mint a different, still-valid fingerprint that
    /// names nothing the compiler produced. An identity the epoch's scalar
    /// gate refuses is refused here too — through the validator itself, on the
    /// whole index, with the change rolled back.
    pub(super) fn resolve_scope(
        &mut self,
        id: &str,
        status: ScopeStatus,
        witness: &str,
    ) -> Result<(), TraceError> {
        if self.finalised {
            return Err(TraceError::Finalised);
        }
        let silent = status == ScopeStatus::Skipped;
        if silent && self.index.events.iter().any(|event| event.scope == id) {
            return Err(TraceError::SkipAfterEvents {
                id: bounded::preview(id),
            });
        }
        let Some(position) = self.index.scopes.iter().position(|s| s.id == id) else {
            return Err(TraceError::UnknownScope {
                id: bounded::preview(id),
            });
        };
        if self.index.scopes[position].status != ScopeStatus::Pending {
            return Err(TraceError::ScopeAlreadyResolved {
                id: bounded::preview(id),
            });
        }
        let restore = self.index.scopes[position].clone();
        let scope = &mut self.index.scopes[position];
        if status == ScopeStatus::Failed {
            scope.failure = Some(bounded::diagnostic(format_args!("{witness}")));
            scope.fingerprint = None;
        } else {
            scope.fingerprint = Some(witness.to_string());
            scope.failure = None;
        }
        scope.status = status;
        match self.publish_index() {
            Ok(()) => Ok(()),
            Err(reason) => {
                self.index.scopes[position] = restore;
                Err(TraceError::IndexRefused { reason })
            }
        }
    }

    /// Write the run's terminal word — the LAST index update of the run.
    ///
    /// `finalised` means DURABLE, not "asked for" — and durability is decided
    /// by the DISK, not by whether the publication call returned `Ok`.
    ///
    /// A terminal index the epoch refuses, and one whose bytes provably never
    /// reached the disk, are the same fact from a cold reader's side: the file
    /// still says `running`. Both restore the in-memory root to `running`,
    /// leave `finalised` false, and report why.
    ///
    /// A publication that failed AFTER its irreversible step is the opposite
    /// case: the terminal bytes may already be what a cold reader sees. The
    /// store re-reads the destination and compares it byte-for-byte, so such a
    /// run is finalised — carrying a warning that names the fault, because
    /// something did go wrong even though the outcome is sound.
    pub(super) fn finish(&mut self, outcome: &RunOutcome, finished: Timestamp) {
        if self.finalised {
            self.warn(TraceWarning::Dropped {
                reason: "the run was already finalised".to_string(),
            });
            return;
        }
        let restore = (self.index.status.clone(), self.index.failure.clone());
        match outcome {
            RunOutcome::Ok => {
                self.index.status = RunStatus::Ok;
                self.index.failure = None;
            }
            RunOutcome::Failed(reason) => {
                self.index.status = RunStatus::Failed;
                self.index.failure = Some(bounded::diagnostic(format_args!("{reason}")));
            }
        }
        self.index.finished = Some(finished);
        match self.store.write_index(&self.index) {
            IndexUpdate::Written(anomaly) => {
                self.note_anomaly(anomaly);
                self.finalised = true;
            }
            IndexUpdate::Deferred(reason) | IndexUpdate::Refused(reason) => {
                self.index.status = restore.0;
                self.index.failure = restore.1;
                self.index.finished = None;
                self.warn(TraceWarning::NotFinalised { reason });
            }
        }
    }

    /// The event publication law, in the order the architecture fixes it.
    pub(super) fn record(&mut self, scope_id: &str, event: &PassTraceEvent<'_>) {
        if self.finalised {
            self.warn(TraceWarning::Dropped {
                reason: "an event arrived after the run was finalised".to_string(),
            });
            return;
        }
        // 0. The scope must still be OPEN — checked before a counter is read
        //    and long before a byte is published. A sink outlives the scope it
        //    was taken from, and an event arriving after `complete`, `fail` or
        //    `skip` would either resurrect a closed scope or break
        //    `skipped-scope-is-silent`.
        let Some(scope) = self.index.scopes.iter().find(|s| s.id == scope_id).cloned() else {
            self.warn(TraceWarning::Dropped {
                reason: bounded::diagnostic(format_args!(
                    "no scope `{}` was declared",
                    bounded::preview(scope_id)
                )),
            });
            return;
        };
        if scope.status != ScopeStatus::Pending {
            self.warn(TraceWarning::Dropped {
                reason: bounded::diagnostic(format_args!(
                    "scope `{}` already reached `{:?}`; a closed scope records nothing",
                    bounded::preview(scope_id),
                    scope.status
                )),
            });
            return;
        }

        // 1. Dense global sequence and dense `(scope, pass)` ordinal. The
        //    sequence is a two-arm value that says exhaustion out loud; the
        //    ordinal obeys the validator's stricter checked-advance law, so an
        //    event that would carry `u32::MAX` refuses rather than being
        //    written into an index the validator then rejects.
        let Some(sequence) = self.next_sequence.value() else {
            self.warn(TraceWarning::Dropped {
                reason: "the run has spent every sequence the epoch can address".to_string(),
            });
            return;
        };
        let key = (scope_id.to_string(), event.pass().to_string());
        let invocation = self.ordinals.get(&key).copied().unwrap_or(0);
        let Some(after_invocation) = invocation.checked_add(1) else {
            self.warn(TraceWarning::Dropped {
                reason: "this (scope, pass) has spent every invocation ordinal".to_string(),
            });
            return;
        };

        // 2. The generated shape/status/duration members, copied — never a
        //    second vocabulary and never a recomputed timing.
        let mut record = PassEvent {
            input_shape: event.input().clone(),
            invocation,
            output_shape: event.output().clone(),
            pass: event.pass().to_string(),
            scope: scope_id.to_string(),
            sequence,
            status: event.status().clone(),
            diagnostic: event.diagnostic().map(str::to_string),
            encode_micros: event.encode_duration().cloned(),
            pass_micros: event.pass_duration().cloned(),
            snapshot: None,
            verify_micros: event.verify_duration().cloned(),
        };

        // 3/4. An accepted output is published BEFORE it is named, so a
        //      filename in the index always refers to a file that landed. A
        //      refusal becomes `snapshot-failed` with the encode duration the
        //      compiler already spent and a bounded reason — and anything the
        //      failed publication nevertheless left on disk is charged and
        //      reserved before the event is even assembled.
        let mut landed: Option<Landed> = None;
        if record.status == PassStatus::Ok {
            if self.budget_exhausted() {
                // Two Send+Sync scopes may both receive `Encode` while the
                // budget is still below its ceiling. Encoding happens outside
                // this mutex; whichever event records first owns the one soft
                // crossing. A later racer is reported truthfully — it DID
                // encode, hence `snapshot-failed` with encode timing — but it
                // never publishes a second crossing payload.
                let reason = "snapshot was encoded concurrently, but another event exhausted \
                              the run budget before this one could publish"
                    .to_string();
                self.warn(TraceWarning::Snapshot {
                    sequence,
                    reason: reason.clone(),
                });
                record.status = PassStatus::SnapshotFailed;
                record.diagnostic = Some(reason);
            } else {
                match self.publish_snapshot(&scope, &record, event.snapshot()) {
                    Ok(published) => {
                        record.snapshot = Some(published.name.clone());
                        landed = Some(published);
                    }
                    Err(refused) => {
                        if let Some(orphan) = refused.landed {
                            self.charge_residue(orphan, "published by a refused publication");
                        }
                        self.warn(TraceWarning::Snapshot {
                            sequence,
                            reason: refused.reason.clone(),
                        });
                        record.status = PassStatus::SnapshotFailed;
                        record.snapshot = None;
                        record.diagnostic = Some(refused.reason);
                    }
                }
            }
        }

        // 5/6/7. The epoch matrix of every other status is preserved exactly
        //        as the compiler spelled it; the table is rebuilt through the
        //        very cell the validator compares against; and the whole index
        //        is validated before it is replaced.
        let restore = std::mem::take(&mut self.index.aggregates);
        self.index.events.push(record);
        match build_aggregates(&self.index.events) {
            Ok(rows) => self.index.aggregates = rows,
            Err(error) => {
                self.index.events.pop();
                self.index.aggregates = restore;
                self.rollback(
                    sequence,
                    landed,
                    &bounded::diagnostic(format_args!("{error}")),
                );
                return;
            }
        }
        match self.publish_index() {
            Ok(()) => {
                self.next_sequence = self.next_sequence.advanced();
                self.ordinals.insert(key, after_invocation);
                if let Some(published) = landed {
                    self.charge(published);
                }
            }
            Err(reason) => {
                self.index.events.pop();
                self.index.aggregates = restore;
                self.rollback(sequence, landed, &reason);
            }
        }
    }

    /// An event the index refused. The counters never advanced, so the
    /// sequence it would have spent is still unspent — but a snapshot may
    /// already be on disk, and an unreferenced file is charged, reserved and
    /// reported rather than deleted or forgotten.
    fn rollback(&mut self, sequence: u32, landed: Option<Landed>, reason: &str) {
        if let Some(published) = landed {
            self.charge_residue(published, "published for an event the index then refused");
        }
        self.warn(TraceWarning::Dropped {
            reason: bounded::diagnostic(format_args!(
                "event {sequence} was not recorded: {reason}"
            )),
        });
    }

    /// Count one landed file against the budget and reserve its name. The ONE
    /// place either happens, so a byte on disk is never uncounted and a name
    /// on disk is never reissued.
    fn charge(&mut self, landed: Landed) {
        self.spent = self.spent.saturating_add(landed.bytes);
        self.published.insert(landed.name);
    }

    /// The same, for a file no event will ever name.
    fn charge_residue(&mut self, landed: Landed, why: &str) {
        let path = self.store.run_path().join(&landed.name);
        self.charge(landed);
        self.warn(TraceWarning::Residue {
            path: bounded::path(&path),
            reason: why.to_string(),
        });
    }

    /// Choose the name through the wire builder, then publish the exact
    /// borrowed bytes create-new.
    fn publish_snapshot(
        &self,
        scope: &Scope,
        record: &PassEvent,
        bytes: Option<&[u8]>,
    ) -> Result<Landed, Refused> {
        let plain = |reason: String| Refused {
            reason,
            landed: None,
        };
        let Some(bytes) = bytes else {
            return Err(plain(
                "the accepted output carried no snapshot bytes".to_string(),
            ));
        };
        let name = SnapshotName {
            sequence: record.sequence,
            invocation: record.invocation,
            kind: &scope.kind,
            pass: &record.pass,
            label: &scope.label,
            artifact: &scope.artifact,
        };
        let Some(name) = name.within(self.store.filename_cap()) else {
            return Err(plain(bounded::diagnostic(format_args!(
                "no canonical snapshot name fits {} units of filename in `{}`",
                self.store.filename_cap(),
                self.store.run_path().display()
            ))));
        };
        if self.published.contains(&name) {
            return Err(plain(bounded::diagnostic(format_args!(
                "`{name}` is already taken by this run; a written snapshot is never overwritten"
            ))));
        }
        match self.store.publish_snapshot(&name, bytes) {
            Ok(()) => Ok(Landed {
                name,
                bytes: bytes.len() as u64,
            }),
            Err(refusal) => Err(Refused {
                reason: refusal.reason,
                landed: refusal.landed.map(|bytes| Landed { name, bytes }),
            }),
        }
    }
}
