//! `Sat` — the `sat` DepSolver cell: chronological backtracking over
//! version choices (DBT-0011; PROP-003 §2.8 "solver upgrade").
//!
//! The naive solver's pinned limitation is *first-pick-wins*: when two
//! paths constrain the same package with overlapping-but-different
//! ranges, the first pick is taken and the second constraint either
//! fits or fails — no second chance. This cell adds the second
//! chance without re-implementing any resolution semantics:
//!
//! > **The naive solver is the branch checker.** `Sat` walks a tree
//! > of version *bounds*; at each node it runs the full naive solve
//! > under a [`BoundedProvider`] that caps selected packages below
//! > their conflicting picks. A `VersionConflict` narrows the bound
//! > on the conflicting package and retries; an exhausted package
//! > backtracks chronologically. Features, conditional deps,
//! > capabilities, conflicts, obsoletes — all evaluated by exactly
//! > the same code the naive path runs, so the two cells cannot
//! > drift semantically (the differential oracle in
//! > `tests/solver_properties.rs` pins it).
//!
//! Termination: every step strictly lowers some package's bound over
//! a finite version set, so the choice tree is finite; a hard
//! attempt cap (`MAX_ATTEMPTS`) backstops pathological providers.
//!
//! `resolvo` as the primary solver (PROP-002 §2.8) remains the
//! recorded deviation — see the `deviates` edge on the impl: this
//! cell retires the *backtracking* half of DBT-0011 while the
//! "industrial solver" half stays an owner option.

use std::collections::HashMap;

use specmark::{cell, spec};
use vibe_core::{Group, PackageRef, VersionSpec};

use crate::{DepProvider, DepProviderError, DepSolver, NaiveDepSolver, ResolvedGraph, SolveError};

/// Hard cap on solve attempts — a backstop, not a tuning knob. Every
/// attempt strictly shrinks some package's candidate set, so real
/// worlds terminate far below it.
const MAX_ATTEMPTS: usize = 256;

/// A provider wrapper that excludes versions at-or-above a per-package
/// bound — the mechanism that turns "try the next lower candidate"
/// into ordinary `resolve_version` calls on the unmodified trait.
struct BoundedProvider<'a, P: DepProvider> {
    inner: &'a P,
    /// `(group, name) → exclusive upper bound`.
    bounds: &'a HashMap<(Group, String), semver::Version>,
}

impl<'a, P: DepProvider> BoundedProvider<'a, P> {
    fn bounded_ref(&self, pkgref: &PackageRef) -> Result<PackageRef, DepProviderError> {
        let Some(group) = pkgref.group.clone() else {
            return Ok(pkgref.clone());
        };
        let key = (group, pkgref.name.clone());
        let Some(bound) = self.bounds.get(&key) else {
            return Ok(pkgref.clone());
        };
        let combined = match &pkgref.version {
            VersionSpec::Latest => format!("<{bound}"),
            VersionSpec::Req(req) => format!("{req}, <{bound}"),
        };
        let req = semver::VersionReq::parse(&combined).map_err(|e| {
            DepProviderError::Other(format!(
                "internal: combined constraint `{combined}` failed to parse: {e}"
            ))
        })?;
        Ok(PackageRef {
            kind: pkgref.kind,
            group: pkgref.group.clone(),
            name: pkgref.name.clone(),
            version: VersionSpec::Req(req),
        })
    }
}

impl<'a, P: DepProvider> DepProvider for BoundedProvider<'a, P> {
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.inner.resolve_version(&self.bounded_ref(pkgref)?)
    }

    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<vibe_core::manifest::Manifest, DepProviderError> {
        self.inner.fetch_manifest(group, name, version)
    }
}

/// The backtracking `DepSolver` cell. Construct it the way the naive
/// cell is constructed and call [`DepSolver::solve`]:
///
/// ```
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_resolver::{DepProvider, DepProviderError, DepSolver, sat::Sat};
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
/// let solver = Sat::new(OnePackage(m));
/// let graph = solver
///     .solve(&[PackageRef::parse("org.vibevm/wal").unwrap()])
///     .unwrap();
/// assert_eq!(graph.packages.len(), 1);
/// ```
#[cell(seam = "DepSolver", variant = "sat")]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade")]
pub struct Sat<P: DepProvider> {
    provider: P,
}

