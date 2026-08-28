//! The ONE carrier any measured failure travels outward on, whatever it
//! measured.
//!
//! ## Why it is neutral, and why it is generic
//!
//! The same execution core is `vibe install`'s body, a phase verb's
//! prerequisite and `vibe update --all`'s delegate, and each of those three
//! outer commands reports a DIFFERENT registered root family. Choosing one here
//! would pick a family and be wrong for the other two, so this carrier names
//! none: it transports what the site measured, the site's own emission policy,
//! and the caller's error object, and the surface decides the family.
//!
//! The surface then needs the SAME transport one layer up — its chosen
//! registered draft, the same error object, the same site-frozen bit — and for
//! a while it had a private copy of this law with different names. Two carriers
//! with one law is one law that can drift: an idempotence rule fixed here and
//! not there, an error re-wrapped on one side only. So the carrier is generic
//! over its evidence ([`Carried<E>`]), a measurement is just `Carried<Measurement>`,
//! and a surface's registered draft is `Carried<ItsOwnDraftSum>`. One
//! implementation of `Display`, `Error::source`, the downcast and the
//! idempotence — and, because `Carried<A>` and `Carried<B>` are different
//! concrete types, a boundary still probes for exactly the evidence it owns.
//!
//! ## Why it owns the original error
//!
//! The outer boundary must return the caller's error UNCHANGED — same downcast
//! identity for the exit code, same context chain for stderr. A carrier that
//! stored a formatted string could not do that, so it stores the object and the
//! boundary that takes it apart gets the object straight back out.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::fmt;

use specmark::spec;
use vibe_install::{InstallProgress, SlotLifecycleReport};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::shared::VerificationEvidence;

/// What a failing run had really measured when it stopped.
///
/// The two shapes are not interchangeable: slot rows describe the install
/// substrate's own barrier work and lifecycle rows describe phase
/// contributions, and a surface renders them into different report families.
///
/// ```
/// use vibe_orchestrator::failure::Measurement;
/// let measured = Measurement::Slot {
///     progress: Box::default(),
///     reports: Vec::new(),
///     packages_resolved: 0,
/// };
/// assert!(matches!(measured, Measurement::Slot { .. }));
/// ```
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub enum Measurement {
    /// The install substrate's OWN barrier failure — a slot row that failed
    /// during the apply.
    ///
    /// Distinct from [`Measurement::Slot`] because the measuring site already
    /// knows the family: this is install-shaped work, and every surface reports
    /// it in ITS install root — including a phase verb, whose prerequisite
    /// install has always emitted an install-shaped draft rather than a
    /// lifecycle one, and `vibe update --all`, whose delegate does the same.
    InstallBarrier {
        /// What was durable when the barrier failed.
        progress: Box<InstallProgress>,
        /// Every slot row the apply produced.
        reports: Vec<SlotLifecycleReport>,
        /// The resolved-package count of the run being reported.
        packages_resolved: usize,
    },
    /// What a RESUME made durable, and the slot rows it produced — neutral.
    ///
    /// The family is deliberately NOT knowable here: the same continuation is
    /// serviced inside `vibe install` (install-shaped), a phase verb
    /// (lifecycle-shaped) and `vibe update --all` (update-shaped), and each of
    /// those three answers differently.
    Slot {
        /// What was durable when the failure struck.
        progress: Box<InstallProgress>,
        /// Every slot row the run produced, taken exactly once.
        reports: Vec<SlotLifecycleReport>,
        /// The resolved-package count of the run being reported.
        packages_resolved: usize,
    },
    /// The phase-contribution rows measured up to the failure.
    Lifecycle {
        /// The rows, in the order they happened.
        rows: Vec<LifecycleContributionReport>,
        /// The phase the chain stopped at.
        stopped_phase: String,
        /// The requested phase of the run.
        requested: String,
        /// The complete requested chain.
        chain: Vec<String>,
        /// The verification-evidence member the verify boundary had already
        /// reconciled when the failure struck, carried verbatim.
        ///
        /// Present for a stale/missing/unstable stop — which IS this
        /// failure — and equally for a matched identity a later verify
        /// handler, state write or checkpoint then failed beside. A failure
        /// projection may never rebuild or drop it: the comparison that
        /// existed before dispatch is the one an external orchestrator reads.
        ///
        /// BOXED, and only for size: inline it is 200 bytes wider than the two
        /// slot-shaped variants, so every `Result` in this crate would pay for
        /// it on its success path too. The box is unwrapped at the projection,
        /// never the value — what an external orchestrator reads is the member
        /// the engine minted, byte for byte.
        verification: Option<Box<VerificationEvidence>>,
    },
}

