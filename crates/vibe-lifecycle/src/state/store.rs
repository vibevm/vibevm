//! Strict generated-type state reader and record-last atomic writer.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use specmark::spec;
use thiserror::Error;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, ExecutionRecordStatus, LifecycleState, SlotContinuation,
    StateRun,
};

use super::validate::validate_state;

const SCHEMA: u32 = 1;

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML")]
pub enum LifecycleStateError {
    #[error(
        "cannot read lifecycle state `{path}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "malformed lifecycle state `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Malformed { path: PathBuf, reason: String },
    #[error(
        "unsupported lifecycle state schema {schema} in `{path}`; this build supports schema 1 \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Unsupported { path: PathBuf, schema: u32 },
    #[error(
        "cannot write lifecycle state `{path}` atomically: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: ensure `.vibe/` is writable and rerun)"
    )]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "cannot encode lifecycle state `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML; \
          fix: report this generated-wire serialization failure)"
    )]
    Encode { path: PathBuf, reason: String },
    #[error(
        "lifecycle state `{path}` violates the delegated-run invariant: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    Invariant { path: PathBuf, reason: String },
}

/// Open current state, replace the whole-run header, preserve every old row,
/// and immediately checkpoint the initial record (even for an empty plan).
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-LIFECYCLE-TOML")]
pub struct LifecycleStateStore {
    path: PathBuf,
    state: LifecycleState,
    /// The ordered payload-event target set this run WOULD need to rebuild
    /// itself, held in memory until a slot-scoped park actually exists.
    ///
    /// Never serialised. A continuation is meaningful exactly while a
    /// slot-scoped delegated row is live, so it becomes durable in the SAME
    /// write that checkpoints that row — never in a write of its own, which
    /// would leave a crash window holding a continuation nothing owes.
    staged_continuation: Option<SlotContinuation>,
}

impl LifecycleStateStore {
    pub const FILE: &'static str = ".vibe/lifecycle.toml";

    pub fn begin(
        workspace_root: &Path,
        requested: String,
        chain: Vec<String>,
        started: String,
        run_id: String,
        compile_trace: bool,
    ) -> Result<Self, LifecycleStateError> {
        let path = workspace_root.join(Self::FILE);
        let prior = read_prior(&path)?;
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
            path,
            staged_continuation: None,
            state: LifecycleState {
                execution,
                run: StateRun {
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
                schema: SCHEMA,
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
    pub fn peek(workspace_root: &Path) -> Result<Option<LifecycleState>, LifecycleStateError> {
        read_prior(&workspace_root.join(Self::FILE))
    }

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

    /// The slot continuation this run still owes, if any.
    #[must_use]
    pub fn slot_continuation(&self) -> Option<&SlotContinuation> {
        self.state.run.slot_continuation.as_ref()
    }

    /// The slot run finished: nothing is owed, so the continuation goes. Kept
    /// separate from checkpointing so "the run completed" is one durable
    /// write, not an implicit side effect of the last row.
    pub fn clear_slot_continuation(&mut self) -> Result<(), LifecycleStateError> {
        if self.state.run.slot_continuation.is_none() {
            return Ok(());
        }
        self.commit(|state| state.run.slot_continuation = None)
    }

    /// Every delegated row still live in this state, with the typed scope the
    /// engine recorded for it. The caller reconciles these against the plan of
    /// their own scope; nothing here parses an execution key or a filename.
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

    #[must_use]
    pub fn state(&self) -> &LifecycleState {
        &self.state
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// The ONE mutating primitive: build a candidate, prove it durable, and
    /// only then let it become this store's state.
    ///
    /// Every verb that changes rows, the header or the continuation goes
    /// through here. `self.state` is never touched until the bytes are on
    /// disk, so an injected fault, a violated invariant, an encode failure or
    /// an I/O failure all leave the in-memory state structurally equal to the
    /// durable file it still describes. The alternative — mutate, write,
    /// hand-restore on failure — has to remember every field it touched, and
    /// the continuation is exactly the field such a restore forgets.
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
        self.persist(&candidate)?;
        self.state = candidate;
        Ok(())
    }

    fn persist(&self, candidate: &LifecycleState) -> Result<(), LifecycleStateError> {
        #[cfg(test)]
        if let Some(reason) = inject::armed() {
            return Err(LifecycleStateError::Write {
                path: self.path.clone(),
                source: std::io::Error::other(reason),
            });
        }
        validate_state(candidate).map_err(|reason| LifecycleStateError::Invariant {
            path: self.path.clone(),
            reason,
        })?;
        let bytes = toml::to_string_pretty(candidate)
            .map_err(|error| LifecycleStateError::Encode {
                path: self.path.clone(),
                reason: error.to_string(),
            })?
            .into_bytes();
        let Some(parent) = self.path.parent() else {
            return Err(LifecycleStateError::Write {
                path: self.path.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "lifecycle state path has no parent",
                ),
            });
        };
        fs::create_dir_all(parent).map_err(|source| LifecycleStateError::Write {
            path: self.path.clone(),
            source,
        })?;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let tmp = parent.join(format!(
            ".lifecycle.toml.tmp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let result = (|| {
            let mut file = File::create(&tmp)?;
            file.write_all(&bytes)?;
            file.flush()?;
            file.sync_all()?;
            fs::rename(&tmp, &self.path)?;
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
            Ok::<_, std::io::Error>(())
        })();
        if let Err(source) = result {
            let _ = fs::remove_file(&tmp);
            return Err(LifecycleStateError::Write {
                path: self.path.clone(),
                source,
            });
        }
        Ok(())
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

/// Parse and semantically validate the prior state file. `Ok(None)` when no
/// state exists yet; every other outcome — unreadable, unsupported schema,
/// TOML-malformed, or invariant-violating — is the erasable-cache refusal.
pub(super) fn read_prior(path: &Path) -> Result<Option<LifecycleState>, LifecycleStateError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LifecycleStateError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let previous: LifecycleState =
        toml::from_str(&text).map_err(|error| LifecycleStateError::Malformed {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    if previous.schema != SCHEMA {
        return Err(LifecycleStateError::Unsupported {
            path: path.to_path_buf(),
            schema: previous.schema,
        });
    }
    validate_state(&previous).map_err(|reason| LifecycleStateError::Invariant {
        path: path.to_path_buf(),
        reason,
    })?;
    Ok(Some(previous))
}

/// Injection point for a durable state-write failure, so the cancellation REDs
/// have a deterministic counterexample instead of one that depends on
/// filesystem permissions. Compiled out entirely outside tests, and it reads
/// no environment: the canonical file's OLD bytes must stay readable, which
/// deleting or chmod-ing it would not preserve.
#[cfg(test)]
pub(crate) mod inject {
    use std::cell::RefCell;

    thread_local! {
        static ARMED: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Make every subsequent state write on THIS thread fail. Pass `None` to
    /// disarm.
    pub(crate) fn fail_state_writes(reason: Option<&str>) {
        ARMED.with(|armed| *armed.borrow_mut() = reason.map(str::to_string));
    }

    pub(super) fn armed() -> Option<String> {
        ARMED.with(|armed| armed.borrow().clone())
    }
}
