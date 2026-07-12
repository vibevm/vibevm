//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use specmark::verifies;

use super::*;
use vibe_core::PackageRef;

use fixtures::*;

/// Build the canonical first-party `Group` for tests.
#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// Test-only fixtures behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps these fixtures
/// reading as test code now that the tests live outside the parent
/// module's inline `mod tests`.
#[cfg(test)]
mod fixtures {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    type ProviderEntries = HashMap<(Group, String), Vec<(semver::Version, Manifest)>>;

    /// In-memory provider for tests. Pre-seeded with `(group, name) →
    /// list-of-(version, manifest)` pairs. Identity is `(group, name)`;
    /// `kind` is only manifest metadata.
    pub(super) struct MapProvider {
        entries: RefCell<ProviderEntries>,
    }

    impl MapProvider {
        pub(super) fn new() -> Self {
            MapProvider {
                entries: RefCell::new(HashMap::new()),
            }
        }
        pub(super) fn seed(&self, name: &str, manifest_toml: &str) {
            let m = Manifest::parse_str(manifest_toml).unwrap();
            let v = m.require_package().unwrap().version.clone();
            self.entries
                .borrow_mut()
                .entry((org(), name.to_string()))
                .or_default()
                .push((v, m));
        }
    }

    impl DepProvider for MapProvider {
        fn resolve_version(
            &self,
            pkgref: &PackageRef,
        ) -> Result<semver::Version, crate::DepProviderError> {
            let group = pkgref.group.clone().unwrap_or_else(org);
            let key = (group.clone(), pkgref.name.to_string());
            let entries = self.entries.borrow();
            let candidates =
                entries
                    .get(&key)
                    .ok_or_else(|| crate::DepProviderError::UnknownPackage {
                        group: group.clone(),
                        name: pkgref.name.to_string(),
                    })?;
            // Pick highest matching version, prefer stable.
            let mut versions: Vec<&semver::Version> = candidates.iter().map(|(v, _)| v).collect();
            versions.sort();
            let pick = versions
                .iter()
                .rev()
                .find(|v| pkgref.version.matches(v) && v.pre.is_empty())
                .or_else(|| versions.iter().rev().find(|v| pkgref.version.matches(v)))
                .copied()
                .ok_or_else(|| crate::DepProviderError::NoMatchingVersion {
                    group,
                    name: pkgref.name.to_string(),
                    constraint: format!("{}", pkgref.version),
                })?;
            Ok(pick.clone())
        }
        fn fetch_manifest(
            &self,
            group: &Group,
            name: &str,
            version: &semver::Version,
        ) -> Result<Manifest, crate::DepProviderError> {
            let key = (group.clone(), name.to_string());
            let entries = self.entries.borrow();
            let candidates =
                entries
                    .get(&key)
                    .ok_or_else(|| crate::DepProviderError::UnknownPackage {
                        group: group.clone(),
                        name: name.to_string(),
                    })?;
            candidates
                .iter()
                .find(|(v, _)| v == version)
                .map(|(_, m)| m.clone())
                .ok_or_else(|| {
                    crate::DepProviderError::Other(format!(
                        "no manifest for {group}/{name}@{version}"
                    ))
                })
        }
    }
}

#[cfg(test)]
fn manifest_minimal(kind: &str, name: &str, version: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n"
    )
}

#[cfg(test)]
fn manifest_with_requires(kind: &str, name: &str, version: &str, requires: &[&str]) -> String {
    // `[requires.packages]` is a TOML table — each key a
    // group-qualified `<group>/<name>` pkgref, each value the version
    // constraint. The test helpers pass entries in the
    // `<group>/<name>@<req>` shorthand; split on the `@` to render
    // the table form.
    let mut s = manifest_minimal(kind, name, version);
    s.push_str("\n[requires.packages]\n");
    for r in requires {
        let (pkg, req) = r.split_once('@').unwrap_or((r, "*"));
        s.push_str(&format!("\"{pkg}\" = \"{req}\"\n"));
    }
    s
}

#[test]
fn resolves_single_root_with_no_deps() {
    let p = MapProvider::new();
    p.seed("wal", &manifest_minimal("flow", "wal", "0.1.0"));
    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm.world/wal").unwrap()])
        .unwrap();
    assert_eq!(graph.packages.len(), 1);
    assert!(graph.packages[0].is_root);
    assert_eq!(graph.packages[0].version.to_string(), "0.1.0");
    assert!(graph.packages[0].dependencies.is_empty());
}

