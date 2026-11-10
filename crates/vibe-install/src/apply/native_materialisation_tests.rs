//! Actual N1 source preparation through the whole apply boundary.
use super::*;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use vibe_core::manifest::{Lockfile, NamingConvention};
use vibe_registry::git_backend::shell::native_admission::{
    GitInstallation, NativeConfigSelection, NativeGitAdmission, TlsTrust,
};
use vibe_registry::{
    CachedPackage, GitPerPackageRegistry, InPlaceMaterialised, RegistryError, ResolvedPackage,
    ShellGit,
};
use vibe_resolver::{FeatureRequest, ResolvedGraph, SolveError};

struct NoResolution;
impl InstallSource for NoResolution {
    fn source_solve_limits(
        &self,
    ) -> std::result::Result<vibe_resolver::SourceSolveLimits, SolveError> {
        panic!("apply must not solve")
    }
    fn solve_metadata(
        &self,
        _: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> std::result::Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        panic!("apply must not solve")
    }
    fn acquire_candidate_for_install(
        &self,
        _: &vibe_registry::source_candidates::RetainedSourceCandidate,
        _: &Path,
        _: Option<&str>,
        _: &Path,
        _: &vibe_core::progress::Progress,
    ) -> std::result::Result<vibe_registry::PackageAcquisition, RegistryError> {
        panic!("apply must retain its source")
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        panic!("apply must retain its source")
    }
    fn solve(&self, _: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("apply must not solve")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("apply must not solve")
    }
    fn manifest_of(&self, _: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        panic!("apply must retain its manifest")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        panic!("apply must retain its preparation")
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
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn payload_filter_refuses_before_any_caller_manifest_publication() {
    exercise(true);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn native_fresh_staged_apply_publishes_exact_selected_bytes_and_lock() {
    exercise(false);
}
fn exercise(filter: bool) {
    let mut server = Server(
        Command::new("python3")
            .args([
                "-u",
                "-c",
                include_str!(
                    "../../../vibe-registry/src/git_backend/shell/native_admission/tls_fixture.py"
                ),
            ])
            .env("VIBE_N2_INSTALL_FIXTURE", if filter { "1" } else { "2" })
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
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    let manifest_path = project.join("vibe.toml");
    let before = "[project]\nname='demo'\nversion='0.0.1'\n";
    fs::write(&manifest_path, before).unwrap();
    let manifest = Manifest::parse_str(before).unwrap();
    let group = vibe_core::Group::parse("org.vibevm").unwrap();
    let slot = vibedeps::in_place_slot_abs_path(&project, &group, "giant");
    let profile = NativeGitAdmission::direct_https(
        Arc::new(vibe_registry::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(fs::read(info["ca"].as_str().unwrap()).unwrap()),
    )
    .with_config_selection(
        NativeConfigSelection::new(
            temp.path().into(),
            Path::new(info["root"].as_str().unwrap()).join("home"),
        )
        .unwrap()
        .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
        .unwrap()
        .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
        .unwrap()
        .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
        .unwrap(),
    );
    let registry = GitPerPackageRegistry::open_with(
        "fixture",
        &format!("{}/public", info["a"].as_str().unwrap()),
        "main",
        NamingConvention::Fqdn,
        &temp.path().join("cache"),
        Arc::new(ShellGit::new().with_native_admission(Arc::new(profile))),
        0,
    )
    .unwrap();
    let selected = ResolvedPackage {
        group: group.clone(),
        name: "giant".into(),
        version: "1.0.0".parse().unwrap(),
        source_dir: PathBuf::new(),
    };
    let prepared = registry.prepare_in_place(&selected, &slot).unwrap();
    let cached = CachedPackage {
        locked_selection: None,
        route_publication: vibe_core::RoutePublication::Local,
        recorded_source_kind: None,
        store_reused: None,
        acquisition: None,
        resolved: selected,
        cache_dir: slot.clone(),
        manifest: prepared.manifest().clone(),
        content_hash: prepared.content_hash().into(),
        source_uri: prepared.source_uri().into(),
        fetched_from: None,
        registry_name: Some("fixture".into()),
        source_ref: Some(prepared.source_ref().into()),
        resolved_commit: Some(prepared.resolved_commit().into()),
        overridden: false,
        is_git_source: false,
        is_path_source: false,
        is_embedded: false,
        is_local: false,
        via_redirect: None,
        template: None,
        via_channel: None,
    };
    let resolution = vec![ResolvedDep {
        kind: cached.package_meta().kind,
        group,
        name: "giant".into(),
        version: cached.resolved.version.clone(),
        content_dir: slot.clone(),
        source_hash: Some(vibe_core::ContentHash::from_validated(
            cached.content_hash.clone(),
        )),
        embedded_sources: vec![],
        manifest: cached.manifest.clone(),
        requires: vec![],
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
        configuration: None,
    }];
    let planned = PlannedInstall {
        project_root: project.clone(),
        request: crate::InstallRequest {
            roots: vec![PackageRef::parse("org.vibevm/giant").unwrap()],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "fixture".into(),
            offline: true,
        },
        manifest,
        lockfile: Lockfile::empty("fixture", "2026-11-07T00:00:00Z"),
        language_chain: vec![],
        fetched: vec![Fetched {
            cached,
            feature_expansion: Default::default(),
            meta: crate::fetched::NodeInstallMeta {
                dependencies: vec![],
                is_root: true,
            },
            embedded_sources: vec![],
            in_place: Some(prepared),
            configuration: None,
        }],
        resolution,
        workspace: Workspace::load(&project).unwrap(),
        progress: Default::default(),
        source_candidates: Default::default(),
    };
    // Keep the old inode alive; atomic publish+undo cannot accidentally reuse it.
    let original = fs::File::open(&manifest_path).unwrap();
    let ino = original.metadata().unwrap().ino();
    let before_contacts =
        fs::read(Path::new(info["root"].as_str().unwrap()).join("a.log")).unwrap();
    let result = apply(
        &NoResolution,
        planned,
        SlotIntegrity::Verify,
        &HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    );
    assert_eq!(
        fs::read(Path::new(info["root"].as_str().unwrap()).join("a.log")).unwrap(),
        before_contacts,
        "apply performed another acquisition"
    );
    if !filter {
        let report = result.unwrap();
        assert_eq!(report.outcome.materialised.len(), 1);
        assert_eq!(
            fs::read(slot.join("payload.txt")).unwrap(),
            b"retained fixture\r\n"
        );
        assert_eq!(
            fs::read_to_string(slot.join(".git/HEAD")).unwrap().trim(),
            info["head"].as_str().unwrap()
        );
        let lock = Lockfile::read(project.join("vibe.lock")).unwrap();
        assert_eq!(
            lock.packages[0].resolved_commit.as_deref(),
            info["head"].as_str()
        );
        return;
    }
    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("payload has an unsupported filter"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(&manifest_path).unwrap(), before);
    assert!(!slot.exists());
    assert!(!project.join("vibe.lock").exists());
    assert!(!project.join(".gitignore").exists());
    assert_eq!(
        fs::metadata(&manifest_path).unwrap().ino(),
        ino,
        "unsupported payload must refuse before even reversible manifest publication: {error}"
    );
}
