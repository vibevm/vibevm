specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#example");

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{
    FilteringDepProvider, InstallRequest, InstallSource, NullObserver, Plan, PlannedInstall,
};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError, ResolvedPackage};
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, FeatureRequest, ResolvedGraph, ResolvedNode,
    ResolvoDepSolver, SolveError, VersionEnumerator,
};
use vibe_workspace::hooks::HookPolicy;
use vibe_workspace::install::ResolvedDep;

const GROUP: &str = "org.test";

struct PackageFixture {
    manifest: Manifest,
    directory: PathBuf,
}

struct TestSource {
    packages: BTreeMap<(String, semver::Version), PackageFixture>,
    fetches: RefCell<Vec<String>>,
    masked_solves: Cell<usize>,
    initial_graph: RefCell<Option<ResolvedGraph>>,
}

#[derive(Clone, Copy)]
struct TestProvider<'a>(&'a TestSource);

impl DepProvider for TestProvider<'_> {
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let group = pkgref
            .group
            .clone()
            .ok_or_else(|| DepProviderError::Other("golden provider requires a group".into()))?;
        self.list_versions(&group, &pkgref.name)?
            .into_iter()
            .filter(|version| pkgref.version.matches(version))
            .max()
            .ok_or_else(|| DepProviderError::UnknownPackage {
                group,
                name: pkgref.name.to_string(),
            })
    }

    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.0
            .packages
            .get(&(format!("{group}/{name}"), version.clone()))
            .map(|package| package.manifest.clone())
            .ok_or_else(|| DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.to_string(),
            })
    }
}

impl VersionEnumerator for TestProvider<'_> {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        let coordinate = format!("{group}/{name}");
        let versions: Vec<_> = self
            .0
            .packages
            .keys()
            .filter(|(candidate, _)| candidate == &coordinate)
            .map(|(_, version)| version.clone())
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.to_string(),
            })
        } else {
            Ok(versions)
        }
    }
}

impl InstallSource for TestSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        _store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let group = pkgref.group.clone().unwrap();
        let provider = TestProvider(self);
        let version = provider.resolve_version(pkgref).unwrap();
        let fixture = self
            .packages
            .get(&(pkgref.qualified_name(), version.clone()))
            .unwrap();
        self.fetches.borrow_mut().push(pkgref.qualified_name());
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group,
                name: pkgref.name.to_string(),
                version,
                source_dir: fixture.directory.clone(),
            },
            cache_dir: fixture.directory.clone(),
            manifest: fixture.manifest.clone(),
            content_hash: "sha256:0123456789abcdef".to_string(),
            source_uri: "https://example.test/golden".to_string(),
            registry_name: Some("golden".to_string()),
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

    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        if let Some(graph) = self.initial_graph.borrow().clone() {
            return Ok(graph);
        }
        ResolvoDepSolver::new(TestProvider(self)).solve(roots)
    }

    fn manifest_of(&self, pkgref: &PackageRef) -> Result<Manifest, SolveError> {
        let provider = TestProvider(self);
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| DepProviderError::Other("golden provider requires a group".into()))?;
        let version = provider.resolve_version(pkgref)?;
        Ok(provider.fetch_manifest(group, &pkgref.name, &version)?)
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.masked_solves.set(self.masked_solves.get() + 1);
        let provider = TestProvider(self);
        ResolvoDepSolver::new(FilteringDepProvider::new(&provider, blocked)).solve(roots)
    }

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        Err(RegistryError::UnknownPackage {
            group: pkgref.group.clone().unwrap(),
            name: pkgref.name.to_string(),
        })
    }
}

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    source: TestSource,
}

