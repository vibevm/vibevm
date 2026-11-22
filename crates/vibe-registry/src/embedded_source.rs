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
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction",
    r = 1
)]
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

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#DECLARED-SOURCE-AUTH"
)]
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
                upstream_authors: declaration.upstream_authors.clone(),
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

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#DECLARED-SOURCE-CACHE"
)]
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

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#DECLARED-SOURCE-CACHE"
)]
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

mod cache;
use cache::*;

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
