//! The strict generated-type state store and its record-last transaction.
//!
//! Every byte this store moves goes through the capability-relative I/O cell
//! in `io.rs` — no-follow bounded reads and the staged atomic replace — and
//! every mutation is the ONE transaction in `commit`: build a candidate,
//! reconcile the continuation against the candidate's own slot debt, validate
//! and encode it, prove the bytes durable, and only then let them become this
//! store's state. The post-publication window (a publication that crossed the
//! rename boundary and then failed) is resolved in `recovery.rs`; a store
//! whose durable state stops being one it can describe is POISONED there and
//! refuses every further write.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_safefs::Project;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, ExecutionRecordStatus, LifecycleState, SlotContinuation,
};

use super::error::LifecycleStateError;
use super::io::{self, PublicationFailure};

/// Open current state, replace the whole-run header, preserve every old row,
/// and immediately checkpoint the initial record (even for an empty plan).
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML")]
pub struct LifecycleStateStore {
    /// The pinned workspace-root capability: every production read and write
    /// of the state file goes through it, never through an ambient path.
    pub(super) project: Project,
    /// The absolute display path of the state file, for diagnostics only —
    /// never opened with ambient authority.
    pub(super) path: PathBuf,
    /// The last PROVEN state: the generated value this store last made or
    /// last saw durable. Inspection exposes exactly this, never a live view
    /// of disk truth.
    pub(super) state: LifecycleState,
    /// The exact durable bytes `state` describes — their original bytes,
    /// never a reserialization guessed to be the file. This is one of the two
    /// byte strings the post-publication recovery compares disk against.
    /// `None` exactly while no state file has been proven durable.
    pub(super) durable: Option<Vec<u8>>,
    /// Why this store refuses every further mutation, once its durable state
    /// stopped being one it can describe. `None` until poisoned; inspection
    /// keeps working against the last proven state.
    pub(super) poisoned: Option<String>,
    /// The ordered payload-event target set this run WOULD need to rebuild
    /// itself, held in memory until a slot-scoped park actually exists.
    ///
    /// Never serialised. A continuation is meaningful exactly while a
    /// slot-scoped delegated row is live, so it becomes durable in the SAME
    /// write that checkpoints that row — never in a write of its own, which
    /// would leave a crash window holding a continuation nothing owes.
    pub(super) staged_continuation: Option<SlotContinuation>,
}

impl LifecycleStateStore {
    pub const FILE: &'static str = ".vibe/lifecycle.toml";

    /// Open current state, replace the whole-run header, preserve every old
    /// row, and immediately checkpoint the initial record (even for an empty
    /// plan).
    ///
    /// `workspace_root` must be an EXISTING ABSOLUTE path — the canonical
    /// workspace root. It is pinned into the capability cell on entry, so a
    /// relative, missing or unopenable root refuses as the typed
    /// [`LifecycleStateError::Root`] with a remedy naming the root; that is a
    /// root problem, never a state-cache problem.
    pub fn begin(
        workspace_root: &Path,
        requested: String,
        chain: Vec<String>,
        started: String,
        run_id: String,
        compile_trace: bool,
    ) -> Result<Self, LifecycleStateError> {
        let path = io::state_path(workspace_root);
        let project = io::open_project(workspace_root, &path)?;
        let prior = io::read_prior(&project, &path)?;
        let (prior_bytes, prior) = match prior {
            Some(prior) => (Some(prior.bytes), Some(prior.state)),
            None => (None, None),
        };
        // Adoption is identity, not similarity: the SAME run id, and one that
        // is a real identity rather than the empty untracked placeholder.
        let adopted = !run_id.is_empty()
            && prior
                .as_ref()
                .is_some_and(|state| state.run.run_id.as_deref() == Some(run_id.as_str()));
        let continuation = prior
            .as_ref()
            .and_then(|state| state.run.slot_continuation.clone());
        let mut execution = prior.map(|state| state.execution).unwrap_or_default();
        if !adopted {
            // Old success/freshness rows are preserved as always, but a fresh
            // run id may not retain the PREVIOUS run's parked work: those task
            // paths live under the other run's outbox directory and were
            // promised to that invocation. The files themselves stay as honest
            // orphans for a later bounded GC — nothing is deleted broadly here,
            // and no fresh run claims to supersede a park it does not own.
            execution.retain(|_, record| record.status != ExecutionRecordStatus::Delegated);
        }
        let mut store = Self {
            project,
            path,
            durable: prior_bytes,
            poisoned: None,
            staged_continuation: None,
            state: LifecycleState {
                execution,
                run: vibe_wire::generated::lifecycle_state::StateRun {
                    chain,
                    requested,
                    run_id: (!run_id.is_empty()).then_some(run_id),
                    // An adopted run inherits the slot continuation it owes;
                    // a fresh one starts with none, exactly as it starts with
                    // no delegated rows.
                    slot_continuation: adopted.then_some(continuation).flatten(),
                    started,
                    // The effective sticky activation, written exactly as the
                    // local writer convention spells a false-defaulted bool:
                    // omitted while false (byte-compatible with pre-R3.4
                    // files), `compile_trace = true` once a run traces.
                    compile_trace,
                },
                schema: 1,
            },
        };
        store.commit(|_| {})?;
        Ok(store)
    }