impl Fixture {
    fn new(project_tail: &str, packages: &[(&str, &str, &str)]) -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(Manifest::FILENAME),
            format!(
                "[project]\nname = \"root\"\ngroup = \"{GROUP}\"\nversion = \"0.1.0\"\n{project_tail}"
            ),
        )
        .unwrap();
        let mut fixtures = BTreeMap::new();
        for (name, version, tail) in packages {
            let directory = temp.path().join("registry").join(name).join(version);
            fs::create_dir_all(&directory).unwrap();
            let text = format!(
                "[package]\ngroup = \"{GROUP}\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n{tail}"
            );
            fs::write(directory.join(Manifest::FILENAME), &text).unwrap();
            fixtures.insert(
                (
                    format!("{GROUP}/{name}"),
                    semver::Version::parse(version).unwrap(),
                ),
                PackageFixture {
                    manifest: Manifest::parse_str(&text).unwrap(),
                    directory,
                },
            );
        }
        Self {
            _temp: temp,
            root,
            source: TestSource {
                packages: fixtures,
                fetches: RefCell::new(Vec::new()),
                masked_solves: Cell::new(0),
                initial_graph: RefCell::new(None),
            },
        }
    }

    fn ready(&self) -> Box<PlannedInstall> {
        let request = InstallRequest {
            roots: Vec::new(),
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "visibility golden".to_string(),
        };
        match vibe_install::plan(&self.source, &self.root, request, &NullObserver).unwrap() {
            Plan::Ready(planned) => planned,
            Plan::Fresh => panic!("a new fixture must require a plan"),
        }
    }

    fn plan_and_apply(&self) -> (Vec<ResolvedDep>, Lockfile) {
        let planned = self.ready();
        let resolution = planned.resolution.clone();
        let policy = HookPolicy {
            allowed_groups: vec![GROUP.to_string()],
            allow_hooks: false,
        };
        vibe_install::apply(
            &self.source,
            *planned,
            SlotIntegrity::TrustPresence,
            &policy,
        )
        .unwrap();
        let lock = Lockfile::read(self.root.join(Lockfile::FILENAME)).unwrap();
        (resolution, lock)
    }
}

fn names(lock: &Lockfile) -> BTreeSet<String> {
    lock.packages
        .iter()
        .map(|package| package.name.to_string())
        .collect()
}

fn package<'a>(lock: &'a Lockfile, name: &str) -> &'a vibe_core::manifest::LockedPackage {
    lock.packages
        .iter()
        .find(|package| package.name == name)
        .unwrap()
}

fn node(name: &str, version: &str, dependencies: &[(&str, &str)], is_root: bool) -> ResolvedNode {
    ResolvedNode {
        group: Group::parse(GROUP).unwrap(),
        name: name.to_string(),
        version: semver::Version::parse(version).unwrap(),
        dependencies: dependencies
            .iter()
            .map(|(target, target_version)| {
                PackageRef::parse(&format!("{GROUP}/{target}@={target_version}")).unwrap()
            })
            .collect(),
        is_root,
    }
}

fn version_pressure_fixture() -> Fixture {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/b\" = \"^1\"\n\"org.test/c\" = \"^1\"\n",
        &[
            (
                "b",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/x\" = { version = \"=1.0.0\", access = \"private\" }\n",
            ),
            (
                "c",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/x\" = \"^2\"\n",
            ),
            ("x", "1.0.0", ""),
            ("x", "2.0.0", ""),
        ],
    );
    fixture.source.initial_graph.replace(Some(ResolvedGraph {
        packages: vec![
            node("b", "1.0.0", &[("x", "1.0.0")], true),
            node("c", "1.0.0", &[("x", "1.0.0")], true),
            node("x", "1.0.0", &[], false),
        ],
    }));
    fixture
}

#[test]
fn private_dep_of_a_non_root_is_absent_everywhere() {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/b\" = \"^1\"\n",
        &[
            (
                "b",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/d\" = { version = \"^1\", access = \"private\" }\n",
            ),
            ("d", "1.0.0", ""),
        ],
    );
    let (resolution, lock) = fixture.plan_and_apply();
    assert_eq!(names(&lock), BTreeSet::from(["b".to_string()]));
    assert!(resolution.iter().all(|dependency| dependency.name != "d"));
    assert!(
        !fixture
            .source
            .fetches
            .borrow()
            .contains(&format!("{GROUP}/d"))
    );
}

