//! `VibevmResolvoProvider` — the adapter that lets resolvo solve over a
//! vibevm [`VersionEnumerator`] world (PROP-017 §2).
//!
//! It implements resolvo's two traits: `Interner` (identity + display)
//! and the async `DependencyProvider` (candidates + dependencies). The
//! adapter holds a `resolvo::utils::Pool` for interning, lazily
//! enumerates a package's versions only when resolvo first asks, and
//! caches the manifests it fetches so the output graph can be built
//! without re-fetching. resolvo's callbacks cannot return a `Result`, so
//! the first provider error is stashed and surfaced after the solve.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use resolvo::utils::Pool;
use resolvo::{
    Candidates, Condition, ConditionId, Dependencies, DependencyProvider,
    HintDependenciesAvailable, Interner, KnownDependencies, NameId, SolvableId, SolverCache,
    StringId, VersionSetId, VersionSetUnionId,
};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, VersionSpec};

use super::capabilities::CapIndex;
use super::version_set::SemverVersionSet;
use crate::{DepProviderError, VersionEnumerator};

/// The qualified `"group/name"` string resolvo interns as a package name.
type PkgName = String;

/// resolvo adapter over a vibevm `VersionEnumerator`.
pub(crate) struct VibevmResolvoProvider<'p, P: VersionEnumerator> {
    provider: &'p P,
    /// capability → providers, pre-scanned from the package closure.
    cap_providers: CapIndex,
    pool: Pool<SemverVersionSet, PkgName>,
    /// Candidate solvables per package name, enumerated on first ask.
    candidates: RefCell<HashMap<NameId, Vec<SolvableId>>>,
    /// Manifests fetched during the solve, reused when building output.
    manifests: RefCell<HashMap<SolvableId, Manifest>>,
    /// First provider error seen mid-solve (callbacks can't return one).
    error: RefCell<Option<DepProviderError>>,
}

impl<'p, P: VersionEnumerator> VibevmResolvoProvider<'p, P> {
    pub(crate) fn new(provider: &'p P, cap_providers: CapIndex) -> Self {
        VibevmResolvoProvider {
            provider,
            cap_providers,
            pool: Pool::new(),
            candidates: RefCell::new(HashMap::new()),
            manifests: RefCell::new(HashMap::new()),
            error: RefCell::new(None),
        }
    }

    /// Intern (or look up) the `NameId` for a qualified package.
    pub(crate) fn intern_name(&self, group: &Group, name: &str) -> NameId {
        self.pool.intern_package_name(format!("{group}/{name}"))
    }

    /// Intern a version set "package `(group, name)` matching `spec`".
    pub(crate) fn intern_version_set(
        &self,
        group: &Group,
        name: &str,
        spec: &VersionSpec,
    ) -> VersionSetId {
        let name_id = self.intern_name(group, name);
        self.pool
            .intern_version_set(name_id, SemverVersionSet::from_spec(spec))
    }

    /// Intern the match-nothing version set for a package — the encoding
    /// of "this package must not be present", used by `[conflicts]`.
    fn intern_excluded(&self, group: &Group, name: &str) -> VersionSetId {
        let name_id = self.intern_name(group, name);
        self.pool
            .intern_version_set(name_id, SemverVersionSet::None)
    }

    /// Intern an explicit-version-set requirement — pins a capability
    /// provider to exactly the versions of it that provide the capability.
    fn intern_explicit(
        &self,
        group: &Group,
        name: &str,
        versions: BTreeSet<semver::Version>,
    ) -> VersionSetId {
        let name_id = self.intern_name(group, name);
        self.pool
            .intern_version_set(name_id, SemverVersionSet::Explicit(versions))
    }

    /// Parse a `NameId` back into `(group, name)`.
    fn name_parts(&self, name_id: NameId) -> Option<(Group, String)> {
        let qualified = self.pool.resolve_package_name(name_id);
        let (group, name) = qualified.rsplit_once('/')?;
        let group = Group::parse(group).ok()?;
        Some((group, name.to_string()))
    }

    /// `(group, name, version)` of a chosen solvable, for output building.
    pub(crate) fn solvable_parts(
        &self,
        id: SolvableId,
    ) -> Option<(Group, String, semver::Version)> {
        let solvable = self.pool.resolve_solvable(id);
        let (group, name) = self.name_parts(solvable.name)?;
        Some((group, name, solvable.record.clone()))
    }

