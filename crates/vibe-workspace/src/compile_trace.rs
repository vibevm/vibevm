//! The durable compile trace: one run directory, one sink per artifact
//! compilation, and nothing the compiler can trip over.
//!
//! `vibe-spec` owns the observation seam — one [`PassTraceEvent`] per attempted
//! pass, plus the exact pretty `compiler_ir/e1` bytes of every accepted
//! carrier. `vibe-wire` owns the metadata epoch: the generated index type, its
//! relational validator, the canonical snapshot filename codec and the
//! aggregate table. What was missing between them is HERE: the run.
//!
//! ```text
//! .vibe/trace/<run-id>/
//!   index.json                                   ← atomic replace, continuously readable
//!   0000-parse-node_._static%2Dmd-000.json       ← create-new, written once
//!   0001-parse-node_._static%2Dmd-001.json
//!   …
//! ```
//!
//! A recorder is opened ONCE per lifecycle run, immediately after the run id
//! is allocated, and every artifact compilation of that run declares a scope
//! on it and hands the resulting [`TraceScope`] to a traced compile. One
//! `vibe install` is therefore one run directory with one dense event
//! sequence, however many nodes and package units it touches — creating a
//! recorder inside an artifact compile would mint a directory per artifact and
//! reset the numbering.
//!
//! ## The law nothing here is allowed to break
//!
//! **A trace failure is never a compile failure.** Opening the run can fail,
//! and a caller that cannot open one simply compiles untraced. After that,
//! nothing fails at all: a snapshot that cannot be written becomes a
//! `snapshot-failed` event, an index update that cannot land becomes a warning
//! the next update retries, an event that arrives for an undeclared scope is
//! dropped with a reason. The compiler's artifact, its errors and their
//! identities are exactly what they would have been unobserved. That is why
//! [`CompileTraceSink`](vibe_spec::CompileTraceSink) is infallible from the
//! compiler's side, and why the run state recovers a poisoned mutex instead of
//! panicking: an observer is a witness, never a veto.
//!
//! ## What is bounded
//!
//! Two independent measures, and they answer different questions. The **byte
//! budget** bounds one run: once 128 MiB of snapshot payload has been
//! published, later invocations stand down as `snapshot-skipped-budget` before
//! any encode clock starts, while their pass and verify timings still land.
//! **Retention** bounds the directory across runs: opening a fresh run keeps
//! the newest nine provably complete traces and reports everything it refused
//! to touch.
//!
//! ## Not here
//!
//! Threading a recorder through install/lifecycle/CLI, the `--trace-compile`
//! flag and the manifest read are the next atom. Trace-disabled compilation is
//! not this module's business either: with no sink `vibe-spec` takes the old
//! path and allocates nothing, which is already proved there — a second
//! no-trace path here would be a second thing to keep true.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vibe_spec::{CompileTraceSink, PassTraceEvent, SnapshotDecision};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    ArtifactTarget, CompilerTraceIndex, PassShape, RunStatus, Scope, ScopeKind, ScopeStatus,
    Timestamp, TimingRow,
};

/// The one cooperative lock every VibeVM trace writer contends for, held for
/// the whole life of a run. Its home is `.vibe/`, beside the other
/// project-scoped state, and never inside `.vibe/trace` — a lock that lives in
/// the directory it protects cannot protect that directory's creation.
const TRACE_LOCK: &str = "compile-trace.lock";

mod attempts;
mod bounded;
mod descriptors;
mod identity;
mod open;
mod retention;
mod sequence;
mod state;
mod store;

// The descriptor constructors and the attempt allocator are the workspace's
// own MUTABLE authority over a run's identity space, so they stay
// crate-private: a scope base is minted from typed workspace data (a
// `(group, name)` coordinate, a canonical node rel, a `SpecFormat`) and is
// acquired only through [`ScopeAcquisition`], never from a string an outside
// caller composed. The exact `declare_scope` API remains public for tests and
// other callers.
pub(crate) use descriptors::ScopeAcquisition;
// Production code reaches a base only through `ScopeAcquisition`; the reds
// that plant an interrupted occurrence need the constructor itself.
#[cfg(test)]
pub(crate) use descriptors::node_descriptor;

use state::RunState;

#[cfg(test)]
mod tests;

