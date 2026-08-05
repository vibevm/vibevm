//! The kind-prefix validation law (PROP-008 §2.4 KIND-VALIDATION), in one
//! place. Every solver cell calls [`assert_kind_matches`] at the single
//! point it holds a pkgref — carrying its optional `kind` prefix — together
//! with the resolved manifest's declared `kind`: the naive cell in
//! `process_one` (which also covers `sat` via delegation), and the resolvo
//! cell in `solve_hard` (for root solvables; a resolvo solvable is interned
//! by `(group, name, version)` and drops a transitive dep's prefix at intern
//! time, so only a root's prefix survives to be checked there).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref");

use vibe_core::PackageKind;

use crate::SolveError;

/// Assert a pkgref's optional `kind` prefix matches the resolved package's
/// declared `kind`. `requested` `None` ⇒ nothing to check; equal ⇒ nothing
/// to report; otherwise it is a `KindMismatch` carrying the package identity
/// and both kinds. `package` is the qualified `<group>/<name>` identity.
pub(crate) fn assert_kind_matches(
    requested: Option<PackageKind>,
    actual: PackageKind,
    package: &str,
) -> Result<(), SolveError> {
    match requested {
        Some(req) if req != actual => Err(SolveError::KindMismatch {
            package: package.to_string(),
            requested: req,
            actual,
        }),
        _ => Ok(()),
    }
}