    /// Read current state WITHOUT writing a new header.
    ///
    /// `begin` is a mutation: it replaces the run header and rewrites the
    /// file. A caller that only needs to ask "what does this run still owe?"
    /// must not, as a side effect, overwrite the persisted chain with its own
    /// — that is how a clean-composed run's phases-only chain got replaced by
    /// the complete one. `Ok(None)` when no state exists yet.
    ///
    /// `workspace_root` must be an EXISTING ABSOLUTE path (see
    /// [`begin`](Self::begin)); a relative or missing root refuses as the
    /// typed [`LifecycleStateError::Root`], never as a state-cache problem.
    pub fn peek(workspace_root: &Path) -> Result<Option<LifecycleState>, LifecycleStateError> {
        io::read_prior_state(workspace_root)
    }

    /// One row of the last PROVEN state — see [`state`](Self::state): this is
    /// what the store last saw durable, never a live view of disk truth.
    #[must_use]
    pub fn prior(&self, key: &str) -> Option<&ExecutionRecord> {
        self.state.execution.get(key)
    }

    #[must_use]
    pub fn reusable(&self, key: &str, fingerprint: &str) -> bool {
        self.reusable_record(key, fingerprint).is_some()
    }

    #[must_use]
    pub fn reusable_record(&self, key: &str, fingerprint: &str) -> Option<&ExecutionRecord> {
        self.prior(key).filter(|record| {
            record.fingerprint == fingerprint
                && matches!(
                    record.status,
                    ExecutionRecordStatus::Ok
                        | ExecutionRecordStatus::Skip
                        | ExecutionRecordStatus::Fresh
                )
        })
    }

    pub fn checkpoint(
        &mut self,
        key: String,
        record: ExecutionRecord,
    ) -> Result<(), LifecycleStateError> {
        self.refuse_if_poisoned()?;
        self.commit(move |state| {
            state.execution.insert(key, record);
        })
    }

    /// Stage the exact ordered payload-event target set this slot run
    /// selected, BEFORE the first pre-install callback can park it.
    ///
    /// A post-install park happens after the lockfile is written, so the
    /// resume sees a FRESH lock and would otherwise never rebuild the slot
    /// run at all. Recording the set — rather than re-deriving it later from
    /// directory enumeration or by parsing task filenames — is what lets the
    /// resume reconstruct the SAME run it left.
    ///
    /// The set is staged, not written. It becomes durable in the same write
    /// that checkpoints the first slot-scoped park, and is dropped the moment
    /// no such park remains, so "a continuation exists exactly while slot work
    /// is owed" holds across every crash point rather than merely at the ends.
    ///
    /// Staging is UNCONDITIONAL, and that is the point. An invocation that
    /// adopts a parked run may satisfy its last delegated row and then park a
    /// LATER row in the same pass; between those two writes the run owes
    /// nothing, so the durable continuation is correctly dropped. Only the
    /// staged set can put it back — a construction that declined to stage
    /// because a durable continuation already existed left the new store with
    /// nothing to restore, and the later park had no target set to name.
    ///
    /// Against an adopted continuation the staged set is CHECKED, never
    /// overwritten: a non-empty set that disagrees with what the parked run
    /// recorded is a typed invariant refusal, because one of the two is not
    /// describing the run this store is servicing. An EMPTY set is not a
    /// disagreement — it is this pass reporting that nothing is
    /// payload-changing right now, which is exactly what a resume's ordinary
    /// materialise pass reports — so it retains the adopted set instead.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
    pub fn record_slot_continuation(
        &mut self,
        continuation: SlotContinuation,
    ) -> Result<(), LifecycleStateError> {
        self.refuse_if_poisoned()?;
        if let Some(adopted) = self.state.run.slot_continuation.clone() {
            if continuation.targets.is_empty() {
                self.staged_continuation = Some(adopted);
                return Ok(());
            }
            if continuation != adopted {
                return Err(LifecycleStateError::Invariant {
                    path: self.path.clone(),
                    reason: format!(
                        "this pass selected {} payload-event target(s) but the run it adopted \
                         parked against {}; the recorded set is what the resume must rebuild, \
                         so it is never overwritten",
                        continuation.targets.len(),
                        adopted.targets.len(),
                    ),
                });
            }
        }
        self.staged_continuation = Some(continuation);
        Ok(())
    }

