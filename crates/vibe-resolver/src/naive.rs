//! Naive depth-first dependency solver.
//!
//! Single pass, no backtracking. See [`crate`] module docs for the
//! pinned limitations and when to upgrade to a SAT-style solver.

use vibe_core::manifest::PackageManifest;
use vibe_core::{CapabilityRef, PackageKind, PackageRef, VersionSpec};

use crate::{
    ChosenEntry, DepProvider, DepSolver, EnqueuedPkg, ResolvedGraph, ResolvedNode, SolveError,
    SolverState, version_satisfies,
};

/// DFS solver over a [`DepProvider`].
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
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        let mut state = SolverState::new();
        let root_keys: Vec<(PackageKind, String)> = roots
            .iter()
            .map(|r| (r.kind, r.name.clone()))
            .collect();
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
        for (kind, name) in &root_keys {
            if let Some(entry) = state.chosen.remove(&(*kind, name.clone())) {
                packages.push(node_from_entry(*kind, name.clone(), entry, true));
            }
        }
        let mut rest: Vec<((PackageKind, String), ChosenEntry)> =
            state.chosen.into_iter().collect();
        rest.sort_by(|a, b| {
            (a.0.0.as_str(), a.0.1.as_str()).cmp(&(b.0.0.as_str(), b.0.1.as_str()))
        });
        for ((kind, name), entry) in rest {
            packages.push(node_from_entry(kind, name, entry, false));
        }

        // Pin every dependency reference to the exact version chosen for
        // it in the graph. Lockfile `dependencies` per entry stores
        // exact-pinned `kind:name@=version` so a later `vibe install`
        // off this lockfile reproduces the same install bit-for-bit.
        let resolved_versions: std::collections::HashMap<(PackageKind, String), semver::Version> =
            packages
                .iter()
                .map(|n| ((n.kind, n.name.clone()), n.version.clone()))
                .collect();
        for node in packages.iter_mut() {
            for dep in node.dependencies.iter_mut() {
                if let Some(version) = resolved_versions.get(&(dep.kind, dep.name.clone()))
                    && let Ok(req) = semver::VersionReq::parse(&format!("={version}"))
                {
                    *dep = PackageRef {
                        kind: dep.kind,
                        name: dep.name.clone(),
                        version: VersionSpec::Req(req),
                    };
                }
            }
        }

        Ok(ResolvedGraph { packages })
    }
}

fn node_from_entry(
    kind: PackageKind,
    name: String,
    entry: ChosenEntry,
    is_root: bool,
) -> ResolvedNode {
    ResolvedNode {
        kind,
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
        let key = (pkgref.kind, pkgref.name.clone());

        // If a conflict was declared against this exact (kind, name) by
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
                        .any(|c| c.kind == pkgref.kind && c.name == pkgref.name)
                })
                .map(|((k, n), _)| format!("{k}:{n}"))
                .unwrap_or_else(|| "<unknown>".to_string());
            return Err(SolveError::ConflictsDeclared {
                package: against,
                against: format!("{}:{}", pkgref.kind, pkgref.name),
            });
        }

        // Already chosen? Reconcile constraints.
        if let Some(existing) = state.chosen.get(&key) {
            if !version_satisfies(&pkgref.version, &existing.version) {
                return Err(SolveError::VersionConflict {
                    package: format!("{}:{}", pkgref.kind, pkgref.name),
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
            .fetch_manifest(pkgref.kind, &pkgref.name, &version)?;

        // Refuse if this package's declared `[conflicts]` collide with
        // anything already in the graph.
        for c in &manifest.conflicts.packages {
            let ck = (c.kind, c.name.clone());
            if state.chosen.contains_key(&ck) {
                return Err(SolveError::ConflictsDeclared {
                    package: format!("{}:{}", pkgref.kind, pkgref.name),
                    against: format!("{}:{}", c.kind, c.name),
                });
            }
        }

        // Mark conflicts and obsoletes so future enqueues respect them.
        for c in &manifest.conflicts.packages {
            state.declared_conflicts.insert((c.kind, c.name.clone()));
        }
        for o in &manifest.obsoletes.packages {
            state.declared_obsolete.insert((o.kind, o.name.clone()));
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
                .push((pkgref.kind, pkgref.name.clone(), cap_version));
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
                via: Some(format!("{}:{}", pkgref.kind, pkgref.name)),
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
    manifest: &PackageManifest,
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
                requirer: format!("{}:{}", pkgref.kind, pkgref.name),
            });
        }
    }
    Ok(())
}

