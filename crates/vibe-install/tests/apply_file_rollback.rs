//! Actual apply rollback ownership probes through the ordinary pre-install hook.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO");
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::{Path, PathBuf};
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError, ResolvedPackage};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_workspace::hooks::HookPolicy;

#[path = "support/retained.rs"]
mod retained;
const BEFORE: &str = "[project]\nname='rollback-probe'\nversion='0.1.0'\n";
const FAILURE_EXIT: i32 = 37;
const PACKAGE: &str = "[package]\ngroup='org.vibevm'\nname='giant'\nversion='1.0.0'\nkind='feat'\n";
/// The fixture package's directory inside its temp directory registry.
const GIANT_DIR: &str = "org.vibevm/giant/v1.0.0";
#[derive(Clone, Copy, Debug)]
enum Case {
    Unchanged,
    Complete,
    Edited,
    ReplacedSameBytes,
    Symlink,
    Hardlink,
}
struct Source {
    root: PathBuf,
    cache: tempfile::TempDir,
    declaration: String,
}
impl Source {
    fn package_dir(&self) -> PathBuf {
        self.cache.path().join(GIANT_DIR)
    }
    fn calls(&self) -> usize {
        fs::read_to_string(self.root.join("hook-calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
    fn observation(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("hook-observation.json")).unwrap()).unwrap()
    }
}
fn graph() -> ResolvedGraph {
    ResolvedGraph {
        packages: vec![ResolvedNode {
            group: Group::parse("org.vibevm").unwrap(),
            name: "giant".into(),
            version: "1.0.0".parse().unwrap(),
            dependencies: vec![],
            is_root: true,
        }],
    }
}
fn identity(path: &Path) -> Value {
    let m = fs::symlink_metadata(path).unwrap();
    json!({"dev":m.dev(),"ino":m.ino(),"links":m.nlink(),"symlink":m.file_type().is_symlink()})
}

impl InstallSource for Source {
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        retained::limits()
    }
    fn solve_metadata(
        &self,
        request: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        retained::solve(self.cache.path(), request)
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
            &retained::exact(candidate),
            store_root,
            expected_hash,
            slot,
            progress,
        )
    }
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        Ok(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: None,
            acquisition: None,
            resolved: ResolvedPackage {
                group: p.group.clone().unwrap(),
                name: p.name.to_string(),
                version: "1.0.0".parse().unwrap(),
                source_dir: self.package_dir(),
            },
            cache_dir: self.package_dir(),
            manifest: Manifest::parse_str(&self.declaration).unwrap(),
            content_hash: vibe_registry::compute_content_hash(&self.package_dir()).unwrap(),
            source_uri: format!("file://{}", self.package_dir().display()),
            fetched_from: None,
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: true,
            via_redirect: None,
            template: None,
            via_channel: None,
        })
    }
    fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(graph())
    }
    fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(Manifest::parse_str(&self.declaration).unwrap())
    }
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(roots)
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        panic!("copy fixture must reach its actual pre-install hook, never in-place placement")
    }
}