    /// The slot continuation this run still owes, if any — from the last
    /// proven state, never a live view of disk.
    #[must_use]
    pub fn slot_continuation(&self) -> Option<&SlotContinuation> {
        self.state.run.slot_continuation.as_ref()
    }

    /// The slot run finished: nothing is owed, so the continuation goes. Kept
    /// separate from checkpointing so "the run completed" is one durable
    /// write, not an implicit side effect of the last row.
    pub fn clear_slot_continuation(&mut self) -> Result<(), LifecycleStateError> {
        self.refuse_if_poisoned()?;
        if self.state.run.slot_continuation.is_none() {
            return Ok(());
        }
        self.commit(|state| state.run.slot_continuation = None)
    }

    /// Every delegated row still live in the last proven state, with the
    /// typed scope the engine recorded for it. The caller reconciles these
    /// against the plan of their own scope; nothing here parses an execution
    /// key or a filename.
    #[must_use]
    pub fn delegated_rows(&self) -> Vec<(String, ExecutionRecord)> {
        self.state
            .execution
            .iter()
            .filter(|(_, record)| record.status == ExecutionRecordStatus::Delegated)
            .map(|(key, record)| (key.clone(), record.clone()))
            .collect()
    }

    /// Drop one execution row outright — the cancellation half of reconciling
    /// a delegated row whose declaration no longer exists.
    /// Drop one execution row outright — TRANSACTIONALLY.
    ///
    /// If the durable write fails the in-memory row is restored, so the store
    /// and the file on disk still agree. Without that, a failed forget left a
    /// live store whose row had already vanished from memory while the durable
    /// bytes still named it.
    pub fn forget(&mut self, key: &str) -> Result<(), LifecycleStateError> {
        self.refuse_if_poisoned()?;
        if !self.state.execution.contains_key(key) {
            return Ok(());
        }
        self.commit(|state| {
            state.execution.remove(key);
        })
    }

    /// Prune vanished synthetic rows only after their owner has reconciled
    /// durable outputs successfully. Other lifecycle history is untouched.
    pub fn retain_prefixed(
        &mut self,
        prefix: &str,
        keep: &BTreeSet<String>,
    ) -> Result<(), LifecycleStateError> {
        self.refuse_if_poisoned()?;
        let doomed = self
            .state
            .execution
            .keys()
            .any(|key| key.starts_with(prefix) && !keep.contains(key));
        if !doomed {
            return Ok(());
        }
        self.commit(|state| {
            state
                .execution
                .retain(|key, _| !key.starts_with(prefix) || keep.contains(key));
        })
    }

