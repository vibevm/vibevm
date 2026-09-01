//! The one analyzer observer seam of the compiler schedule (R4.3, the
//! packages-2026-09 architecture §9), and nothing else.
//!
//! A caller that wants the compile's ATTRIBUTION evidence — not its
//! diagnostics — hands one [`CompileObserver`] to the observed sibling
//! of [`crate::compile_artifact`]. The emit boundary then reports one
//! [`EmissionEvent`] per accepted artifact: every contribution's content
//! byte count and occurrence count, read from the Lane/Emission
//! WITNESSES the manager already holds, plus the frame bytes no
//! contribution owns. Each lane- and emitted-position transform wrapper
//! reports one [`StageDeltaEvent`]: the byte counts of what it received
//! and what it returned, labelled by stage, with the lane-byte and
//! artifact-byte measures carried as the two different members they are
//! (§9's parenthetical — a lane's chunk stream and the emitted tape are
//! different things, and the framing the backend adds at emit is exactly
//! the difference between them).
//!
//! This is a SIBLING of the trace seam ([`super::trace`]), not a widening
//! of it, and that seam's own doc is the reason: the trace module's first
//! boundary law is that its vocabulary is the GENERATED
//! `compiler_trace_index/e1` types and that it declares no second
//! vocabulary. The analyzer's evidence belongs to a different report
//! (`extensions_analyze/e1`), so growing the trace sink to carry it
//! would put a second vocabulary inside the one module whose law forbids
//! exactly that. The seams share every other property instead, stated
//! here once:
//!
//! * **Witness never veto.** The observer is infallible from the
//!   compiler's side; no method returns anything the compile can read. A
//!   PANIC is caught at this boundary and treated as an observer defect,
//!   so the run still returns the artifact — or the compiler's own
//!   error — it would have returned unobserved.
//! * **Off means off.** With no observer the wrappers keep the old path:
//!   no byte is counted, no event is built. Every observation sits
//!   behind an `Option<Arc<dyn CompileObserver>>` threaded once at
//!   schedule construction, never a per-run parameter the unobserved
//!   caller must spell.
//! * **Nothing is persisted.** The events are values handed to an
//!   in-process observer for the process's lifetime; this module owns no
//!   clock, no file, no run directory (the frozen §9.1 ruling).
//! * **Witnesses, never artifact text.** Every byte count the events
//!   carry is computed from the Lane's chunk stream, the prepared
//!   emission target, or the artifact's own length — never by reading
//!   the emitted tape's generated comment markers back. Attribution that
//!   parses the artifact's comments instead of the witnesses is the one
//!   implementation this seam exists to make unnecessary, and the
//!   accounting cell's fence test pins it red.
//!
//! Scope, sequence numbers, timing and retention are NOT here — the
//! observer owns none of them, and the CLI that stands at this seam
//! lowers what it receives into its own report shape.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use specmark::spec;

use super::ir::{ArtifactContext, EmittedArtifact};

/// Where a compile's analyzer evidence goes.
///
/// Object-safe, `Send + Sync` and infallible from the compiler's side:
/// an observer is a witness, never a veto. Neither method may influence
/// the compile — a refusal has nowhere to go, and a PANIC is caught at
/// this module's boundary and read as an observer defect, so the run
/// still returns what it would have returned unobserved.
///
/// The one implementation shape this seam is for is an in-process
/// collector the CLI hands in; the trait stands alone so a future host
/// (an MCP surface, a test oracle) can stand at the same seam without a
/// second one being invented beside it.
pub trait CompileObserver: Send + Sync {
    /// Receive one accepted artifact's emission evidence, exactly once
    /// per artifact the schedule emitted and every post-emit validation
    /// accepted.
    ///
    /// A panic here is contained: the observation is lost, the compile
    /// is not.
    fn emission(&self, event: &EmissionEvent);

    /// Receive one transform pass's byte effect, exactly once per
    /// lane-position or emitted-position transform that ran.
    ///
    /// A panic here is contained the same way.
    fn stage_delta(&self, event: &StageDeltaEvent);
}

/// The observer handle the schedule's passes carry. `None` is the
/// unobserved path — the historical instructions, bytes and errors
/// exactly.
pub(crate) type Observing = Option<Arc<dyn CompileObserver>>;