/// Render `args` into at most the wire epoch's diagnostic cap, marker
/// included, without ever materialising anything larger.
///
/// This is the writer's bounded formatter, exposed because its consumers are
/// not all inside this crate. A CLI that renders a [`TraceWarning`] is
/// rendering text whose FIELDS this cell already bounded — and then adding a
/// prefix, which can push the whole message back over the cap the epoch's
/// validator actually enforces. Clamping the finished message is therefore not
/// a courtesy; it is the difference between a report that validates and one
/// that does not. Callers pass `format_args!`, so the unbounded intermediate
/// string never exists at all.
///
/// It is deliberately the SAME cell the index writer uses. A second cap or a
/// second truncation marker in a consumer would be a second definition of what
/// "bounded" means, and the one that drifted would be the one a hostile
/// diagnostic reached.
#[must_use]
pub fn bounded_diagnostic(args: fmt::Arguments<'_>) -> String {
    bounded::diagnostic(args)
}

/// The two measures that bound a trace, and the only place their numbers are
/// written down.
///
/// There is deliberately no manifest table and no CLI knob for either: a
/// diagnostic that a user can widen is a diagnostic that can fill a disk, and
/// one they can narrow is one that silently stops answering. Tests construct
/// tiny values through a crate-private constructor, which is why the budget
/// REDs need no production configuration to exist.
#[derive(Debug, Clone, Copy)]
pub struct TraceLimits {
    snapshot_budget_bytes: u64,
    retained_runs: usize,
}

impl TraceLimits {
    /// Exactly 128 MiB of published snapshot payload per run, and nine older
    /// complete runs kept beside the live one.
    #[must_use]
    pub const fn production() -> Self {
        Self {
            snapshot_budget_bytes: 128 * 1024 * 1024,
            retained_runs: 9,
        }
    }

    #[cfg(test)]
    pub(crate) const fn for_test(snapshot_budget_bytes: u64, retained_runs: usize) -> Self {
        Self {
            snapshot_budget_bytes,
            retained_runs,
        }
    }
}

impl Default for TraceLimits {
    fn default() -> Self {
        Self::production()
    }
}

/// Why a trace run could not be opened. Every arm is a reason to compile
/// UNTRACED — never a reason to fail a compile.
#[derive(Debug, thiserror::Error)]
pub enum TraceOpenError {
    #[error("the trace project root must be absolute: `{root}`")]
    RelativeRoot { root: String },
    #[error("`{run_id}` is not an exact 32-lowercase-hex lifecycle run id")]
    RunId { run_id: String },
    #[error(
        "the run directory is {directory_units} path units deep, leaving {remaining} for a \
         snapshot name; the shortest canonical name needs {floor}"
    )]
    RunDirectoryTooDeep {
        directory_units: usize,
        remaining: usize,
        floor: usize,
    },
    #[error("the trace directory could not be opened: {reason}")]
    Directory { reason: String },
    #[error("`{path}` is trace residue and was left untouched: {reason}")]
    Residue { path: String, reason: String },
    #[error(
        "another VibeVM trace writer already owns `{project}`; this run is not traced rather \
         than waiting for it"
    )]
    Busy { project: String },
}

/// Something the trace could not do, reported to whoever renders the run.
///
/// Warnings accumulate; none of them ever reaches the compiler.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TraceWarning {
    #[error("`{path}` was left in place: {reason}")]
    Residue { path: String, reason: String },
    #[error("a trace index update did not land and will be retried: {reason}")]
    IndexWrite { reason: String },
    #[error(
        "a trace index update landed, but its publication reported a fault after the point of \
         no return: {reason}"
    )]
    IndexAnomaly { reason: String },
    #[error("event {sequence} produced no snapshot: {reason}")]
    Snapshot { sequence: u32, reason: String },
    #[error("an observation was dropped: {reason}")]
    Dropped { reason: String },
    #[error("the run's terminal status was not written, so the index stays `running`: {reason}")]
    NotFinalised { reason: String },
}

/// Why one scope operation refused. Also never a compile failure — a caller
/// that cannot record a transition has still compiled the artifact.
#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("no scope `{id}` was declared on this run")]
    UnknownScope { id: String },
    #[error("scope `{id}` is already declared with a different identity")]
    ScopeConflict { id: String },
    #[error("scope `{id}` already reached a terminal status")]
    ScopeAlreadyResolved { id: String },
    #[error("scope `{id}` recorded events, so it cannot be reported as skipped")]
    SkipAfterEvents { id: String },
    #[error(
        "scope base `{base}` has spent every attempt id the attempt grammar can address; the \
         counter refuses rather than saturating or wrapping"
    )]
    AttemptExhausted { base: String },
    #[error("the trace index refused the update: {reason}")]
    IndexRefused { reason: String },
    #[error("the run is already finalised")]
    Finalised,
}

