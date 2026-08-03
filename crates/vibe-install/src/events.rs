//! Typed progress events the planning phase emits — the orchestrator
//! never prints; the caller renders (or ignores) each event in its own
//! voice. Tool output is the agent's percept: typed events beat free
//! text at the seam (R3-011).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

/// One observable step of [`plan`](crate::plan). Fields carry exactly
/// what a renderer needs; no pre-formatted prose crosses the seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanEvent {
    /// Case-c migration: the entry manifest was empty but the lockfile
    /// carried `meta.root_dependencies` — the manifest was seeded from
    /// the snapshot and persisted (PROP-002 §2.7).
    MigratingRequires { entries: usize },
    /// The PROP-011 §5.3 stale path: `vibe.lock` no longer matches the
    /// manifests and a re-resolve is starting, with held pins.
    Reresolving { reason: String },
    /// The held-pin set over-constrained the solve; falling back to a
    /// full free re-resolve (PROP-011 §5.3).
    HeldPinsConflicted { error: String },
    /// The depsolver is about to run over this many roots.
    ResolvingRoots { roots: usize },
    /// The solve finished and pulled transitives beyond the roots.
    GraphSolved { roots: usize, total: usize },
    /// One pass of the conditional-dependency fixpoint added extras
    /// (PROP-003 §2.6.1).
    ConditionalIteration { iteration: usize, extras: usize },
    /// `--features` named features no root package declares — they
    /// were silently filtered per root and matched nothing overall.
    FeaturesUnmatched { features: Vec<String> },
}

/// The caller's view into a running plan. Implemented by the CLI to
/// render progress; tests and headless callers use [`NullObserver`].
///
/// ```
/// use vibe_install::{PlanEvent, PlanObserver};
///
/// struct Collector(std::sync::Mutex<Vec<PlanEvent>>);
/// impl PlanObserver for Collector {
///     fn on(&self, event: PlanEvent) {
///         self.0.lock().unwrap_or_else(|e| e.into_inner()).push(event);
///     }
/// }
///
/// let observer = Collector(std::sync::Mutex::new(Vec::new()));
/// observer.on(PlanEvent::ResolvingRoots { roots: 2 });
/// assert_eq!(
///     observer.0.into_inner().unwrap_or_else(|e| e.into_inner()),
///     vec![PlanEvent::ResolvingRoots { roots: 2 }],
/// );
/// ```
pub trait PlanObserver {
    fn on(&self, event: PlanEvent);
}

/// Ignores every event — for headless callers and tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullObserver;

impl PlanObserver for NullObserver {
    fn on(&self, _event: PlanEvent) {}
}
