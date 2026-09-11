//! Immutable external source cache for reference-backed packages.
//!
//! The published package owns only its manifest and adapters. Upstream bytes
//! arrive from the original public Git repository, are authenticated by both
//! exact commit and recipe-labelled tree hash, and land below the user's Vibe
//! cache. The cache path is an implementation detail; portable lock records
//! retain provenance and identities only.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction");

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use vibe_core::ContentHash;
use vibe_core::manifest::{EmbeddedSourceDecl, LockedEmbeddedSource};

use crate::git_backend::{GitBackend, GitError, ShellGit};
use crate::hash_recipe::RecipeId;
use crate::{RegistryError, compute_content_hash_with, copy_dir_recursive, store};

const CACHE_NAMESPACE: &[&str] = &["embedded", "git", "v1"];
const RECEIPT_FILE: &str = "source.toml";
const COMPLETE_FILE: &str = "complete";
const TREE_DIR: &str = "tree";

/// One authenticated, link-free extracted upstream tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedEmbeddedSource {
    pub tree: PathBuf,
    pub resolved_commit: String,
    pub tree_oid: String,
    pub content_hash: ContentHash,
}

#[derive(Debug, Error)]
pub enum EmbeddedSourceError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(
        "embedded source cache I/O failure at `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction; \
          fix: restore a writable Vibe settings cache and retry)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "embedded source `{url}` resolved to commit `{actual}`, expected `{expected}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction; \
          fix: correct the bridge's immutable commit pin)"
    )]
    CommitMismatch {
        url: String,
        expected: String,
        actual: String,
    },
    #[error(
        "embedded source `{url}` did not expose a checked-out commit/tree identity \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction; \
          fix: use a Git backend that can report HEAD and HEAD^{{tree}})"
    )]
    MissingGitIdentity { url: String },
    #[error(
        "embedded source `{url}` at `{commit}` hashes to `{actual}`, expected `{expected}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction; \
          fix: recompute the bridge pin from the exact upstream commit or investigate upstream drift)"
    )]
    HashMismatch {
        url: String,
        commit: String,
        expected: String,
        actual: String,
    },
    #[error(
        "embedded source cache entry `{path}` is incomplete or disagrees with its portable receipt \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction; \
          fix: remove this one derived cache entry and retry online)"
    )]
    InvalidCache { path: PathBuf },
    #[error(
        "embedded source `{url}` at `{commit}` is not available offline in `{path}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#offline; \
          fix: run the install once online to hydrate this exact source pin)"
    )]
    OfflineUnavailable {
        url: String,
        commit: String,
        path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheReceipt {
    source_url: String,
    resolved_commit: String,
    tree_oid: String,
    content_hash: String,
}

/// Authenticate every declaration and return the portable nested lock rows.
pub fn lock_embedded_sources(
    declarations: &[EmbeddedSourceDecl],
) -> Result<Vec<LockedEmbeddedSource>, EmbeddedSourceError> {
    lock_embedded_sources_with(declarations, false)
}

pub fn lock_embedded_sources_with(
    declarations: &[EmbeddedSourceDecl],
    offline: bool,
) -> Result<Vec<LockedEmbeddedSource>, EmbeddedSourceError> {
    let root = embedded_cache_root()?;
    let git: Arc<dyn GitBackend> = ShellGit::new()
        .anonymized_for_public()
        .unwrap_or_else(|| Arc::new(ShellGit::new()));
    declarations
        .iter()
        .map(|declaration| {
            let cached = ensure_declared_at(&root, declaration, Arc::clone(&git), offline)?;
            let license_bytes = read_regular_contained(&cached.tree, &declaration.license_path)?;
            let license_file_sha256 =
                ContentHash::from_validated(format!("sha256:{}", hex_digest(&license_bytes)));
            Ok(LockedEmbeddedSource {
                name: declaration.name.clone(),
                source_url: vibe_core::SourceUrl::new(declaration.url.clone()),
                source_ref: declaration.ref_hint.clone(),
                resolved_commit: cached.resolved_commit,
                tree_oid: cached.tree_oid,
                content_hash: cached.content_hash,
                upstream_license: declaration.upstream_license.clone(),
                license_path: declaration.license_path.clone(),
                license_url: declaration.license_url.clone(),
                license_file_sha256,
            })
        })
        .collect()
}

/// Ensure one manifest declaration is present in the default machine cache.
pub fn cache_embedded_source(
    declaration: &EmbeddedSourceDecl,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    cache_embedded_source_with(declaration, false)
}

pub fn cache_embedded_source_with(
    declaration: &EmbeddedSourceDecl,
    offline: bool,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    let root = embedded_cache_root()?;
    let git: Arc<dyn GitBackend> = ShellGit::new()
        .anonymized_for_public()
        .unwrap_or_else(|| Arc::new(ShellGit::new()));
    ensure_declared_at(&root, declaration, git, offline)
}

/// Re-open or hydrate one source from portable lock evidence.
pub fn cache_locked_embedded_source(
    locked: &LockedEmbeddedSource,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    cache_locked_embedded_source_with(locked, false)
}

pub fn cache_locked_embedded_source_with(
    locked: &LockedEmbeddedSource,
    offline: bool,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    let root = embedded_cache_root()?;
    let git: Arc<dyn GitBackend> = ShellGit::new()
        .anonymized_for_public()
        .unwrap_or_else(|| Arc::new(ShellGit::new()));
    ensure_at(
        &root,
        locked.source_url.as_str(),
        &locked.resolved_commit,
        &locked.content_hash,
        git,
        offline,
    )
}

fn ensure_declared_at(
    root: &Path,
    declaration: &EmbeddedSourceDecl,
    git: Arc<dyn GitBackend>,
    offline: bool,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    ensure_at(
        root,
        &declaration.url,
        &declaration.commit,
        &declaration.content_hash,
        git,
        offline,
    )
}

fn embedded_cache_root() -> Result<PathBuf, RegistryError> {
    let mut root = store::store_root()?;
    for component in CACHE_NAMESPACE {
        root.push(component);
    }
    Ok(root)
}

fn ensure_at(
    root: &Path,
    url: &str,
    commit: &str,
    expected_hash: &ContentHash,
    git: Arc<dyn GitBackend>,
    offline: bool,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    let entry = entry_path(root, url, commit, expected_hash);
    if entry.join(COMPLETE_FILE).is_file() {
        return verify_entry(&entry, url, commit, expected_hash);
    }
    if offline {
        return Err(EmbeddedSourceError::OfflineUnavailable {
            url: url.to_string(),
            commit: commit.to_string(),
            path: entry,
        });
    }

    let parent = entry
        .parent()
        .ok_or_else(|| EmbeddedSourceError::InvalidCache {
            path: entry.clone(),
        })?;
    fs::create_dir_all(parent).map_err(|source| io(parent, source))?;
    let temporary = parent.join(format!(".fetch-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(|source| io(&temporary, source))?;
    }
    fs::create_dir(&temporary).map_err(|source| io(&temporary, source))?;
    let checkout = temporary.join("checkout");

    let build = (|| {
        git.bootstrap_embedded(url, commit, &checkout)?;
        let actual_commit =
            git.head_commit(&checkout)?
                .ok_or_else(|| EmbeddedSourceError::MissingGitIdentity {
                    url: url.to_string(),
                })?;
        let tree_oid =
            git.head_tree(&checkout)?
                .ok_or_else(|| EmbeddedSourceError::MissingGitIdentity {
                    url: url.to_string(),
                })?;
        if actual_commit != commit {
            return Err(EmbeddedSourceError::CommitMismatch {
                url: url.to_string(),
                expected: commit.to_string(),
                actual: actual_commit,
            });
        }
        let actual_hash = compute_content_hash_with(RecipeId::Tree1, &checkout)?;
        if actual_hash != expected_hash.as_str() {
            return Err(EmbeddedSourceError::HashMismatch {
                url: url.to_string(),
                commit: commit.to_string(),
                expected: expected_hash.to_string(),
                actual: actual_hash,
            });
        }

        let tree = temporary.join(TREE_DIR);
        copy_dir_recursive(&checkout, &tree)?;
        fs::remove_dir_all(&checkout).map_err(|source| io(&checkout, source))?;
        let receipt = CacheReceipt {
            source_url: url.to_string(),
            resolved_commit: commit.to_string(),
            tree_oid,
            content_hash: expected_hash.to_string(),
        };
        let receipt_text =
            toml::to_string_pretty(&receipt).map_err(|source| EmbeddedSourceError::Io {
                path: temporary.join(RECEIPT_FILE),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
            })?;
        fs::write(temporary.join(RECEIPT_FILE), receipt_text)
            .map_err(|source| io(&temporary.join(RECEIPT_FILE), source))?;
        fs::write(temporary.join(COMPLETE_FILE), b"ok\n")
            .map_err(|source| io(&temporary.join(COMPLETE_FILE), source))?;
        Ok::<(), EmbeddedSourceError>(())
    })();

    if let Err(error) = build {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error);
    }

    match fs::rename(&temporary, &entry) {
        Ok(()) => {}
        Err(_) if entry.join(COMPLETE_FILE).is_file() => {
            let _ = fs::remove_dir_all(&temporary);
        }
        Err(source) => {
            let _ = fs::remove_dir_all(&temporary);
            return Err(io(&entry, source));
        }
    }
    verify_entry(&entry, url, commit, expected_hash)
}

fn verify_entry(
    entry: &Path,
    url: &str,
    commit: &str,
    expected_hash: &ContentHash,
) -> Result<CachedEmbeddedSource, EmbeddedSourceError> {
    require_real_directory(entry)?;
    let receipt_path = entry.join(RECEIPT_FILE);
    require_real_file(&receipt_path)?;
    require_real_file(&entry.join(COMPLETE_FILE))?;
    let raw = fs::read_to_string(&receipt_path).map_err(|source| io(&receipt_path, source))?;
    let receipt: CacheReceipt = toml::from_str(&raw)
        .map_err(|_| EmbeddedSourceError::InvalidCache { path: entry.into() })?;
    if receipt.source_url != url
        || receipt.resolved_commit != commit
        || receipt.content_hash != expected_hash.as_str()
    {
        return Err(EmbeddedSourceError::InvalidCache { path: entry.into() });
    }
    let tree = entry.join(TREE_DIR);
    require_link_free_tree(&tree)?;
    let actual_hash = compute_content_hash_with(RecipeId::Tree1, &tree)?;
    if actual_hash != expected_hash.as_str() {
        return Err(EmbeddedSourceError::HashMismatch {
            url: url.to_string(),
            commit: commit.to_string(),
            expected: expected_hash.to_string(),
            actual: actual_hash,
        });
    }
    Ok(CachedEmbeddedSource {
        tree,
        resolved_commit: receipt.resolved_commit,
        tree_oid: receipt.tree_oid,
        content_hash: expected_hash.clone(),
    })
}

fn require_real_directory(path: &Path) -> Result<(), EmbeddedSourceError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(EmbeddedSourceError::InvalidCache { path: path.into() });
    }
    Ok(())
}