#[test]
fn resolves_chain_of_three() {
    let p = MapProvider::new();
    p.seed(
        "ui",
        &manifest_with_requires("feat", "ui", "0.1.0", &["org.vibevm/rust@^0.1"]),
    );
    p.seed(
        "rust",
        &manifest_with_requires("stack", "rust", "0.1.0", &["org.vibevm.world/wal@^0.1"]),
    );
    p.seed("wal", &manifest_minimal("flow", "wal", "0.1.0"));

    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm/ui").unwrap()])
        .unwrap();
    assert_eq!(graph.packages.len(), 3);
    assert!(graph.find(&org(), "ui").unwrap().is_root);
    assert!(!graph.find(&org(), "rust").unwrap().is_root);
    assert!(!graph.find(&org(), "wal").unwrap().is_root);
    assert_eq!(graph.find(&org(), "ui").unwrap().dependencies.len(), 1);
}

#[test]
fn picks_highest_matching_for_range() {
    let p = MapProvider::new();
    p.seed("wal", &manifest_minimal("flow", "wal", "0.1.0"));
    p.seed("wal", &manifest_minimal("flow", "wal", "0.1.5"));
    p.seed("wal", &manifest_minimal("flow", "wal", "0.2.0"));

    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm.world/wal@^0.1").unwrap()])
        .unwrap();
    assert_eq!(graph.packages[0].version.to_string(), "0.1.5");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn detects_version_conflict_across_paths() {
    // Two roots: one wants ^0.1, the other wants ^0.2. Naive picks
    // the first root's version (0.1.5) and the second's constraint
    // is checked against it — fails because 0.2.0 doesn't match ^0.1.
    let p = MapProvider::new();
    p.seed("wal", &manifest_minimal("flow", "wal", "0.1.5"));
    p.seed("wal", &manifest_minimal("flow", "wal", "0.2.0"));

    let solver = NaiveDepSolver::new(p);
    let err = solver
        .solve(&[
            PackageRef::parse("org.vibevm.world/wal@^0.1").unwrap(),
            PackageRef::parse("org.vibevm.world/wal@^0.2").unwrap(),
        ])
        .unwrap_err();
    match err {
        SolveError::VersionConflict { package, .. } => {
            assert_eq!(package, "org.vibevm.world/wal");
        }
        other => panic!("expected VersionConflict, got: {other:?}"),
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn detects_conflicts_declaration() {
    // org.vibevm/ui declares conflicts with org.vibevm/legacy-wal; if
    // both are roots, solver refuses.
    let m_ui = r#"
[package]
group = "org.vibevm"
name = "ui"
kind = "feat"
version = "0.1.0"

[conflicts]
packages = ["org.vibevm/legacy-wal"]
"#;
    let p = MapProvider::new();
    p.seed("ui", m_ui);
    p.seed(
        "legacy-wal",
        &manifest_minimal("flow", "legacy-wal", "0.0.1"),
    );

    let solver = NaiveDepSolver::new(p);
    let err = solver
        .solve(&[
            PackageRef::parse("org.vibevm/legacy-wal").unwrap(),
            PackageRef::parse("org.vibevm/ui").unwrap(),
        ])
        .unwrap_err();
    match err {
        SolveError::ConflictsDeclared { .. } => {}
        other => panic!("expected ConflictsDeclared, got: {other:?}"),
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn capability_requires_satisfied_by_already_seen_provider() {
    // Order matters: org.vibevm/rust provides ui:landing-page, then
    // org.vibevm/home requires it. Naive provider-then-consumer
    // ordering succeeds. Reversed order would fail (limitation).
    let m_stack = r#"
[package]
group = "org.vibevm"
name = "rust"
kind = "stack"
version = "0.1.0"

[provides]
capabilities = ["ui:landing-page@0.3.0"]
"#;
    let m_feat = r#"
[package]
group = "org.vibevm"
name = "home"
kind = "feat"
version = "0.1.0"

[requires]
capabilities = ["ui:landing-page@^0.3"]
"#;
    let p = MapProvider::new();
    p.seed("rust", m_stack);
    p.seed("home", m_feat);

    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[
            PackageRef::parse("org.vibevm/rust").unwrap(),
            PackageRef::parse("org.vibevm/home").unwrap(),
        ])
        .unwrap();
    assert_eq!(graph.packages.len(), 2);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn capability_requires_self_satisfaction() {
    // A package that both provides and requires the same capability
    // with the same exact version trivially satisfies itself.
    let m = r#"
[package]
group = "org.vibevm"
name = "magic"
kind = "feat"
version = "0.1.0"

[provides]
capabilities = ["x:y@0.1.0"]

[requires]
capabilities = ["x:y@^0.1"]
"#;
    let p = MapProvider::new();
    p.seed("magic", m);
    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm/magic").unwrap()])
        .unwrap();
    assert_eq!(graph.packages.len(), 1);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn capability_requires_unmet_errors() {
    let m = r#"
[package]
group = "org.vibevm"
name = "home"
kind = "feat"
version = "0.1.0"

[requires]
capabilities = ["ui:landing-page@^0.3"]
"#;
    let p = MapProvider::new();
    p.seed("home", m);
    let solver = NaiveDepSolver::new(p);
    let err = solver
        .solve(&[PackageRef::parse("org.vibevm/home").unwrap()])
        .unwrap_err();
    match err {
        SolveError::CapabilityUnmet { capability, .. } => {
            assert!(capability.contains("ui:landing-page"));
        }
        other => panic!("expected CapabilityUnmet, got: {other:?}"),
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn unknown_package_propagates() {
    let p = MapProvider::new();
    let solver = NaiveDepSolver::new(p);
    let err = solver
        .solve(&[PackageRef::parse("org.vibevm/ghost").unwrap()])
        .unwrap_err();
    match err {
        SolveError::Provider(crate::DepProviderError::UnknownPackage { .. }) => {}
        other => panic!("expected UnknownPackage, got: {other:?}"),
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn obsoletes_drops_obsolete_entry() {
    // root: org.vibevm/welcome-page that obsoletes
    // org.vibevm/welcome-page-legacy. legacy: standalone, also a root.
    // After solve, legacy is removed via obsoletes.
    let m_new = r#"
[package]
group = "org.vibevm"
name = "welcome-page"
kind = "feat"
version = "0.2.0"

[obsoletes]
packages = ["org.vibevm/welcome-page-legacy"]
"#;
    let p = MapProvider::new();
    p.seed("welcome-page", m_new);
    p.seed(
        "welcome-page-legacy",
        &manifest_minimal("feat", "welcome-page-legacy", "0.1.0"),
    );

    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[
            PackageRef::parse("org.vibevm/welcome-page-legacy").unwrap(),
            PackageRef::parse("org.vibevm/welcome-page").unwrap(),
        ])
        .unwrap();
    assert_eq!(graph.packages.len(), 1);
    assert_eq!(graph.packages[0].name, "welcome-page");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#capability")]
fn requires_any_picks_first_alternative() {
    // org.vibevm/x requires_any [org.vibevm/a, org.vibevm/b]; only
    // org.vibevm/a available.
    let m_x = r#"
[package]
group = "org.vibevm"
name = "x"
kind = "feat"
version = "0.1.0"

[[requires_any]]
one_of = ["org.vibevm/a@^0.1", "org.vibevm/b@^0.1"]
"#;
    let p = MapProvider::new();
    p.seed("x", m_x);
    p.seed("a", &manifest_minimal("stack", "a", "0.1.0"));

    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm/x").unwrap()])
        .unwrap();
    // First alternative gets enqueued; resolution succeeds.
    assert_eq!(graph.packages.len(), 2);
    assert!(graph.find(&org(), "a").is_some());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
fn root_dependencies_marked() {
    let p = MapProvider::new();
    p.seed(
        "wal",
        &manifest_with_requires("flow", "wal", "0.1.0", &["org.vibevm.world/atomic-commits@^0.1"]),
    );
    p.seed(
        "atomic-commits",
        &manifest_minimal("flow", "atomic-commits", "0.1.0"),
    );
    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm.world/wal").unwrap()])
        .unwrap();
    let roots: Vec<_> = graph.roots().collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].name, "wal");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
fn dependencies_are_exact_pinned_after_solve() {
    let p = MapProvider::new();
    p.seed(
        "wal",
        &manifest_with_requires("flow", "wal", "0.1.0", &["org.vibevm.world/atomic-commits@^0.1"]),
    );
    // Two versions of atomic-commits; ^0.1 should resolve to 0.1.5.
    p.seed(
        "atomic-commits",
        &manifest_minimal("flow", "atomic-commits", "0.1.0"),
    );
    p.seed(
        "atomic-commits",
        &manifest_minimal("flow", "atomic-commits", "0.1.5"),
    );

    let solver = NaiveDepSolver::new(p);
    let graph = solver
        .solve(&[PackageRef::parse("org.vibevm.world/wal").unwrap()])
        .unwrap();
    let wal = graph.find(&org(), "wal").unwrap();
    assert_eq!(wal.dependencies.len(), 1);
    // Dep must be pinned to the exact version chosen, not the
    // original `^0.1` constraint. A future re-install reads this
    // pin verbatim to reproduce the same graph.
    let dep = &wal.dependencies[0];
    assert_eq!(dep.qualified_name(), "org.vibevm.world/atomic-commits");
    let pinned = semver::Version::parse("0.1.5").unwrap();
    assert!(dep.version.matches(&pinned));
    let other = semver::Version::parse("0.1.0").unwrap();
    assert!(!dep.version.matches(&other), "pin should not match older");
}
