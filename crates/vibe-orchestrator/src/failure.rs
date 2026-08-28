//! The one neutral carrier a measured failure travels outward on.
//!
//! ## Why it is neutral
//!
//! The same execution core is `vibe install`'s body, a phase verb's
//! prerequisite and `vibe update --all`'s delegate, and each of those three
//! outer commands reports a DIFFERENT registered root family. Choosing one here
//! would pick a family and be wrong for the other two, so this carrier names
//! none: it transports the measurement, the surface's own emission policy, and
//! the caller's error object, and the surface decides the family.
//!
//! ## Why it owns the original error
//!
//! The outer boundary must return the caller's error UNCHANGED — same downcast
//! identity for the exit code, same context chain for stderr. A carrier that
//! stored a formatted string could not do that, so it stores the object and the
//! boundary that takes it apart gets the object straight back out.
//!
//! This is the same transport the install resume seam has always used; it is
//! widened, not multiplied, so a boundary probes exactly once.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::fmt;

use specmark::spec;
use vibe_install::{InstallProgress, SlotLifecycleReport};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;

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
    },
}

/// A measured failure travelling outward: what the site measured, the exact
/// error object to return, and the emission policy that site already had.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct MeasuredFailure {
    /// The caller's error, unchanged — same downcast identity, same chain.
    pub original: anyhow::Error,
    /// What had really run when it stopped.
    pub measurement: Measurement,
    /// Whether this failure emitted its machine document when tracing was OFF.
    /// A property of the SITE and of the observer that owns it, never something
    /// inferred later from the error.
    pub emit_machine_failure: bool,
}

impl fmt::Display for MeasuredFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.original, formatter)
    }
}

impl std::error::Error for MeasuredFailure {
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
/// use vibe_orchestrator::failure::{Measurement, MeasuredFailure, carry, take};
///
/// let original = anyhow::Error::msg("the handler refused").context("phase `build` stopped");
/// let rendered = format!("{original:#}");
/// let carried = carry(MeasuredFailure {
///     original,
///     measurement: Measurement::Lifecycle {
///         rows: Vec::new(),
///         stopped_phase: "build".into(),
///         requested: "build".into(),
///         chain: vec!["build".into()],
///     },
///     emit_machine_failure: true,
/// });
/// let taken = take(carried).unwrap();
/// assert!(taken.emit_machine_failure);
/// assert_eq!(format!("{:#}", taken.original), rendered);
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn carry(failure: MeasuredFailure) -> anyhow::Error {
    anyhow::Error::new(failure)
}

/// Take a transported measured failure back out, exactly, or return the error
/// untouched.
///
/// Total, so a caller can branch without consuming an error it may need to pass
/// on unchanged. The struct-level example of [`carry`] demonstrates the round
/// trip.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn take(error: anyhow::Error) -> Result<MeasuredFailure, anyhow::Error> {
    error.downcast::<MeasuredFailure>()
}

/// Whether this error already carries a measurement.
///
/// Asked, rather than discovered by a failed `downcast`, so a site can branch
/// without consuming an error it may need to pass on untouched.
///
/// ```
/// use vibe_orchestrator::failure::is_measured;
/// assert!(!is_measured(&anyhow::Error::msg("plain")));
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn is_measured(error: &anyhow::Error) -> bool {
    error.is::<MeasuredFailure>()
}

/// Attach a measurement to an error that is not carrying one yet.
///
/// The site that runs contributions accumulates rows as it goes, and only ONE
/// of the ways it can fail (a handler that reported a failed transition) knows
/// to freeze them. Every other failure after that point — a state write, a park
/// reconciliation, a checkpoint — would otherwise return with the rows still
/// sitting in a local, and the report would claim the run did nothing when it
/// had already done several things successfully.
///
/// Idempotent: an error that already carries a measurement keeps the one its own
/// site froze, because that one is more specific than this.
///
/// ```
/// use vibe_orchestrator::failure::{Measurement, carry_measured, take};
///
/// let carried = carry_measured(anyhow::Error::msg("writing the checkpoint"), || {
///     Measurement::Lifecycle {
///         rows: Vec::new(),
///         stopped_phase: "build".into(),
///         requested: "build".into(),
///         chain: vec!["build".into()],
///     }
/// });
/// // A generic post-row failure was historically silent.
/// assert!(!take(carried).unwrap().emit_machine_failure);
/// ```
#[must_use]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn carry_measured(
    error: anyhow::Error,
    measurement: impl FnOnce() -> Measurement,
) -> anyhow::Error {
    if is_measured(&error) {
        return error;
    }
    // Historically silent: these failures never emitted a machine root with
    // tracing off, and adding one now would be a new document on an old path.
    carry(MeasuredFailure {
        original: error,
        measurement: measurement(),
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
    match take(error) {
        Ok(MeasuredFailure {
            original,
            measurement:
                Measurement::Lifecycle {
                    rows,
                    stopped_phase,
                    requested,
                    chain,
                },
            emit_machine_failure,
        }) => {
            let mut joined = prefix;
            joined.extend(rows);
            carry(MeasuredFailure {
                original,
                measurement: Measurement::Lifecycle {
                    rows: joined,
                    stopped_phase,
                    requested,
                    chain,
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
