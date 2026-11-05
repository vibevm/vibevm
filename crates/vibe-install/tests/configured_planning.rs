//! Actual local source drift between metadata solve and acquisition must refuse
//! before any project slot, lock, boot or hook is published.
use std::{cell::Cell, collections::BTreeSet, path::Path};
use vibe_core::{
    PackageRef,
    manifest::{Manifest, SpecFormat},
};
use vibe_install::{InstallRequest, InstallSource, NullObserver};
use vibe_registry::{CachedPackage, InPlaceMaterialised, LocalRegistry, RegistryError};
use vibe_resolver::{
    DepSolver, FeatureRequest, LocalRegistryDepProvider, ResolvedGraph, ResolvoDepSolver,
    SolveError, configuration::ConfigurationSnapshot,
};

struct DriftingSource {
    registry: LocalRegistry,
    owner: std::path::PathBuf,
    mutated: Cell<bool>,
    cache: std::path::PathBuf,
}
impl InstallSource for DriftingSource {
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        ResolvoDepSolver::new(LocalRegistryDepProvider::new(&self.registry)).solve(roots)
    }
    fn solve_configured(
        &self,
        roots: &[PackageRef],
        snapshot: &ConfigurationSnapshot,
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        snapshot.solve(
            LocalRegistryDepProvider::new(&self.registry),
            roots,
            blocked,
        )
    }
    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        vibe_install::metadata_manifest(&LocalRegistryDepProvider::new(&self.registry), p)
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        vibe_install::solve_with_visibility_mask(
            LocalRegistryDepProvider::new(&self.registry),
            None,
            roots,
            blocked,
        )
    }
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _store: &Path,
        _expected: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        if !self.mutated.replace(true) {
            let text = std::fs::read_to_string(&self.owner).unwrap();
            assert!(text.contains("optional=true"));
            std::fs::write(&self.owner, text.replace("optional=true", "optional=false")).unwrap();
        }
        self.registry.fetch(&self.registry.resolve(p)?, &self.cache)
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        panic!("copy-only fixture must not materialise an in-place slot")
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
fn actual_fetched_manifest_cannot_replace_captured_activation_meaning() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    let registry = temp.path().join("registry");
    let owner = registry.join("org.test/owner/v1.0.0");
    std::fs::create_dir_all(&owner).unwrap();
    std::fs::write(owner.join("vibe.toml"),"[package]\ngroup='org.test'\nname='owner'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.test/absent'={version='^1',optional=true}\n[[configuration.features]]\nid='enable'\nname='Enable'\neffects=[{kind='dependency',target='org.test/absent'}]").unwrap();
    let authored = "[project]\nname='test'\nversion='1.0.0'\n[requires.packages]\n'org.test/owner'={version='^1',configuration={policy='require-selection',default_features=false}}";
    std::fs::write(root.join("vibe.toml"), authored).unwrap();
    let source = DriftingSource {
        registry: LocalRegistry::new(&registry).unwrap(),
        owner: owner.join("vibe.toml"),
        mutated: Cell::new(false),
        cache: temp.path().join("isolated-store"),
    };
    let mut manifest = Manifest::read(root.join("vibe.toml")).unwrap();
    let mut workspace = vibe_workspace::Workspace::discover(&root).unwrap();
    let request = InstallRequest {
        roots: vec![],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "fixture".into(),
        offline: true,
    };
    let error = vibe_install::plan_prepared_without_publication(
        &source,
        &root,
        &mut manifest,
        &mut workspace,
        request,
        SpecFormat::Markdown,
        &NullObserver,
    )
    .unwrap_err();
    assert!(source.mutated.get());
    assert!(
        error
            .to_string()
            .contains("actual fetched manifest differs"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("vibe.toml")).unwrap(),
        authored
    );
    assert!(!root.join("vibe.lock").exists());
    let slot = vibe_workspace::vibedeps::slot_abs_path(
        &workspace.root,
        &vibe_core::Group::parse("org.test").unwrap(),
        "owner",
        &semver::Version::parse("1.0.0").unwrap(),
    );
    assert!(!slot.exists());
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        1,
        "only the original manifest may remain in the project"
    );
}
