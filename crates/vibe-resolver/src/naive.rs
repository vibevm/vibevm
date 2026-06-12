//! Naive depth-first dependency solver.
//!
//! Single pass, no backtracking. See [`crate`] module docs for the
//! pinned limitations and when to upgrade to a SAT-style solver.

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{CapabilityRef, Group, PackageRef, VersionSpec};

use crate::{
    ChosenEntry, DepProvider, DepProviderError, DepSolver, EnqueuedPkg, ResolvedGraph,
    ResolvedNode, SolveError, SolverState, version_satisfies,
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
            .map(|r| Ok((require_group(r)?.clone(), r.name.clone())))
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

        // Drop obsoleted entries.
        for ob in state.declared_obsolete.iter() {
            state.chosen.remove(ob);
        }

        // Build the output graph. Order: roots first (input order
        // preserved), then the rest sorted for determinism.
        let mut packages: Vec<ResolvedNode> = Vec::with_capacity(state.chosen.len());
        for (group, name) in &root_keys {
            if let Some(entry) = state.chosen.remove(&(group.clone(), name.clone())) {
                packages.push(node_from_entry(group.clone(), name.clone(), entry, true));
            }
        }
        let mut rest: Vec<((Group, String), ChosenEntry)> = state.chosen.into_iter().collect();
        rest.sort_by(|a, b| {
            (a.0.0.as_str(), a.0.1.as_str()).cmp(&(b.0.0.as_str(), b.0.1.as_str()))
        });
        for ((group, name), entry) in rest {
            // The roots-first pass above removed every root key from
            // `chosen`; a root-flagged entry surviving into `rest`
            // would break the "roots are a prefix" ordering contract
            // the lockfile's `[meta].root_dependencies` is built from.
            debug_assert!(
                !entry.is_root,
                "root-flagged entry escaped the roots-first pass: {group}/{name}"
            );
            packages.push(node_from_entry(group, name, entry, false));
        }

        // Pin every dependency reference to the exact version chosen for
        // it in the graph. Lockfile `dependencies` per entry stores
        // exact-pinned `group/name@=version` so a later `vibe install`
        // off this lockfile reproduces the same install bit-for-bit.
        let resolved_versions: std::collections::HashMap<(Group, String), semver::Version> =
            packages
                .iter()
                .map(|n| ((n.group.clone(), n.name.clone()), n.version.clone()))
                .collect();
        for node in packages.iter_mut() {
            for dep in node.dependencies.iter_mut() {
                if let Some(group) = dep.group.clone()
                    && let Some(version) = resolved_versions.get(&(group, dep.name.clone()))
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

        Ok(ResolvedGraph { packages })
    }
}

fn node_from_entry(group: Group, name: String, entry: ChosenEntry, is_root: bool) -> ResolvedNode {
    ResolvedNode {
        group,
        name,
        version: entry.version,
        dependencies: entry.direct_deps,
        is_root: is_root || entry.is_root,
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
        let key = (group.clone(), pkgref.name.clone());

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
            .fetch_manifest(&group, &pkgref.name, &version)?;

        // Refuse if this package's declared `[conflicts]` collide with
        // anything already in the graph.
        for c in &manifest.conflicts.packages {
            let cg = require_group(c)?;
            let ck = (cg.clone(), c.name.clone());
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
                .insert((cg.clone(), c.name.clone()));
        }
        for o in &manifest.obsoletes.packages {
            let og = require_group(o)?;
            state.declared_obsolete.insert((og.clone(), o.name.clone()));
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
                .push((group.clone(), pkgref.name.clone(), cap_version));
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
fn capability_version_for_provider(
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
        if state.chosen.contains_key(&(og.clone(), opt.name.clone())) {
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
mod tests {
    use specmark::verifies;

    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use vibe_core::PackageRef;

    type ProviderEntries = HashMap<(Group, String), Vec<(semver::Version, Manifest)>>;

    /// Build the canonical first-party `Group` for tests.
    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    /// In-memory provider for tests. Pre-seeded with `(group, name) →
    /// list-of-(version, manifest)` pairs. Identity is `(group, name)`;
    /// `kind` is only manifest metadata.
    struct MapProvider {
        entries: RefCell<ProviderEntries>,
    }

    impl MapProvider {
        fn new() -> Self {
            MapProvider {
                entries: RefCell::new(HashMap::new()),
            }
        }
        fn seed(&self, name: &str, manifest_toml: &str) {
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
            let key = (group.clone(), pkgref.name.clone());
            let entries = self.entries.borrow();
            let candidates =
                entries
                    .get(&key)
                    .ok_or_else(|| crate::DepProviderError::UnknownPackage {
                        group: group.clone(),
                        name: pkgref.name.clone(),
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
                    name: pkgref.name.clone(),
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

    fn manifest_minimal(kind: &str, name: &str, version: &str) -> String {
        format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n"
        )
    }

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
            .solve(&[PackageRef::parse("org.vibevm/wal").unwrap()])
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
            &manifest_with_requires("stack", "rust", "0.1.0", &["org.vibevm/wal@^0.1"]),
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
            .solve(&[PackageRef::parse("org.vibevm/wal@^0.1").unwrap()])
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
                PackageRef::parse("org.vibevm/wal@^0.1").unwrap(),
                PackageRef::parse("org.vibevm/wal@^0.2").unwrap(),
            ])
            .unwrap_err();
        match err {
            SolveError::VersionConflict { package, .. } => {
                assert_eq!(package, "org.vibevm/wal");
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
            &manifest_with_requires("flow", "wal", "0.1.0", &["org.vibevm/atomic-commits@^0.1"]),
        );
        p.seed(
            "atomic-commits",
            &manifest_minimal("flow", "atomic-commits", "0.1.0"),
        );
        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("org.vibevm/wal").unwrap()])
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
            &manifest_with_requires("flow", "wal", "0.1.0", &["org.vibevm/atomic-commits@^0.1"]),
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
            .solve(&[PackageRef::parse("org.vibevm/wal").unwrap()])
            .unwrap();
        let wal = graph.find(&org(), "wal").unwrap();
        assert_eq!(wal.dependencies.len(), 1);
        // Dep must be pinned to the exact version chosen, not the
        // original `^0.1` constraint. A future re-install reads this
        // pin verbatim to reproduce the same graph.
        let dep = &wal.dependencies[0];
        assert_eq!(dep.qualified_name(), "org.vibevm/atomic-commits");
        let pinned = semver::Version::parse("0.1.5").unwrap();
        assert!(dep.version.matches(&pinned));
        let other = semver::Version::parse("0.1.0").unwrap();
        assert!(!dep.version.matches(&other), "pin should not match older");
    }
}
