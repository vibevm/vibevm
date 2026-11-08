//! Dependency resolution for vibevm.
//!
//! Two provider traits, one consumer trait, three solver cells:
//!
//! - [`DepProvider`] — what the solver needs from the registry layer:
//!   pick a concrete version for a [`PackageRef`], read the manifest at
//!   that version. Implemented by [`MultiRegistryDepProvider`] for the
//!   production path; tests provide their own. [`VersionEnumerator`]
//!   adds `list_versions` for the cells that pick among candidates
//!   themselves (PROP-017 §2.2).
//! - [`DepSolver`] — what consumers (the install pipeline) call:
//!   resolve a list of root [`PackageRef`]s into a [`ResolvedGraph`]
//!   that includes transitive deps.
//! - [`ResolvoDepSolver`] — resolvo (CDCL SAT) behind that seam, and the
//!   shipped default (`--solver` defaults to `resolvo`; PROP-017).
//! - [`SatDepSolver`](sat::SatDepSolver) — chronological backtracking over version bounds,
//!   with the naive solve as its branch checker.
//! - [`NaiveDepSolver`] — a depth-first single-pass solver, kept as the
//!   small-graph fast path and the differential oracle's reference cell
//!   (PROP-017 §6). Pinned limitations, and why it is not the default:
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
//! Those three limits were the trigger list for adding a `ResolvoSolver`
//! (PROP-002 §2.8 primary), and **it fired**: decided 2026-06-14, port
//! complete (PROP-017 §6). Constraint-merging and disjunction
//! backtracking are resolvo's by construction (`[[requires_any]]` → a
//! resolvo `Union`, PROP-017 §3); capability routing now reaches any
//! provider in the transitive closure via [`resolvo_engine`]'s pre-scan,
//! stronger than naive's already-seen match but still blind to a
//! provider no package edge reaches — the one part of the trigger still
//! live, and it now points at a registry reverse-index (PROP-017 §8),
//! not at a solver. The swap cost what the seam promised consumer-side —
//! one new `impl DepSolver`, no consumer, manifest, or lockfile change
//! (PROP-017 §5) — plus one world-side enrichment,
//! [`VersionEnumerator`]. The `GitBackend`-style indirection PROP-001
//! §2.2 uses still holds the door open for `libsolv`, which PROP-002
//! §2.8 keeps as the feature-gated fallback; that trigger has not fired.
//!
//! Spec: PROP-002 §2.8 (depsolver), §2.9 (capability vocabulary);
//! PROP-017 (the resolvo port).

#![forbid(unsafe_code)]

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};

pub mod activation;
pub mod conditional;
pub mod embedded_provider;
pub mod features;
pub mod fixpoint_model;
pub mod local_composite_provider;
pub mod local_registry_provider;
pub mod multi_registry_provider;
pub mod source_candidates;
pub mod source_edges;
mod source_solve;
pub use source_candidates::{
    CandidateKey, CandidateMetadata, HistoricalActivation, RootRequirement,
    SourceCandidateProvider, SourceResolvedGraph, SourceSolveLimits, SourceSolveRequest,
};
pub mod naive;
pub mod resolvo_engine;
pub mod sat;

mod error;
mod kind_check;

pub use activation::{ActivationContext, ActivationOutcome, CapabilityTag, TagError};
pub use embedded_provider::{EmbeddedDepProvider, EmbeddedPrecedence};
pub use features::{
    FeatureError, FeatureExpansion, FeatureRequest, FeatureValue, expand_features,
    validate_features_table,
};
pub use local_composite_provider::LocalCompositeDepProvider;
pub use local_registry_provider::LocalRegistryDepProvider;
pub use multi_registry_provider::MultiRegistryDepProvider;
pub use naive::NaiveDepSolver;
pub use resolvo_engine::ResolvoDepSolver;
pub mod configuration;
mod visibility_mask;
pub use visibility_mask::mask_manifest_edges;

