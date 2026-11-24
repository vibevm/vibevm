//! Contribution enumeration through the actual metadata-only preview entrypoint.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS");

use super::*;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use vibe_registry::{CachedPackage, InPlaceMaterialised, LocalRegistry, RegistryError};
use vibe_resolver::{LocalRegistryDepProvider, ResolvedGraph, ResolvoDepSolver, SolveError};

/// Real metadata solver with all payload paths forbidden and solve calls counted.
struct MetadataOnlySource {
    registry: LocalRegistry,
    solves: Cell<usize>,
    graph_order: RefCell<Vec<String>>,
}

impl InstallSource for MetadataOnlySource {
    fn source_solve_limits(
        &self,
    ) -> std::result::Result<vibe_resolver::SourceSolveLimits, SolveError> {
        Ok(vibe_resolver::SourceSolveLimits::standard_operation())
    }

    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> std::result::Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        self.solves.set(self.solves.get() + 1);
        let solved = ResolvoDepSolver::new(LocalRegistryDepProvider::new(&self.registry))
            .solve_source_aware(request)?;
        *self.graph_order.borrow_mut() = solved
            .graph
            .iter()
            .map(|node| format!("{}/{}@{}", node.group, node.name, node.version))
            .collect();
        Ok(solved)
    }

    fn solve(&self, _: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("preview must use the retained metadata solver")
    }

    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("preview must use the retained metadata solver")
    }

    fn manifest_of(&self, _: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        panic!("enumeration must use the already selected view")
    }

    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        panic!("preview must not fetch payload")
    }

    fn acquire_candidate_for_install(
        &self,
        _: &vibe_registry::source_candidates::RetainedSourceCandidate,
        _: &Path,
        _: Option<&str>,
        _: &Path,
        _: &vibe_core::progress::Progress,
    ) -> std::result::Result<vibe_registry::PackageAcquisition, RegistryError> {
        panic!("preview must not acquire payload")
    }

    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        panic!("preview must not materialise a slot")
    }
}

/// Fixture declarations are metadata: their script bases deliberately do not exist.
fn preview_fixture(declarations: bool) -> (tempfile::TempDir, MetadataOnlySource, InstallPreview) {
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    for (name, version, requires, extensions) in [
        (
            "z-root",
            "2.3.4",
            "[requires.packages]\n'org.test/a-dependency' = '^1'\n",
            "[[extension]]\nid='shared'\npoint='slot:post-install'\n\
             handler={kind='script',base='never/root'}\ntitle='Root first'\n\
             unless={offline=true}\n\
             [[extension]]\nid='a-second'\npoint='slot:pre-install'\n\
             handler={kind='binary',name='not-installed'}\n",
        ),
        (
            "a-dependency",
            "1.2.3",
            "",
            "[[extension]]\nid='shared'\npoint='slot:post-install'\n\
             handler={kind='script',base='never/dependency'}\ntitle='Dependency'\n",
        ),
    ] {
        let package = registry
            .join("org.test")
            .join(name)
            .join(format!("v{version}"));
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.test'\nname='{name}'\nversion='{version}'\nkind='flow'\n{requires}{}",
                if declarations { extensions } else { "" },
            ),
        )
        .unwrap();
    }
    let root = temp.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("vibe.toml"),
        "[project]\nname='preview'\nversion='1.0.0'\n",
    )
    .unwrap();
    let mut manifest = Manifest::read(root.join("vibe.toml")).unwrap();
    let mut workspace = Workspace::discover(&root).unwrap();
    let source = MetadataOnlySource {
        registry: LocalRegistry::new(&registry).unwrap(),
        solves: Cell::new(0),
        graph_order: RefCell::new(Vec::new()),
    };
    let preview = preview_prepared(
        &source,
        &root,
        &mut manifest,
        &mut workspace,
        &Lockfile::empty("preview-test", ""),
        PreviewSourceStorage {
            read_only_store: None,
            acquisition_store: &temp.path().join("scratch"),
        },
        InstallRequest {
            roots: vec![PackageRef::parse("org.test/z-root@^2").unwrap()],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "preview-test".into(),
            offline: true,
            no_recommends: true,
        },
        SpecFormat::Markdown,
        &crate::NullObserver,
    )
    .unwrap();
    assert!(!root.join("vibe.lock").exists());
    assert!(!root.join("vibedeps").exists());
    assert!(!temp.path().join("scratch").exists());
    (temp, source, preview)
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS")]
fn selected_contributions_follow_solver_graph_and_declaration_order() {
    let (_temp, source, preview) = preview_fixture(true);
    // build_resolved_graph owns roots-first ordering, not BTreeMap iteration
    // or dependency-first topology: the real solve must put z-root before a-dependency.
    assert_eq!(
        *source.graph_order.borrow(),
        ["org.test/z-root@2.3.4", "org.test/a-dependency@1.2.3"]
    );
    let solves = source.solves.get();
    let rows = preview
        .package_contributions()
        .map(|(group, name, version, id, handler, title)| {
            (
                format!("{group}/{name}@{version}"),
                id.to_owned(),
                handler.clone(),
                title.map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    use vibe_core::manifest::ExtensionHandler;
    assert_eq!(
        rows,
        vec![
            (
                source.graph_order.borrow()[0].clone(),
                "shared".into(),
                ExtensionHandler::Script {
                    base: "never/root".into()
                },
                Some("Root first".into()),
            ),
            (
                source.graph_order.borrow()[0].clone(),
                "a-second".into(),
                ExtensionHandler::Binary {
                    name: "not-installed".into()
                },
                None,
            ),
            (
                source.graph_order.borrow()[1].clone(),
                "shared".into(),
                ExtensionHandler::Script {
                    base: "never/dependency".into()
                },
                Some("Dependency".into()),
            ),
        ]
    );
    assert_eq!(preview.package_contributions().count(), 3);
    assert_eq!(
        source.solves.get(),
        solves,
        "enumeration must not solve again"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS")]
fn selected_contributions_empty_declarations_control() {
    let (_temp, source, preview) = preview_fixture(false);
    assert_eq!(preview.package_sources().count(), 2);
    let solves = source.solves.get();
    assert_eq!(preview.package_contributions().count(), 0);
    assert_eq!(source.solves.get(), solves);
}
