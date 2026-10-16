use super::*;
use crate::DepProvider;
use std::cell::RefCell;
use vibe_core::manifest::Manifest;

struct ChangingProvider {
    reads: RefCell<HashMap<String, usize>>,
    corrupt: bool,
}

impl DepProvider for ChangingProvider {
    fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
        Ok(semver::Version::new(1, 0, 0))
    }
    fn fetch_manifest(
        &self,
        _: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        let key = format!("{name}@{version}");
        let mut reads = self.reads.borrow_mut();
        let count = reads.entry(key).or_default();
        *count += 1;
        if self.corrupt {
            return Err(DepProviderError::Other("corrupt catalog fixture".into()));
        }
        let changed = if *count > 1 {
            "[requires.packages]\n'org.example/changed'='*'\n"
        } else {
            ""
        };
        Manifest::parse_str(&format!("[package]\ngroup='org.example'\nname='{name}'\nkind='flow'\nversion='{version}'\n{changed}[features]\na=[]\nb=[]\n[features.exclusive]\nmode=['a','b']\n"))
            .map_err(|e| DepProviderError::Other(e.to_string()))
    }
}
impl VersionEnumerator for ChangingProvider {
    fn list_versions(&self, _: &Group, _: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        Ok(vec![
            semver::Version::new(1, 0, 0),
            semver::Version::new(2, 0, 0),
        ])
    }
}

#[test]
fn solve_and_manifest_observation_share_catalog_identity() {
    let solver = ResolvoDepSolver::new(ChangingProvider {
        reads: RefCell::new(HashMap::new()),
        corrupt: false,
    });
    let graph = solver
        .solve(&[PackageRef::parse("org.example/app@=1.0.0").unwrap()])
        .unwrap();
    assert_eq!(graph.packages.len(), 1);
    assert!(graph.packages[0].dependencies.is_empty());
    assert_eq!(solver.provider.reads.borrow().len(), 2);
    assert!(solver.provider.reads.borrow().values().all(|n| *n == 1));
    inspect_bound_solve();
}

#[test]
fn discarded_candidate_metadata_never_requests_global_selection() {
    let solver = ResolvoDepSolver::new(ChangingProvider {
        reads: RefCell::new(HashMap::new()),
        corrupt: false,
    });
    let root = PackageRef::parse("org.example/app@=1.0.0").unwrap();
    let graph = solver.solve(&[root.clone()]).unwrap();
    assert_eq!(graph.packages[0].version, semver::Version::new(1, 0, 0));
    assert_eq!(solver.provider.reads.borrow()["app@2.0.0"], 1);
    let corrupt = ResolvoDepSolver::new(ChangingProvider {
        reads: RefCell::new(HashMap::new()),
        corrupt: true,
    });
    assert!(
        matches!(corrupt.solve(&[root]), Err(SolveError::Provider(DepProviderError::Other(message))) if message == "corrupt catalog fixture")
    );
}

fn inspect_bound_solve() {
    struct Bound(vibe_registry::CandidateMetadata);
    impl DepProvider for Bound {
        fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
            Ok(semver::Version::new(1, 0, 0))
        }
        fn fetch_manifest(
            &self,
            _: &Group,
            _: &str,
            _: &semver::Version,
        ) -> Result<Manifest, DepProviderError> {
            Ok(self.0.manifest.clone())
        }
        fn candidate_metadata(
            &self,
            _: &Group,
            _: &str,
            _: &semver::Version,
        ) -> Result<vibe_registry::CandidateMetadata, DepProviderError> {
            Ok(self.0.clone())
        }
    }
    impl VersionEnumerator for Bound {
        fn list_versions(
            &self,
            _: &Group,
            _: &str,
        ) -> Result<Vec<semver::Version>, DepProviderError> {
            Ok(vec![semver::Version::new(1, 0, 0)])
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let bytes = "[package]\ngroup='org.example'\nname='app'\nversion='1.0.0'\nkind='flow'\n[provides]\ncapabilities=['capability:c@1.0.0']\n";
    std::fs::write(directory.path().join("vibe.toml"), bytes).unwrap();
    let bound = Bound(
        vibe_registry::CandidateMetadata::read_local(
            directory.path(),
            format!("file://{}", directory.path().display()),
        )
        .unwrap(),
    );
    let root = PackageRef::parse("org.example/app@=1.0.0").unwrap();
    let catalog = TransactionCatalog::capture(&bound, &[root.clone()]).unwrap();
    let inspection = catalog.inspect().clone();
    assert_eq!(
        inspection.digest,
        vibe_registry::catalog_digest(&inspection.candidates, &inspection.inventory)
    );
    assert_eq!(catalog.cached_capabilities().unwrap().len(), 1);
    std::fs::write(directory.path().join("vibe.toml"), "corrupt replacement").unwrap();
    let graph = ResolvoDepSolver::new(&catalog).solve(&[root]).unwrap();
    let node = &graph.packages[0];
    assert_eq!(
        catalog
            .fetch_manifest(&node.group, &node.name, &node.version)
            .unwrap(),
        bound.0.manifest
    );
    assert_eq!(catalog.inspect(), &inspection);
}
