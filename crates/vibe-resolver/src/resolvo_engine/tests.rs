//! Unit tests for [`super::ResolvoDepSolver`], out-of-line per the
//! file-length budget. Included via `#[cfg(test)] #[path] mod tests;`, so
//! `use super::*` resolves exactly as in the inline form. Non-`#[test]`
//! helpers carry `#[cfg(test)]` so the file-grain conform frontend scopes
//! their `unwrap`s as test code.

use super::*;
use crate::{DepProviderError, NaiveDepSolver};
use vibe_core::VersionSpec;

use fixtures::*;

#[cfg(test)]
mod fixtures {
    use std::collections::HashMap as Map;

    use vibe_core::manifest::Manifest;
    use vibe_core::{Group, PackageRef};

    use crate::{DepProvider, DepProviderError, VersionEnumerator};

    /// In-memory `VersionEnumerator` over a set of manifests — the
    /// registry fake the naive/sat tests use, plus `list_versions`.
    pub(super) struct MapProvider {
        entries: Map<String, Vec<(semver::Version, Manifest)>>,
    }

    #[cfg(test)]
    impl MapProvider {
        pub(super) fn new(seeds: &[&str]) -> Self {
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

    #[cfg(test)]
    impl DepProvider for MapProvider {
        fn resolve_version(
            &self,
            pkgref: &PackageRef,
        ) -> Result<semver::Version, DepProviderError> {
            let cands = self.entries.get(pkgref.name.as_str()).ok_or_else(|| {
                DepProviderError::UnknownPackage {
                    group: Group::parse("org.vibevm").unwrap(),
                    name: pkgref.name.to_string(),
                }
            })?;
            cands
                .iter()
                .rev()
                .map(|(v, _)| v)
                .find(|v| pkgref.version.matches(v))
                .cloned()
                .ok_or_else(|| DepProviderError::NoMatchingVersion {
                    group: Group::parse("org.vibevm").unwrap(),
                    name: pkgref.name.to_string(),
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

    #[cfg(test)]
    impl VersionEnumerator for MapProvider {
        fn list_versions(
            &self,
            _group: &Group,
            name: &str,
        ) -> Result<Vec<semver::Version>, DepProviderError> {
            let cands = self
                .entries
                .get(name)
                .ok_or_else(|| DepProviderError::UnknownPackage {
                    group: Group::parse("org.vibevm").unwrap(),
                    name: name.to_string(),
                })?;
            Ok(cands.iter().map(|(v, _)| v.clone()).collect())
        }
    }

    pub(super) fn pkg(name: &str, version: &str, requires: &[(&str, &str)]) -> String {
        pkg_with(name, version, requires, &[])
    }

    /// Manifest with optional `[requires.packages]` and a single
    /// `[[requires_any]]` over `org.vibevm/<name>` alternatives.
    pub(super) fn pkg_with(
        name: &str,
        version: &str,
        requires: &[(&str, &str)],
        any: &[&str],
    ) -> String {
        let mut s = format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n"
        );
        if !requires.is_empty() {
            s.push_str("\n[requires.packages]\n");
            for (dep, req) in requires {
                s.push_str(&format!("\"org.vibevm/{dep}\" = \"{req}\"\n"));
            }
        }
        if !any.is_empty() {
            let entries: Vec<String> = any.iter().map(|a| format!("\"org.vibevm/{a}\"")).collect();
            s.push_str(&format!(
                "\n[[requires_any]]\none_of = [{}]\n",
                entries.join(", ")
            ));
        }
        s
    }

    pub(super) fn roots(names: &[&str]) -> Vec<PackageRef> {
        names
            .iter()
            .map(|n| PackageRef::parse(&format!("org.vibevm/{n}")).unwrap())
            .collect()
    }

    pub(super) fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }
}

/// The first-pick-wins trap (the very case naive cannot pass): `a`
/// accepts `c >=1`, `b` demands `c ^1`, `c` has 1.0 and 2.0. Naive
/// picks c=2.0 for `a` then `b` conflicts; resolvo narrows c to 1.0.
#[test]
fn resolvo_solves_where_naive_first_pick_fails() {
    let seeds = [
        pkg("a", "1.0.0", &[("c", ">=1")]),
        pkg("b", "1.0.0", &[("c", "^1")]),
        pkg("c", "1.0.0", &[]),
        pkg("c", "2.0.0", &[]),
    ];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();

    let naive = NaiveDepSolver::new(MapProvider::new(&seeds));
    assert!(matches!(
        naive.solve(&roots(&["a", "b"])).unwrap_err(),
        SolveError::VersionConflict { .. }
    ));

    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a", "b"])).unwrap();
    let c = graph.find(&org(), "c").expect("c resolved");
    assert_eq!(c.version, semver::Version::new(1, 0, 0));
}

/// Newest-feasible preference: with no narrowing pressure, resolvo
/// takes the highest version, like naive.
#[test]
fn resolvo_prefers_newest() {
    let seeds = [
        pkg("a", "1.0.0", &[("c", "^1")]),
        pkg("c", "1.0.0", &[]),
        pkg("c", "1.5.0", &[]),
    ];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a"])).unwrap();
    assert_eq!(
        graph.find(&org(), "c").unwrap().version,
        semver::Version::new(1, 5, 0)
    );
}

/// A genuinely unsatisfiable world fails as `Unsatisfiable`, carrying
/// resolvo's human-readable derivation (PROP-017 §2.4).
#[test]
fn resolvo_reports_unsatisfiable_with_explanation() {
    let seeds = [
        pkg("a", "1.0.0", &[("c", "^2")]),
        pkg("b", "1.0.0", &[("c", "^1")]),
        pkg("c", "1.0.0", &[]),
        pkg("c", "2.0.0", &[]),
    ];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    match resolvo.solve(&roots(&["a", "b"])).unwrap_err() {
        SolveError::Unsatisfiable { explanation } => {
            assert!(!explanation.is_empty(), "explanation should be populated");
        }
        other => panic!("expected Unsatisfiable, got {other:?}"),
    }
}

/// A missing root surfaces the provider's `UnknownPackage`, not an
/// opaque resolvo failure.
#[test]
fn resolvo_surfaces_unknown_package() {
    let seeds = [pkg("a", "1.0.0", &[])];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    match resolvo.solve(&roots(&["ghost"])).unwrap_err() {
        SolveError::Provider(DepProviderError::UnknownPackage { name, .. }) => {
            assert_eq!(name, "ghost");
        }
        other => panic!("expected Provider(UnknownPackage), got {other:?}"),
    }
}

/// Transitive closure with exact pinning, end to end.
#[test]
fn resolvo_pins_transitive_deps() {
    let seeds = [
        pkg("a", "1.0.0", &[("b", "^1")]),
        pkg("b", "1.2.0", &[("c", "^1")]),
        pkg("c", "1.3.0", &[]),
    ];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a"])).unwrap();
    assert_eq!(graph.packages.len(), 3);
    let a = graph.find(&org(), "a").unwrap();
    assert!(a.is_root);
    // a's dep on b is pinned to the chosen exact version.
    let dep_b = a
        .dependencies
        .iter()
        .find(|d| d.name.as_str() == "b")
        .unwrap();
    assert!(matches!(&dep_b.version, VersionSpec::Req(r) if r.to_string() == "=1.2.0"));
}

/// A `[[requires_any]]` is satisfied by whichever alternative is
/// available: naive takes the first (`x`) and would fail; resolvo
/// uses the present one (`y`).
#[test]
fn resolvo_solves_disjunction_via_available_alternative() {
    let seeds = [
        pkg_with("a", "1.0.0", &[], &["x", "y"]),
        pkg("y", "1.0.0", &[]),
    ];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a"])).unwrap();
    assert!(
        graph.find(&org(), "y").is_some(),
        "the available alternative is selected"
    );
    assert!(
        graph.find(&org(), "x").is_none(),
        "the absent alternative is not"
    );
}

/// The marquee win over naive: a disjunction whose first alternative
/// is satisfiable in isolation but conflicts downstream. `a` needs
/// `c ^1` and requires_any [x, y]; `x` forces `c ^2` (a conflict),
/// `y` needs `c ^1`. naive picks `x` and dies; resolvo backtracks to
/// `y` — the first-pick-wins trap, now across a disjunction.
#[test]
fn resolvo_disjunction_backtracks_past_conflicting_alternative() {
    let seeds = [
        pkg_with("a", "1.0.0", &[("c", "^1")], &["x", "y"]),
        pkg_with("x", "1.0.0", &[("c", "^2")], &[]),
        pkg_with("y", "1.0.0", &[("c", "^1")], &[]),
        pkg("c", "1.0.0", &[]),
        pkg("c", "2.0.0", &[]),
    ];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a"])).unwrap();
    assert!(
        graph.find(&org(), "y").is_some(),
        "backtracks to the satisfiable alternative"
    );
    assert!(
        graph.find(&org(), "x").is_none(),
        "drops the conflicting alternative"
    );
    assert_eq!(
        graph.find(&org(), "c").unwrap().version,
        semver::Version::new(1, 0, 0)
    );
}

/// `[conflicts]`: `a` requires both `x` and `y`, but `x` declares a
/// conflict against `y` (a constraint to the match-nothing set) — so
/// the two cannot coexist and the graph is unsatisfiable.
#[test]
fn resolvo_rejects_conflicting_packages() {
    let a = "[package]\ngroup = \"org.vibevm\"\nname = \"a\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[requires.packages]\n\"org.vibevm/x\" = \"^1\"\n\"org.vibevm/y\" = \"^1\"\n";
    let x = "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[conflicts]\npackages = [\"org.vibevm/y\"]\n";
    let seeds = [a.to_string(), x.to_string(), pkg("y", "1.0.0", &[])];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    match resolvo.solve(&roots(&["a"])).unwrap_err() {
        SolveError::Unsatisfiable { .. } => {}
        other => panic!("expected Unsatisfiable from the conflict, got {other:?}"),
    }
}

/// `[obsoletes]`: `new` supersedes `old`; `a` requires both, and the
/// output drops `old` (mirroring the naive cell's obsolete handling).
#[test]
fn resolvo_drops_obsoleted_packages() {
    let a = "[package]\ngroup = \"org.vibevm\"\nname = \"a\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[requires.packages]\n\"org.vibevm/old\" = \"^1\"\n\"org.vibevm/new\" = \"^1\"\n";
    let new = "[package]\ngroup = \"org.vibevm\"\nname = \"new\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[obsoletes]\npackages = [\"org.vibevm/old\"]\n";
    let seeds = [a.to_string(), new.to_string(), pkg("old", "1.0.0", &[])];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a"])).unwrap();
    assert!(
        graph.find(&org(), "new").is_some(),
        "the superseding package stays"
    );
    assert!(
        graph.find(&org(), "old").is_none(),
        "the obsoleted package is dropped from the output"
    );
}

/// Capabilities: `a` requires package `b` AND capability `capability:c`;
/// `b` provides it. naive checks the capability before `b` is processed
/// and fails (order-dependence); resolvo's pre-scan finds the provider
/// across the closure and the post-solve check passes.
#[test]
fn resolvo_satisfies_capability_via_required_package() {
    let a = "[package]\ngroup = \"org.vibevm\"\nname = \"a\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[requires.packages]\n\"org.vibevm/b\" = \"^1\"\n\n[requires]\ncapabilities = [\"capability:c@^1\"]\n";
    let b = "[package]\ngroup = \"org.vibevm\"\nname = \"b\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[provides]\ncapabilities = [\"capability:c@1.0.0\"]\n";
    let seeds = [a.to_string(), b.to_string()];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();

    let naive = NaiveDepSolver::new(MapProvider::new(&seeds));
    assert!(
        matches!(
            naive.solve(&roots(&["a"])).unwrap_err(),
            SolveError::CapabilityUnmet { .. }
        ),
        "naive's order-dependent capability check fails this world"
    );

    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&roots(&["a"])).unwrap();
    assert!(
        graph.find(&org(), "b").is_some(),
        "the capability provider is pulled into the graph"
    );
}

/// A required capability that nothing in the closure provides fails
/// with the clean `CapabilityUnmet` verdict, naming the requirer.
#[test]
fn resolvo_reports_unmet_capability() {
    let a = "[package]\ngroup = \"org.vibevm\"\nname = \"a\"\nkind = \"flow\"\nversion = \"1.0.0\"\n\n[requires]\ncapabilities = [\"capability:c@^1\"]\n";
    let seeds = [a.to_string()];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    match resolvo.solve(&roots(&["a"])).unwrap_err() {
        SolveError::CapabilityUnmet {
            capability,
            requirer,
        } => {
            assert!(capability.contains("capability:c"));
            assert_eq!(requirer, "org.vibevm/a");
        }
        other => panic!("expected CapabilityUnmet, got {other:?}"),
    }
}

/// A root's `kind` prefix that disagrees with the resolved manifest's
/// declared `kind` is a `KindMismatch` under resolvo too (PROP-008 §2.4
/// KIND-VALIDATION) — the production cell and the naive cell agree.
#[test]
fn resolvo_kind_prefix_mismatch_is_a_kind_mismatch() {
    let seeds = [pkg("wal", "1.0.0", &[])];
    let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let err = resolvo
        .solve(&[PackageRef::parse("feat:org.vibevm/wal").unwrap()])
        .unwrap_err();
    assert!(
        matches!(err, SolveError::KindMismatch { .. }),
        "expected KindMismatch, got {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("feat"), "must name the requested kind: {msg}");
    assert!(msg.contains("flow"), "must name the actual kind: {msg}");
    assert!(
        msg.contains("org.vibevm/wal"),
        "must name the package: {msg}"
    );
}
