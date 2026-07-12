//! Tests for the git-monorepo registry cell — clone-vs-update
//! freshness, transport detection, and the git-shaped `source_uri`
//! recorded at fetch time.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#registry-trait");

use super::*;
use crate::git_backend::GitError;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::tempdir;

use fixtures::*;

#[test]
fn normalize_url_strips_git_and_lowercases() {
    assert_eq!(
        normalize_url("git@Gitverse.ru:Anarchic/VibeSpecs.git"),
        "git@gitverse.ru:anarchic/vibespecs"
    );
    assert_eq!(
        normalize_url("https://Gitverse.ru/anarchic/vibespecs.git/"),
        "https://gitverse.ru/anarchic/vibespecs"
    );
}

#[test]
fn detect_transport_variants() {
    assert_eq!(detect_transport("git@host:o/r.git"), "git+ssh");
    assert_eq!(detect_transport("ssh://git@host/o/r.git"), "git+ssh");
    assert_eq!(detect_transport("https://host/o/r.git"), "git+https");
    assert_eq!(detect_transport("file:///a/b"), "git+file");
    assert_eq!(detect_transport("git+https://host/o/r"), "git+https");
}

#[test]
fn source_uri_for_git_produces_fragment() {
    let group = Group::parse("org.vibevm").unwrap();
    let s = source_uri_for_git(
        "git@gitverse.ru:anarchic/vibespecs.git",
        &group,
        "wal",
        "0.1.0",
    );
    assert_eq!(
        s,
        "git+ssh://git@gitverse.ru/anarchic/vibespecs.git#org.vibevm.world/wal/v0.1.0"
    );
}

#[test]
fn open_clones_on_first_use_and_skips_when_fresh() {
    let tmp = tempdir().unwrap();
    let cache_root = tmp.path().join("cache");
    let upstream = tmp.path().join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    seed_fixture_layout(&upstream);

    let fake = Arc::new(FakeGit::default());
    fake.seed_source(upstream.clone());

    // First open → clone.
    let r1 = GitRegistry::open_with(
        "git@host:o/r.git",
        "main",
        &cache_root,
        fake.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();
    assert_eq!(fake.clone_count(), 1);
    assert_eq!(fake.update_count(), 0);
    assert!(
        r1.clone_dir()
            .join("org.vibevm.world/wal/v0.1.0/vibe.toml")
            .exists()
    );
    assert!(r1.cache_dir().join("meta.toml").exists());

    // Second open with fresh TTL → no update.
    let _r2 = GitRegistry::open_with(
        "git@host:o/r.git",
        "main",
        &cache_root,
        fake.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();
    assert_eq!(fake.clone_count(), 1);
    assert_eq!(fake.update_count(), 0);

    // Second open with zero TTL → update fires.
    let _r3 =
        GitRegistry::open_with("git@host:o/r.git", "main", &cache_root, fake.clone(), 0).unwrap();
    assert_eq!(fake.clone_count(), 1);
    assert_eq!(fake.update_count(), 1);
}

#[test]
fn sync_always_updates() {
    let tmp = tempdir().unwrap();
    let cache_root = tmp.path().join("cache");
    let upstream = tmp.path().join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    seed_fixture_layout(&upstream);

    let fake = Arc::new(FakeGit::default());
    fake.seed_source(upstream.clone());

    let r = GitRegistry::open_with(
        "git@host:o/r.git",
        "main",
        &cache_root,
        fake.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();
    assert_eq!(fake.update_count(), 0);
    r.sync().unwrap();
    assert_eq!(fake.update_count(), 1);
    r.sync().unwrap();
    assert_eq!(fake.update_count(), 2);
}

#[test]
fn resolve_and_fetch_produce_git_source_uri() {
    let tmp = tempdir().unwrap();
    let cache_root = tmp.path().join("cache");
    let upstream = tmp.path().join("upstream");
    fs::create_dir_all(&upstream).unwrap();
    seed_fixture_layout(&upstream);

    let fake = Arc::new(FakeGit::default());
    fake.seed_source(upstream.clone());

    let r = GitRegistry::open_with(
        "git@gitverse.ru:anarchic/vibespecs.git",
        "main",
        &cache_root,
        fake.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap();
    let pkgref = PackageRef::parse("org.vibevm.world/wal@0.1.0").unwrap();
    let resolved = r.resolve(&pkgref).unwrap();
    assert_eq!(resolved.version.to_string(), "0.1.0");

    let pkg_cache = tmp.path().join("pkg_cache");
    fs::create_dir_all(&pkg_cache).unwrap();
    let cached = r.fetch(&resolved, &pkg_cache).unwrap();
    assert!(cached.source_uri.starts_with("git+ssh://"));
    assert!(cached.source_uri.ends_with("#org.vibevm.world/wal/v0.1.0"));
}

/// Test-only fixtures behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps these fixtures
/// reading as test code now that the tests live outside the parent
/// module's inline `mod tests`.
#[cfg(test)]
mod fixtures {
    use super::*;

    /// Test-only backend that records calls and expects the caller to
    /// pre-seed the clone directory with a file tree. Replaces a real
    /// git clone with a filesystem copy.
    #[derive(Debug, Default)]
    pub(super) struct FakeGit {
        source: Mutex<Option<PathBuf>>,
        clone_calls: Mutex<u32>,
        update_calls: Mutex<u32>,
    }

    impl FakeGit {
        pub(super) fn seed_source(&self, src: PathBuf) {
            *self.source.lock().unwrap() = Some(src);
        }
        pub(super) fn clone_count(&self) -> u32 {
            *self.clone_calls.lock().unwrap()
        }
        pub(super) fn update_count(&self) -> u32 {
            *self.update_calls.lock().unwrap()
        }
    }

    impl GitBackend for FakeGit {
        fn bootstrap(&self, _url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            *self.clone_calls.lock().unwrap() += 1;
            let src = self.source.lock().unwrap().clone().unwrap();
            fs::create_dir_all(dest).unwrap();
            copy_tree(&src, dest);
            // Mark as a real git repo for the `.git` presence check.
            fs::create_dir_all(dest.join(".git")).unwrap();
            Ok(())
        }
        fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
            *self.update_calls.lock().unwrap() += 1;
            Ok(())
        }
        fn list_tags(&self, _url: &str) -> Result<Vec<String>, GitError> {
            // Tests using FakeGit pre-seed a working tree directly; they
            // do not exercise the resolver's tag-listing path. Default to
            // empty so a stray call does not panic.
            Ok(Vec::new())
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            refname: &str,
            path: &str,
        ) -> Result<Vec<u8>, GitError> {
            Err(GitError::FileNotFoundInRef {
                url: url.to_string(),
                refname: refname.to_string(),
                path: path.to_string(),
            })
        }
    }

    fn copy_tree(src: &Path, dst: &Path) {
        for entry in walkdir::WalkDir::new(src)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let rel = entry.path().strip_prefix(src).unwrap();
            let target = dst.join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target).unwrap();
            } else if entry.file_type().is_file() {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    pub(super) fn seed_fixture_layout(root: &Path) {
        // Group-native on-disk layout (PROP-008): `<group>/<name>/v<ver>/`.
        let v = root.join("org.vibevm.world/wal/v0.1.0");
        fs::create_dir_all(&v).unwrap();
        fs::write(
            v.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"
description = "WAL v0.1.0"
"#,
        )
        .unwrap();
        fs::write(v.join("README.md"), "# wal 0.1.0\n").unwrap();
    }
}
