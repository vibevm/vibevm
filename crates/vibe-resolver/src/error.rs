//! Solver-side error type — [`SolveError`] — what
//! [`DepSolver::solve`](crate::DepSolver) returns on failure. Extracted to
//! its own module so the public error surface stays addressable independently
//! of the solver machinery in [`crate`]; the variant set is closed and each
//! variant's `Display` embeds the `spec://` REQ it guards plus a fix hint.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");

use specmark::spec;
use thiserror::Error;
use vibe_core::PackageKind;

use crate::DepProviderError;

/// Solver-side failures. Messages name the conflict and the fix
/// surface — they are agent food, not just human prose:
///
/// ```
/// use vibe_resolver::SolveError;
///
/// let err = SolveError::VersionConflict {
///     package: "org.vibevm/wal".to_string(),
///     existing: "0.1.0".to_string(),
///     new_constraint: "^0.2".to_string(),
/// };
/// assert!(err.to_string().contains("version conflict on `org.vibevm/wal`"));
/// assert!(err.to_string().contains("[[override]]"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#unsatisfiable")]
pub enum SolveError {
    #[error(transparent)]
    Provider(#[from] DepProviderError),

    #[error(
        "version conflict on `{package}`: already chose `{existing}`, but \
         a later constraint requires `{new_constraint}`. Pin a single \
         constraint that satisfies both, or use `[[override]]` to break the tie. \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability; \
         fix: pin one [requires] constraint satisfying both, or add an [[override]])"
    )]
    VersionConflict {
        package: String,
        existing: String,
        new_constraint: String,
    },

    #[error(
        "package `{package}` declares `[conflicts]` against `{against}`, which \
         is also being installed in this graph \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability; \
         fix: remove one of the two packages, or drop the [conflicts] entry)"
    )]
    ConflictsDeclared { package: String, against: String },

    #[error(
        "capability `{capability}` required by `{requirer}` is not provided by \
         any package in the resolved graph. Add a package whose `[provides].capabilities` \
         includes `{capability}`, or pin a concrete `[requires].packages` entry. \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability; \
         fix: add a provider of the capability or a concrete [requires].packages entry)"
    )]
    CapabilityUnmet {
        capability: String,
        requirer: String,
    },

    #[error(
        "all alternatives in `[[requires_any]]` declared by `{requirer}` failed to \
         resolve: {alternatives:?} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability; \
         fix: make at least one `one_of` alternative resolvable)"
    )]
    DisjunctionUnsatisfiable {
        requirer: String,
        alternatives: Vec<String>,
    },

    /// The engine proved the graph unsatisfiable and produced a
    /// human-readable derivation. Carried verbatim from resolvo's
    /// `Conflict::display_user_friendly` (PROP-017 §2.4) — the
    /// "why did it fail" payload a raw UNSAT verdict cannot give.
    #[error(
        "dependency resolution is unsatisfiable:\n{explanation}\n\
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#unsatisfiable; \
         fix: relax a version constraint, drop a conflicting package, or accept a downgrade)"
    )]
    Unsatisfiable { explanation: String },

    /// A pkgref's `kind` prefix that disagrees with the resolved package's
    /// declared `kind` (PROP-008 §2.4 KIND-VALIDATION). The prefix is
    /// validation + a UX signal, never a disambiguator — `(group, name)` is
    /// already unique (§2.3 KIND-METADATA). Maps to exit code `4`
    /// (`TYPE_MISMATCH`) at the CLI boundary.
    #[error(
        "the pkgref `{package}` asked for kind `{requested}` but the package declares \
         `kind = \"{actual}\"` — the `kind` prefix is metadata, a validation signal \
         and not a disambiguator, so this is the package disagreeing with what you \
         wrote, not an ambiguity to resolve \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
         fix: drop the `{requested}:` prefix, or correct it to `{actual}:`)"
    )]
    KindMismatch {
        package: String,
        requested: PackageKind,
        actual: PackageKind,
    },
}