/// How a lifecycle run ended, as the trace records it.
///
/// This is the COMPILE's outcome, not the observer's health: a run whose only
/// blemishes are `snapshot-failed` or `snapshot-skipped-budget` events is
/// [`RunOutcome::Ok`], because enabling a diagnostic must not be able to turn
/// a green run red.
#[derive(Debug, Clone)]
pub enum RunOutcome {
    Ok,
    Failed(String),
}

/// One artifact compilation, as the trace will name it.
#[derive(Debug, Clone)]
pub struct ScopeDescriptor {
    /// Stable id every event of this scope references, unique in the run.
    pub id: String,
    /// Workspace node, package unit, or publish artifact.
    pub kind: ScopeKind,
    /// The exact unencoded outer node/unit label. `ArtifactId` alone repeats
    /// `static-md`/`static-xml` across artifacts; this is what disambiguates.
    pub label: String,
    /// The artifact id, e.g. `static-md`.
    pub artifact: String,
    /// The final-artifact target, spelled as the compiler's own open
    /// vocabulary spells it.
    pub target: ArtifactTarget,
}

impl ScopeDescriptor {
    /// Whether an already-declared scope is THIS scope. Used only on the
    /// reopen path, where a pending scope may be reacquired but never
    /// redefined.
    fn matches(&self, scope: &Scope) -> bool {
        scope.kind == self.kind
            && scope.label == self.label
            && scope.artifact == self.artifact
            && scope.target == self.target
    }
}

/// What a later CLI renders: where the run is, what it cost, and everything
/// the observer could not do.
#[derive(Debug, Clone)]
pub struct TraceSummary {
    pub run_dir: PathBuf,
    pub status: RunStatus,
    pub events: usize,
    pub snapshots: usize,
    pub snapshot_bytes: u64,
    pub budget_exhausted: bool,
    /// Whether the terminal status actually reached disk. `false` means the
    /// on-disk index is still `running`, and the warnings say why.
    pub finalised: bool,
    pub aggregates: Vec<TimingRow>,
    pub warnings: Vec<TraceWarning>,
}

/// One lifecycle run's durable trace.
#[derive(Debug, Clone)]
pub struct TraceRun {
    inner: Arc<Mutex<RunState>>,
    run_dir: PathBuf,
}

/// One artifact compilation's sink — what a traced compile is handed.
///
/// Scope-bound but run-shared: the global sequence and the per-`(scope, pass)`
/// ordinals live in the one run state behind it, so two artifact scopes
/// recording concurrently interleave into one dense numbering instead of
/// colliding.
#[derive(Debug, Clone)]
pub struct TraceScope {
    inner: Arc<Mutex<RunState>>,
    id: String,
}

impl TraceRun {
    /// Open — or safely reopen — the run directory for `run_id` under an
    /// absolute project root, with production limits.
    pub fn open(root: &Path, run_id: &str, started: Timestamp) -> Result<Self, TraceOpenError> {
        Self::open_with_limits(root, run_id, started, TraceLimits::production())
    }

    /// The same, under explicit limits.
    pub fn open_with_limits(
        root: &Path,
        run_id: &str,
        started: Timestamp,
        limits: TraceLimits,
    ) -> Result<Self, TraceOpenError> {
        open::create_or_reopen(root, run_id, started, limits)
    }

