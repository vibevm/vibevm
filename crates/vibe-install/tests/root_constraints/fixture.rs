//! Owned package candidates served through the real resolvo and install owners.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-CONSTRAINTS"
);

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use tempfile::TempDir;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{FilteringDepProvider, InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError, ResolvedPackage};
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, FeatureRequest, ResolvedGraph, ResolvoDepSolver,
    SolveError, VersionEnumerator,
};
use vibe_workspace::hooks::HookPolicy;

pub const GROUP: &str = "org.constraints";

struct Candidate {
    manifest: Manifest,
    path: PathBuf,
    hash: String,
}

/// Candidates live on disk in a real directory registry
/// (`<registry>/<group>/<name>/v<version>/vibe.toml`); planning solves it
/// through the production local-registry cell.
#[derive(Default)]
pub struct Source {
    registry: PathBuf,
    candidates: BTreeMap<(String, Version), Candidate>,
    pub solves: RefCell<Vec<Vec<PackageRef>>>,
    pub fetches: RefCell<Vec<PackageRef>>,
}

#[derive(Clone, Copy)]
struct Provider<'a>(&'a Source);

impl DepProvider for Provider<'_> {
    fn resolve_version(&self, p: &PackageRef) -> Result<Version, DepProviderError> {
        let group = p.group.as_ref().unwrap();
        self.list_versions(group, &p.name)?
            .into_iter()
            .filter(|v| p.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::UnknownPackage {
                group: group.clone(),
                name: p.name.to_string(),
            })
    }

    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &Version,
    ) -> Result<Manifest, DepProviderError> {
        self.0
            .candidates
            .get(&(format!("{group}/{name}"), version.clone()))
            .map(|c| c.manifest.clone())
            .ok_or_else(|| DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.to_string(),
            })
    }
}

impl VersionEnumerator for Provider<'_> {
    fn list_versions(&self, group: &Group, name: &str) -> Result<Vec<Version>, DepProviderError> {
        Ok(self
            .0
            .candidates
            .keys()
            .filter(|(id, _)| id == &format!("{group}/{name}"))
            .map(|(_, v)| v.clone())
            .collect())
    }
}

impl InstallSource for Source {
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        crate::retained::limits()
    }

    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        self.solves.borrow_mut().push(request.held_roots.to_vec());
        // This double is a plain directory registry with no held-source
        // owner: like the old double, lock respect is the planner's held
        // roots, not a replay of the (synthetic) recorded source identity,
        // which a held-owner-less local provider always refuses.
        crate::retained::solve(
            &self.registry,
            &vibe_resolver::SourceSolveRequest {
                roots: request.roots,
                held_roots: request.held_roots,
                authoritative_locked: &[],
                activated: request.activated,
                limits: request.limits,
                configuration: request.configuration,
                blocked: request.blocked,
            },
        )
    }

    fn acquire_candidate_for_install(
        &self,
        candidate: &vibe_registry::source_candidates::RetainedSourceCandidate,
        store_root: &Path,
        expected_hash: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        self.acquire_for_install(
            &crate::retained::exact(candidate),
            store_root,
            expected_hash,
            slot,
            progress,
        )
    }

    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        self.solves.borrow_mut().push(roots.to_vec());
        ResolvoDepSolver::new(Provider(self)).solve(roots)
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solves.borrow_mut().push(roots.to_vec());
        let provider = Provider(self);
        ResolvoDepSolver::new(FilteringDepProvider::new(&provider, blocked)).solve(roots)
    }

    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        let provider = Provider(self);
        let version = provider.resolve_version(p)?;
        Ok(provider.fetch_manifest(p.group.as_ref().unwrap(), &p.name, &version)?)
    }

    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _unused_store: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetches.borrow_mut().push(p.clone());
        let version = Provider(self).resolve_version(p).unwrap();
        let c = &self.candidates[&(p.qualified_name(), version.clone())];
        Ok(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: None,
            acquisition: None,
            resolved: ResolvedPackage {
                group: p.group.clone().unwrap(),
                name: p.name.to_string(),
                version,
                source_dir: c.path.clone(),
            },
            cache_dir: c.path.clone(),
            manifest: c.manifest.clone(),
            content_hash: c.hash.clone(),
            source_uri: "https://registry.invalid/constraints".into(),
            fetched_from: None,
            registry_name: Some("constraints".into()),
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }

    fn materialise_in_place(
        &self,
        p: &PackageRef,
        _slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        Err(RegistryError::UnknownPackage {
            group: p.group.clone().unwrap(),
            name: p.name.to_string(),
        })
    }
}