pub use error::SolveError;
pub(crate) use kind_check::assert_kind_matches;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One node in the resolved dependency graph.
///
/// Identity is the `(group, name)` tuple plus the solver-chosen exact
/// `version`; `kind` is metadata and deliberately not carried here:
///
/// ```
/// use vibe_core::Group;
/// use vibe_resolver::ResolvedNode;
///
/// let node = ResolvedNode {
///     group: Group::parse("org.vibevm").unwrap(),
///     name: "wal".to_string(),
///     version: semver::Version::parse("0.1.0").unwrap(),
///     dependencies: vec![],
///     is_root: true,
/// };
/// assert_eq!(node.group.as_str(), "org.vibevm");
/// assert!(node.is_root);
/// ```
#[derive(Debug, Clone)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile")]
pub struct ResolvedNode {
    /// Reverse-FQDN group — half of the `(group, name)` identity tuple
    /// (PROP-008 §2.2). `kind` is pure metadata and is not carried here.
    pub group: Group,
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
///
/// Query it by `(group, name)` identity; roots are the nodes the user
/// asked for directly:
///
/// ```
/// use vibe_core::Group;
/// use vibe_resolver::{ResolvedGraph, ResolvedNode};
///
/// let org = Group::parse("org.vibevm").unwrap();
/// let graph = ResolvedGraph {
///     packages: vec![ResolvedNode {
///         group: org.clone(),
///         name: "wal".to_string(),
///         version: semver::Version::parse("0.1.0").unwrap(),
///         dependencies: vec![],
///         is_root: true,
///     }],
/// };
/// assert!(graph.find(&org, "wal").is_some());
/// assert_eq!(graph.roots().count(), 1);
/// ```
#[derive(Debug, Clone, Default)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#solver-upgrade")]
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

    /// Find a node by `(group, name)` identity.
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity")]
    pub fn find(&self, group: &Group, name: &str) -> Option<&ResolvedNode> {
        self.packages
            .iter()
            .find(|n| &n.group == group && n.name == name)
    }
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// What the solver needs to know about packages it's resolving over.
///
/// The canonical implementation shape — answer the two questions from
/// whatever backing store you have:
///
/// ```
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_resolver::{DepProvider, DepProviderError};
///
/// struct OnePackage(Manifest);
///
/// impl DepProvider for OnePackage {
///     fn resolve_version(&self, pkgref: &PackageRef)
///         -> Result<semver::Version, DepProviderError>
///     {
///         Ok(self.0.require_package().unwrap().version.clone())
///     }
///     fn fetch_manifest(&self, _: &Group, _: &str, _: &semver::Version)
///         -> Result<Manifest, DepProviderError>
///     {
///         Ok(self.0.clone())
///     }
/// }
///
/// let m = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let provider = OnePackage(m);
/// let picked = provider
///     .resolve_version(&PackageRef::parse("org.vibevm/wal").unwrap())
///     .unwrap();
/// assert_eq!(picked, semver::Version::parse("0.1.0").unwrap());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver")]
pub trait DepProvider {
    /// Observe a successfully selected graph before the incumbent obsoletes view.
    /// Default is inert; retained metadata adapters use it only as solve evidence.
    fn observe_selected_before_projection(
        &self,
        _graph: &ResolvedGraph,
    ) -> Result<(), DepProviderError> {
        Ok(())
    }

    /// Pick a concrete version satisfying `pkgref.version` from the
    /// available versions of `(pkgref.kind, pkgref.name)`. Implementors
    /// fan out to multi-registry / mirror / override resolution as
    /// needed; the solver treats this as a black box.
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError>;

    /// Read the package manifest at a specific version.
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError>;

