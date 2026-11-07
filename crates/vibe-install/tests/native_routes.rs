//! Real resolver, retained acquisition and workspace publication on numeric TLS.
#![cfg(unix)]
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};
use vibe_core::{
    PackageRef,
    manifest::{Lockfile, Manifest},
    user_config::SlotIntegrity,
};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::git_backend::shell::native_admission::{
    GitInstallation, NativeConfigSelection, NativeGitAdmission, TlsTrust,
};
use vibe_registry::{
    CachedPackage, InPlaceMaterialised, MultiRegistryResolver, PackageAcquisition, RegistryError,
};
use vibe_resolver::{DepProviderError, FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};

struct Source {
    resolver: MultiRegistryResolver,
    store: PathBuf,
}
impl InstallSource for Source {
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _: &Path,
        expected: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let selected = self.resolver.resolve(p)?;
        self.resolver
            .fetch_with_expected_hash(&selected, &self.store, expected)
    }
    fn acquire_for_install(
        &self,
        p: &PackageRef,
        _: &Path,
        expected: Option<&str>,
        slot: &Path,
        progress: &vibe_core::progress::Progress,
    ) -> Result<PackageAcquisition, RegistryError> {
        let selected = self.resolver.resolve(p)?;
        self.resolver
            .fetch_for_install(&selected, &self.store, expected, slot, progress)
    }
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        let packages = roots
            .iter()
            .map(|p| {
                let selected = self
                    .resolver
                    .resolve(p)
                    .map_err(|e| DepProviderError::Other(e.to_string()))?
                    .resolved;
                Ok(ResolvedNode {
                    group: selected.group,
                    name: selected.name,
                    version: selected.version,
                    dependencies: vec![],
                    is_root: true,
                })
            })
            .collect::<Result<Vec<_>, SolveError>>()?;
        Ok(ResolvedGraph { packages })
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(roots)
    }
    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        let selected = self
            .resolver
            .resolve(p)
            .map_err(|e| DepProviderError::Other(e.to_string()))?
            .resolved;
        self.resolver
            .fetch_manifest(&selected.group, &selected.name, &selected.version)
            .map_err(|e| DepProviderError::Other(e.to_string()).into())
    }
    fn prepare_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<vibe_registry::PreparedInPlace, RegistryError> {
        let selected = self.resolver.resolve(p)?;
        self.resolver.prepare_in_place(&selected, slot)
    }
    fn materialise_in_place(
        &self,
        p: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        Err(RegistryError::PreparedInPlace {
            path: slot.into(),
            reason: format!("unexpected unbound operation for {p}"),
        })
    }
}
struct Server(std::process::Child);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.stdin.take();
        let _ = self.0.wait();
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn denied_primary_allowed_mirror_completes_public_plan_and_install() {
    let mut server = Server(
        Command::new("python3")
            .args([
                "-u",
                "-c",
                include_str!(
                    "../../vibe-registry/src/git_backend/shell/native_admission/tls_fixture.py"
                ),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let info: serde_json::Value = serde_json::from_str(&line).unwrap();
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("vibe.toml"),
        "[project]\nname='routes'\nversion='0.1.0'\n",
    )
    .unwrap();
    let selection = NativeConfigSelection::new(
        root.path().into(),
        Path::new(info["root"].as_str().unwrap()).join("home"),
    )
    .unwrap()
    .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
    .unwrap()
    .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
    .unwrap()
    .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
    .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(vibe_registry::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(fs::read(info["ca"].as_str().unwrap()).unwrap()),
    )
    .with_config_selection(selection);
    let primary = format!("{}/public", info["b"].as_str().unwrap());
    let mirror = format!("{}/public", info["a"].as_str().unwrap());
    let authored = format!(
        "[project]\nname='routes'\nversion='0.1.0'\n[[registry]]\nname='n4-fixture'\nurl='{primary}'\nauth='none'\n[[mirror]]\nof='n4-fixture'\nurl='{mirror}'\npriority=0\n"
    );
    fs::write(project.join("vibe.toml"), &authored).unwrap();
    let manifest = Manifest::parse_str(&authored).unwrap();
    let resolver = MultiRegistryResolver::from_manifest_with_native_admission(
        &manifest.registries,
        &manifest.mirrors,
        &[],
        root.path().join("registry"),
        0,
        Arc::new(profile),
    )
    .unwrap();
    let source = Source {
        resolver,
        store: root.path().join("store"),
    };
    let request = InstallRequest {
        roots: vec![PackageRef::parse("org.fixture/demo@=1.0.0").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "route fixture".into(),
        offline: false,
    };
    let Plan::Ready(planned) =
        vibe_install::plan(&source, &project, request, &NullObserver).unwrap()
    else {
        panic!("first install must be planned")
    };
    let (_, _, _, evidence) = planned.package_sources().next().unwrap();
    assert_eq!(
        evidence.source_url.as_deref(),
        Some(format!("{primary}/org.fixture.demo.git").as_str())
    );
    assert_eq!(
        evidence.fetched_from.as_deref(),
        Some(format!("{mirror}/org.fixture.demo.git").as_str())
    );
    let log = Path::new(info["root"].as_str().unwrap()).join("a.log");
    let contacts = fs::read(&log).unwrap();
    let report = vibe_install::apply(
        &source,
        *planned,
        SlotIntegrity::Verify,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap();
    assert_eq!(report.outcome.materialised.len(), 1);
    assert_eq!(fs::read(log).unwrap(), contacts);
    let slot = project
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.fixture.demo/1.0.0");
    assert_eq!(
        fs::read(slot.join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    let lock = Lockfile::read(project.join("vibe.lock")).unwrap();
    assert_eq!(
        lock.packages[0].source_url.as_str(),
        format!("{primary}/org.fixture.demo.git")
    );
    assert_eq!(
        lock.packages[0].resolved_commit.as_deref(),
        info["head"].as_str()
    );
    assert!(
        !Path::new(info["root"].as_str().unwrap())
            .join("b.log")
            .exists()
    );
}

#[path = "native_routes/recursive.rs"]
mod recursive;
