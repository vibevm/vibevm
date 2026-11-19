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

/// What one native capture produced: the verified tree, and the identities
/// a durable snapshot records so a later run can decide whether to reuse it.
pub(super) struct Captured {
    pub tree: PathBuf,
    pub commit: String,
    pub object: String,
    pub refname: String,
}

/// Capture `refname` of `url` natively into `work`, copying its verified
/// tree to `work/tree`.
///
/// `durable` names the directory as one the capability keeps: the bare
/// object store is then cloned on first use and **fetched** afterwards, with
/// every received object checked at receipt, instead of the whole repository
/// being cloned afresh (PROP-073 `#NATIVE-PREPARATION`). An ordinary
/// temporary capture passes `false` and behaves exactly as before.
pub(super) fn native_snapshot(
    backend: &dyn GitBackend,
    url: &str,
    refname: &str,
    work: &Path,
    durable: bool,
    namespace: Option<&str>,
) -> Result<Captured, RegistryError> {
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
    let address = strip_git_plus_prefix(url);
    let token = if durable {
        backend.prepare_cached_native_source(address, refname, namespace, &objects)?
    } else {
        backend.prepare_native_source(address, refname, None, &objects)?
    }
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
    // Publish a new immutable generation. Existing registry readers retain
    // their selected tree even when another process refreshes the cache.
    let generation = tempfile::Builder::new()
        .prefix("snapshot-")
        .tempdir_in(work)
        .map_err(|source| RegistryError::Io {
            path: work.to_path_buf(),
            source,
        })?;
    let tree = generation.path().join("tree");
    staged.copy_for_cache(&tree)?;
    let _retained_generation = generation.keep();
    Ok(Captured {
        tree,
        commit: revision.commit.clone(),
        object: revision.object.clone(),
        refname: revision.reference.clone(),
    })
}