/// This ignored item is an executable fixture, not an extra coverage test.
/// Only the real package hook launches it in a separate process with exact paths.
#[test]
#[ignore = "fixture helper invoked by the actual pre-install hook"]
fn hook_fixture_interleaving() {
    let root =
        PathBuf::from(std::env::var_os("VIBE_UNDO_FIXTURE_ROOT").expect("hook fixture root"));
    assert!(root.is_absolute());
    let expected =
        PathBuf::from(std::env::var_os("VIBE_UNDO_FIXTURE_MARKER").expect("hook fixture marker"));
    assert_eq!(expected.parent(), Some(root.as_path()));
    assert_eq!(fs::read(&expected).unwrap(), b"exact rollback fixture");
    let case = match std::env::var("VIBE_UNDO_FIXTURE_CASE").unwrap().as_str() {
        "Unchanged" => Case::Unchanged,
        "Complete" => Case::Complete,
        "Edited" => Case::Edited,
        "ReplacedSameBytes" => Case::ReplacedSameBytes,
        "Symlink" => Case::Symlink,
        "Hardlink" => Case::Hardlink,
        other => panic!("unknown fixture cut {other}"),
    };
    let path = root.join("vibe.toml");
    let published = fs::read_to_string(&path).unwrap();
    assert_ne!(published, BEFORE, "actual apply must have published B");
    assert!(published.contains("org.vibevm/giant"));
    Manifest::parse_str(&published).unwrap();
    let published_identity = identity(&path);
    let changed = format!("{published}\n# SYNTHETIC_OTHER_ACTOR_C\n");
    Manifest::parse_str(&changed).unwrap();
    let sibling = root.join("synthetic-sibling.toml");
    let wanted = match case {
        Case::Unchanged | Case::Complete => BEFORE.to_owned(),
        Case::Edited => {
            fs::write(&path, &changed).unwrap();
            assert_eq!(identity(&path), published_identity);
            changed
        }
        Case::ReplacedSameBytes => {
            let replacement = root.join("replacement.tmp");
            fs::write(&replacement, &published).unwrap();
            fs::rename(&replacement, &path).unwrap();
            assert_ne!(identity(&path)["ino"], published_identity["ino"]);
            published.clone()
        }
        Case::Symlink => {
            fs::write(&sibling, &changed).unwrap();
            fs::remove_file(&path).unwrap();
            symlink(&sibling, &path).unwrap();
            assert!(
                fs::symlink_metadata(&path)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            changed
        }
        Case::Hardlink => {
            fs::write(&sibling, &changed).unwrap();
            fs::remove_file(&path).unwrap();
            fs::hard_link(&sibling, &path).unwrap();
            assert_eq!(identity(&path), identity(&sibling));
            assert_eq!(identity(&path)["links"], 2);
            changed
        }
    };
    let observation = json!({
        "case":format!("{:?}",case), "before":BEFORE,
        "published":published,"published_identity":published_identity,
        "interleaved":fs::read_to_string(&path).unwrap(),
        "interleaved_identity":identity(&path), "wanted_after_error":wanted,
        "sibling_before_error":sibling.exists().then(||fs::read_to_string(&sibling).unwrap())
    });

    fs::write(
        root.join("hook-observation.json"),
        serde_json::to_vec(&observation).unwrap(),
    )
    .unwrap();
    let calls = root.join("hook-calls");
    let previous = fs::read_to_string(&calls).unwrap_or_default();
    fs::write(calls, format!("{previous}called\n")).unwrap();
}

fn write(root: &Path, name: impl AsRef<Path>, body: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
fn hook_policy() -> HookPolicy {
    HookPolicy {
        allowed_groups: vec!["org.vibevm".into()],
        allow_hooks: false,
    }
}
fn prepared(case: Case) -> (tempfile::TempDir, Source, Box<vibe_install::PlannedInstall>) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "vibe.toml", BEFORE);
    write(
        root,
        "vibe.lock",
        &format!(
            "[meta]\ngenerated_by='rollback probe'\ngenerated_at='2026-11-04T00:00:00Z'\nschema_version={CURRENT_SCHEMA_VERSION}\n"
        ),
    );
    write(root, "hook-fixture-marker", "exact rollback fixture");
    let cache = tempfile::tempdir().unwrap();
    let declaration = format!("{PACKAGE}\n[hooks]\npre-install='hooks/interleave'\n");
    let package = cache.path().join(GIANT_DIR);
    write(&package, "vibe.toml", &declaration);
    write(&package, "payload.txt", "actual cached copy source");
    let program = std::env::current_exe().unwrap();
    let script = format!(
        "set -eu\nVIBE_UNDO_FIXTURE_ROOT={} VIBE_UNDO_FIXTURE_MARKER={} VIBE_UNDO_FIXTURE_CASE={} {} --ignored --exact hook_fixture_interleaving --nocapture --test-threads=1\nexit {}\n",
        shell_quote(root.to_str().unwrap()),
        shell_quote(root.join("hook-fixture-marker").to_str().unwrap()),
        shell_quote(&format!("{case:?}")),
        shell_quote(program.to_str().unwrap()),
        if matches!(case, Case::Complete) {
            0
        } else {
            FAILURE_EXIT
        }
    );
    write(&package, "hooks/interleave.sh", &script);
    let source = Source {
        root: root.to_owned(),
        cache,
        declaration,
    };
    let request = InstallRequest {
        roots: vec![PackageRef::parse("org.vibevm/giant").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "rollback probe".into(),
        offline: true,
    };
    let Plan::Ready(planned) = vibe_install::plan(&source, root, request, &NullObserver).unwrap()
    else {
        panic!("explicit root must plan")
    };
    assert_eq!(source.calls(), 0, "planning must not run the mutation hook");
    (temp, source, planned)
}
fn assert_hook_failure(error: &vibe_install::Error) {
    fn workspace_code(error: &vibe_workspace::WorkspaceError) -> Option<i32> {
        match error {
            vibe_workspace::WorkspaceError::Hook(
                vibe_workspace::hooks::HookError::PreInstallFailed { code, .. },
            ) => Some(*code),
            vibe_workspace::WorkspaceError::InstallRollback { primary, .. } => {
                workspace_code(primary)
            }
            _ => None,
        }
    }
    fn code(error: &vibe_install::Error) -> Option<i32> {
        match error {
            vibe_install::Error::Workspace(error) => workspace_code(error),
            vibe_install::Error::RollbackFailed { primary, .. } => code(primary),
            vibe_install::Error::SlotFailed { source, .. } => code(source),
            _ => None,
        }
    }
    assert_eq!(
        code(error),
        Some(FAILURE_EXIT),
        "original typed hook failure must survive: {error}"
    );
    assert!(error.to_string().contains("pre-install"));
    assert!(error.to_string().contains(&format!("exit {FAILURE_EXIT}")));
}
fn probe(case: Case) {
    let (temp, source, planned) = prepared(case);
    let root = temp.path();
    let lock = fs::read_to_string(root.join("vibe.lock")).unwrap();
    let error = match vibe_install::apply(&source, *planned, SlotIntegrity::Verify, &hook_policy())
    {
        Ok(_) => panic!("actual pre-install hook error must fail actual apply"),
        Err(error) => error,
    };
    assert_eq!(source.calls(), 1);
    assert_hook_failure(&error);
    assert_eq!(
        matches!(error, vibe_install::Error::RollbackFailed { .. }),
        !matches!(case, Case::Unchanged),
        "{error}"
    );
    let path = root.join("vibe.toml");
    let sibling = root.join("synthetic-sibling.toml");
    let after = fs::read_to_string(&path).unwrap();
    let after_identity = identity(&path);
    let mut observation = source.observation();
    observation["apply_error"] = json!(error.to_string());
    observation["hook_calls"] = json!(source.calls());
    observation["after"] = json!(after);
    observation["after_identity"] = after_identity.clone();
    observation["lock_unchanged"] =
        json!(fs::read_to_string(root.join("vibe.lock")).unwrap() == lock);
    observation["sibling_after"] = json!(
        sibling
            .exists()
            .then(|| fs::read_to_string(&sibling).unwrap())
    );
    observation["sibling_after_identity"] = json!(sibling.exists().then(|| identity(&sibling)));
    println!("OBSERVATION {observation}");
    assert_eq!(fs::read_to_string(root.join("vibe.lock")).unwrap(), lock);
    if !matches!(case, Case::Unchanged) {
        assert_eq!(after_identity, observation["interleaved_identity"]);
    }
    assert_eq!(
        after,
        observation["wanted_after_error"].as_str().unwrap(),
        "failed apply must preserve third-state bytes; control must restore A"
    );
    if sibling.exists() {
        assert_eq!(
            fs::read_to_string(sibling).unwrap(),
            observation["wanted_after_error"].as_str().unwrap()
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn ordinary_failed_apply_restores_original_manifest() {
    probe(Case::Unchanged);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn failed_apply_preserves_same_inode_third_state_edit() {
    probe(Case::Edited);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn failed_apply_preserves_same_bytes_new_object() {
    probe(Case::ReplacedSameBytes);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn failed_apply_does_not_follow_substituted_symlink() {
    probe(Case::Symlink);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
fn failed_apply_does_not_truncate_substituted_hardlink_sibling() {
    probe(Case::Hardlink);
}

#[path = "apply_file_rollback/edges.rs"]
mod edges;
#[path = "apply_file_rollback/parking.rs"]
mod parking;
#[path = "apply_file_rollback/recording.rs"]
mod recording;
