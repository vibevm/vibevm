use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vibe_registry::{CandidateMetadata, InPlaceMaterialised, RegistryError};
use vibe_resolver::resolvo_engine::{CatalogInspection, TransactionCatalog};
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, ResolvedGraph, ResolvoDepSolver, SolveError,
    VersionEnumerator,
};

struct Provider(PathBuf);
impl DepProvider for Provider {
    fn resolve_version(
        &self,
        _: &PackageRef,
    ) -> std::result::Result<semver::Version, DepProviderError> {
        Ok(semver::Version::new(1, 0, 0))
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> std::result::Result<Manifest, DepProviderError> {
        Ok(self.candidate_metadata(g, n, v)?.manifest)
    }
    fn candidate_metadata(
        &self,
        _: &Group,
        _: &str,
        _: &semver::Version,
    ) -> std::result::Result<CandidateMetadata, DepProviderError> {
        CandidateMetadata::read_local(&self.0, format!("file://{}", self.0.display()))
            .map_err(|e| DepProviderError::Other(e.to_string()))
    }
}
impl VersionEnumerator for Provider {
    fn list_versions(
        &self,
        _: &Group,
        _: &str,
    ) -> std::result::Result<Vec<semver::Version>, DepProviderError> {
        Ok(vec![semver::Version::new(1, 0, 0)])
    }
}
struct Source {
    provider: Provider,
    effects: Rc<Cell<usize>>,
}
struct Frozen<'a> {
    original: &'a Source,
    catalog: TransactionCatalog,
}
impl InstallSource for Source {
    fn begin_transaction(
        &self,
        roots: &[PackageRef],
    ) -> std::result::Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        Ok(Some(Box::new(Frozen {
            original: self,
            catalog: TransactionCatalog::capture(&self.provider, roots)?,
        })))
    }
    fn solve(&self, _: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("must use captured source")
    }
    fn manifest_of(&self, _: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        panic!("must use captured source")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("must use captured source")
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        panic!("must use captured source")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        panic!("must not materialise")
    }
}
impl InstallSource for Frozen<'_> {
    fn catalog_inspection(&self) -> Option<&CatalogInspection> {
        Some(self.catalog.inspect())
    }
    fn solve(&self, roots: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        ResolvoDepSolver::new(&self.catalog).solve(roots)
    }
    fn manifest_of(&self, pkg: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        Ok(self.catalog.fetch_manifest(
            pkg.group.as_ref().unwrap(),
            pkg.name.as_str(),
            &self.catalog.resolve_version(pkg)?,
        )?)
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        crate::solve_with_visibility_mask(&self.catalog, None, roots, blocked)
    }
    fn resolve_and_fetch(
        &self,
        pkg: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        self.original.effects.set(self.original.effects.get() + 1);
        let metadata = self
            .catalog
            .metadata(
                pkg.group.as_ref().unwrap(),
                pkg.name.as_str(),
                &semver::Version::new(1, 0, 0),
            )
            .unwrap();
        let meta = metadata.manifest.require_package().unwrap();
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: meta.group.clone(),
                name: meta.name.clone(),
                version: meta.version.clone(),
                source_dir: self.original.provider.0.clone(),
            },
            cache_dir: self.original.provider.0.clone(),
            manifest: metadata.manifest.clone(),
            content_hash: metadata.identity.content.clone(),
            source_uri: metadata.identity.source.clone(),
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: true,
            via_redirect: None,
        })
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        panic!("must not materialise")
    }
}
struct Observer {
    effects: Rc<Cell<usize>>,
    seen: RefCell<Vec<CatalogInspection>>,
}
impl PlanObserver for Observer {
    fn on(&self, _: PlanEvent) {}
    fn catalog(&self, catalog: &CatalogInspection) {
        assert_eq!(self.effects.get(), 0);
        assert_eq!(
            catalog.digest,
            vibe_registry::catalog_digest(&catalog.candidates, &catalog.inventory)
        );
        self.seen.borrow_mut().push(catalog.clone());
    }
}
fn fixture() -> (tempfile::TempDir, Source, Observer, InstallRequest) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"),"[package]\ngroup='org.example'\nname='project'\nversion='1.0.0'\nkind='flow'\n[requires.packages]\n'org.example/app'='=1.0.0'\n").unwrap();
    let candidate = dir.path().join("candidate");
    std::fs::create_dir(&candidate).unwrap();
    std::fs::write(
        candidate.join("vibe.toml"),
        "[package]\ngroup='org.example'\nname='app'\nversion='1.0.0'\nkind='flow'\n",
    )
    .unwrap();
    let effects = Rc::new(Cell::new(0));
    let observer = Observer {
        effects: effects.clone(),
        seen: RefCell::new(Vec::new()),
    };
    let source = Source {
        provider: Provider(candidate),
        effects,
    };
    let request = InstallRequest {
        roots: vec![PackageRef::parse("org.example/app@=1.0.0").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: true,
        generated_by: "m05c1".into(),
        offline: true,
    };
    (dir, source, observer, request)
}
#[test]
fn catalog_identity_is_inspected_through_eto_before_effects() {
    let (dir, source, observer, request) = fixture();
    let before = std::fs::read(dir.path().join("vibe.toml")).unwrap();
    assert!(matches!(
        plan(&source, dir.path(), request, &observer).unwrap(),
        Plan::Ready(_)
    ));
    assert_eq!(observer.seen.borrow().len(), 1);
    assert_eq!(source.effects.get(), 1);
    assert_eq!(std::fs::read(dir.path().join("vibe.toml")).unwrap(), before);
    println!("ETO catalog: {:?}", observer.seen.borrow()[0]);
}
#[test]
fn unavailable_or_corrupt_catalog_prevents_install_effects() {
    let (dir, source, observer, mut request) = fixture();
    request.roots.clear();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[package]\ngroup='org.example'\nname='project'\nversion='1.0.0'\nkind='flow'\n",
    )
    .unwrap();
    let mut lock = Lockfile::empty("test", "2026-10-16T00:00:00Z");
    lock.meta
        .root_dependencies
        .push(PackageRef::parse("org.example/app@=1.0.0").unwrap());
    lock.write(dir.path().join("vibe.lock")).unwrap();
    let lock_before = std::fs::read(dir.path().join("vibe.lock")).unwrap();
    let before = std::fs::read(dir.path().join("vibe.toml")).unwrap();
    let path = source.provider.0.join("vibe.toml");
    std::fs::write(&path, "broken = [").unwrap();
    assert!(plan(&source, dir.path(), request.clone(), &observer).is_err());
    std::fs::remove_file(path).unwrap();
    assert!(plan(&source, dir.path(), request, &observer).is_err());
    assert_eq!(source.effects.get(), 0);
    assert!(observer.seen.borrow().is_empty());
    assert_eq!(std::fs::read(dir.path().join("vibe.toml")).unwrap(), before);
    assert_eq!(
        std::fs::read(dir.path().join("vibe.lock")).unwrap(),
        lock_before
    );
}