fn require_real_file(path: &Path) -> Result<(), EmbeddedSourceError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(EmbeddedSourceError::InvalidCache { path: path.into() });
    }
    Ok(())
}

fn require_link_free_tree(root: &Path) -> Result<(), EmbeddedSourceError> {
    require_real_directory(root)?;
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.map_err(|source| {
            io(
                root,
                std::io::Error::other(format!("cannot inspect cache tree: {source}")),
            )
        })?;
        if entry.file_type().is_symlink() {
            return Err(EmbeddedSourceError::InvalidCache {
                path: entry.path().to_path_buf(),
            });
        }
    }
    Ok(())
}

fn entry_path(root: &Path, url: &str, commit: &str, content_hash: &ContentHash) -> PathBuf {
    let mut key = Sha256::new();
    key.update(b"vibe-embedded-source-v1\0");
    key.update(url.as_bytes());
    key.update([0]);
    key.update(commit.as_bytes());
    key.update([0]);
    key.update(content_hash.as_str().as_bytes());
    root.join(hex(key.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    use std::fmt::Write;
    bytes.into_iter().fold(String::new(), |mut rendered, byte| {
        let _ = write!(&mut rendered, "{byte:02x}");
        rendered
    })
}

fn read_regular_contained(root: &Path, relative: &Path) -> Result<Vec<u8>, EmbeddedSourceError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(EmbeddedSourceError::InvalidCache { path: current });
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|source| io(&current, source))?;
        if metadata.file_type().is_symlink() {
            return Err(EmbeddedSourceError::InvalidCache { path: current });
        }
    }
    let metadata = fs::symlink_metadata(&current).map_err(|source| io(&current, source))?;
    if !metadata.is_file() {
        return Err(EmbeddedSourceError::InvalidCache { path: current });
    }
    fs::read(&current).map_err(|source| io(&current, source))
}

