use super::*;
use std::{cell::RefCell, sync::Arc};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError, ResolvedPackage};
use vibe_resolver::resolvo_engine::request::NormalizedRequest;
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, ResolvedGraph, ResolvoDepSolver, SolveError,
    VersionEnumerator,
};
fn p(n: &str) -> PackageRef {
    PackageRef::parse(&format!("org.example/{n}")).unwrap()
}
fn m(n: &str, body: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "[package]\ngroup='org.example'\nname='{n}'\nkind='flow'\nversion='1.0.0'\n{body}"
    ))
    .unwrap()
}
#[derive(Clone)]
struct Source {
    manifests: Arc<Vec<Manifest>>,
    intent: Arc<NormalizedRequest>,
    calls: Arc<RefCell<Vec<Vec<PackageRef>>>>,
    bound: bool,
}
impl Source {
    fn new(manifests: Vec<Manifest>, roots: Vec<PackageRef>) -> Self {
        Self {
            manifests: Arc::new(manifests),
            intent: Arc::new(NormalizedRequest::legacy(
                &roots,
                &FeatureRequest {
                    explicit: vec!["kept".into()],
                    no_defaults: true,
                    all: false,
                },
            )),
            calls: Default::default(),
            bound: false,
        }
    }
}
impl DepProvider for Source {
    fn resolve_version(
        &self,
        p: &PackageRef,
    ) -> std::result::Result<semver::Version, DepProviderError> {
        self.list_versions(p.group.as_ref().unwrap(), p.name.as_str())?
            .into_iter()
            .filter(|v| p.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::NoMatchingVersion {
                group: p.group.clone().unwrap(),
                name: p.name.to_string(),
                constraint: p.version.to_string(),
            })
    }
    fn fetch_manifest(
        &self,
        _: &Group,
        n: &str,
        _v: &semver::Version,
    ) -> std::result::Result<Manifest, DepProviderError> {
        self.manifests
            .iter()
            .find(|m| {
                m.require_package().unwrap().name == n
                    && m.require_package().unwrap().version == *_v
            })
            .cloned()
            .ok_or_else(|| DepProviderError::Other("missing fixture".into()))
    }
}
impl VersionEnumerator for Source {
    fn list_versions(
        &self,
        g: &Group,
        n: &str,
    ) -> std::result::Result<Vec<semver::Version>, DepProviderError> {
        let versions: Vec<_> = self
            .manifests
            .iter()
            .filter_map(|m| {
                let p = m.require_package().unwrap();
                (p.name == n).then(|| p.version.clone())
            })
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: g.clone(),
                name: n.into(),
            })
        } else {
            Ok(versions)
        }
    }
}
impl InstallSource for Source {
    fn begin_normalized_transaction(
        &self,
        r: &NormalizedRequest,
    ) -> std::result::Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        assert_eq!(r.roots, self.intent.roots);
        assert_eq!(r.features.explicit, self.intent.features.explicit);
        assert!(r.features.no_defaults);
        let mut bound = self.clone();
        bound.bound = true;
        Ok(Some(Box::new(bound)))
    }
    fn normalized_request(&self) -> Option<&NormalizedRequest> {
        assert!(self.bound);
        Some(&self.intent)
    }
    fn solve(&self, roots: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        assert!(self.bound);
        self.calls.borrow_mut().push(roots.to_vec());
        ResolvoDepSolver::new(self.clone()).solve(roots)
    }
    fn manifest_of(&self, p: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        assert!(self.bound);
        Ok(self.fetch_manifest(
            p.group.as_ref().unwrap(),
            p.name.as_str(),
            &self.resolve_version(p)?,
        )?)
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        assert!(self.bound);
        self.calls.borrow_mut().push(roots.to_vec());
        crate::solve_with_visibility_mask(self.clone(), None, roots, blocked)
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        panic!("no fetch before consistent resolution")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        panic!("no install effects")
    }
}
#[test]
fn visibility_retry_preserves_requests_and_exact_choices() {
    let source = Source::new(
        vec![
            m(
                "app",
                "[requires.packages]\n'org.example/a'='*'\n'org.example/b'='*'\n[[requires_any]]\none_of=['org.example/missing','org.example/b']\n",
            ),
            m(
                "a",
                "[requires.packages]\n'org.example/hidden'={version='*',access='private'}\n",
            ),
            m("b", ""),
            m("hidden", ""),
        ],
        vec![p("app@=1.0.0"), p("app@^1")],
    );
    let planning = transaction::PlanningSource::capture_request(&source, &source.intent).unwrap();
    let initial = planning.solve(&source.intent.roots).unwrap();
    let root = Manifest::parse_str("[project]\nname='fixture'\nversion='1.0.0'\n").unwrap();
    let effective = resolve_effective(
        &planning,
        &source.intent.roots,
        &root,
        "org.example/root",
        initial,
    )
    .unwrap();
    assert!(effective.iterations > 1);
    assert!(
        effective
            .graph
            .find(p("hidden").group.as_ref().unwrap(), "hidden")
            .is_none()
    );
    assert_eq!(
        effective.graph.packages[0].dependencies,
        vec![p("a@=1.0.0"), p("b@=1.0.0"), p("b@=1.0.0")]
    );
    assert!(
        source
            .calls
            .borrow()
            .iter()
            .all(|r| *r == source.intent.roots)
    );
    assert!(std::ptr::eq(
        planning.normalized_request().unwrap(),
        source.intent.as_ref()
    ));
    let again = planning
        .solve_masked(
            &source.intent.roots,
            &[("org.example/a".into(), "org.example/hidden".into())].into(),
        )
        .unwrap();
    assert_eq!(format!("{:?}", effective.graph), format!("{again:?}"));
}
#[test]
fn conditional_route_retains_all_root_constraints() {
    let source = Source::new(
        vec![m(
            "app",
            "[provides]\ncapabilities=['stack:rust']\n[target.'context(stack:rust)'.dependencies.packages]\n'org.example/app'='=2.0.0'\n",
        )],
        vec![p("app@=1.0.0"), p("app@^1")],
    );
    let planning = transaction::PlanningSource::capture_request(&source, &source.intent).unwrap();
    let manifest = source.manifests[0].clone();
    let meta = manifest.require_package().unwrap().clone();
    let dir = tempfile::tempdir().unwrap();
    let cached = CachedPackage {
        resolved: ResolvedPackage {
            group: meta.group,
            name: meta.name,
            version: meta.version,
            source_dir: dir.path().into(),
        },
        cache_dir: dir.path().into(),
        manifest,
        content_hash: "fixture".into(),
        source_uri: "fixture".into(),
        registry_name: None,
        source_ref: None,
        resolved_commit: None,
        overridden: false,
        is_git_source: false,
        is_path_source: false,
        is_embedded: false,
        is_local: true,
        via_redirect: None,
    };
    let mut fetched = vec![Fetched {
        cached,
        feature_expansion: Default::default(),
        meta: NodeInstallMeta {
            dependencies: vec![],
            is_root: true,
        },
        embedded_sources: vec![],
        in_place_incremental: false,
    }];
    let progress = vibe_core::progress::Progress::default();
    let task = progress.task("conditional fixture");
    let error = expand_conditional_deps(
        &planning,
        &source.intent.roots,
        &Lockfile::empty("fixture", "now"),
        dir.path(),
        dir.path(),
        dir.path(),
        &[],
        &source.intent.features,
        true,
        &Manifest::parse_str("[project]\nname='fixture'\nversion='1.0.0'\n").unwrap(),
        "org.example/root",
        &mut fetched,
        &mut Analysis::default(),
        &crate::NullObserver,
        &task,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        crate::Error::Solve(SolveError::Unsatisfiable { .. })
    ));
    let calls = source.calls.borrow();
    let roots = calls.last().unwrap();
    assert_eq!(&roots[..2], source.intent.roots.as_slice());
    assert_eq!(roots[2], p("app@=2.0.0"));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
