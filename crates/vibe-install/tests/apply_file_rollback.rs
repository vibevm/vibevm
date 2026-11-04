//! Actual apply rollback ownership probes; all interleavings use the public source seam.
#![cfg(unix)]

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO");

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{FeatureRequest, ResolvedGraph, ResolvedNode, SolveError};
use vibe_workspace::hooks::HookPolicy;

const BEFORE: &str = "[project]\nname='rollback-probe'\nversion='0.1.0'\n";
const FAILURE: &str = "SYNTHETIC_DEFERRED_FAILURE_AFTER_INTERLEAVING";
const PACKAGE: &str = "[package]\ngroup='org.vibevm'\nname='giant'\nversion='1.0.0'\nkind='feat'\nmaterialization='in-place'\n";

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
    case: Case,
    calls: Cell<usize>,
    observation: RefCell<Option<Value>>,
    wanted: RefCell<String>,
}

fn identity(path: &Path) -> Value {
    let m = fs::symlink_metadata(path).unwrap();
    json!({"dev":m.dev(),"ino":m.ino(),"links":m.nlink(),"symlink":m.file_type().is_symlink()})
}

impl InstallSource for Source {
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        panic!("fixture must use deferred materialise_in_place, not fetching")
    }

    fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(ResolvedGraph {
            packages: vec![ResolvedNode {
                group: Group::parse("org.vibevm").unwrap(),
                name: "giant".into(),
                version: "1.0.0".parse().unwrap(),
                dependencies: vec![],
                is_root: true,
            }],
        })
    }

    fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(Manifest::parse_str(PACKAGE).unwrap())
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
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(
            slot,
            self.root
                .join(vibe_core::layout::current_vibedeps_root())
                .join("org.vibevm.giant")
        );
        let path = self.root.join("vibe.toml");
        let published = fs::read_to_string(&path).unwrap();
        assert_ne!(published, BEFORE, "actual apply must have published B");
        assert!(published.contains("org.vibevm/giant"));
        Manifest::parse_str(&published).unwrap();
        let published_identity = identity(&path);
        let changed = format!("{published}\n# SYNTHETIC_OTHER_ACTOR_C\n");
        Manifest::parse_str(&changed).unwrap();
        let sibling = self.root.join("synthetic-sibling.toml");
        let wanted = match self.case {
            Case::Unchanged | Case::Complete => BEFORE.to_owned(),
            Case::Edited => {
                fs::write(&path, &changed).unwrap();
                assert_eq!(identity(&path), published_identity);
                changed
            }
            Case::ReplacedSameBytes => {
                let replacement = self.root.join("replacement.tmp");
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
        *self.wanted.borrow_mut() = wanted.clone();
        *self.observation.borrow_mut() = Some(json!({
            "case":format!("{:?}",self.case), "before":BEFORE,
            "published":published,"published_identity":published_identity,
            "interleaved":fs::read_to_string(&path).unwrap(),
            "interleaved_identity":identity(&path), "wanted_after_error":wanted,
            "sibling_before_error":sibling.exists().then(||fs::read_to_string(&sibling).unwrap())
        }));
        if matches!(self.case, Case::Complete) {
            return Ok(InPlaceMaterialised {
                source_uri: "https://example.test/giant.git".into(),
                source_ref: "v1.0.0".into(),
                resolved_commit: Some("1111111111111111111111111111111111111111".into()),
                content_hash: "sha256:0000".into(),
                changed: false,
                manifest: Manifest::parse_str(PACKAGE).unwrap(),
            });
        }
        Err(RegistryError::MalformedMeta {
            path: slot.join("vibe.toml"),
            reason: FAILURE.into(),
        })
    }
}

fn write(root: &Path, name: impl AsRef<Path>, body: &str) {
    let p = root.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn prepared(case: Case) -> (tempfile::TempDir, Source, Box<vibe_install::PlannedInstall>) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "vibe.toml", BEFORE);
    let lock = format!(
        "[meta]\ngenerated_by='rollback probe'\ngenerated_at='2026-11-04T00:00:00Z'\nschema_version={CURRENT_SCHEMA_VERSION}\n[[package]]\nkind='feat'\ngroup='org.vibevm'\nname='giant'\nversion='1.0.0'\nsource_url='https://example.test/giant.git'\ncontent_hash='sha256:0000'\nsource_kind='registry'\nresolved_commit='1111111111111111111111111111111111111111'\nmaterialization='in-place'\n"
    );
    write(root, "vibe.lock", &lock);
    let slot = vibe_core::layout::current_vibedeps_root();
    write(root, slot.join("org.vibevm.giant/vibe.toml"), PACKAGE);
    write(
        root,
        slot.join("org.vibevm.giant/.git/HEAD"),
        "ref: refs/heads/main\n",
    );
    let source = Source {
        root: root.to_path_buf(),
        case,
        calls: Cell::new(0),
        observation: RefCell::new(None),
        wanted: RefCell::new(String::new()),
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
        panic!("explicit root must produce a real apply plan");
    };
    assert_eq!(source.calls.get(), 0, "plan must defer source mutation");
    (temp, source, planned)
}

fn probe(case: Case) {
    let (temp, source, planned) = prepared(case);
    let root = temp.path();
    let lock = fs::read_to_string(root.join("vibe.lock")).unwrap();
    let error = match vibe_install::apply(
        &source,
        *planned,
        SlotIntegrity::Verify,
        &HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    ) {
        Ok(_) => panic!("synthetic deferred source error must fail actual apply"),
        Err(error) => error,
    };
    assert_eq!(source.calls.get(), 1);
    assert!(error.to_string().contains(FAILURE), "{error}");
    assert_eq!(
        matches!(error, vibe_install::Error::RollbackFailed { .. }),
        !matches!(case, Case::Unchanged),
        "{error}"
    );
    let path = root.join("vibe.toml");
    let sibling = root.join("synthetic-sibling.toml");
    let after = fs::read_to_string(&path).unwrap();
    let after_identity = identity(&path);
    let mut observation = source.observation.borrow_mut().take().unwrap();
    observation["apply_error"] = json!(error.to_string());
    observation["callback_calls"] = json!(source.calls.get());
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
        *source.wanted.borrow(),
        "failed apply must preserve third-state bytes; control must restore A"
    );
    if sibling.exists() {
        assert_eq!(
            fs::read_to_string(sibling).unwrap(),
            *source.wanted.borrow()
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