/// One-shot completion handle for a deferred analyzer emission event.
///
/// Managed workspace compilation buffers the backend's emission evidence until
/// pending-header finalization has produced the publishable artifact. Dropping
/// this handle delivers nothing; [`Self::deliver`] consumes it, so a caller
/// cannot report one artifact twice.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub struct DeferredEmission {
    state: Arc<DeferredEmissionState>,
}

struct DeferredEmissionObserver {
    state: Arc<DeferredEmissionState>,
}

struct DeferredEmissionState {
    downstream: Arc<dyn CompileObserver>,
    slot: Mutex<DeferredEmissionSlot>,
}

enum DeferredEmissionSlot {
    Open(Option<EmissionEvent>),
    Closed,
}

/// Buffer one managed compile's emission event while forwarding stage deltas.
///
/// Both returned values share one state: pass the observer to the managed
/// compiler, then consume the completion handle only after Ready or pending
/// finalization has produced the final artifact.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub fn defer_emission(
    downstream: Arc<dyn CompileObserver>,
) -> (Arc<dyn CompileObserver>, DeferredEmission) {
    let state = Arc::new(DeferredEmissionState {
        downstream,
        slot: Mutex::new(DeferredEmissionSlot::Open(None)),
    });
    let observer: Arc<dyn CompileObserver> = Arc::new(DeferredEmissionObserver {
        state: Arc::clone(&state),
    });
    (observer, DeferredEmission { state })
}

impl DeferredEmission {
    /// Deliver the buffered event once, reframed to the final artifact bytes.
    ///
    /// Missing, poisoned, already-closed or context-mismatched state loses the
    /// observation silently. Observer delivery uses the existing panic
    /// boundary and cannot affect the artifact.
    pub fn deliver(self, artifact: &EmittedArtifact) {
        let event = self.state.slot.lock().ok().and_then(|mut slot| {
            match std::mem::replace(&mut *slot, DeferredEmissionSlot::Closed) {
                DeferredEmissionSlot::Open(event) => event,
                DeferredEmissionSlot::Closed => None,
            }
        });
        let Some(event) = event else {
            return;
        };
        if event.context() != artifact.provenance().context() {
            return;
        }
        let event = event.reframed_total(artifact.bytes().len());
        deliver_emission(self.state.downstream.as_ref(), &event);
    }
}

impl CompileObserver for DeferredEmissionObserver {
    fn emission(&self, event: &EmissionEvent) {
        if let Ok(mut slot) = self.state.slot.lock()
            && let DeferredEmissionSlot::Open(buffered) = &mut *slot
            && buffered.is_none()
        {
            *buffered = Some(event.clone());
        }
    }

    fn stage_delta(&self, event: &StageDeltaEvent) {
        deliver_stage_delta(self.state.downstream.as_ref(), event);
    }
}

/// One contribution's attribution evidence, as the emit boundary holds
/// it.
///
/// Every member is derived from the witness material (the contribution's
/// chunk stream, its prepared document, or its target address) — none
/// from reading the emitted tape back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmissionContribution {
    kind: EmissionKind,
    origin: String,
    path: String,
    bytes: usize,
    occurrences: u32,
}

/// Which of the four lane-contribution kinds one attribution row
/// describes. The report epoch's own closed vocabulary, not a kernel or
/// manifest type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionKind {
    /// A compiled closure root.
    Normal,
    /// A document carried verbatim.
    Simple,
    /// A zone elided to its marker.
    Elided,
    /// A whole-document hoist referenced by `#use`.
    Hoisted,
}

impl EmissionContribution {
    /// The one constructor, beside the accounting cell that fills it in.
    pub(crate) fn new(
        kind: EmissionKind,
        origin: String,
        path: String,
        bytes: usize,
        occurrences: u32,
    ) -> Self {
        Self {
            kind,
            origin,
            path,
            bytes,
            occurrences,
        }
    }

    pub fn kind(&self) -> EmissionKind {
        self.kind
    }

    /// The contribution's display provenance (`ContributionMeta.origin`).
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The contribution's declared path (`ContributionMeta.path`).
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The emitted bytes this contribution's own material occupies.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// How many occurrences of this contribution the lane brackets.
    pub fn occurrences(&self) -> u32 {
        self.occurrences
    }
}

/// One accepted artifact's full emission evidence.
///
/// The contributions are in lane order (the effective-boot order the
/// plan declared), and the frame is the artifact's total minus the
/// contributions' bytes — the bytes no contribution owns, by
/// construction rather than by a second accounting that could drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmissionEvent {
    context: ArtifactContext,
    contributions: Vec<EmissionContribution>,
    total_bytes: usize,
    frame_bytes: usize,
}