    /// The last PROVEN state: the generated value this store last made or saw
    /// durable. This is never a live view of disk — a poisoned store still
    /// exposes its last proven state here while refusing to claim it is what
    /// `.vibe/lifecycle.toml` holds now.
    #[must_use]
    pub fn state(&self) -> &LifecycleState {
        &self.state
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Why this store refuses every further mutation, if it is poisoned. The
    /// durable state it names was never proven to be either the candidate or
    /// the prior bytes, so no later write may touch it from this store.
    #[must_use]
    pub fn poison_reason(&self) -> Option<&str> {
        self.poisoned.as_deref()
    }

    /// A poisoned store refuses BEFORE another write, including on the
    /// no-op early-return paths: a "success" reported by a store that cannot
    /// describe its own disk would be a lie about store health, not a
    /// kindness.
    fn refuse_if_poisoned(&self) -> Result<(), LifecycleStateError> {
        match &self.poisoned {
            Some(reason) => Err(LifecycleStateError::Poisoned {
                path: self.path.clone(),
                reason: reason.clone(),
            }),
            None => Ok(()),
        }
    }

    /// The ONE mutating primitive: build a candidate, prove it durable, and
    /// only then let it become this store's state.
    ///
    /// Every verb that changes rows, the header or the continuation goes
    /// through here. `self.state` and `self.durable` are never touched until
    /// the bytes are on disk, so an injected fault, a violated invariant, an
    /// encode failure or an I/O failure all leave the in-memory state
    /// structurally equal to the durable file it still describes. The one
    /// deliberate exception is the post-publication window: there the
    /// recovery re-reads once and lets the PROVEN bytes — candidate or prior —
    /// become current, so memory never disagrees with disk merely because a
    /// rename succeeded before an error was reported.
    fn commit(
        &mut self,
        mutate: impl FnOnce(&mut LifecycleState),
    ) -> Result<(), LifecycleStateError> {
        let mut candidate = self.state.clone();
        mutate(&mut candidate);
        // Reconciled on the CANDIDATE, against the candidate's own slot debt:
        // the pair-law is a property of the bytes about to be written, not a
        // side effect a caller has to sequence correctly.
        reconcile_continuation(&mut candidate, self.staged_continuation.as_ref());
        self.persist(candidate)
    }

    /// Validate and encode the candidate first, then publish it through the
    /// pinned capability and classify the failure by its TYPED stage. A
    /// `BeforePublication` failure is provably invisible: the prior bytes and
    /// state simply stay current, and the typed stage says so. A
    /// `PossiblyPublished` failure hands the decision to the recovery.
    fn persist(&mut self, candidate: LifecycleState) -> Result<(), LifecycleStateError> {
        #[cfg(test)]
        if let Some(reason) = inject::armed() {
            // The injected durable-write fault refuses before anything is
            // attempted — the same class and typed stage a real safefs
            // before-publication failure surfaces.
            return Err(LifecycleStateError::Publication {
                path: self.path.clone(),
                stage: vibe_safefs::PublishStage::BeforePublication,
                failure: reason,
            });
        }
        let bytes = io::encode(&candidate, &self.path)?;
        match self.publish(&bytes) {
            Ok(()) => {
                self.durable = Some(bytes);
                self.state = candidate;
                Ok(())
            }
            Err(failure) => match failure.stage {
                vibe_safefs::PublishStage::BeforePublication => {
                    Err(LifecycleStateError::Publication {
                        path: self.path.clone(),
                        stage: failure.stage,
                        failure: failure.rendered,
                    })
                }
                vibe_safefs::PublishStage::PossiblyPublished => {
                    self.recover_after_possibly_published(candidate, bytes, failure.rendered)
                }
            },
        }
    }

    /// The one publication attempt. The staged atomic replace itself is the
    /// safefs cell's; this wrapper exists only so the deterministic test seam
    /// can stand exactly where a real publication would.
    fn publish(&self, bytes: &[u8]) -> Result<(), PublicationFailure> {
        #[cfg(test)]
        if let Some((reason, plant)) = inject::armed_possibly_plant() {
            // The concurrent-writer window: the plant runs between the prior
            // read and the recovery's re-read, then the publication fails as
            // if its rename had been attempted.
            plant();
            return Err(PublicationFailure::synthetic_possibly(reason));
        }
        #[cfg(test)]
        if let Some(reason) = inject::armed_possibly() {
            return Err(PublicationFailure::synthetic_possibly(reason));
        }
        self.project
            .write_atomic(Self::FILE, bytes)
            .map(|_| ())
            .map_err(PublicationFailure::from_publish)
    }
}

/// Bring a CANDIDATE state's continuation into agreement with its own slot
/// debt. A continuation is meaningful exactly while a slot-scoped delegated
/// row is live: it appears in the same write that records the first such row
/// and disappears in the write that clears the last one.
fn reconcile_continuation(candidate: &mut LifecycleState, staged: Option<&SlotContinuation>) {
    let owed = candidate.execution.values().any(|record| {
        record.status == ExecutionRecordStatus::Delegated
            && record.scope == Some(ExecutionRecordScope::Slot)
    });
    match (owed, candidate.run.slot_continuation.is_some()) {
        (true, false) => candidate.run.slot_continuation = staged.cloned(),
        (false, true) => candidate.run.slot_continuation = None,
        _ => {}
    }
}

#[cfg(test)]
#[path = "inject.rs"]
pub(crate) mod inject;
