//! Characterization oracles for the `Registry`-seam cells that no
//! other integration test in this crate references: [`LocalRegistry`]
//! (variant `local`) and [`GitRegistry`] (variant `git-monorepo`).
//! `GitPackageRegistry` (variant `git-per-package`) already has its
//! oracle in `index_fast_path.rs`.
//!
//! Each test drives the cell through the `Registry` seam against a
//! hermetic fixture and pins observable behavior — resolved version,
//! cache materialisation, `source_uri` shape, error discriminants —
//! so a future cell replacement diffs against real assertions.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use specmark::verifies;
use tempfile::tempdir;

use vibe_core::{Group, PackageRef};
use vibe_registry::git_backend::{GitBackend, GitError};
use vibe_registry::git_registry::DEFAULT_FRESHNESS_SECS;
use vibe_registry::{GitRegistry, LocalRegistry, Registry, RegistryError};

/// Group-native registry layout (`<root>/<group>/<name>/v<ver>/`) with
/// `org.vibevm/wal` at 0.1.0 and 0.2.0 — the same fixture shape the
/// crate's unit tests use.
fn seed_local_layout(root: &Path) {
    for version in ["0.1.0", "0.2.0"] {
        let dir = root.join("org.vibevm/wal").join(format!("v{version}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"{version}\"\n"
            ),
        )
        .unwrap();
        fs::write(dir.join("README.md"), format!("# wal {version}\n")).unwrap();
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#registry-model")]
fn local_registry_resolves_and_fetches_through_the_seam() {
    let fixture = tempdir().unwrap();
    seed_local_layout(fixture.path());
    let local = LocalRegistry::new(fixture.path()).unwrap();
    let reg: &dyn Registry = &local;

    // Version discovery is sorted ascending.
    let versions = reg
        .list_versions(&Group::parse("org.vibevm").unwrap(), "wal")
        .unwrap();
    assert_eq!(
        versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
        vec!["0.1.0", "0.2.0"]
    );

    // Unconstrained resolve picks the highest stable version.
    let resolved = reg
        .resolve(&PackageRef::parse("org.vibevm/wal").unwrap())
        .unwrap();
    assert_eq!(resolved.version.to_string(), "0.2.0");

    // Fetch materialises the per-project cache and hashes the payload.
    let cache = tempdir().unwrap();
    let cached = reg.fetch(&resolved, cache.path()).unwrap();
    assert!(cached.cache_dir.join("vibe.toml").exists());
    assert!(cached.cache_dir.join("README.md").exists());
    assert_eq!(cached.package_meta().version.to_string(), "0.2.0");
    assert!(cached.content_hash.starts_with("sha256:"));
    assert!(cached.source_uri.starts_with("file://"));
    // The M0 local backend leaves lockfile-v2 provenance blank.
    assert_eq!(cached.registry_name, None);
    assert!(!cached.overridden);

    // Unknown package surfaces the walk-routable discriminant.
    let err = reg
        .resolve(&PackageRef::parse("org.vibevm/nope").unwrap())
        .unwrap_err();
    assert!(matches!(err, RegistryError::UnknownPackage { .. }));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#registry-model")]
fn local_registry_missing_root_is_rejected_at_open() {
    let fixture = tempdir().unwrap();
    let absent = fixture.path().join("definitely-not-a-registry");
    let err = match LocalRegistry::new(&absent) {
        Ok(_) => panic!("opening a missing root must fail"),
        Err(e) => e,
    };
    assert!(matches!(err, RegistryError::MissingRoot(p) if p == absent));
}

/// Test-only backend: `bootstrap` copies a pre-seeded tree into the
/// clone destination (plus a `.git/` marker) and counts invocations,
/// so the monorepo registry can be driven without spawning git.
struct FakeGit {
    source: PathBuf,
    clone_calls: Mutex<u32>,
}

impl FakeGit {
    fn new(source: PathBuf) -> Self {
        FakeGit {
            source,
            clone_calls: Mutex::new(0),
        }
    }
    fn clone_count(&self) -> u32 {
        *self.clone_calls.lock().unwrap()
    }
}

impl GitBackend for FakeGit {
    fn bootstrap(&self, _url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
        *self.clone_calls.lock().unwrap() += 1;
        for entry in walkdir::WalkDir::new(&self.source)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let rel = entry.path().strip_prefix(&self.source).unwrap();
            if rel.as_os_str().is_empty() {
                continue;
            }
            let target = dest.join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target).unwrap();
            } else if entry.file_type().is_file() {
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::copy(entry.path(), &target).unwrap();
            }
        }
        fs::create_dir_all(dest.join(".git")).unwrap();
        Ok(())
    }
    fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
        Ok(())
    }
    fn list_tags(&self, _url: &str) -> Result<Vec<String>, GitError> {
        Ok(Vec::new())
    }
    fn fetch_file_at_ref(&self, url: &str, refname: &str, path: &str) -> Result<Vec<u8>, GitError> {
        Err(GitError::FileNotFoundInRef {
            url: url.to_string(),
            refname: refname.to_string(),
            path: path.to_string(),
        })
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-001#registry-trait")]
fn git_registry_clones_once_and_serves_git_shaped_source_uris() {
    let tmp = tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    seed_local_layout(&upstream);

    let backend = Arc::new(FakeGit::new(upstream));
    let cache_root = tmp.path().join("cache");
    let git = GitRegistry::open_with(
        "git@gitverse.ru:anarchic/vibespecs.git",
        "main",
        &cache_root,
        backend.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();
    assert_eq!(backend.clone_count(), 1, "first open must clone");
    assert!(
        git.clone_dir()
            .join("org.vibevm/wal/v0.2.0/vibe.toml")
            .exists()
    );
    assert!(git.cache_dir().join("meta.toml").exists());

    // Resolve + fetch through the seam: delegation to the local layout
    // for discovery, git-shaped `source_uri` on the way out.
    let reg: &dyn Registry = &git;
    let resolved = reg
        .resolve(&PackageRef::parse("org.vibevm/wal@0.1.0").unwrap())
        .unwrap();
    assert_eq!(resolved.version.to_string(), "0.1.0");

    let pkg_cache = tmp.path().join("pkg-cache");
    let cached = reg.fetch(&resolved, &pkg_cache).unwrap();
    assert_eq!(
        cached.source_uri,
        "git+ssh://git@gitverse.ru/anarchic/vibespecs.git#org.vibevm/wal/v0.1.0"
    );
    assert!(cached.cache_dir.join("vibe.toml").exists());
    assert!(cached.content_hash.starts_with("sha256:"));

    // A second open within the freshness TTL reuses the clone.
    let _again = GitRegistry::open_with(
        "git@gitverse.ru:anarchic/vibespecs.git",
        "main",
        &cache_root,
        backend.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();
    assert_eq!(backend.clone_count(), 1, "fresh cache must not re-clone");
}