impl EmissionEvent {
    pub(crate) fn new(
        context: ArtifactContext,
        contributions: Vec<EmissionContribution>,
        total_bytes: usize,
        frame_bytes: usize,
    ) -> Self {
        Self {
            context,
            contributions,
            total_bytes,
            frame_bytes,
        }
    }

    /// The artifact's immutable identity — id, target, frame and mode.
    pub fn context(&self) -> &ArtifactContext {
        &self.context
    }

    /// The artifact id's exact spelling (`static-md`, `static-xml`).
    pub fn artifact_id(&self) -> &str {
        self.context.artifact().as_str()
    }

    /// The target's backend spelling — the one identity string a caller
    /// outside this crate can read off the event without the target
    /// type's own (crate-private) discriminators. An owned string because
    /// the target value is cloned out of the context.
    pub fn target_id(&self) -> String {
        self.context.target().backend_id().to_string()
    }

    /// The attribution rows, in lane order.
    pub fn contributions(&self) -> &[EmissionContribution] {
        &self.contributions
    }

    /// The emitted artifact's exact byte length.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// The bytes no contribution owns: prologue, markers, separators.
    pub fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    fn reframed_total(mut self, total_bytes: usize) -> Self {
        let contribution_bytes = self
            .contributions
            .iter()
            .map(EmissionContribution::bytes)
            .fold(0usize, usize::saturating_add);
        self.total_bytes = total_bytes;
        self.frame_bytes = total_bytes.saturating_sub(contribution_bytes);
        self
    }
}

/// Which schedule stage a transform pass ran at — the two stages a byte
/// delta is defined for. The report epoch's own vocabulary, carried so
/// the lowering never conflates the two measures §9 names apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaStage {
    /// The whole-artifact chunk stream, before the backend runs.
    Lane,
    /// The emitted tape, after the backend ran.
    Emitted,
}

/// One transform pass's byte effect: what it received, what it returned,
/// labelled by stage.
///
/// Exactly one measure is carried per event — the one its stage owns.
/// The other stays unset rather than being defaulted, because a lane's
/// chunk-stream bytes and an artifact's tape bytes are different
/// quantities and a zero would claim one was measured when it was not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDeltaEvent {
    pass: String,
    stage: DeltaStage,
    before: usize,
    after: usize,
}

impl StageDeltaEvent {
    pub(crate) fn new(pass: &str, stage: DeltaStage, before: usize, after: usize) -> Self {
        Self {
            pass: pass.to_string(),
            stage,
            before,
            after,
        }
    }

    /// The schedule pass name (`transform:lane:<key>`).
    pub fn pass(&self) -> &str {
        &self.pass
    }

    /// The stage the pass ran at — which measure the pair below is.
    pub fn stage(&self) -> DeltaStage {
        self.stage
    }

    /// The byte count the pass received.
    pub fn before(&self) -> usize {
        self.before
    }

    /// The byte count the pass returned.
    pub fn after(&self) -> usize {
        self.after
    }
}

/// One accepted emission, delivered through the ONE panic boundary every
/// crossing into a downstream observer goes through — the same
/// containment law the trace seam carries.
///
/// A sink is arbitrary foreign code. If it panics, that is an observer
/// defect, and the witness-only law says an observer defect must not
/// become the compile's answer: the unwind is caught here so the caller
/// returns the artifact — or the compiler's own pass error — it would
/// have returned with no observer at all. The payload is dropped WITHOUT
/// being formatted or cloned, so a hostile panic message cannot ride out
/// of the observer either.
pub(crate) fn deliver_emission(observer: &dyn CompileObserver, event: &EmissionEvent) {
    let _ = panic::catch_unwind(AssertUnwindSafe(|| observer.emission(event)));
}

/// One stage delta, delivered through the panic boundary.
pub(crate) fn deliver_stage_delta(observer: &dyn CompileObserver, event: &StageDeltaEvent) {
    let _ = panic::catch_unwind(AssertUnwindSafe(|| observer.stage_delta(event)));
}

#[cfg(test)]
mod deferred_tests {
    use super::*;
    use crate::compiler::ir::{
        ArtifactContext, ArtifactFrame, ArtifactId, ArtifactTarget, EmittedArtifact,
        StaticCompileMode,
    };