/// A measured failure travelling outward: what the site measured, the exact
/// error object to return, and the emission policy that site already had.
///
/// Generic over the EVIDENCE, because the law is the same all the way up: the
/// lower layer carries a [`Measurement`] it refuses to name a report family
/// for, and the surface carries the registered draft it chose from that
/// measurement. Same owned evidence, same untouched error, same site-frozen
/// bit — so it is the same type, instantiated twice, rather than two structs
/// whose invariants can drift apart.
///
/// ```
/// use vibe_orchestrator::failure::{Carried, carry, take};
///
/// // Any owned evidence at all: the carrier never inspects it.
/// let carried = carry(Carried {
///     original: anyhow::Error::msg("the handler refused"),
///     evidence: "a surface's own registered draft",
///     emit_machine_failure: true,
/// });
/// let taken: Carried<&str> = take(carried).unwrap();
/// assert_eq!(taken.evidence, "a surface's own registered draft");
/// assert!(taken.emit_machine_failure);
/// ```
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct Carried<E> {
    /// The caller's error, unchanged — same downcast identity, same chain.
    pub original: anyhow::Error,
    /// What the failing site had really measured, in whatever shape that site
    /// owns.
    pub evidence: E,
    /// Whether this failure emitted its machine document when tracing was OFF.
    /// A property of the SITE and of the observer that owns it, never something
    /// inferred later from the error.
    pub emit_machine_failure: bool,
}

/// The lower instantiation: a measurement no layer below the surface may give a
/// report family to.
///
/// ```
/// use vibe_orchestrator::failure::{MeasuredFailure, Measurement};
/// fn takes(failure: MeasuredFailure) -> bool {
///     matches!(failure.evidence, Measurement::Lifecycle { .. })
/// }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub type MeasuredFailure = Carried<Measurement>;

impl<E: fmt::Debug> fmt::Display for Carried<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.original, formatter)
    }
}

impl<E: fmt::Debug> std::error::Error for Carried<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.original.source()
    }
}

/// Hand a measured failure outward through the ordinary `Result` channel.
///
/// The `anyhow` wrapper is TRANSPORT only: every boundary that can receive one
/// calls [`take`], which takes the owned value straight back
/// out. Nothing formats it, and it never reaches a process exit.
///
/// ```
/// use vibe_orchestrator::failure::{Carried, Measurement, carry, take};
///
/// let original = anyhow::Error::msg("the handler refused").context("phase `build` stopped");
/// let rendered = format!("{original:#}");
/// let carried = carry(Carried {
///     original,
///     evidence: Measurement::Lifecycle {
///         rows: Vec::new(),
///         stopped_phase: "build".into(),
///         requested: "build".into(),
///         chain: vec!["build".into()],
///         verification: None,
///     },
///     emit_machine_failure: true,
/// });
/// let taken = take::<Measurement>(carried).unwrap();
/// assert!(taken.emit_machine_failure);
/// assert_eq!(format!("{:#}", taken.original), rendered);
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn carry<E>(failure: Carried<E>) -> anyhow::Error
where
    E: fmt::Debug + Send + Sync + 'static,
{
    anyhow::Error::new(failure)
}

/// Take a transported measured failure back out, exactly, or return the error
/// untouched.
///
/// Total, so a caller can branch without consuming an error it may need to pass
/// on unchanged. Evidence-typed: an error carrying a surface's registered draft
/// is NOT a `Measurement` carrier and is handed straight back, which is what
/// lets both layers probe the same error in turn. The example on [`carry`]
/// demonstrates the round trip.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn take<E>(error: anyhow::Error) -> Result<Carried<E>, anyhow::Error>
where
    E: fmt::Debug + Send + Sync + 'static,
{
    error.downcast::<Carried<E>>()
}