#[test]
fn friends_chain_lands_with_provenance() {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/a\" = { version = \"^1\", friend = true }\n",
        &[
            (
                "a",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/b\" = { version = \"^1\", access = \"friends-only\" }\n",
            ),
            (
                "b",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/c\" = { version = \"^1\", access = \"friends-only\" }\n",
            ),
            ("c", "1.0.0", ""),
        ],
    );
    let (_, lock) = fixture.plan_and_apply();
    assert_eq!(
        package(&lock, "c").admitted_by.as_deref(),
        Some("friends-chain")
    );
}

#[test]
fn public_default_flows_as_today() {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/a\" = \"^1\"\n",
        &[
            (
                "a",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/b\" = \"^1\"\n",
            ),
            (
                "b",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/c\" = \"^1\"\n",
            ),
            ("c", "1.0.0", ""),
        ],
    );
    let (_, lock) = fixture.plan_and_apply();
    assert_eq!(
        names(&lock),
        BTreeSet::from(["a".into(), "b".into(), "c".into()])
    );
    assert_eq!(
        package(&lock, "c").admitted_by.as_deref(),
        Some("public-chain")
    );
}

#[test]
fn exclude_diamond_keeps_the_other_path() {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/a\" = \"^1\"\n",
        &[
            (
                "a",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/b\" = { version = \"^1\", exclude = [\"org.test/d\"] }\n\"org.test/c\" = \"^1\"\n",
            ),
            (
                "b",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/d\" = \"^1\"\n",
            ),
            (
                "c",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/d\" = \"^1\"\n",
            ),
            ("d", "1.0.0", ""),
        ],
    );
    let (resolution, lock) = fixture.plan_and_apply();
    assert_eq!(
        package(&lock, "d").admitted_by.as_deref(),
        Some("public-chain")
    );
    let b = resolution
        .iter()
        .find(|dependency| dependency.name == "b")
        .unwrap();
    let c = resolution
        .iter()
        .find(|dependency| dependency.name == "c")
        .unwrap();
    assert!(b.requires.iter().all(|(_, name)| name != "d"));
    assert!(c.requires.iter().any(|(_, name)| name == "d"));
}

#[test]
fn invisible_constraint_stops_pinning_versions() {
    let fixture = version_pressure_fixture();
    let (_, lock) = fixture.plan_and_apply();
    assert_eq!(
        package(&lock, "x").version,
        semver::Version::parse("2.0.0").unwrap()
    );
    assert_eq!(fixture.source.masked_solves.get(), 1);
}

#[test]
fn iteration_converges_and_caps() {
    let fixture = version_pressure_fixture();
    let planned = fixture.ready();
    assert!(
        planned
            .resolution
            .iter()
            .any(|dependency| dependency.name == "x")
    );
    assert_eq!(
        fixture.source.masked_solves.get(),
        1,
        "solve plus one masked solve is two passes"
    );
    assert!(fixture.source.masked_solves.get() < 4);
}

#[test]
fn resolved_dep_requires_are_pruned() {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/b\" = \"^1\"\n",
        &[
            (
                "b",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/d\" = { version = \"^1\", access = \"private\" }\n",
            ),
            ("d", "1.0.0", ""),
        ],
    );
    let (resolution, lock) = fixture.plan_and_apply();
    let resolved_b = resolution
        .iter()
        .find(|dependency| dependency.name == "b")
        .unwrap();
    assert!(resolved_b.requires.is_empty());
    assert!(package(&lock, "b").dependencies.is_empty());
}

#[test]
fn root_override_expands_a_foreign_private_edge() {
    let fixture = Fixture::new(
        "\n[requires.packages]\n\"org.test/a\" = \"^1\"\n\n[override]\n\"org.test/a -> org.test/d\" = { access = \"public\" }\n",
        &[
            (
                "a",
                "1.0.0",
                "\n[requires.packages]\n\"org.test/d\" = { version = \"^1\", access = \"private\" }\n",
            ),
            ("d", "1.0.0", ""),
        ],
    );
    let (_, lock) = fixture.plan_and_apply();
    assert_eq!(
        package(&lock, "d").via_override.as_deref(),
        Some("org.test/root")
    );
}
