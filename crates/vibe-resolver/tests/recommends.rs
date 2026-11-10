//! Forward weak-deps end-to-end (PROP-003 §2.3.3): `[recommends]` is a
//! best-effort expansion — pulled in when it fits, silently skipped when
//! it conflicts, never failing the solve; `[suggests]` is never
//! auto-installed. Driven through the public `ResolvoDepSolver` seam.

use std::collections::HashMap;

use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, ResolvoDepSolver, VersionEnumerator,
};

fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// In-memory registry fake over a set of manifests.
struct MapProvider {
    entries: HashMap<String, Vec<(semver::Version, Manifest)>>,
}

impl MapProvider {
    fn new(seeds: &[String]) -> Self {
        let mut entries: HashMap<String, Vec<(semver::Version, Manifest)>> = HashMap::new();
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
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let cands = self.entries.get(pkgref.name.as_str()).ok_or_else(|| {
            DepProviderError::UnknownPackage {
                group: org(),
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
                group: org(),
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
                group: org(),
                name: name.to_string(),
            })?;
        Ok(cands.iter().map(|(v, _)| v.clone()).collect())
    }
}

/// Build a manifest with optional `[requires.packages]`, `[recommends]`,
/// and `[suggests]` (the latter two as `org.vibevm/<name>` arrays).
fn manifest(
    name: &str,
    version: &str,
    requires: &[(&str, &str)],
    recommends: &[&str],
    suggests: &[&str],
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
    let array = |xs: &[&str]| {
        xs.iter()
            .map(|x| format!("\"org.vibevm/{x}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !recommends.is_empty() {
        s.push_str(&format!(
            "\n[recommends]\npackages = [{}]\n",
            array(recommends)
        ));
    }
    if !suggests.is_empty() {
        s.push_str(&format!("\n[suggests]\npackages = [{}]\n", array(suggests)));
    }
    s
}

fn root(name: &str) -> Vec<PackageRef> {
    vec![PackageRef::parse(&format!("org.vibevm/{name}")).unwrap()]
}

/// A recommend that fits is pulled into the graph — but as a non-root.
#[test]
fn recommend_is_pulled_in_when_it_fits() {
    let seeds = [
        manifest("a", "1.0.0", &[], &["b"], &[]),
        manifest("b", "1.0.0", &[], &[], &[]),
    ];
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&root("a")).unwrap();
    let b = graph
        .find(&org(), "b")
        .expect("recommended `b` was pulled in");
    assert!(!b.is_root, "a recommend is not a user root");
    assert!(graph.find(&org(), "a").unwrap().is_root);
}

/// A recommend that conflicts with the hard graph is silently skipped —
/// the solve still succeeds, just without it.
#[test]
fn recommend_is_skipped_when_it_conflicts() {
    let seeds = [
        // `a` needs `c ^1` and recommends `d`; `d` needs `c ^2` (a
        // conflict), so `d` cannot be added without breaking `a`.
        manifest("a", "1.0.0", &[("c", "^1")], &["d"], &[]),
        manifest("d", "1.0.0", &[("c", "^2")], &[], &[]),
        manifest("c", "1.0.0", &[], &[], &[]),
        manifest("c", "2.0.0", &[], &[], &[]),
    ];
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&root("a")).unwrap();
    assert!(
        graph.find(&org(), "d").is_none(),
        "the conflicting recommend is dropped, not installed"
    );
    assert!(
        graph.find(&org(), "a").is_some(),
        "the solve still succeeds"
    );
    assert_eq!(graph.find(&org(), "c").unwrap().version.major, 1);
}

/// A suggested package is never auto-installed — the solver ignores it.
#[test]
fn suggest_is_never_installed() {
    let seeds = [
        manifest("a", "1.0.0", &[], &[], &["b"]),
        manifest("b", "1.0.0", &[], &[], &[]),
    ];
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let graph = resolvo.solve(&root("a")).unwrap();
    assert!(
        graph.find(&org(), "b").is_none(),
        "a suggested package is not auto-installed"
    );
}

/// `--no-recommends`: the greedy expansion is not run at all, so the solve
/// is the hard solve and nothing else — the recommended package is absent
/// although it fits perfectly (`PROP-077#NO-RECOMMENDS-FLAG`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
fn no_recommends_runs_the_hard_solve_and_nothing_else() {
    let seeds = [
        manifest("a", "1.0.0", &[("c", "^1")], &["b"], &[]),
        manifest("b", "1.0.0", &[], &[], &[]),
        manifest("c", "1.0.0", &[], &[], &[]),
    ];
    let resolvo = ResolvoDepSolver::new(MapProvider::new(&seeds));
    let declined = resolvo.solve_without_recommends(&root("a")).unwrap();
    assert!(
        declined.find(&org(), "b").is_none(),
        "the flag declines a recommendation that would otherwise fit"
    );
    // And the hard graph is untouched: a real requirement is still resolved.
    assert!(declined.find(&org(), "c").is_some());
    assert!(declined.find(&org(), "a").unwrap().is_root);

    // The same solver, asked the ordinary way, still brings it in — so the
    // difference is the flag and not the fixture.
    let kept = resolvo.solve(&root("a")).unwrap();
    assert!(kept.find(&org(), "b").is_some());
}

/// A cell that expands no weak dependency answers the flag with its
/// ordinary solve, which is the honest answer rather than a refusal.
#[test]
fn a_solver_without_weak_expansion_answers_with_its_ordinary_solve() {
    let seeds = [
        manifest("a", "1.0.0", &[], &["b"], &[]),
        manifest("b", "1.0.0", &[], &[], &[]),
    ];
    let naive = vibe_resolver::NaiveDepSolver::new(MapProvider::new(&seeds));
    let declined = naive.solve_without_recommends(&root("a")).unwrap();
    let ordinary = naive.solve(&root("a")).unwrap();
    let keys = |graph: &vibe_resolver::ResolvedGraph| -> Vec<String> {
        graph
            .iter()
            .map(|node| format!("{}/{}@{}", node.group, node.name, node.version))
            .collect()
    };
    assert_eq!(keys(&declined), keys(&ordinary));
}