/// The capability's "declared version" for the providers_index. A
/// provides-side `CapabilityRef` is semantically expected to be an
/// exact version (`@0.3.0`); the type system still uses the same
/// `VersionSpec` as the requires side, so we extract the exact form
/// here. If the provider declared no version (`VersionSpec::Latest`),
/// fall back to the providing package's resolved version — that's the
/// most useful default for capability matching across versions.
fn capability_version_for_provider(
    cap: &CapabilityRef,
    package_version: &semver::Version,
) -> semver::Version {
    match &cap.version {
        VersionSpec::Latest => package_version.clone(),
        VersionSpec::Req(req) => {
            let s = req.to_string();
            if let Some(rest) = s.strip_prefix('=')
                && let Ok(v) = semver::Version::parse(rest.trim())
            {
                return v;
            }
            // Non-exact form on the provides side is unusual; treat it
            // as "this version" and fall back to the package's version
            // so capability range-matching still works for the common
            // case the user intended.
            package_version.clone()
        }
    }
}

fn handle_disjunction(
    state: &mut SolverState,
    requirer: &PackageRef,
    disj: &vibe_core::manifest::RequiresAny,
) -> Result<(), SolveError> {
    if disj.one_of.is_empty() {
        return Err(SolveError::DisjunctionUnsatisfiable {
            requirer: format!("{}:{}", requirer.kind, requirer.name),
            alternatives: Vec::new(),
        });
    }
    // If any alternative is already chosen, satisfied.
    for opt in &disj.one_of {
        if state.chosen.contains_key(&(opt.kind, opt.name.clone())) {
            return Ok(());
        }
    }
    // Otherwise: enqueue the first alternative. Naive — no backtracking
    // when downstream conflicts emerge.
    let first = disj.one_of.first().expect("one_of non-empty above");
    state.queue.push_back(EnqueuedPkg {
        pkgref: first.clone(),
        via: Some(format!(
            "[[requires_any]] of {}:{}",
            requirer.kind, requirer.name
        )),
        is_root: false,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use vibe_core::PackageRef;

    type ProviderEntries =
        HashMap<(PackageKind, String), Vec<(semver::Version, PackageManifest)>>;

    /// In-memory provider for tests. Pre-seeded with `(kind, name) →
    /// list-of-(version, manifest)` pairs.
    struct MapProvider {
        entries: RefCell<ProviderEntries>,
    }

    impl MapProvider {
        fn new() -> Self {
            MapProvider {
                entries: RefCell::new(HashMap::new()),
            }
        }
        fn seed(&self, kind: PackageKind, name: &str, manifest_toml: &str) {
            let m: PackageManifest = toml::from_str(manifest_toml).unwrap();
            let v = m.package.version.clone();
            self.entries
                .borrow_mut()
                .entry((kind, name.to_string()))
                .or_default()
                .push((v, m));
        }
    }

    impl DepProvider for MapProvider {
        fn resolve_version(
            &self,
            pkgref: &PackageRef,
        ) -> Result<semver::Version, crate::DepProviderError> {
            let key = (pkgref.kind, pkgref.name.clone());
            let entries = self.entries.borrow();
            let candidates = entries
                .get(&key)
                .ok_or_else(|| crate::DepProviderError::UnknownPackage {
                    kind: pkgref.kind,
                    name: pkgref.name.clone(),
                })?;
            // Pick highest matching version, prefer stable.
            let mut versions: Vec<&semver::Version> = candidates.iter().map(|(v, _)| v).collect();
            versions.sort();
            let pick = versions
                .iter()
                .rev()
                .find(|v| pkgref.version.matches(v) && v.pre.is_empty())
                .or_else(|| {
                    versions.iter().rev().find(|v| pkgref.version.matches(v))
                })
                .copied()
                .ok_or_else(|| crate::DepProviderError::NoMatchingVersion {
                    kind: pkgref.kind,
                    name: pkgref.name.clone(),
                    constraint: format!("{}", pkgref.version),
                })?;
            Ok(pick.clone())
        }
        fn fetch_manifest(
            &self,
            kind: PackageKind,
            name: &str,
            version: &semver::Version,
        ) -> Result<PackageManifest, crate::DepProviderError> {
            let key = (kind, name.to_string());
            let entries = self.entries.borrow();
            let candidates =
                entries
                    .get(&key)
                    .ok_or_else(|| crate::DepProviderError::UnknownPackage {
                        kind,
                        name: name.to_string(),
                    })?;
            candidates
                .iter()
                .find(|(v, _)| v == version)
                .map(|(_, m)| m.clone())
                .ok_or_else(|| crate::DepProviderError::Other(format!(
                    "no manifest for {kind}:{name}@{version}"
                )))
        }
    }

    fn manifest_minimal(kind: &str, name: &str, version: &str) -> String {
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n")
    }

    fn manifest_with_requires(
        kind: &str,
        name: &str,
        version: &str,
        requires: &[&str],
    ) -> String {
        let mut s = manifest_minimal(kind, name, version);
        s.push_str("\n[requires]\npackages = [\n");
        for r in requires {
            s.push_str(&format!("    \"{r}\",\n"));
        }
        s.push_str("]\n");
        s
    }

    #[test]
    fn resolves_single_root_with_no_deps() {
        let p = MapProvider::new();
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.1.0"),
        );
        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("flow:wal").unwrap()])
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
            PackageKind::Feat,
            "ui",
            &manifest_with_requires("feat", "ui", "0.1.0", &["stack:rust@^0.1"]),
        );
        p.seed(
            PackageKind::Stack,
            "rust",
            &manifest_with_requires("stack", "rust", "0.1.0", &["flow:wal@^0.1"]),
        );
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.1.0"),
        );

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("feat:ui").unwrap()])
            .unwrap();
        assert_eq!(graph.packages.len(), 3);
        assert!(graph.find(PackageKind::Feat, "ui").unwrap().is_root);
        assert!(!graph.find(PackageKind::Stack, "rust").unwrap().is_root);
        assert!(!graph.find(PackageKind::Flow, "wal").unwrap().is_root);
        assert_eq!(
            graph.find(PackageKind::Feat, "ui").unwrap().dependencies.len(),
            1
        );
    }

    #[test]
    fn picks_highest_matching_for_range() {
        let p = MapProvider::new();
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.1.0"),
        );
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.1.5"),
        );
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.2.0"),
        );

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("flow:wal@^0.1").unwrap()])
            .unwrap();
        assert_eq!(graph.packages[0].version.to_string(), "0.1.5");
    }

    #[test]
    fn detects_version_conflict_across_paths() {
        // Two roots: one wants ^0.1, the other wants ^0.2. Naive picks
        // the first root's version (0.1.5) and the second's constraint
        // is checked against it — fails because 0.2.0 doesn't match ^0.1.
        let p = MapProvider::new();
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.1.5"),
        );
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.2.0"),
        );

        let solver = NaiveDepSolver::new(p);
        let err = solver
            .solve(&[
                PackageRef::parse("flow:wal@^0.1").unwrap(),
                PackageRef::parse("flow:wal@^0.2").unwrap(),
            ])
            .unwrap_err();
        match err {
            SolveError::VersionConflict { package, .. } => {
                assert_eq!(package, "flow:wal");
            }
            other => panic!("expected VersionConflict, got: {other:?}"),
        }
    }

    #[test]
    fn detects_conflicts_declaration() {
        // feat:ui declares conflicts with flow:legacy-wal; if both are
        // roots, solver refuses.
        let m_ui = r#"
[package]
name = "ui"
kind = "feat"
version = "0.1.0"

[conflicts]
packages = ["flow:legacy-wal"]
"#;
        let p = MapProvider::new();
        p.seed(PackageKind::Feat, "ui", m_ui);
        p.seed(
            PackageKind::Flow,
            "legacy-wal",
            &manifest_minimal("flow", "legacy-wal", "0.0.1"),
        );

        let solver = NaiveDepSolver::new(p);
        let err = solver
            .solve(&[
                PackageRef::parse("flow:legacy-wal").unwrap(),
                PackageRef::parse("feat:ui").unwrap(),
            ])
            .unwrap_err();
        match err {
            SolveError::ConflictsDeclared { .. } => {}
            other => panic!("expected ConflictsDeclared, got: {other:?}"),
        }
    }

    #[test]
    fn capability_requires_satisfied_by_already_seen_provider() {
        // Order matters: stack:rust provides ui:landing-page, then
        // feat:home requires it. Naive provider-then-consumer ordering
        // succeeds. Reversed order would fail (limitation).
        let m_stack = r#"
[package]
name = "rust"
kind = "stack"
version = "0.1.0"

[provides]
capabilities = ["ui:landing-page@0.3.0"]
"#;
        let m_feat = r#"
[package]
name = "home"
kind = "feat"
version = "0.1.0"

[requires]
capabilities = ["ui:landing-page@^0.3"]
"#;
        let p = MapProvider::new();
        p.seed(PackageKind::Stack, "rust", m_stack);
        p.seed(PackageKind::Feat, "home", m_feat);

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[
                PackageRef::parse("stack:rust").unwrap(),
                PackageRef::parse("feat:home").unwrap(),
            ])
            .unwrap();
        assert_eq!(graph.packages.len(), 2);
    }

    #[test]
    fn capability_requires_self_satisfaction() {
        // A package that both provides and requires the same capability
        // with the same exact version trivially satisfies itself.
        let m = r#"
[package]
name = "magic"
kind = "feat"
version = "0.1.0"

[provides]
capabilities = ["x:y@0.1.0"]

[requires]
capabilities = ["x:y@^0.1"]
"#;
        let p = MapProvider::new();
        p.seed(PackageKind::Feat, "magic", m);
        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("feat:magic").unwrap()])
            .unwrap();
        assert_eq!(graph.packages.len(), 1);
    }

    #[test]
    fn capability_requires_unmet_errors() {
        let m = r#"
[package]
name = "home"
kind = "feat"
version = "0.1.0"

[requires]
capabilities = ["ui:landing-page@^0.3"]
"#;
        let p = MapProvider::new();
        p.seed(PackageKind::Feat, "home", m);
        let solver = NaiveDepSolver::new(p);
        let err = solver
            .solve(&[PackageRef::parse("feat:home").unwrap()])
            .unwrap_err();
        match err {
            SolveError::CapabilityUnmet { capability, .. } => {
                assert!(capability.contains("ui:landing-page"));
            }
            other => panic!("expected CapabilityUnmet, got: {other:?}"),
        }
    }

    #[test]
    fn unknown_package_propagates() {
        let p = MapProvider::new();
        let solver = NaiveDepSolver::new(p);
        let err = solver
            .solve(&[PackageRef::parse("flow:ghost").unwrap()])
            .unwrap_err();
        match err {
            SolveError::Provider(crate::DepProviderError::UnknownPackage { .. }) => {}
            other => panic!("expected UnknownPackage, got: {other:?}"),
        }
    }

    #[test]
    fn obsoletes_drops_obsolete_entry() {
        // root: feat:welcome-page that obsoletes feat:welcome-page-legacy.
        // legacy: standalone, also a root.
        // After solve, legacy is removed via obsoletes.
        let m_new = r#"
[package]
name = "welcome-page"
kind = "feat"
version = "0.2.0"

[obsoletes]
packages = ["feat:welcome-page-legacy"]
"#;
        let p = MapProvider::new();
        p.seed(PackageKind::Feat, "welcome-page", m_new);
        p.seed(
            PackageKind::Feat,
            "welcome-page-legacy",
            &manifest_minimal("feat", "welcome-page-legacy", "0.1.0"),
        );

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[
                PackageRef::parse("feat:welcome-page-legacy").unwrap(),
                PackageRef::parse("feat:welcome-page").unwrap(),
            ])
            .unwrap();
        assert_eq!(graph.packages.len(), 1);
        assert_eq!(graph.packages[0].name, "welcome-page");
    }

    #[test]
    fn requires_any_picks_first_alternative() {
        // feat:x requires_any [stack:a, stack:b]; only stack:a available.
        let m_x = r#"
[package]
name = "x"
kind = "feat"
version = "0.1.0"

[[requires_any]]
one_of = ["stack:a@^0.1", "stack:b@^0.1"]
"#;
        let p = MapProvider::new();
        p.seed(PackageKind::Feat, "x", m_x);
        p.seed(
            PackageKind::Stack,
            "a",
            &manifest_minimal("stack", "a", "0.1.0"),
        );

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("feat:x").unwrap()])
            .unwrap();
        // First alternative gets enqueued; resolution succeeds.
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.find(PackageKind::Stack, "a").is_some());
    }

    #[test]
    fn legacy_dependencies_section_migrates_into_solver_graph() {
        // Manifest in the v1 [dependencies] form must be parsed into
        // [requires] before the solver sees it. PackageManifest::read
        // does this, so MapProvider seeding via toml::from_str also
        // gets it via normalize_legacy_deps in our test setup. Verify.
        let legacy = r#"
[package]
name = "legacy"
kind = "feat"
version = "0.1.0"

[dependencies]
required = ["flow:wal@^0.1"]
"#;
        let mut m: PackageManifest = toml::from_str(legacy).unwrap();
        m.normalize_legacy_deps();
        let p = MapProvider::new();
        p.entries
            .borrow_mut()
            .entry((PackageKind::Feat, "legacy".to_string()))
            .or_default()
            .push((m.package.version.clone(), m));
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_minimal("flow", "wal", "0.1.0"),
        );

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("feat:legacy").unwrap()])
            .unwrap();
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.find(PackageKind::Flow, "wal").is_some());
    }

    #[test]
    fn root_dependencies_marked() {
        let p = MapProvider::new();
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_with_requires("flow", "wal", "0.1.0", &["flow:atomic-commits@^0.1"]),
        );
        p.seed(
            PackageKind::Flow,
            "atomic-commits",
            &manifest_minimal("flow", "atomic-commits", "0.1.0"),
        );
        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("flow:wal").unwrap()])
            .unwrap();
        let roots: Vec<_> = graph.roots().collect();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "wal");
    }

    #[test]
    fn dependencies_are_exact_pinned_after_solve() {
        let p = MapProvider::new();
        p.seed(
            PackageKind::Flow,
            "wal",
            &manifest_with_requires("flow", "wal", "0.1.0", &["flow:atomic-commits@^0.1"]),
        );
        // Two versions of atomic-commits; ^0.1 should resolve to 0.1.5.
        p.seed(
            PackageKind::Flow,
            "atomic-commits",
            &manifest_minimal("flow", "atomic-commits", "0.1.0"),
        );
        p.seed(
            PackageKind::Flow,
            "atomic-commits",
            &manifest_minimal("flow", "atomic-commits", "0.1.5"),
        );

        let solver = NaiveDepSolver::new(p);
        let graph = solver
            .solve(&[PackageRef::parse("flow:wal").unwrap()])
            .unwrap();
        let wal = graph.find(PackageKind::Flow, "wal").unwrap();
        assert_eq!(wal.dependencies.len(), 1);
        // Dep must be pinned to the exact version chosen, not the
        // original `^0.1` constraint. A future re-install reads this
        // pin verbatim to reproduce the same graph.
        let dep = &wal.dependencies[0];
        assert_eq!(dep.qualified_name(), "flow:atomic-commits");
        let pinned = semver::Version::parse("0.1.5").unwrap();
        assert!(dep.version.matches(&pinned));
        let other = semver::Version::parse("0.1.0").unwrap();
        assert!(!dep.version.matches(&other), "pin should not match older");
    }
}
