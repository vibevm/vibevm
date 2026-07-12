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

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef, VersionSpec};

pub mod activation;
pub mod conditional;
pub mod features;
pub mod fixpoint_model;
pub mod local_registry_provider;
pub mod multi_registry_provider;
pub mod naive;
pub mod resolvo_engine;
pub mod sat;

pub use activation::{ActivationContext, ActivationOutcome, CapabilityTag, TagError};
pub use features::{
    FeatureError, FeatureExpansion, FeatureRequest, FeatureValue, expand_features,
    validate_features_table,
};
pub use local_registry_provider::LocalRegistryProvider;
pub use multi_registry_provider::MultiRegistryProvider;
pub use naive::NaiveDepSolver;
pub use resolvo_engine::ResolvoDepSolver;

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
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-008#identity")]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
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
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade")]
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
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-008#identity")]
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
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#solver")]
pub trait DepProvider {
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
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#provider-enrichment")]
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
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#solver")]
#[spec(
    deviates = "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade",
    reason = "PROP-003 §2.1 adds `pin_preferences(&mut self, pins)` to this trait for \
              minimum-churn re-resolution; the method is absent — PROP-011 Phase 3 \
              holds pins via constraint-tightening at the install layer instead, and \
              SatDepSolver is not in tree (see DBT-0011)"
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
///      (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
///      fix: check the name, or add the registry that hosts it to [[registry]])",
/// );
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
pub enum DepProviderError {
    #[error(
        "package `{group}/{name}` is not available in any configured registry \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: check the name, or add the registry that hosts it to [[registry]])"
    )]
    UnknownPackage { group: Group, name: String },

    #[error(
        "no version of `{group}/{name}` matches `{constraint}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
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
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: check the name, or add the registry that hosts it to [[registry]])"
    )]
    AggregateNotFound {
        group: Group,
        name: String,
        summary: String,
        attempts: Vec<vibe_registry::RegistryWalkAttempt>,
    },

    #[error(
        "{0} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
         fix: act on the underlying provider failure named in the message)"
    )]
    Other(String),
}

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
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#capability")]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#unsatisfiable")]
pub enum SolveError {
    #[error(transparent)]
    Provider(#[from] DepProviderError),

    #[error(
        "version conflict on `{package}`: already chose `{existing}`, but \
         a later constraint requires `{new_constraint}`. Pin a single \
         constraint that satisfies both, or use `[[override]]` to break the tie. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#capability; \
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
         (violates spec://vibevm/modules/vibe-registry/PROP-002#capability; \
         fix: remove one of the two packages, or drop the [conflicts] entry)"
    )]
    ConflictsDeclared { package: String, against: String },

    #[error(
        "capability `{capability}` required by `{requirer}` is not provided by \
         any package in the resolved graph. Add a package whose `[provides].capabilities` \
         includes `{capability}`, or pin a concrete `[requires].packages` entry. \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#capability; \
         fix: add a provider of the capability or a concrete [requires].packages entry)"
    )]
    CapabilityUnmet {
        capability: String,
        requirer: String,
    },

    #[error(
        "all alternatives in `[[requires_any]]` declared by `{requirer}` failed to \
         resolve: {alternatives:?} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#capability; \
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
         (violates spec://vibevm/modules/vibe-resolver/PROP-017#unsatisfiable; \
         fix: relax a version constraint, drop a conflicting package, or accept a downgrade)"
    )]
    Unsatisfiable { explanation: String },
}

// ---------------------------------------------------------------------------
// Helpers used by both the naive impl and (eventually) the resolvo impl.
// ---------------------------------------------------------------------------

/// Per-(group,name) state the solver accumulates as it walks. `pub(crate)`
/// rather than module-private so the multiple solver impls in this crate
/// share one definition.
pub(crate) struct SolverState {
    pub chosen: HashMap<(Group, String), ChosenEntry>,
    pub providers_index: HashMap<String, Vec<(Group, String, semver::Version)>>,
    pub declared_conflicts: HashSet<(Group, String)>,
    pub declared_obsolete: HashSet<(Group, String)>,
    pub queue: VecDeque<EnqueuedPkg>,
}

pub(crate) struct ChosenEntry {
    pub version: semver::Version,
    pub manifest: Manifest,
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

/// One solver-chosen package, reduced to what the output builder needs.
/// Both `NaiveDepSolver` and `ResolvoDepSolver` collapse their internal
/// state into this before calling [`build_resolved_graph`].
pub(crate) struct Chosen {
    pub version: semver::Version,
    pub direct_deps: Vec<PackageRef>,
    pub is_root: bool,
}

/// Build the final [`ResolvedGraph`] from a solver's choices — the single
/// home of the output contract (PROP-017 §2.3): drop obsoleted entries,
/// list roots first in input order then the rest sorted for determinism,
/// and exact-pin every dependency edge to the version chosen for it. Both
/// solver cells route through here, so they produce byte-identical graphs
/// — which is exactly what the differential oracle asserts.
pub(crate) fn build_resolved_graph(
    root_order: &[(Group, String)],
    mut chosen: HashMap<(Group, String), Chosen>,
    obsolete: &HashSet<(Group, String)>,
) -> ResolvedGraph {
    for ob in obsolete {
        chosen.remove(ob);
    }

    let mut packages: Vec<ResolvedNode> = Vec::with_capacity(chosen.len());
    // Roots first, input order preserved.
    for (group, name) in root_order {
        if let Some(entry) = chosen.remove(&(group.clone(), name.clone())) {
            packages.push(node_from_chosen(group.clone(), name.clone(), entry, true));
        }
    }
    // The rest, sorted by (group, name) for a deterministic tail.
    let mut rest: Vec<((Group, String), Chosen)> = chosen.into_iter().collect();
    rest.sort_by(|a, b| (a.0.0.as_str(), a.0.1.as_str()).cmp(&(b.0.0.as_str(), b.0.1.as_str())));
    for ((group, name), entry) in rest {
        packages.push(node_from_chosen(group, name, entry, false));
    }

    // Exact-pin every dependency reference to the chosen version, so a
    // later `vibe install` off this lockfile reproduces it bit-for-bit.
    let resolved_versions: HashMap<(Group, String), semver::Version> = packages
        .iter()
        .map(|n| ((n.group.clone(), n.name.clone()), n.version.clone()))
        .collect();
    for node in packages.iter_mut() {
        for dep in node.dependencies.iter_mut() {
            if let Some(group) = dep.group.clone()
                && let Some(version) = resolved_versions.get(&(group, dep.name.to_string()))
                && let Ok(req) = semver::VersionReq::parse(&format!("={version}"))
            {
                *dep = PackageRef {
                    kind: dep.kind,
                    group: dep.group.clone(),
                    name: dep.name.clone(),
                    version: VersionSpec::Req(req),
                };
            }
        }
    }

    ResolvedGraph { packages }
}

fn node_from_chosen(group: Group, name: String, entry: Chosen, is_root: bool) -> ResolvedNode {
    ResolvedNode {
        group,
        name,
        version: entry.version,
        dependencies: entry.direct_deps,
        is_root: is_root || entry.is_root,
    }
}
