//! Naive depth-first dependency solver.
//!
//! Single pass, no backtracking. See [`crate`] module docs for the
//! pinned limitations and when to upgrade to a SAT-style solver.

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{CapabilityRef, Group, PackageRef, VersionSpec};

use crate::{
    ChosenEntry, DepProvider, DepProviderError, DepSolver, EnqueuedPkg, ResolvedGraph, SolveError,
    SolverState, version_satisfies,
};

/// Extract the `(group, name)` identity from a pkgref. Solver-internal
/// refs — roots, `[requires.packages]` deps, `[conflicts]` / `[obsoletes]`
/// / `[[requires_any]]` entries — are all group-qualified (PROP-008 §2.6
/// makes every manifest pkgref carry a group); an unqualified ref reaching
/// the solver is a contract violation surfaced as a provider error.
fn require_group(pkgref: &PackageRef) -> Result<&Group, SolveError> {
    pkgref.group.as_ref().ok_or_else(|| {
        SolveError::Provider(DepProviderError::Other(format!(
            "package reference `{pkgref}` is not group-qualified — \
             dependency resolution needs `<group>/<name>`"
        )))
    })
}

/// DFS solver over a [`DepProvider`].
#[cell(seam = "DepSolver", variant = "naive", flag = "solver")]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade")]
#[spec(
    deviates = "spec://vibevm/modules/vibe-registry/PROP-002#solver",
    reason = "PROP-002 §2.8 decides resolvo is the PRIMARY depsolver; no ResolvoSolver \
              exists in tree and NaiveDepSolver is the only DepSolver impl — the known \
              SAT/resolvo upgrade debt (DBT-0011), recorded honestly until the second \
              impl lands"
)]
pub struct NaiveDepSolver<P: DepProvider> {
    provider: P,
}

impl<P: DepProvider> NaiveDepSolver<P> {
    pub fn new(provider: P) -> Self {
        NaiveDepSolver { provider }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }
}

impl<P: DepProvider> DepSolver for NaiveDepSolver<P> {
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
    #[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#determinism")]
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        let mut state = SolverState::new();
        let root_keys: Vec<(Group, String)> = roots
            .iter()
            .map(|r| Ok((require_group(r)?.clone(), r.name.to_string())))
            .collect::<Result<_, SolveError>>()?;
        // Duplicate root keys are a legal input (`vibe install x@1 x@2`
        // must surface VersionConflict through the normal path, never a
        // panic): each duplicate is enqueued and constraint-checked
        // individually; the roots-first output pass below then collapses
        // the key on the second remove(), which finds nothing. Only the
        // OUTPUT side carries a uniqueness contract — witnessed in the
        // `rest` loop below.
        for r in roots {
            state.queue.push_back(EnqueuedPkg {
                pkgref: r.clone(),
                via: None,
                is_root: true,
            });
        }

        while let Some(EnqueuedPkg {
            pkgref,
            via,
            is_root,
        }) = state.queue.pop_front()
        {
            self.process_one(&mut state, pkgref, via, is_root)?;
        }

        // Hand the accumulated choices to the shared output builder
        // (PROP-017 §2.3): roots-first ordering, exact-version pinning,
        // and obsolete-dropping live in one place so `ResolvoDepSolver`
        // yields a byte-identical graph — the differential oracle pins it.
        let chosen: std::collections::HashMap<(Group, String), crate::Chosen> = state
            .chosen
            .into_iter()
            .map(|(k, e)| {
                (
                    k,
                    crate::Chosen {
                        version: e.version,
                        direct_deps: e.direct_deps,
                        is_root: e.is_root,
                    },
                )
            })
            .collect();
        Ok(crate::build_resolved_graph(
            &root_keys,
            chosen,
            &state.declared_obsolete,
        ))
    }
}