fn io(path: &Path, source: std::io::Error) -> EmbeddedSourceError {
    EmbeddedSourceError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FixtureGit {
        source: PathBuf,
        calls: AtomicUsize,
        commit: String,
    }

    impl GitBackend for FixtureGit {
        fn bootstrap(&self, _url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            copy_fixture(&self.source, dest);
            Ok(())
        }

        fn bootstrap_embedded(
            &self,
            _url: &str,
            _refname: &str,
            dest: &Path,
        ) -> Result<(), GitError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            copy_fixture(&self.source, dest);
            Ok(())
        }

        fn head_commit(&self, _dest: &Path) -> Result<Option<String>, GitError> {
            Ok(Some(self.commit.clone()))
        }

        fn head_tree(&self, _dest: &Path) -> Result<Option<String>, GitError> {
            Ok(Some("0123456789abcdef0123456789abcdef01234567".into()))
        }

        fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn list_tags(&self, _url: &str) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }

        fn fetch_file_at_ref(
            &self,
            url: &str,
            refname: &str,
            path: &str,
        ) -> Result<Vec<u8>, GitError> {
            Err(GitError::FileNotFoundInRef {
                url: url.into(),
                refname: refname.into(),
                path: path.into(),
            })
        }
    }

    fn copy_fixture(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in walkdir::WalkDir::new(source).into_iter().skip(1) {
            let entry = entry.unwrap();
            let relative = entry.path().strip_prefix(source).unwrap();
            let target = destination.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(target).unwrap();
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    #[test]
    fn exact_source_is_fetched_once_then_verified_from_cache() {
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("SKILL.md"), "# upstream\n").unwrap();
        let expected = ContentHash::from_validated(
            compute_content_hash_with(RecipeId::Tree1, source.path()).unwrap(),
        );
        let root = tempfile::tempdir().unwrap();
        let commit = "0123456789abcdef0123456789abcdef01234567".to_string();
        let git = Arc::new(FixtureGit {
            source: source.path().to_path_buf(),
            calls: AtomicUsize::new(0),
            commit: commit.clone(),
        });

        let first = ensure_at(
            root.path(),
            "https://example.test/upstream.git",
            &commit,
            &expected,
            git.clone(),
            false,
        )
        .unwrap();
        let second = ensure_at(
            root.path(),
            "https://example.test/upstream.git",
            &commit,
            &expected,
            git.clone(),
            false,
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(git.calls.load(Ordering::SeqCst), 1);
        assert!(first.tree.join("SKILL.md").is_file());
        assert!(!first.tree.join(".git").exists());
    }

    #[test]
    fn content_mismatch_never_publishes_a_complete_entry() {
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("SKILL.md"), "# upstream\n").unwrap();
        let root = tempfile::tempdir().unwrap();
        let commit = "0123456789abcdef0123456789abcdef01234567".to_string();
        let expected = ContentHash::from_validated(format!("sha256-tree/1:{}", "f".repeat(64)));
        let git = Arc::new(FixtureGit {
            source: source.path().to_path_buf(),
            calls: AtomicUsize::new(0),
            commit: commit.clone(),
        });

        let error = ensure_at(
            root.path(),
            "https://example.test/upstream.git",
            &commit,
            &expected,
            git,
            false,
        )
        .unwrap_err();
        assert!(matches!(error, EmbeddedSourceError::HashMismatch { .. }));
        assert!(
            !entry_path(
                root.path(),
                "https://example.test/upstream.git",
                &commit,
                &expected
            )
            .exists()
        );
    }

    #[test]
    fn offline_cache_miss_does_not_invoke_git() {
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("SKILL.md"), "# upstream\n").unwrap();
        let root = tempfile::tempdir().unwrap();
        let commit = "0123456789abcdef0123456789abcdef01234567".to_string();
        let expected = ContentHash::from_validated(format!("sha256-tree/1:{}", "f".repeat(64)));
        let git = Arc::new(FixtureGit {
            source: source.path().to_path_buf(),
            calls: AtomicUsize::new(0),
            commit: commit.clone(),
        });

        let error = ensure_at(
            root.path(),
            "https://example.test/upstream.git",
            &commit,
            &expected,
            git.clone(),
            true,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            EmbeddedSourceError::OfflineUnavailable { .. }
        ));
        assert_eq!(git.calls.load(Ordering::SeqCst), 0);
    }
}