#[test]
fn inconsistent_or_unsupported_route_refuses_before_effects() {
    let source = Source::new(vec![m("app", "")], vec![p("app@=1.0.0"), p("app@=2.0.0")]);
    let planning = transaction::PlanningSource::capture_request(&source, &source.intent).unwrap();
    assert!(matches!(
        planning.solve(&source.intent.roots),
        Err(SolveError::Unsatisfiable { .. })
    ));
    assert!(matches!(
        planning.solve_masked(&source.intent.roots, &Default::default()),
        Err(SolveError::Unsatisfiable { .. })
    ));
    assert_eq!(source.calls.borrow().len(), 2);
}

#[test]
fn workspace_root_constraints() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("member")).unwrap();
    std::fs::write(dir.path().join("vibe.toml"),"[project]\nname='root'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n[requires.packages]\n'org.example/app'='=1.0.0'\n'org.example/unrelated'='*'\n").unwrap();
    std::fs::write(dir.path().join("member/vibe.toml"),"[package]\ngroup='org.example'\nname='member'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.example/app'='>=1'\n").unwrap();
    let mut workspace = Workspace::discover(dir.path()).unwrap();
    let lock_path = dir.path().join("vibe.lock");
    let packages: String = ["app", "unrelated"].iter().map(|name| format!(
        "[[package]]\nkind='flow'\ngroup='org.example'\nname='{name}'\nversion='1.0.0'\nsource_url='fixture'\nsource_kind='registry'\ncontent_hash='sha256:{}'\n", "a".repeat(64)
    )).collect();
    std::fs::write(
        &lock_path,
        format!(
            "[meta]\ngenerated_by='C3'\ngenerated_at='now'\nschema_version={}\n{packages}",
            vibe_core::manifest::CURRENT_SCHEMA_VERSION
        ),
    )
    .unwrap();
    let lock = Lockfile::read(lock_path).unwrap();
    let (bare, _, origins) =
        effective_root_constraints(&workspace, &workspace.root, &[], &lock).unwrap();
    assert_eq!(bare.iter().filter(|p| p.name == "app").count(), 2);
    assert_eq!(origins.len(), 3);
    let (explicit, preferred, _) =
        effective_root_constraints(&workspace, &workspace.root, &[p("app@=2.0.0")], &lock).unwrap();
    assert!(explicit.contains(&p("app@>=1")));
    assert!(!explicit.contains(&p("app@=1.0.0")));
    assert!(explicit.contains(&p("unrelated@*")));
    assert!(preferred.contains(&p("app@>=1")));
    assert!(!preferred.contains(&p("app@=1.0.0")));
    assert!(preferred.contains(&p("unrelated@=1.0.0")));
    let mut v2 = m("app", "");
    v2.package.as_mut().unwrap().version = semver::Version::new(2, 0, 0);
    let source = Source::new(vec![m("app", ""), v2, m("unrelated", "")], vec![]);
    let selected = ResolvoDepSolver::new(source.clone())
        .solve(&preferred)
        .unwrap();
    assert_eq!(
        selected
            .find(p("app").group.as_ref().unwrap(), "app")
            .unwrap()
            .version,
        semver::Version::new(2, 0, 0)
    );
    workspace.members[0].manifest.requires.packages = vec![p("app@=1.0.0")];
    let (conflicting, _, _) =
        effective_root_constraints(&workspace, &workspace.root, &[p("app@=2.0.0")], &lock).unwrap();
    assert!(matches!(
        ResolvoDepSolver::new(source.clone()).solve(&conflicting),
        Err(SolveError::Unsatisfiable { .. })
    ));
    workspace.members[0].manifest.requires.packages = vec![p("app@=2.0.0")];
    let (bare, _, _) = effective_root_constraints(&workspace, &workspace.root, &[], &lock).unwrap();
    assert!(matches!(
        ResolvoDepSolver::new(source).solve(&bare),
        Err(SolveError::Unsatisfiable { .. })
    ));
}
