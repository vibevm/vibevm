//! Characterizations of planner request loss; no solver or lock semantics change.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;

use vibe_core::manifest::{LockedPackage, Lockfile, Manifest};
use vibe_core::{ContentHash, Group, PackageRef};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, RegistryError};
use vibe_resolver::{FeatureRequest, ResolvedGraph, SolveError};

#[derive(Default)]
struct RecordingSource(Mutex<Vec<Vec<PackageRef>>>);

impl InstallSource for RecordingSource {
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        self.0.lock().unwrap().push(roots.to_vec());
        Err(SolveError::CapabilityUnmet {
            capability: "characterization stop".into(),
            requirer: "M-05-A.1 fixture".into(),
        })
    }

    fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> {
        panic!("must stop before metadata lookup")
    }

    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        panic!("must stop before masked solve")
    }

    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        panic!("must not fetch")
    }

    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        panic!("must not install")
    }
}

fn pkg(raw: &str) -> PackageRef {
    PackageRef::parse(raw).unwrap()
}

fn request() -> InstallRequest {
    InstallRequest {
        roots: vec![],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "M-05-A.1 fixture".into(),
        offline: true,
    }
}

fn write(root: &Path, name: &str, body: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

// Uses the existing public materialisation seam demonstrated by
// tests/store_materialisation.rs; all payload and slot writes are temporary.
fn locked_fixture(root: &Path, name: &str) -> LockedPackage {
    let source = tempfile::tempdir().unwrap();
    write(
        source.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup='org.demo'\nname='{name}'\nkind='flow'\nversion='1.0.0'\n[features]\nextra=[]\n"
        ),
    );
    let hash = vibe_registry::compute_content_hash(source.path()).unwrap();
    vibe_workspace::vibedeps::materialise(
        root,
        &Group::parse("org.demo").unwrap(),
        name,
        &semver::Version::parse("1.0.0").unwrap(),
        source.path(),
        &ContentHash::from_validated(hash.clone()),
    )
    .unwrap();
    serde_json::from_value(serde_json::json!({
        "kind": "flow", "group": "org.demo", "name": name, "version": "1.0.0",
        "registry": "fixture", "source_url": "https://example.invalid/fixture",
        "content_hash": hash, "features": [], "dependencies": [],
        "files_written": [], "subskills_active": [], "overridden": false
    }))
    .unwrap()
}

fn refused(source: &RecordingSource, root: &Path, req: InstallRequest) {
    let error = vibe_install::plan(source, root, req, &NullObserver).unwrap_err();
    assert!(error.to_string().contains("characterization stop"));
}

#[test]
fn root_union() {
    // spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05A1-ROOT-UNION
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "vibe.toml",
        "[project]\nname='consumer-a'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n[requires.packages]\n'org.demo/shared'='^1'\n'flow:org.demo/held'='=1.0.0'\n",
    );
    write(
        dir.path(),
        "member/vibe.toml",
        "[project]\nname='consumer-b'\nversion='1.0.0'\n[requires.packages]\n'org.demo/shared'='=2.0.0'\n",
    );
    let a = pkg("org.demo/shared@^1");
    let b = pkg("org.demo/shared@=2.0.0");
    let held = pkg("flow:org.demo/held@=1.0.0");
    let source = RecordingSource::default();
    refused(&source, dir.path(), request());
    assert_eq!(
        source.0.lock().unwrap().as_slice(),
        &[vec![held.clone(), a.clone()]]
    );
    assert!(!source.0.lock().unwrap()[0].contains(&b));

    let mut lock = Lockfile::empty("fixture", "2026-10-14T00:00:00Z");
    lock.meta.root_dependencies = vec![held.clone()];
    lock.packages.push(locked_fixture(dir.path(), "held"));
    lock.write(dir.path().join("vibe.lock")).unwrap();
    let before = std::fs::read(dir.path().join("vibe.lock")).unwrap();
    let source = RecordingSource::default();
    let explicit = pkg("org.demo/shared@=3.0.0");
    let mut req = request();
    req.roots = vec![explicit.clone()];
    refused(&source, dir.path(), req);
    assert_eq!(source.0.lock().unwrap().as_slice(), &[vec![explicit, held]]);
    assert_eq!(std::fs::read(dir.path().join("vibe.lock")).unwrap(), before);
    // Confirmed: consumer-b is lost in bare mode; both existing shared
    // constraints are absent in explicit mode. Held roots survive, with
    // their exact existing requirement. No final solver outcome is claimed.
}

#[test]
fn fresh_requested_features() {
    // spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05A1-FRESH-REQUESTED-FEATURES
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "vibe.toml",
        "[project]\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'org.demo/shared'='^1'\n",
    );
    let mut lock = Lockfile::empty("fixture", "2026-10-14T00:00:00Z");
    lock.meta.root_dependencies = vec![pkg("org.demo/shared@^1")];
    lock.packages.push(locked_fixture(dir.path(), "shared"));
    assert!(lock.packages[0].features.is_empty());
    lock.write(dir.path().join("vibe.lock")).unwrap();
    let before = std::fs::read(dir.path().join("vibe.lock")).unwrap();
    let source = RecordingSource::default();
    assert!(matches!(
        vibe_install::plan(&source, dir.path(), request(), &NullObserver).unwrap(),
        Plan::Fresh
    ));
    let mut changed = request();
    changed.features.explicit.push("extra".into());
    assert!(matches!(
        vibe_install::plan(&source, dir.path(), changed.clone(), &NullObserver).unwrap(),
        Plan::Fresh
    ));
    assert!(source.0.lock().unwrap().is_empty());
    assert_eq!(std::fs::read(dir.path().join("vibe.lock")).unwrap(), before);
    changed.roots = vec![pkg("org.demo/shared@^1")];
    refused(&source, dir.path(), changed);
    assert_eq!(source.0.lock().unwrap().len(), 1);
    // Confirmed: changing only the requested feature still returns Fresh,
    // leaving the lock without extra. The explicit-root control reaches solve.
}
