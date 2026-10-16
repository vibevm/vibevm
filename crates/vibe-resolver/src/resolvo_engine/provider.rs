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
use vibe_core::{Group, PackageRef, VersionSpec};

use super::capabilities::CapIndex;
use super::version_set::SemverVersionSet;
use crate::{DepProviderError, VersionEnumerator};

/// The qualified `"group/name"` string resolvo interns as a package name.
type PkgName = String;

/// A solver-selected witness for one disjunction declaration. Its target is
/// a real version set; the witness is an internal literal, never a package.
#[derive(Clone)]
struct Choice {
    parent: SolvableId,
    declaration: String,
    alternative: usize,
    target: VersionSetId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedChoice {
    pub parent: PackageRef,
    pub declaration: String,
    pub alternative: usize,
    pub provider: PackageRef,
}

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
    choices: RefCell<HashMap<SolvableId, Choice>>,
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
            choices: RefCell::new(HashMap::new()),
        }
    }

    fn choice_requirement(
        &self,
        parent: SolvableId,
        declaration: String,
        targets: Vec<VersionSetId>,
    ) -> VersionSetId {
        // NUL cannot occur in an admitted package coordinate. The name is
        // local to this provider/solve and cannot escape into the graph.
        let name = self
            .pool
            .intern_package_name(format!("\0choice:{parent:?}:{declaration}"));
        let mut candidates = Vec::new();
        let (group, package, version) = self.solvable_parts(parent).expect("choice parent is real");
        let retained = self.provider.retained_choices().iter().find(|e| {
            e.parent.group.as_ref() == Some(&group)
                && e.parent.name == package
                && e.parent.version.matches(&version)
                && e.declaration == declaration
        });
        for (alternative, target) in targets.into_iter().enumerate() {
            let target_name = self.pool.resolve_version_set_package_name(target);
            let Some((target_group, target_package)) = self.name_parts(target_name) else {
                continue;
            };
            if self.provider.edge_blocked(
                &format!("{group}/{package}"),
                &format!("{target_group}/{target_package}"),
            ) {
                continue;
            }
            if retained.is_some_and(|e| {
                e.provider.group.as_ref() != Some(&target_group)
                    || e.provider.name != target_package
            }) {
                continue;
            }
            let target = if let Some(edge) = retained {
                let pinned = match &edge.provider.version {
                    VersionSpec::Req(req) => req.comparators.first().map(|c| {
                        let mut version = semver::Version::new(
                            c.major,
                            c.minor.unwrap_or(0),
                            c.patch.unwrap_or(0),
                        );
                        version.pre = c.pre.clone();
                        version
                    }),
                    _ => None,
                };
                let Some(pinned) =
                    pinned.filter(|v| self.pool.resolve_version_set(target).contains(v))
                else {
                    continue;
                };
                self.intern_explicit(&target_group, &target_package, BTreeSet::from([pinned]))
            } else {
                target
            };
            let id = self
                .pool
                .intern_solvable(name, semver::Version::new(0, 0, alternative as u64));
            self.choices.borrow_mut().insert(
                id,
                Choice {
                    parent,
                    declaration: declaration.clone(),
                    alternative,
                    target,
                },
            );
            candidates.push(id);
        }
        self.candidates.borrow_mut().insert(name, candidates);
        self.pool
            .intern_version_set(name, SemverVersionSet::from_spec(&VersionSpec::Latest))
    }

    fn choice_blocked(&self, parent: SolvableId, declaration: &str) -> bool {
        let Some((group, name, version)) = self.solvable_parts(parent) else {
            return false;
        };
        self.provider.retained_choices().iter().any(|e| {
            e.parent.group.as_ref() == Some(&group)
                && e.parent.name == name
                && e.parent.version.matches(&version)
                && e.declaration == declaration
                && self
                    .provider
                    .edge_blocked(&format!("{group}/{name}"), &e.provider.qualified_name())
        })
    }

    pub(crate) fn is_choice(&self, id: SolvableId) -> bool {
        self.choices.borrow().contains_key(&id)
    }

    pub(crate) fn selected_choices(
        &self,
        selected: &[SolvableId],
    ) -> Result<Vec<SelectedChoice>, DepProviderError> {
        let pin = |id| {
            let (group, name, version) = self.solvable_parts(id).ok_or_else(|| {
                DepProviderError::Other("selected choice has no package coordinate".into())
            })?;
            PackageRef::parse(&format!("{group}/{name}@={version}"))
                .map_err(|e| DepProviderError::Other(e.to_string()))
        };
        let mut edges = Vec::new();
        for id in selected {
            let Some(choice) = self.choices.borrow().get(id).cloned() else {
                continue;
            };
            if !selected.contains(&choice.parent) {
                return Err(DepProviderError::Other(
                    "selected witness lost its declaring candidate".into(),
                ));
            }
            let target_name = self.pool.resolve_version_set_package_name(choice.target);
            let target_set = self.pool.resolve_version_set(choice.target);
            let targets: Vec<_> = selected
                .iter()
                .copied()
                .filter(|id| {
                    let solvable = self.pool.resolve_solvable(*id);
                    solvable.name == target_name && target_set.contains(&solvable.record)
                })
                .collect();
            if targets.len() != 1 {
                return Err(DepProviderError::Other(
                    "selected witness has no unique selected provider".into(),
                ));
            }
            edges.push(SelectedChoice {
                parent: pin(choice.parent)?,
                declaration: choice.declaration,
                alternative: choice.alternative,
                provider: pin(targets[0])?,
            });
        }
        edges.sort_by_key(|e| (e.parent.to_string(), e.declaration.clone(), e.alternative));
        Ok(edges)
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
        if solvables.first().is_some_and(|id| self.is_choice(*id)) {
            // Preserve authored alternative order as a preference only; SAT
            // may backtrack and the returned witness records the actual choice.
            return;
        }
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
        if let Some(choice) = self.choices.borrow().get(&solvable).cloned() {
            let parent = self.pool.resolve_solvable(choice.parent);
            let parent_set = self.pool.intern_version_set(
                parent.name,
                SemverVersionSet::Explicit(BTreeSet::from([parent.record.clone()])),
            );
            return Dependencies::Known(KnownDependencies {
                requirements: vec![choice.target.into(), parent_set.into()],
                constrains: Vec::new(),
            });
        }
        let Some((group, name, version)) = self.solvable_parts(solvable) else {
            return Dependencies::Unknown(
                self.pool.intern_string("internal: unresolvable solvable"),
            );
        };
        if let Some(reason) = self.provider.candidate_rejection(&group, &name, &version) {
            return Dependencies::Unknown(self.pool.intern_string(reason));
        }
        let manifest = match self.provider.fetch_manifest(&group, &name, &version) {
            Ok(m) => m,
            Err(e) => {
                self.record_error(e);
                return Dependencies::Unknown(self.pool.intern_string("manifest fetch failed"));
            }
        };

        let mut requirements = Vec::new();
        for (index, dep) in manifest.requires.packages.iter().enumerate() {
            if self
                .provider
                .edge_blocked(&format!("{group}/{name}"), &dep.qualified_name())
            {
                continue;
            }
            let Some(dep_group) = dep.group.clone() else {
                self.record_error(DepProviderError::Other(format!(
                    "dependency `{}` of `{group}/{name}` is not group-qualified",
                    dep.name
                )));
                continue;
            };
            let retained = self.provider.retained_choices().iter().find(|e| {
                e.parent.group.as_ref() == Some(&group)
                    && e.parent.name == name
                    && e.parent.version.matches(&version)
                    && e.declaration == format!("requires.packages[{index}]")
            });
            if let Some(edge) = retained {
                requirements.push(
                    self.intern_version_set(&dep_group, dep.name.as_str(), &edge.provider.version)
                        .into(),
                );
            }
            let vs = self.intern_version_set(&dep_group, dep.name.as_str(), &dep.version);
            requirements.push(vs.into());
        }

        // `[[requires_any]]` → a resolvo Union requirement: native OR
        // with backtracking (PROP-017 §3). naive takes the first option
        // and cannot reconsider; resolvo explores the alternatives.
        for (index, disj) in manifest.requires_any.iter().enumerate() {
            if self.choice_blocked(solvable, &format!("requires_any[{index}]")) {
                continue;
            }
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
            if alts.is_empty() {
                self.record_error(DepProviderError::Other(format!(
                    "`[[requires_any]]` declared by `{group}/{name}` has no group-qualified alternative"
                )));
            } else {
                requirements.push(
                    self.choice_requirement(solvable, format!("requires_any[{index}]"), alts)
                        .into(),
                );
            }
        }

        // `[requires.capabilities]` → a Union over the pre-scanned
        // providers whose advertised version matches the requirement,
        // each pinned to exactly the versions that provide it (PROP-017
        // §3). naive can only satisfy a capability from packages already
        // pulled in; this pulls a provider in. No match → no requirement
        // here; `capabilities::verify` yields CapabilityUnmet post-solve.
        for (index, cap_req) in manifest.requires.capabilities.iter().enumerate() {
            if self.choice_blocked(
                solvable,
                &format!("requires.capabilities[{index}]:{cap_req}"),
            ) {
                continue;
            }
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
            if !vs_ids.is_empty() {
                requirements.push(
                    self.choice_requirement(
                        solvable,
                        format!("requires.capabilities[{index}]:{cap_req}"),
                        vs_ids,
                    )
                    .into(),
                );
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