impl<P: DepProvider> Sat<P> {
    pub fn new(provider: P) -> Self {
        Sat { provider }
    }

    pub fn into_inner(self) -> P {
        self.provider
    }

    /// Next candidate of `key` strictly below `bound`, if any.
    fn next_lower(
        &self,
        key: &(Group, String),
        bound: &semver::Version,
    ) -> Option<semver::Version> {
        let req = semver::VersionReq::parse(&format!("<{bound}")).ok()?;
        let probe = PackageRef {
            kind: None,
            group: Some(key.0.clone()),
            name: key.1.clone(),
            version: VersionSpec::Req(req),
        };
        self.provider.resolve_version(&probe).ok()
    }
}

/// Parse the `group/name` form `SolveError::VersionConflict.package`
/// carries back into a key. The naive solver always emits the
/// qualified form (it requires groups on every ref), so a parse miss
/// is an internal contract break, surfaced loudly.
fn conflict_key(package: &str) -> Result<(Group, String), SolveError> {
    let Some((group, name)) = package.rsplit_once('/') else {
        return Err(SolveError::Provider(DepProviderError::Other(format!(
            "internal: conflict package `{package}` is not group-qualified"
        ))));
    };
    let group = Group::parse(group).map_err(|e| {
        SolveError::Provider(DepProviderError::Other(format!(
            "internal: conflict group in `{package}` failed to parse: {e}"
        )))
    })?;
    Ok((group, name.to_string()))
}

#[spec(
    deviates = "spec://vibevm/modules/vibe-registry/PROP-002#solver",
    reason = "PROP-002 §2.8 names resolvo as the primary industrial solver; this cell \
              implements chronological backtracking natively over the unmodified \
              DepProvider trait instead, reusing the naive cell as its branch checker. \
              The backtracking half of DBT-0011 retires here; adopting resolvo stays \
              an owner decision the DepSolver seam keeps open"
)]
impl<P: DepProvider> DepSolver for Sat<P> {
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        // The choice stack: each entry excludes `>= bound` for its
        // package. The effective bound per package is the LOWEST entry
        // (entries for one package only ever descend).
        let mut stack: Vec<((Group, String), semver::Version)> = Vec::new();
        let mut first_error: Option<SolveError> = None;

        for _ in 0..MAX_ATTEMPTS {
            let mut bounds: HashMap<(Group, String), semver::Version> = HashMap::new();
            for (key, bound) in &stack {
                let slot = bounds.entry(key.clone()).or_insert_with(|| bound.clone());
                if bound < slot {
                    *slot = bound.clone();
                }
            }
            let bounded = BoundedProvider {
                inner: &self.provider,
                bounds: &bounds,
            };
            let attempt = NaiveDepSolver::new(bounded).solve(roots);
            match attempt {
                Ok(graph) => return Ok(graph),
                Err(SolveError::VersionConflict {
                    package,
                    existing,
                    new_constraint,
                }) => {
                    let key = conflict_key(&package)?;
                    let existing_v: semver::Version = existing.parse().map_err(|e| {
                        SolveError::Provider(DepProviderError::Other(format!(
                            "internal: conflict version `{existing}` failed to parse: {e}"
                        )))
                    })?;
                    let conflict = SolveError::VersionConflict {
                        package,
                        existing,
                        new_constraint,
                    };
                    let tightens = bounds.get(&key).is_none_or(|b| existing_v < *b);
                    if tightens {
                        // Narrow: try the world where this package is
                        // capped below its conflicting pick.
                        stack.push((key, existing_v));
                        if first_error.is_none() {
                            first_error = Some(conflict);
                        }
                    } else if !backtrack(&mut stack, |k, b| self.next_lower(k, b)) {
                        // An earlier round's conflict if one was recorded;
                        // otherwise this conflict IS the first.
                        return Err(first_error.unwrap_or(conflict));
                    } else if first_error.is_none() {
                        first_error = Some(conflict);
                    }
                }
                Err(SolveError::Provider(DepProviderError::NoMatchingVersion {
                    group,
                    name,
                    ..
                })) if bounds.contains_key(&(group.clone(), name.clone())) => {
                    // A bound we introduced exhausted this package —
                    // that branch of the choice tree is dead.
                    if !backtrack(&mut stack, |k, b| self.next_lower(k, b)) {
                        return Err(first_error.unwrap_or(SolveError::Provider(
                            DepProviderError::NoMatchingVersion {
                                group,
                                name,
                                constraint: "<backtracked>".into(),
                            },
                        )));
                    }
                }
                Err(other) => return Err(other),
            }
        }
        Err(
            first_error.unwrap_or(SolveError::Provider(DepProviderError::Other(
                "sat: attempt cap reached without a verdict".into(),
            ))),
        )
    }
}