impl<P: DepProvider> NaiveDepSolver<P> {
    fn process_one(
        &self,
        state: &mut SolverState,
        pkgref: PackageRef,
        _via: Option<String>,
        is_root: bool,
    ) -> Result<(), SolveError> {
        let group = require_group(&pkgref)?.clone();
        let key = (group.clone(), pkgref.name.to_string());

        // If a conflict was declared against this exact (group, name) by
        // some prior package in the graph, refuse to add it.
        if state.declared_conflicts.contains(&key) {
            // Find which prior package declared the conflict for a useful
            // error. Cheap O(N) walk over chosen entries — graphs are small.
            let against = state
                .chosen
                .iter()
                .find(|(_, e)| {
                    e.manifest
                        .conflicts
                        .packages
                        .iter()
                        .any(|c| c.group.as_ref() == Some(&group) && c.name == pkgref.name)
                })
                .map(|((g, n), _)| format!("{g}/{n}"))
                .unwrap_or_else(|| "<unknown>".to_string());
            return Err(SolveError::ConflictsDeclared {
                package: against,
                against: pkgref.qualified_name(),
            });
        }

        // Already chosen? Reconcile constraints.
        if let Some(existing) = state.chosen.get(&key) {
            if !version_satisfies(&pkgref.version, &existing.version) {
                return Err(SolveError::VersionConflict {
                    package: pkgref.qualified_name(),
                    existing: existing.version.to_string(),
                    new_constraint: format!("{}", pkgref.version),
                });
            }
            // Mark root if this re-discovery came in as a root.
            if is_root && let Some(e) = state.chosen.get_mut(&key) {
                e.is_root = true;
            }
            return Ok(());
        }

        // Pick concrete version + manifest.
        let version = self.provider.resolve_version(&pkgref)?;
        let manifest = self
            .provider
            .fetch_manifest(&group, pkgref.name.as_str(), &version)?;

        // Refuse if this package's declared `[conflicts]` collide with
        // anything already in the graph.
        for c in &manifest.conflicts.packages {
            let cg = require_group(c)?;
            let ck = (cg.clone(), c.name.to_string());
            if state.chosen.contains_key(&ck) {
                return Err(SolveError::ConflictsDeclared {
                    package: pkgref.qualified_name(),
                    against: c.qualified_name(),
                });
            }
        }

        // Mark conflicts and obsoletes so future enqueues respect them.
        for c in &manifest.conflicts.packages {
            let cg = require_group(c)?;
            state
                .declared_conflicts
                .insert((cg.clone(), c.name.to_string()));
        }
        for o in &manifest.obsoletes.packages {
            let og = require_group(o)?;
            state
                .declared_obsolete
                .insert((og.clone(), o.name.to_string()));
        }

        // Index provided capabilities BEFORE verifying requires — a
        // package that both `provides` and `requires` the same capability
        // self-satisfies through the normal index path.
        for cap in &manifest.provides.capabilities {
            let cap_version = capability_version_for_provider(cap, &version);
            state
                .providers_index
                .entry(cap.qualified())
                .or_default()
                .push((group.clone(), pkgref.name.to_string(), cap_version));
        }

        // Capture direct package deps verbatim — they go straight into the
        // resolved node (used by lockfile `dependencies`).
        let direct_deps: Vec<PackageRef> = manifest.requires.packages.clone();

        // Resolve capability requires against the already-known provider
        // index (which now includes this package's own provides).
        verify_capability_requires(state, &pkgref, &manifest)?;

        // Resolve disjunctions: pick first option not already in conflict
        // and that has a registered provider OR enqueue it.
        for disj in &manifest.requires_any {
            handle_disjunction(state, &pkgref, disj)?;
        }

        // Enqueue concrete deps for the next iteration.
        for d in &direct_deps {
            state.queue.push_back(EnqueuedPkg {
                pkgref: d.clone(),
                via: Some(pkgref.qualified_name()),
                is_root: false,
            });
        }

        // Commit the chosen entry.
        state.chosen.insert(
            key,
            ChosenEntry {
                version,
                manifest,
                direct_deps,
                is_root,
            },
        );
        Ok(())
    }
}

fn verify_capability_requires(
    state: &SolverState,
    pkgref: &PackageRef,
    manifest: &Manifest,
) -> Result<(), SolveError> {
    for cap_req in &manifest.requires.capabilities {
        let any_match = state
            .providers_index
            .get(&cap_req.qualified())
            .map(|providers| {
                providers
                    .iter()
                    .any(|(_, _, ver)| cap_req.version.matches(ver))
            })
            .unwrap_or(false);
        if !any_match {
            return Err(SolveError::CapabilityUnmet {
                capability: cap_req.to_string(),
                requirer: pkgref.qualified_name(),
            });
        }
    }
    Ok(())
}

/// The capability's "declared version" for the providers_index. A
/// provides-side `CapabilityRef` is semantically "this API version is
/// provided" — but the type system still uses the same `VersionSpec`
/// as the requires side, and after the Cargo-shape parser change a
/// bare semver like `0.3.0` is shorthand for `^0.3.0`. We treat
/// `0.3.0`, `=0.3.0`, `^0.3.0`, `~0.3.0` all as "the package
/// provides version 0.3.0" by walking the first comparator and
/// reading off its (major, minor, patch). For ranges like `>=1.0`
/// the same reading produces `1.0.0` which is a sensible anchor.
/// `VersionSpec::Latest` (no version on the provides line) falls
/// back to the providing package's resolved version.
pub(crate) fn capability_version_for_provider(
    cap: &CapabilityRef,
    package_version: &semver::Version,
) -> semver::Version {
    match &cap.version {
        VersionSpec::Latest => package_version.clone(),
        VersionSpec::Req(req) => req
            .comparators
            .first()
            .map(|c| semver::Version::new(c.major, c.minor.unwrap_or(0), c.patch.unwrap_or(0)))
            .unwrap_or_else(|| package_version.clone()),
    }
}

fn handle_disjunction(
    state: &mut SolverState,
    requirer: &PackageRef,
    disj: &vibe_core::manifest::RequiresAny,
) -> Result<(), SolveError> {
    // Binding the first alternative IS the emptiness check — the
    // compiler carries the invariant from here to the enqueue below.
    let Some(first) = disj.one_of.first() else {
        return Err(SolveError::DisjunctionUnsatisfiable {
            requirer: requirer.qualified_name(),
            alternatives: Vec::new(),
        });
    };
    // If any alternative is already chosen, satisfied.
    for opt in &disj.one_of {
        let og = require_group(opt)?;
        if state
            .chosen
            .contains_key(&(og.clone(), opt.name.to_string()))
        {
            return Ok(());
        }
    }
    // Otherwise: enqueue the first alternative. Naive — no backtracking
    // when downstream conflicts emerge.
    state.queue.push_back(EnqueuedPkg {
        pkgref: first.clone(),
        via: Some(format!("[[requires_any]] of {}", requirer.qualified_name())),
        is_root: false,
    });
    Ok(())
}

#[cfg(test)]
#[path = "naive/tests.rs"]
mod tests;