    /// Validate the selected graph without changing candidate selection. A
    /// missing configuration answer is returned here, not encoded as permission
    /// to backtrack to an older question-free definition. Called per hard solve,
    /// so a declined optional recommendation does not corrupt the retained graph.
    fn validate_selected(&self, _graph: &ResolvedGraph) -> Result<(), DepProviderError> {
        Ok(())
    }
}

/// A [`DepProvider`] that can also enumerate every available version of a
/// package — the capability a candidate-choosing solver needs beyond the
/// pull-one [`DepProvider::resolve_version`]. `NaiveDepSolver` and the
/// [`sat`](crate::sat) cell select a version *inside* the provider;
/// `ResolvoDepSolver` enumerates candidates and selects among them itself,
/// so it takes `P: VersionEnumerator`.
///
/// ```
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_resolver::{DepProvider, DepProviderError, VersionEnumerator};
///
/// struct OnePackage(Manifest);
/// impl DepProvider for OnePackage {
///     fn resolve_version(&self, _: &PackageRef)
///         -> Result<semver::Version, DepProviderError>
///     { Ok(self.0.require_package().unwrap().version.clone()) }
///     fn fetch_manifest(&self, _: &Group, _: &str, _: &semver::Version)
///         -> Result<Manifest, DepProviderError>
///     { Ok(self.0.clone()) }
/// }
/// impl VersionEnumerator for OnePackage {
///     fn list_versions(&self, _: &Group, _: &str)
///         -> Result<Vec<semver::Version>, DepProviderError>
///     { Ok(vec![self.0.require_package().unwrap().version.clone()]) }
/// }
///
/// let m = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let versions = OnePackage(m)
///     .list_versions(&Group::parse("org.vibevm").unwrap(), "wal")
///     .unwrap();
/// assert_eq!(versions, vec![semver::Version::parse("0.1.0").unwrap()]);
/// ```
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#provider-enrichment"
)]
pub trait VersionEnumerator: DepProvider {
    /// All available versions of `(group, name)`, in any order — the
    /// solver sorts. Backed by `Registry::list_versions` in production.
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError>;
}