    /// The chosen solvable's manifest — from the solve-time cache,
    /// falling back to a fresh fetch. Output building reads its
    /// `[requires.packages]` (for the graph edges) and `[obsoletes]`
    /// (to drop superseded nodes).
    pub(crate) fn manifest_of(
        &self,
        id: SolvableId,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        if let Some(m) = self.manifests.borrow().get(&id) {
            return Ok(m.clone());
        }
        self.provider.fetch_manifest(group, name, version)
    }

    /// Take the stashed provider error, if any.
    pub(crate) fn take_error(&self) -> Option<DepProviderError> {
        self.error.borrow_mut().take()
    }

    fn record_error(&self, err: DepProviderError) {
        let mut slot = self.error.borrow_mut();
        if slot.is_none() {
            *slot = Some(err);
        }
    }

    fn candidates_of(&self, solvables: Vec<SolvableId>) -> Candidates {
        Candidates {
            candidates: solvables,
            favored: None,
            locked: None,
            hint_dependencies_available: HintDependenciesAvailable::All,
            excluded: Vec::new(),
        }
    }
}

impl<'p, P: VersionEnumerator> Interner for VibevmResolvoProvider<'p, P> {
    type NameId = NameId;
    type SolvableId = SolvableId;

    fn display_solvable(&self, solvable: SolvableId) -> impl std::fmt::Display + '_ {
        let s = self.pool.resolve_solvable(solvable);
        let name = self.pool.resolve_package_name(s.name).clone();
        format!("{name}@{}", s.record)
    }

    fn display_name(&self, name: NameId) -> impl std::fmt::Display + '_ {
        self.pool.resolve_package_name(name).clone()
    }

    fn display_version_set(&self, version_set: VersionSetId) -> impl std::fmt::Display + '_ {
        self.pool.resolve_version_set(version_set).to_string()
    }

    fn display_string(&self, string_id: StringId) -> impl std::fmt::Display + '_ {
        self.pool.resolve_string(string_id).to_owned()
    }

    fn version_set_name(&self, version_set: VersionSetId) -> NameId {
        self.pool.resolve_version_set_package_name(version_set)
    }

    fn solvable_name(&self, solvable: SolvableId) -> NameId {
        self.pool.resolve_solvable(solvable).name
    }

    fn version_sets_in_union(
        &self,
        version_set_union: VersionSetUnionId,
    ) -> impl Iterator<Item = VersionSetId> {
        self.pool.resolve_version_set_union(version_set_union)
    }

    fn resolve_condition(&self, condition: ConditionId) -> Condition {
        self.pool.resolve_condition(condition).clone()
    }
}

impl<'p, P: VersionEnumerator> DependencyProvider for VibevmResolvoProvider<'p, P> {
    async fn filter_candidates(
        &self,
        candidates: &[SolvableId],
        version_set: VersionSetId,
        inverse: bool,
    ) -> Vec<SolvableId> {
        let set = self.pool.resolve_version_set(version_set).clone();
        candidates
            .iter()
            .copied()
            .filter(|&s| {
                let matched = set.contains(&self.pool.resolve_solvable(s).record);
                if inverse { !matched } else { matched }
            })
            .collect()
    }

    async fn get_candidates(&self, name: NameId) -> Option<Candidates> {
        if let Some(cached) = self.candidates.borrow().get(&name) {
            return Some(self.candidates_of(cached.clone()));
        }
        let Some((group, pkg)) = self.name_parts(name) else {
            self.record_error(DepProviderError::Other(
                "internal: a NameId did not resolve to `group/name`".to_string(),
            ));
            return None;
        };
        let versions = match self.provider.list_versions(&group, &pkg) {
            Ok(v) => v,
            // An absent package simply has no candidates: a hard
            // requirement on it becomes unsatisfiable, while a
            // `[[requires_any]]` disjunction falls back to its other
            // alternatives. Only a genuine provider failure is fatal.
            Err(
                DepProviderError::UnknownPackage { .. }
                | DepProviderError::NoMatchingVersion { .. }
                | DepProviderError::AggregateNotFound { .. },
            ) => Vec::new(),
            Err(e) => {
                self.record_error(e);
                return None;
            }
        };
        let solvables: Vec<SolvableId> = versions
            .into_iter()
            .map(|v| self.pool.intern_solvable(name, v))
            .collect();
        self.candidates.borrow_mut().insert(name, solvables.clone());
        Some(self.candidates_of(solvables))
    }

    async fn sort_candidates(&self, _cache: &SolverCache<Self>, solvables: &mut [SolvableId]) {
        // Highest version first → resolvo's first found solution is the
        // newest-feasible one (PROP-017 §3, §7 — "prefer newest" for a
        // sort, not a separate optimisation pass).
        solvables.sort_by(|&a, &b| {
            let va = self.pool.resolve_solvable(a).record.clone();
            let vb = self.pool.resolve_solvable(b).record.clone();
            vb.cmp(&va)
        });
    }