/// Chronological backtrack: drop the newest choice; if its package
/// still has a lower candidate, descend to it and resume, otherwise
/// keep popping. Returns false when the stack is exhausted — the
/// whole tree is unsatisfiable.
fn backtrack<F>(stack: &mut Vec<((Group, String), semver::Version)>, mut next_lower: F) -> bool
where
    F: FnMut(&(Group, String), &semver::Version) -> Option<semver::Version>,
{
    while let Some((key, bound)) = stack.pop() {
        if let Some(lower) = next_lower(&key, &bound) {
            // Exclude the candidate we already tried: bound to it.
            stack.push((key, lower));
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap as Map;

    use super::*;
    use vibe_core::manifest::Manifest;

    /// Same in-memory provider shape the naive tests use.
    struct MapProvider {
        entries: Map<String, Vec<(semver::Version, Manifest)>>,
    }

    impl MapProvider {
        fn new(seeds: &[&str]) -> Self {
            let mut entries: Map<String, Vec<(semver::Version, Manifest)>> = Map::new();
            for toml in seeds {
                let m = Manifest::parse_str(toml).unwrap();
                let p = m.require_package().unwrap();
                entries
                    .entry(p.name.clone())
                    .or_default()
                    .push((p.version.clone(), m.clone()));
            }
            for v in entries.values_mut() {
                v.sort_by(|a, b| a.0.cmp(&b.0));
            }
            MapProvider { entries }
        }
    }

    impl DepProvider for MapProvider {
        fn resolve_version(
            &self,
            pkgref: &PackageRef,
        ) -> Result<semver::Version, DepProviderError> {
            let cands =
                self.entries
                    .get(&pkgref.name)
                    .ok_or_else(|| DepProviderError::UnknownPackage {
                        group: Group::parse("org.vibevm").unwrap(),
                        name: pkgref.name.clone(),
                    })?;
            cands
                .iter()
                .rev()
                .map(|(v, _)| v)
                .find(|v| pkgref.version.matches(v))
                .cloned()
                .ok_or_else(|| DepProviderError::NoMatchingVersion {
                    group: Group::parse("org.vibevm").unwrap(),
                    name: pkgref.name.clone(),
                    constraint: format!("{}", pkgref.version),
                })
        }

        fn fetch_manifest(
            &self,
            _group: &Group,
            name: &str,
            version: &semver::Version,
        ) -> Result<Manifest, DepProviderError> {
            self.entries
                .get(name)
                .and_then(|c| c.iter().find(|(v, _)| v == version))
                .map(|(_, m)| m.clone())
                .ok_or_else(|| DepProviderError::Other(format!("no {name}@{version}")))
        }
    }

    fn pkg(name: &str, version: &str, requires: &[(&str, &str)]) -> String {
        let mut s = format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n"
        );
        if !requires.is_empty() {
            s.push_str("\n[requires.packages]\n");
            for (dep, req) in requires {
                s.push_str(&format!("\"org.vibevm/{dep}\" = \"{req}\"\n"));
            }
        }
        s
    }

    fn roots(names: &[&str]) -> Vec<PackageRef> {
        names
            .iter()
            .map(|n| PackageRef::parse(&format!("org.vibevm/{n}")).unwrap())
            .collect()
    }

    /// The first-pick-wins trap: naive fails, sat backtracks and
    /// solves. `a` accepts c >=1; `b` demands c ^1; c has 1.0 and 2.0.
    /// Naive picks c=2.0 for `a` (highest), then `b` conflicts. Sat
    /// caps c below 2.0 and lands c=1.0 — satisfying both.
    #[test]
    fn sat_solves_where_naive_first_pick_fails() {
        let seeds = [
            pkg("a", "1.0.0", &[("c", ">=1")]),
            pkg("b", "1.0.0", &[("c", "^1")]),
            pkg("c", "1.0.0", &[]),
            pkg("c", "2.0.0", &[]),
        ];
        let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();

        let naive = NaiveDepSolver::new(MapProvider::new(&seeds));
        let err = naive.solve(&roots(&["a", "b"])).unwrap_err();
        assert!(
            matches!(err, SolveError::VersionConflict { .. }),
            "the trap must actually trap naive: {err:?}"
        );

        let sat = Sat::new(MapProvider::new(&seeds));
        let graph = sat.solve(&roots(&["a", "b"])).unwrap();
        let c = graph
            .find(&Group::parse("org.vibevm").unwrap(), "c")
            .expect("c resolved");
        assert_eq!(c.version, semver::Version::new(1, 0, 0));
    }

    /// Two-level backtracking: the fix for one conflict surfaces a
    /// second; sat descends twice.
    #[test]
    fn sat_backtracks_through_chained_conflicts() {
        let seeds = [
            pkg("a", "1.0.0", &[("c", ">=1"), ("d", ">=1")]),
            pkg("b", "1.0.0", &[("c", "^1"), ("d", "^1")]),
            pkg("c", "1.0.0", &[]),
            pkg("c", "2.0.0", &[]),
            pkg("d", "1.0.0", &[]),
            pkg("d", "2.0.0", &[]),
        ];
        let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
        let sat = Sat::new(MapProvider::new(&seeds));
        let graph = sat.solve(&roots(&["a", "b"])).unwrap();
        let org = Group::parse("org.vibevm").unwrap();
        assert_eq!(graph.find(&org, "c").unwrap().version.major, 1);
        assert_eq!(graph.find(&org, "d").unwrap().version.major, 1);
    }

    /// A genuinely unsatisfiable world reports the ORIGINAL conflict,
    /// not an internal backtracking artifact.
    #[test]
    fn sat_reports_the_first_conflict_when_unsatisfiable() {
        let seeds = [
            pkg("a", "1.0.0", &[("c", "^2")]),
            pkg("b", "1.0.0", &[("c", "^1")]),
            pkg("c", "1.0.0", &[]),
            pkg("c", "2.0.0", &[]),
        ];
        let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
        let sat = Sat::new(MapProvider::new(&seeds));
        let err = sat.solve(&roots(&["a", "b"])).unwrap_err();
        match err {
            SolveError::VersionConflict { package, .. } => {
                assert_eq!(package, "org.vibevm/c");
            }
            other => panic!("expected the original VersionConflict, got {other:?}"),
        }
    }

    /// On a conflict-free world sat takes the naive fast path: one
    /// attempt, identical graph (the differential property suite
    /// covers this across generated worlds; this is the unit smoke).
    #[test]
    fn sat_matches_naive_on_conflict_free_worlds() {
        let seeds = [pkg("a", "1.0.0", &[("c", "^1")]), pkg("c", "1.2.0", &[])];
        let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
        let naive_graph = NaiveDepSolver::new(MapProvider::new(&seeds))
            .solve(&roots(&["a"]))
            .unwrap();
        let sat_graph = Sat::new(MapProvider::new(&seeds))
            .solve(&roots(&["a"]))
            .unwrap();
        let render = |g: &ResolvedGraph| {
            g.packages
                .iter()
                .map(|n| format!("{}@{} root={}", n.name, n.version, n.is_root))
                .collect::<Vec<_>>()
        };
        assert_eq!(render(&naive_graph), render(&sat_graph));
    }
}