    #[derive(Default)]
    struct Collector {
        emissions: Mutex<Vec<EmissionEvent>>,
        deltas: Mutex<Vec<StageDeltaEvent>>,
    }

    impl CompileObserver for Collector {
        fn emission(&self, event: &EmissionEvent) {
            self.emissions.lock().unwrap().push(event.clone());
        }

        fn stage_delta(&self, event: &StageDeltaEvent) {
            self.deltas.lock().unwrap().push(event.clone());
        }
    }

    struct Panics;

    impl CompileObserver for Panics {
        fn emission(&self, _event: &EmissionEvent) {
            panic!("observer emission panic");
        }

        fn stage_delta(&self, _event: &StageDeltaEvent) {
            panic!("observer delta panic");
        }
    }

    fn static_context(name: &str) -> ArtifactContext {
        ArtifactContext::new(
            ArtifactId::new("static-md").unwrap(),
            ArtifactTarget::StaticMarkdown,
            ArtifactFrame::StaticLane {
                generated_path: format!("{name}.md"),
                source_root: "sources".to_owned(),
            },
            StaticCompileMode::QualifyPerNode,
        )
        .unwrap()
    }

    fn event(context: ArtifactContext, total: usize) -> EmissionEvent {
        EmissionEvent::new(
            context,
            vec![EmissionContribution::new(
                EmissionKind::Simple,
                "host".to_owned(),
                "boot/body.md".to_owned(),
                7,
                1,
            )],
            total,
            total.saturating_sub(7),
        )
    }

    #[test]
    fn delivery_is_deferred_reframed_and_stage_delta_is_immediate() {
        let context = static_context("STATIC");
        let downstream = Arc::new(Collector::default());
        let (observer, completion) = defer_emission(downstream.clone());
        observer.emission(&event(context.clone(), 10));
        assert!(downstream.emissions.lock().unwrap().is_empty());

        observer.stage_delta(&StageDeltaEvent::new(
            "transform:lane:test",
            DeltaStage::Lane,
            4,
            5,
        ));
        assert_eq!(downstream.deltas.lock().unwrap().len(), 1);

        let artifact = EmittedArtifact::testing(context, vec![0; 19]);
        completion.deliver(&artifact);
        let emissions = downstream.emissions.lock().unwrap();
        assert_eq!(emissions.len(), 1);
        assert_eq!(emissions[0].total_bytes(), 19);
        assert_eq!(emissions[0].frame_bytes(), 12);
        assert_eq!(emissions[0].contributions()[0].bytes(), 7);
    }

    #[test]
    fn ready_total_drop_and_context_mismatch_are_exact() {
        let context = static_context("STATIC");
        let ready = Arc::new(Collector::default());
        let (observer, completion) = defer_emission(ready.clone());
        observer.emission(&event(context.clone(), 13));
        completion.deliver(&EmittedArtifact::testing(context.clone(), vec![0; 13]));
        assert_eq!(ready.emissions.lock().unwrap()[0].total_bytes(), 13);

        let dropped = Arc::new(Collector::default());
        let (observer, completion) = defer_emission(dropped.clone());
        observer.emission(&event(context.clone(), 10));
        drop(completion);
        assert!(dropped.emissions.lock().unwrap().is_empty());

        let mismatched = Arc::new(Collector::default());
        let (observer, completion) = defer_emission(mismatched.clone());
        observer.emission(&event(context, 10));
        completion.deliver(&EmittedArtifact::testing(
            static_context("OTHER"),
            vec![0; 15],
        ));
        assert!(mismatched.emissions.lock().unwrap().is_empty());
    }

    #[test]
    fn downstream_panics_are_contained_and_accounting_never_parses_tape() {
        let context = static_context("STATIC");
        let (observer, completion) = defer_emission(Arc::new(Panics));
        observer.emission(&event(context.clone(), 10));
        observer.stage_delta(&StageDeltaEvent::new(
            "transform:emitted:test",
            DeltaStage::Emitted,
            10,
            11,
        ));
        completion.deliver(&EmittedArtifact::testing(context, vec![0; 12]));

        let source = include_str!("observer.rs");
        for forbidden in [
            concat!("vibe:", "transforms"),
            concat!("<!", "--"),
            concat!("vibe_", "specdoc"),
        ] {
            assert!(
                !source.contains(forbidden),
                "forbidden tape parser `{forbidden}`"
            );
        }
        assert!(!source.contains(concat!("impl Clone for ", "DeferredEmission")));
    }
}
