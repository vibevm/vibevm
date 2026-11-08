//! A real legacy Git slot cannot bypass a configured copy operation's boundary.
use super::*;
use vibe_registry::LocalRegistry;
use vibe_resolver::{
    DepProvider, DepProviderError, LocalRegistryDepProvider, VersionEnumerator,
    configuration::ConfigurationSnapshot,
};

#[derive(Clone, Copy)]
struct Mixed<'a> {
    git: &'a Source,
    local: &'a LocalRegistry,
    cache: &'a Path,
}
impl DepProvider for Mixed<'_> {
    fn resolve_version(&self, p: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.list_versions(p.group.as_ref().unwrap(), p.name.as_str())?
            .into_iter()
            .filter(|v| p.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::Other("fixture candidate absent".into()))
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        if n == "giant" {
            self.git
                .registry
                .fetch_dep_manifest(g, n, v)
                .map_err(|e| DepProviderError::Other(e.to_string()))
        } else {
            LocalRegistryDepProvider::new(self.local).fetch_manifest(g, n, v)
        }
    }
}
impl VersionEnumerator for Mixed<'_> {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        if n == "giant" {
            self.git
                .registry
                .list_versions(g, n)
                .map_err(|e| DepProviderError::Other(e.to_string()))
        } else {
            LocalRegistryDepProvider::new(self.local).list_versions(g, n)
        }
    }
}
impl InstallSource for Mixed<'_> {
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        retained::limits()
    }
    /// One metadata registry: the local packages plus the Git-mirrored giant.
    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        let root = self.git.metadata.with_file_name("mixed-metadata");
        self.git.mirror_metadata(&root)?;
        copy_tree(self.local.root(), &root)
            .map_err(|e| SolveError::from(DepProviderError::Other(e.to_string())))?;
        retained::solve(&root, &mocked_selection(request))
    }
    fn acquire_candidate_for_install(
        &self,
        candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        store: &Path,
        expected: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        incumbent_acquire(
            self,
            &self.git.lock,
            &retained::exact(candidate),
            store,
            expected,
            slot,
            progress,
        )
    }
    fn solve(&self, r: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        self.git.solve(r)
    }
    fn solve_masked(
        &self,
        r: &[PackageRef],
        b: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.git.solve_masked(r, b)
    }
    fn solve_configured(
        &self,
        r: &[PackageRef],
        snapshot: &ConfigurationSnapshot,
        b: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        snapshot.solve(*self, r, b)
    }
    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        vibe_install::metadata_manifest(self, p)
    }
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.local.fetch(&self.local.resolve(p)?, self.cache)
    }
    fn prepare_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<PreparedInPlace, RegistryError> {
        self.git.prepare_in_place(p, slot)
    }
    fn materialise_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        self.git.materialise_in_place(p, slot)
    }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn configured_copy_graph_refuses_legacy_in_place_before_preparing_its_checkout() {
    let f = Fixture::new();
    let first = f.publish("1.0.0", "");
    f.seed();
    f.publish("1.1.0", "");
    let local = f._temp.path().join("local");
    write(
        &local,
        "org.vibevm/driver/v1.0.0/vibe.toml",
        "[package]\ngroup='org.vibevm'\nname='driver'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.vibevm/giant'='^1'\n[configuration]\n",
    );
    write(
        &f.project,
        "vibe.toml",
        "[project]\nname='mixed'\nversion='1.0.0'\n[requires.packages]\n'org.vibevm/driver'={version='^1',configuration={policy='require-selection',default_features=false}}\n",
    );
    let registry = LocalRegistry::new(&local).unwrap();
    let cache = f._temp.path().join("copy-store");
    let source = Mixed {
        git: &f.source,
        local: &registry,
        cache: &cache,
    };
    let before = worktree(&f.project);
    let config = fs::read(f.slot.join(".git/config")).unwrap();
    let error = vibe_install::plan(
        &source,
        &f.project,
        InstallRequest {
            roots: vec![],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "fixture".into(),
            offline: true,
        },
        &NullObserver,
    )
    .unwrap_err();
    assert!(error.to_string().contains("in-place"), "{error}");
    assert!(
        matches!(
            error,
            vibe_install::Error::Registry(RegistryError::Configuration(
                vibe_core::manifest::PackageConfigurationError::Unsupported { .. }
            ))
        ),
        "{error}"
    );
    assert!(
        f.source.prepared.borrow().is_empty(),
        "configured planning must not invoke legacy preparation"
    );
    assert_eq!(f.source.unbound_placements.get(), 0);
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), first);
    assert_eq!(fs::read(f.slot.join(".git/config")).unwrap(), config);
    assert_eq!(worktree(&f.project), before);
    assert!(!f.slot.join("hook.log").exists());
}