/// What the install / update pipeline calls.
///
/// The canonical use — pick a solver cell, hand it roots, get the
/// transitive graph (here over a single self-contained package):
///
/// ```
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_resolver::{DepProvider, DepProviderError, DepSolver, NaiveDepSolver};
///
/// struct OnePackage(Manifest);
/// impl DepProvider for OnePackage {
///     fn resolve_version(&self, _: &PackageRef)
///         -> Result<semver::Version, DepProviderError>
///     {
///         Ok(self.0.require_package().unwrap().version.clone())
///     }
///     fn fetch_manifest(&self, _: &Group, _: &str, _: &semver::Version)
///         -> Result<Manifest, DepProviderError>
///     {
///         Ok(self.0.clone())
///     }
/// }
///
/// let m = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let solver = NaiveDepSolver::new(OnePackage(m));
/// let graph = solver
///     .solve(&[PackageRef::parse("org.vibevm/wal").unwrap()])
///     .unwrap();
/// assert_eq!(graph.packages.len(), 1);
/// assert!(graph.packages[0].is_root);
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver")]
#[spec(
    deviates = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#solver-upgrade",
    reason = "PROP-003 §2.1 adds `pin_preferences(&mut self, pins)` to this trait for \
              minimum-churn re-resolution; the method is still absent. PROP-011 Phase 3 \
              holds pins at the install layer instead — `freshness::hold_pins` rewrites \
              every unchanged root to `=<locked>` before the solve — so the property \
              ships without the trait method, at the cost of a registry walk the solver \
              could have skipped. It stops being acceptable when a re-resolve needs \
              preference *inside* the solve; resolvo's `sort_candidates` (PROP-017 §3) \
              is where that would land"
)]
pub trait DepSolver {
    /// Resolve `roots` into a transitive [`ResolvedGraph`].
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Provider-side failures, discriminated so the install layer can
/// route them (PROP-002 §2.8 failure discriminator).
///
/// ```
/// use vibe_core::Group;
/// use vibe_resolver::DepProviderError;
///
/// let err = DepProviderError::UnknownPackage {
///     group: Group::parse("org.vibevm").unwrap(),
///     name: "nope".to_string(),
/// };
/// assert_eq!(
///     err.to_string(),
///     "package `org.vibevm/nope` is not available in any configured registry \
///      (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
///      fix: check the name, or add the registry that hosts it to [[registry]])",
/// );
/// ```
#[derive(Debug, Clone, Error)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
)]
pub enum DepProviderError {
    #[error(transparent)]
    SourcePolicy(#[from] vibe_core::manifest::SourcePolicyError),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver: retained metadata count {observed} exceeds {limit}; fix surface: operation source budget"
    )]
    SourceMetadataLimit { observed: usize, limit: usize },

    #[error(
        "source-aware dependency discovery requires a fresh retained-candidate solve before acquisition \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver; \
         fix surface: restart the solve over the retained source candidates)"
    )]
    SourceAwareRestart,
    #[error(transparent)]
    Configuration(#[from] vibe_core::manifest::PackageConfigurationError),

    #[error(
        "package `{group}/{name}` is not available in any configured registry \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: check the name, or add the registry that hosts it to [[registry]])"
    )]
    UnknownPackage { group: Group, name: String },

    #[error(
        "no version of `{group}/{name}` matches `{constraint}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: relax the version constraint or publish a matching release)"
    )]
    NoMatchingVersion {
        group: Group,
        name: String,
        constraint: String,
    },

    /// Aggregate-walk variant of `UnknownPackage` — same "not
    /// available" verdict, but with structured per-registry
    /// `attempts` for downstream JSON / programmatic consumers.
    /// `vibe-cli` `install`'s error renderer downcasts through the
    /// anyhow chain to this variant, attaching `attempts` to the
    /// JSON error envelope when present. The text-mode
    /// `Display` carries the same multi-line summary the
    /// underlying `RegistryError::PackageNotFoundEverywhere`
    /// produces, so prose-only consumers see no regression.
    #[error(
        "{summary} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: check the name, or add the registry that hosts it to [[registry]])"
    )]
    AggregateNotFound {
        group: Group,
        name: String,
        summary: String,
        attempts: Vec<vibe_registry::RegistryWalkAttempt>,
    },

    /// The offline posture (PROP-010 `#offline`): this provider's local
    /// sources and the machine store do not hold the coordinate. Composition
    /// treats it as this family's absence so an earlier or later local family
    /// (a project-local `vibepacks/`) can still serve it; when no family does,
    /// it is the verdict. `message` is the registry's own sentence.
    #[error(
        "{message} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: act on the underlying provider failure named in the message)"
    )]
    OfflineUnavailable {
        group: Group,
        name: String,
        message: String,
    },

    #[error(
        "{0} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: act on the underlying provider failure named in the message)"
    )]
    Other(String),
}

impl DepProviderError {
    /// Carry a registry's offline miss as [`Self::OfflineUnavailable`]; any
    /// other registry error stays [`Self::Other`].
    pub(crate) fn offline_unavailable(error: vibe_registry::RegistryError) -> Self {
        let message = error.to_string();
        match error {
            vibe_registry::RegistryError::OfflinePackageUnavailable { group, name, .. } => {
                Self::OfflineUnavailable {
                    group,
                    name,
                    message,
                }
            }
            _ => Self::Other(message),
        }
    }
}

// Solver-side failures live in `error::SolveError` (re-exported above as
// `pub use error::SolveError`), so the public error surface is addressable
// independently of the solver machinery in this file.

// Helpers used by every solver cell live in `solver_support` (split for
// the file-length budget; same items, same `crate::` paths).
mod solver_support;
pub(crate) use solver_support::{
    Chosen, ChosenEntry, EnqueuedPkg, SolverState, build_resolved_graph, version_satisfies,
};

mod source_composition;
