//! How a monorepo registry obtains its tree: a working clone of the first
//! reachable candidate, or a native snapshot under a controlled transport.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type");

use std::fs;
use std::path::{Path, PathBuf};

use crate::RegistryError;
use crate::git_backend::GitBackend;
use crate::registry_cache::strip_git_plus_prefix;

/// Clone the first reachable candidate — `url`, then each alternative — into
/// `clone_dir`, returning the candidate that served. A failed attempt's
/// partial directory is removed before the next one. The canonical url's
/// error is the one reported when every candidate fails.
pub(super) fn clone_first_reachable(
    backend: &dyn GitBackend,
    url: &str,
    candidates: &[String],
    refname: &str,
    clone_dir: &Path,
) -> Result<String, RegistryError> {
    let mut first_error = None;
    for candidate in std::iter::once(url).chain(candidates.iter().map(String::as_str)) {
        // Clean up any half-populated clone dir from a prior failure.
        if clone_dir.exists() {
            fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                path: clone_dir.to_path_buf(),
                source,
            })?;
        }
        tracing::info!(target: "vibe_registry", url = %candidate, dest = %clone_dir.display(), "cloning registry");
        // The registry working copy is data vibe reads and hashes.
        match backend.bootstrap_data(strip_git_plus_prefix(candidate), refname, clone_dir) {
            Ok(()) => return Ok(candidate.to_string()),
            Err(error) => {
                tracing::warn!(target: "vibe_registry", url = %candidate, %error, "registry clone candidate failed");
                first_error.get_or_insert(error);
            }
        }
    }
    Err(match first_error {
        Some(error) => RegistryError::Git(error),
        None => RegistryError::MissingRoot(clone_dir.to_path_buf()),
    })
}

/// Capture `refname` of `url` natively into `work` and copy its verified
/// tree to `work/tree`; returns the tree and the commit.
pub(super) fn native_snapshot(
    backend: &dyn GitBackend,
    url: &str,
    refname: &str,
    work: &Path,
) -> Result<(PathBuf, String), RegistryError> {
    fs::create_dir_all(work).map_err(|source| RegistryError::Io {
        path: work.to_path_buf(),
        source,
    })?;
    let objects = work.join("objects.git");
    let unavailable = || {
        RegistryError::Git(crate::GitError::NativeUnavailable {
            operation: "monorepo snapshot under the controlled transport",
        })
    };
    let token = backend
        .prepare_native_source(strip_git_plus_prefix(url), refname, None, &objects)?
        .ok_or_else(unavailable)?;
    // The bound destination stays absent: the verified tree is copied to
    // a sibling, exactly as a cache candidate is.
    let revision = token.bind(&work.join("bound"))?;
    // A registry working copy is read as data — its manifests, its template
    // snapshot and the digest documents hashed against it — so it holds the
    // recorded blob bytes, not this user's text conversion
    // (`crate::git_backend::verbatim`).
    let staged = backend
        .stage_native_materialisation(
            &objects,
            &revision,
            crate::git_backend::PayloadBytes::Verbatim,
        )?
        .ok_or_else(unavailable)?;
    let tree = work.join("tree");
    staged.copy_for_cache(&tree)?;
    Ok((tree, revision.commit.clone()))
}