pub struct Fixture {
    _temp: TempDir,
    pub root: PathBuf,
    pub source: Source,
}

impl Fixture {
    pub fn new(root_tail: &str, members: &[(&str, &str)]) -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let names: Vec<_> = members.iter().map(|(name, _)| *name).collect();
        write(
            &root.join("vibe.toml"),
            &format!(
                "[project]\ngroup='{GROUP}'\nname='root'\nversion='1.0.0'\n\
                 [workspace]\nmembers={names:?}\n{root_tail}"
            ),
        );
        for (name, tail) in members {
            write(
                &root.join(name).join("vibe.toml"),
                &format!("[project]\ngroup='{GROUP}'\nname='{name}'\nversion='1.0.0'\n{tail}"),
            );
        }
        let registry = temp.path().join("candidates");
        fs::create_dir_all(&registry).unwrap();
        Self {
            _temp: temp,
            root,
            source: Source {
                registry,
                ..Source::default()
            },
        }
    }

    pub fn add(&mut self, name: &str, version: &str, tail: &str) {
        let path = self
            .source
            .registry
            .join(GROUP)
            .join(name)
            .join(format!("v{version}"));
        let text = format!(
            "[package]\ngroup='{GROUP}'\nname='{name}'\nkind='flow'\nversion='{version}'\n{tail}"
        );
        write(&path.join("vibe.toml"), &text);
        write(&path.join("payload.txt"), version);
        self.source.candidates.insert(
            (format!("{GROUP}/{name}"), Version::parse(version).unwrap()),
            Candidate {
                manifest: Manifest::parse_str(&text).unwrap(),
                hash: vibe_registry::compute_content_hash(&path).unwrap(),
                path,
            },
        );
    }

    pub fn plan(&self, selected: &str, refs: &[&str]) -> Result<Plan, vibe_install::Error> {
        vibe_install::plan(
            &self.source,
            &self.root.join(selected),
            request(refs),
            &NullObserver,
        )
    }

    pub fn apply(&self, selected: &str, refs: &[&str]) -> Lockfile {
        self.apply_request(selected, request(refs))
    }

    pub fn apply_request(&self, selected: &str, req: InstallRequest) -> Lockfile {
        let plan = vibe_install::plan(&self.source, &self.root.join(selected), req, &NullObserver)
            .unwrap();
        let Plan::Ready(plan) = plan else {
            panic!("fixture requires a real apply")
        };
        vibe_install::apply(
            &self.source,
            *plan,
            SlotIntegrity::Verify,
            &HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap();
        self.lock()
    }

    pub fn lock(&self) -> Lockfile {
        Lockfile::read(self.root.join("vibe.lock")).unwrap()
    }
}

pub fn request(refs: &[&str]) -> InstallRequest {
    InstallRequest {
        roots: refs.iter().map(|p| p.parse().unwrap()).collect(),
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "root constraint fixture".into(),
        offline: true,
    }
}

pub fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

pub fn version(plan: &Plan, name: &str) -> String {
    let Plan::Ready(plan) = plan else {
        panic!("expected Ready")
    };
    plan.resolution
        .iter()
        .find(|p| p.name == name)
        .unwrap()
        .version
        .to_string()
}

pub fn requires(name: &str, range: &str) -> String {
    format!("[requires.packages]\n'{GROUP}/{name}'='{range}'\n")
}