    /// Reopen an EXISTING run, or say that there is none — and either way
    /// create no TRACE state.
    ///
    /// This is the seam a displaced or an adopted run needs. Both have a state
    /// fact that says tracing was *requested*, which is not the same as saying
    /// a recorder ever opened: the original invocation may have found the
    /// project busy, or the directory too deep, and compiled untraced. If that
    /// caller then reached for [`open`](Self::open), it would create a run
    /// directory halfway through a lifecycle run and publish a history whose
    /// early compiles are simply absent — a supersede would manufacture a
    /// phantom terminal run to supersede, and a resume would turn "never
    /// observed" into "apparently complete". So absence is answered with
    /// `Ok(None)`, and no trace directory, run or index is written.
    ///
    /// **It is not a read-only call.** Deciding requires the cooperative
    /// project lock, and taking that lock creates `.vibe/` and
    /// `.vibe/compile-trace.lock` when they are absent. That is the point
    /// rather than a side effect: contention has to be settled before absence
    /// can be claimed, or this call would report "there is no run" about a
    /// directory another writer is at that moment creating.
    ///
    /// Every other answer is exactly the ordinary reopen's answer: same
    /// project lock, same identity/start/status/entry validation, same typed
    /// refusals. A present-but-unsafe object is `Residue`, never absence.
    /// Retention does not run — collecting old runs is a fresh run's job, and
    /// a caller that only wants to look must not be able to delete.
    pub fn open_existing(
        root: &Path,
        run_id: &str,
        started: Timestamp,
    ) -> Result<Option<Self>, TraceOpenError> {
        Self::open_existing_with_limits(root, run_id, started, TraceLimits::production())
    }

    /// The same, under explicit limits — the budget half of which is
    /// load-bearing on this path too, because a reopened run inherits the
    /// snapshot bytes already on disk.
    pub fn open_existing_with_limits(
        root: &Path,
        run_id: &str,
        started: Timestamp,
        limits: TraceLimits,
    ) -> Result<Option<Self>, TraceOpenError> {
        open::existing_only(root, run_id, started, limits)
    }

    /// The absolute run directory.
    #[must_use]
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    /// Declare one artifact compilation, initially `pending`, and take the
    /// sink a traced compile is handed.
    pub fn declare_scope(&self, descriptor: &ScopeDescriptor) -> Result<TraceScope, TraceError> {
        locked(&self.inner).declare_scope(descriptor)?;
        Ok(TraceScope {
            inner: Arc::clone(&self.inner),
            id: descriptor.id.clone(),
        })
    }

    /// Declare the NEXT attempt of a manager-owned BASE descriptor and take
    /// its sink — the integration form of [`declare_scope`](Self::declare_scope).
    ///
    /// The base id is the stable artifact identity (minted by the workspace
    /// descriptor constructors); the occurrence id this run declares is
    /// `<base>::attempt:<positive-decimal>`, allocated by the closed attempt
    /// grammar: the latest still-`pending` attempt is reacquired exactly after
    /// a crash, a terminal attempt deterministically mints the next positive
    /// number, a pending descriptor conflict refuses, and counter exhaustion
    /// is an explicit error rather than a saturation or a wrap.
    pub(crate) fn acquire_scope(&self, base: &ScopeDescriptor) -> Result<TraceScope, TraceError> {
        let id = locked(&self.inner).acquire_scope(base)?;
        Ok(TraceScope {
            inner: Arc::clone(&self.inner),
            id,
        })
    }

    /// [`acquire_scope`](Self::acquire_scope) as an observer: a refusal
    /// becomes a bounded [`TraceWarning::Dropped`] on the run and the answer
    /// is `None`, meaning the caller compiles UNTRACED. The integration
    /// adapters never let a declaration fault reach the compiler.
    pub(crate) fn acquire_scope_lossy(&self, base: &ScopeDescriptor) -> Option<TraceScope> {
        match self.acquire_scope(base) {
            Ok(scope) => Some(scope),
            Err(error) => {
                locked(&self.inner).dropped(bounded::diagnostic(format_args!(
                    "scope `{}` could not be declared; this compilation runs untraced: {error}",
                    bounded::preview(&base.id)
                )));
                None
            }
        }
    }

    /// The run's terminal word, written LAST.
    pub fn finish(&self, outcome: &RunOutcome, finished: Timestamp) -> TraceSummary {
        let mut state = locked(&self.inner);
        state.finish(outcome, finished);
        summarise(&state, &self.run_dir)
    }

    /// What the run looks like right now, without ending it.
    #[must_use]
    pub fn summary(&self) -> TraceSummary {
        summarise(&locked(&self.inner), &self.run_dir)
    }

    /// Record an observer-side diagnostic as a bounded
    /// [`TraceWarning::Dropped`] — the one integration inlet for a trace
    /// observation that failed without a scope of its own (for example, a
    /// fresh unit whose emitted bytes could not be re-read for the skip's
    /// fingerprint). Never a compile or install error.
    pub(crate) fn note_dropped(&self, reason: &str) {
        locked(&self.inner).dropped(bounded::diagnostic(format_args!("{reason}")));
    }
}

