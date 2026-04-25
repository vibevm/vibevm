//! Dependency resolution for vibevm.
//!
//! Two traits and one implementation in this crate:
//!
//! - [`DepProvider`] — what the solver needs from the registry layer:
//!   pick a concrete version for a [`PackageRef`], read the manifest at
//!   that version. Implemented by [`MultiRegistryProvider`] for the
//!   production path; tests provide their own.
//! - [`DepSolver`] — what consumers (the install pipeline) call:
//!   resolve a list of root [`PackageRef`]s into a [`ResolvedGraph`]
//!   that includes transitive deps.
//! - [`NaiveDepSolver`] — a depth-first single-pass solver. Handles the
//!   straight-line cases that today's fixtures and any first-cut
//!   real-world dep graph hit. Pinned limitations:
//!   - **First-pick wins.** When a package is referenced from two paths
//!     with overlapping but different version constraints, the first
//!     pick is taken and the second constraint is checked against it;
//!     no constraint-narrowing-then-pick.
//!   - **Capabilities resolved against the already-seen graph.** A
//!     `[requires.capabilities]` entry must match a package already
//!     processed (or being added on the same path); the solver does
//!     not enumerate candidate packages that *might* provide it.
//!   - **Disjunctions take the first option.** `[[requires_any]]` picks
//!     the first `one_of` entry that resolves, no backtracking when a
//!     downstream conflict appears.
//!
//! When any of these limits start hurting real users — capability
//! routing across packages-not-yet-seen, optimal-version-after-merging
//! constraints, disjunction backtracking — that is the trigger for
//! adding a `ResolvoSolver` (PROP-002 §2.8 primary). The traits are
//! shaped so the swap is one new `impl DepSolver`, no consumer-side
//! changes. Same `GitBackend`-style indirection PROP-001 §2.2 uses to
//! leave the door open for `libsolv`.
//!
//! Spec: PROP-002 §2.8 (depsolver), §2.9 (capability vocabulary).

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};

use thiserror::Error;
use vibe_core::manifest::PackageManifest;
use vibe_core::{PackageKind, PackageRef, VersionSpec};

pub mod multi_registry_provider;
pub mod naive;

pub use multi_registry_provider::MultiRegistryProvider;
pub use naive::NaiveDepSolver;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One node in the resolved dependency graph.
#[derive(Debug, Clone)]
pub struct ResolvedNode {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    /// Direct dependencies of this node, pinned to exact versions chosen
    /// by the solver. Lockfile `dependencies` field is built from this.
    pub dependencies: Vec<PackageRef>,
    /// `true` iff the user directly asked for this package (a root in
    /// the input). Lockfile `[meta].root_dependencies` is built by
    /// pulling these out.
    pub is_root: bool,
}

/// The full resolved graph for one solver invocation.
#[derive(Debug, Clone, Default)]
pub struct ResolvedGraph {
    pub packages: Vec<ResolvedNode>,
}

impl ResolvedGraph {
    pub fn iter(&self) -> impl Iterator<Item = &ResolvedNode> {
        self.packages.iter()
    }

    pub fn roots(&self) -> impl Iterator<Item = &ResolvedNode> {
        self.packages.iter().filter(|n| n.is_root)
    }

    /// Find a node by `(kind, name)` identity.
    pub fn find(&self, kind: PackageKind, name: &str) -> Option<&ResolvedNode> {
        self.packages.iter().find(|n| n.kind == kind && n.name == name)
    }
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// What the solver needs to know about packages it's resolving over.
pub trait DepProvider {
    /// Pick a concrete version satisfying `pkgref.version` from the
    /// available versions of `(pkgref.kind, pkgref.name)`. Implementors
    /// fan out to multi-registry / mirror / override resolution as
    /// needed; the solver treats this as a black box.
    fn resolve_version(
        &self,
        pkgref: &PackageRef,
    ) -> Result<semver::Version, DepProviderError>;

    /// Read the package manifest at a specific version.
    fn fetch_manifest(
        &self,
        kind: PackageKind,
        name: &str,
        version: &semver::Version,
    ) -> Result<PackageManifest, DepProviderError>;
}

/// What the install / update pipeline calls.
pub trait DepSolver {
    /// Resolve `roots` into a transitive [`ResolvedGraph`].
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum DepProviderError {
    #[error("package `{kind}:{name}` is not available in any configured registry")]
    UnknownPackage { kind: PackageKind, name: String },

    #[error("no version of `{kind}:{name}` matches `{constraint}`")]
    NoMatchingVersion {
        kind: PackageKind,
        name: String,
        constraint: String,
    },

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum SolveError {
    #[error(transparent)]
    Provider(#[from] DepProviderError),

    #[error(
        "version conflict on `{package}`: already chose `{existing}`, but \
         a later constraint requires `{new_constraint}`. Pin a single \
         constraint that satisfies both, or use `[[override]]` to break the tie."
    )]
    VersionConflict {
        package: String,
        existing: String,
        new_constraint: String,
    },

    #[error(
        "package `{package}` declares `[conflicts]` against `{against}`, which \
         is also being installed in this graph"
    )]
    ConflictsDeclared { package: String, against: String },

    #[error(
        "capability `{capability}` required by `{requirer}` is not provided by \
         any package in the resolved graph. Add a package whose `[provides].capabilities` \
         includes `{capability}`, or pin a concrete `[requires].packages` entry."
    )]
    CapabilityUnmet { capability: String, requirer: String },

    #[error(
        "all alternatives in `[[requires_any]]` declared by `{requirer}` failed to \
         resolve: {alternatives:?}"
    )]
    DisjunctionUnsatisfiable {
        requirer: String,
        alternatives: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Helpers used by both the naive impl and (eventually) the resolvo impl.
// ---------------------------------------------------------------------------

/// Per-(kind,name) state the solver accumulates as it walks. `pub(crate)`
/// rather than module-private so the multiple solver impls in this crate
/// share one definition.
pub(crate) struct SolverState {
    pub chosen: HashMap<(PackageKind, String), ChosenEntry>,
    pub providers_index: HashMap<String, Vec<(PackageKind, String, semver::Version)>>,
    pub declared_conflicts: HashSet<(PackageKind, String)>,
    pub declared_obsolete: HashSet<(PackageKind, String)>,
    pub queue: VecDeque<EnqueuedPkg>,
}

pub(crate) struct ChosenEntry {
    pub version: semver::Version,
    pub manifest: PackageManifest,
    pub direct_deps: Vec<PackageRef>,
    pub is_root: bool,
}

pub(crate) struct EnqueuedPkg {
    pub pkgref: PackageRef,
    pub via: Option<String>,
    pub is_root: bool,
}

impl SolverState {
    pub(crate) fn new() -> Self {
        SolverState {
            chosen: HashMap::new(),
            providers_index: HashMap::new(),
            declared_conflicts: HashSet::new(),
            declared_obsolete: HashSet::new(),
            queue: VecDeque::new(),
        }
    }
}

/// `true` iff the version satisfies the spec.
pub(crate) fn version_satisfies(spec: &VersionSpec, version: &semver::Version) -> bool {
    spec.matches(version)
}