/// Whether this error already carries evidence of the given shape.
///
/// Asked, rather than discovered by a failed `downcast`, so a site can branch
/// without consuming an error it may need to pass on untouched.
///
/// ```
/// use vibe_orchestrator::failure::{Measurement, is_carried};
/// assert!(!is_carried::<Measurement>(&anyhow::Error::msg("plain")));
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn is_carried<E>(error: &anyhow::Error) -> bool
where
    E: fmt::Debug + Send + Sync + 'static,
{
    error.is::<Carried<E>>()
}

/// Attach evidence to an error that is not carrying any of this shape yet.
///
/// The site that runs contributions accumulates rows as it goes, and only ONE
/// of the ways it can fail (a handler that reported a failed transition) knows
/// to freeze them. Every other failure after that point — a state write, a park
/// reconciliation, a checkpoint — would otherwise return with the rows still
/// sitting in a local, and the report would claim the run did nothing when it
/// had already done several things successfully.
///
/// Idempotent: an error that already carries this shape keeps the evidence its
/// own site froze, because that one is more specific than this.
///
/// ```
/// use vibe_orchestrator::failure::{Measurement, carry_once, take};
///
/// let carried = carry_once(anyhow::Error::msg("writing the checkpoint"), || {
///     Measurement::Lifecycle {
///         rows: Vec::new(),
///         stopped_phase: "build".into(),
///         requested: "build".into(),
///         chain: vec!["build".into()],
///         verification: None,
///     }
/// });
/// // A generic post-row failure was historically silent.
/// assert!(!take::<Measurement>(carried).unwrap().emit_machine_failure);
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn carry_once<E>(error: anyhow::Error, evidence: impl FnOnce() -> E) -> anyhow::Error
where
    E: fmt::Debug + Send + Sync + 'static,
{
    if is_carried::<E>(&error) {
        return error;
    }
    // Historically silent: these failures never emitted a machine root with
    // tracing off, and adding one now would be a new document on an old path.
    carry(Carried {
        original: error,
        evidence: evidence(),
        emit_machine_failure: false,
    })
}

/// Prepend rows measured BEFORE the failing site to a carried LIFECYCLE
/// measurement.
///
/// A site that fails freezes what IT measured. It cannot know about work an
/// outer stage already did — a chained clean's contributions, a prerequisite
/// install's slot rows — because those never passed through it. So the outer
/// stage hands them down here, once, and everything else the carrier owns
/// survives: the same error object, the same emission policy.
///
/// A `Slot` measurement belongs to a different report family and never carries
/// lifecycle rows, so it is returned exactly as it arrived. An uncarried error
/// is returned untouched too: its caller's fallback builds from the same
/// accumulator, so prepending here would duplicate every row.
///
/// The verification member is moved through UNCONDITIONALLY. This is the one
/// production site that takes a lifecycle measurement apart and builds another
/// one, so it is exactly where a stale stop's comparison could be silently
/// dropped on its way past an outer stage that only knew about rows.
///
/// ```
/// use vibe_orchestrator::failure::prepend_rows;
/// // An empty prefix is a no-op, and an uncarried error is returned exactly.
/// let untouched = prepend_rows(anyhow::Error::msg("plain"), Vec::new());
/// assert_eq!(untouched.to_string(), "plain");
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn prepend_rows(
    error: anyhow::Error,
    prefix: Vec<LifecycleContributionReport>,
) -> anyhow::Error {
    if prefix.is_empty() {
        return error;
    }
    match take::<Measurement>(error) {
        Ok(MeasuredFailure {
            original,
            evidence:
                Measurement::Lifecycle {
                    rows,
                    stopped_phase,
                    requested,
                    chain,
                    verification,
                },
            emit_machine_failure,
        }) => {
            let mut joined = prefix;
            joined.extend(rows);
            carry(MeasuredFailure {
                original,
                evidence: Measurement::Lifecycle {
                    rows: joined,
                    stopped_phase,
                    requested,
                    chain,
                    verification,
                },
                emit_machine_failure,
            })
        }
        Ok(carried) => carry(carried),
        Err(error) => error,
    }
}

#[cfg(test)]
#[path = "failure/tests.rs"]
mod tests;