    async fn get_dependencies(&self, solvable: SolvableId) -> Dependencies {
        let Some((group, name, version)) = self.solvable_parts(solvable) else {
            return Dependencies::Unknown(
                self.pool.intern_string("internal: unresolvable solvable"),
            );
        };
        let manifest = match self.provider.fetch_manifest(&group, &name, &version) {
            Ok(m) => m,
            Err(e) => {
                self.record_error(e);
                return Dependencies::Unknown(self.pool.intern_string("manifest fetch failed"));
            }
        };

        let mut requirements = Vec::new();
        for dep in &manifest.requires.packages {
            let Some(dep_group) = dep.group.clone() else {
                self.record_error(DepProviderError::Other(format!(
                    "dependency `{}` of `{group}/{name}` is not group-qualified",
                    dep.name
                )));
                continue;
            };
            let vs = self.intern_version_set(&dep_group, dep.name.as_str(), &dep.version);
            requirements.push(vs.into());
        }

        // `[[requires_any]]` → a resolvo Union requirement: native OR
        // with backtracking (PROP-017 §3). naive takes the first option
        // and cannot reconsider; resolvo explores the alternatives.
        for disj in &manifest.requires_any {
            let mut alts = Vec::with_capacity(disj.one_of.len());
            for alt in &disj.one_of {
                let Some(alt_group) = alt.group.clone() else {
                    self.record_error(DepProviderError::Other(format!(
                        "`[[requires_any]]` alternative `{}` of `{group}/{name}` \
                         is not group-qualified",
                        alt.name
                    )));
                    continue;
                };
                alts.push(self.intern_version_set(&alt_group, alt.name.as_str(), &alt.version));
            }
            let mut it = alts.into_iter();
            match it.next() {
                Some(first) => {
                    let union = self.pool.intern_version_set_union(first, it);
                    requirements.push(union.into());
                }
                None => self.record_error(DepProviderError::Other(format!(
                    "`[[requires_any]]` declared by `{group}/{name}` has no \
                     group-qualified alternative"
                ))),
            }
        }

        // `[requires.capabilities]` → a Union over the pre-scanned
        // providers whose advertised version matches the requirement,
        // each pinned to exactly the versions that provide it (PROP-017
        // §3). naive can only satisfy a capability from packages already
        // pulled in; this pulls a provider in. No match → no requirement
        // here; `capabilities::verify` yields CapabilityUnmet post-solve.
        for cap_req in &manifest.requires.capabilities {
            let mut by_provider: HashMap<(Group, String), BTreeSet<semver::Version>> =
                HashMap::new();
            if let Some(providers) = self.cap_providers.get(&cap_req.qualified()) {
                for cp in providers {
                    if cap_req.version.matches(&cp.provided_version) {
                        by_provider
                            .entry((cp.group.clone(), cp.name.clone()))
                            .or_default()
                            .insert(cp.version.clone());
                    }
                }
            }
            let mut sorted: Vec<((Group, String), BTreeSet<semver::Version>)> =
                by_provider.into_iter().collect();
            sorted.sort_by(|a, b| {
                (a.0.0.as_str(), a.0.1.as_str()).cmp(&(b.0.0.as_str(), b.0.1.as_str()))
            });
            let vs_ids: Vec<VersionSetId> = sorted
                .into_iter()
                .map(|((pg, pn), versions)| self.intern_explicit(&pg, &pn, versions))
                .collect();
            let mut it = vs_ids.into_iter();
            if let Some(first) = it.next() {
                let union = self.pool.intern_version_set_union(first, it);
                requirements.push(union.into());
            }
        }

        // `[conflicts]` → resolvo `constrains` to the match-nothing set:
        // whole-package exclusion (the entry's version is ignored, as in
        // the naive cell). Selecting this package forbids the rival.
        let mut constrains = Vec::new();
        for c in &manifest.conflicts.packages {
            let Some(cg) = c.group.clone() else {
                self.record_error(DepProviderError::Other(format!(
                    "`[conflicts]` entry `{}` of `{group}/{name}` is not group-qualified",
                    c.name
                )));
                continue;
            };
            constrains.push(self.intern_excluded(&cg, c.name.as_str()));
        }

        self.manifests.borrow_mut().insert(solvable, manifest);
        Dependencies::Known(KnownDependencies {
            requirements,
            constrains,
        })
    }
}