impl TraceScope {
    /// The scope id every event of this scope carries.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// This artifact compiled, with the fingerprint of what it produced.
    pub fn complete(&self, fingerprint: &str) -> Result<(), TraceError> {
        locked(&self.inner).resolve_scope(&self.id, ScopeStatus::Compiled, fingerprint)
    }

    /// This artifact failed. The diagnostic is bounded before it is stored.
    pub fn fail(&self, failure: &str) -> Result<(), TraceError> {
        locked(&self.inner).resolve_scope(&self.id, ScopeStatus::Failed, failure)
    }

    /// This artifact was already fresh and never compiled. Refuses once the
    /// scope has recorded events, because a skipped scope is silent by law.
    pub fn skip(&self, fingerprint: &str) -> Result<(), TraceError> {
        locked(&self.inner).resolve_scope(&self.id, ScopeStatus::Skipped, fingerprint)
    }

    /// [`complete`](Self::complete) as an observer: a refusal becomes a
    /// bounded [`TraceWarning::Dropped`] and nothing propagates. The artifact
    /// exists; only its recording failed.
    pub fn complete_lossy(&self, fingerprint: &str) {
        self.resolve_lossy(ScopeStatus::Compiled, fingerprint);
    }

    /// [`fail`](Self::fail) as an observer, in the same shape.
    pub fn fail_lossy(&self, failure: &str) {
        self.resolve_lossy(ScopeStatus::Failed, failure);
    }

    /// [`skip`](Self::skip) as an observer, in the same shape.
    pub fn skip_lossy(&self, fingerprint: &str) {
        self.resolve_lossy(ScopeStatus::Skipped, fingerprint);
    }

    fn resolve_lossy(&self, status: ScopeStatus, witness: &str) {
        if let Err(error) = locked(&self.inner).resolve_scope(&self.id, status.clone(), witness) {
            locked(&self.inner).dropped(bounded::diagnostic(format_args!(
                "scope `{}` could not be reported as `{}`: {error}",
                bounded::preview(&self.id),
                // The wire spelling, not `{:?}` of the domain copy.
                match status {
                    ScopeStatus::Compiled => "compiled",
                    ScopeStatus::Failed => "failed",
                    ScopeStatus::Skipped => "skipped",
                    ScopeStatus::Pending => "pending",
                }
            )));
        }
    }
}

impl CompileTraceSink for TraceScope {
    fn record(&self, event: &PassTraceEvent<'_>) {
        locked(&self.inner).record(&self.id, event);
    }

    fn before_snapshot(&self, _pass: &str, _output: &PassShape) -> SnapshotDecision {
        locked(&self.inner).snapshot_decision(&self.id)
    }
}

/// The one lock, taken with poisoning RECOVERED rather than propagated.
///
/// A panic somewhere else in the process must not turn this observer into the
/// thing that ends the compile: the state it guards is a diagnostic model, the
/// worst a torn update leaves is an event the index then refuses, and that is
/// already a warning. `unwrap()` here would make the observer a veto.
fn locked(inner: &Arc<Mutex<RunState>>) -> MutexGuard<'_, RunState> {
    inner.lock().unwrap_or_else(PoisonError::into_inner)
}

fn summarise(state: &RunState, run_dir: &Path) -> TraceSummary {
    let index = state.index();
    TraceSummary {
        run_dir: run_dir.to_path_buf(),
        status: index.status.clone(),
        events: index.events.len(),
        snapshots: index
            .events
            .iter()
            .filter(|event| event.snapshot.is_some())
            .count(),
        snapshot_bytes: state.spent(),
        budget_exhausted: state.budget_exhausted(),
        finalised: state.finalised(),
        aggregates: index.aggregates.clone(),
        warnings: state.take_warnings(),
    }
}

/// The index a fresh run starts from — and, on the reopen path, the identity
/// an existing index has to match exactly. `canonical` is the ONE resolved
/// root; nothing here ever sees the caller's original spelling.
fn fresh_index(canonical: &Path, run_id: &str, started: Timestamp) -> CompilerTraceIndex {
    CompilerTraceIndex {
        aggregates: Vec::new(),
        events: Vec::new(),
        project: identity::project_identity(canonical),
        run_id: run_id.to_string(),
        schema: 1,
        scopes: Vec::new(),
        started,
        status: RunStatus::Running,
        failure: None,
        finished: None,
    }
}
