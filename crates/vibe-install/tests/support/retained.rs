//! Retained-metadata planning bridge for integration-test install sources.
//!
//! Planning now solves through a source-aware, retained-candidate relation and
//! then acquires exactly the candidate it retained. A test double keeps its
//! fixtures in a real `<root>/<group>/<name>/v<version>/vibe.toml` directory
//! registry, solves through the production local-registry cell, and acquires
//! the retained candidate through its own acquisition logic for that exact
//! coordinate, so fault injection and fetch observation stay in the double.

use std::path::Path;

use vibe_core::PackageRef;
use vibe_registry::LocalRegistry;
use vibe_registry::source_candidates::{LocalCandidateKind, RetainedSourceCandidate};
use vibe_resolver::{
    DepProviderError, LocalRegistryDepProvider, ResolvoDepSolver, SolveError, SourceResolvedGraph,
    SourceSolveLimits, SourceSolveRequest,
};

/// The production operation budget for a local directory-registry solve.
pub fn limits() -> Result<SourceSolveLimits, SolveError> {
    Ok(SourceSolveLimits::standard_operation())
}

/// Solve `request` against the directory registry at `root` exactly as the
/// production explicit-registry cell does.
pub fn solve(
    root: &Path,
    request: &SourceSolveRequest<'_>,
) -> Result<SourceResolvedGraph, SolveError> {
    let local = LocalRegistry::new(root)
        .map_err(|error| SolveError::from(DepProviderError::Other(error.to_string())))?;
    ResolvoDepSolver::new(LocalRegistryDepProvider::with_source_context(
        &local,
        LocalCandidateKind::ExplicitRegistry,
        None,
    ))
    .solve_source_aware(request)
}

/// The retained candidate's exact coordinate (`group/name@=version`), used to
/// route acquisition through a double's own coordinate-keyed fetch logic.
pub fn exact(candidate: &RetainedSourceCandidate) -> PackageRef {
    PackageRef::parse(&format!(
        "{}/{}@={}",
        candidate.group(),
        candidate.name(),
        candidate.version()
    ))
    .expect("a retained candidate coordinate is a valid exact pkgref")
}
