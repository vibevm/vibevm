//! Durable controlled-registry state and tree witnesses.
//!
//! Layout: `<cache-root>/ref-<sha256(reference)>/<url-key16>/native/`
//! contains `native.toml`, `native.lock`, `objects.git/`, and immutable
//! `snapshot-<nonce>/tree/` generations. State is published atomically after
//! staging. Validation and publication take the same process lock; readers
//! retain their generation after releasing it, so a refresh cannot replace
//! their selected bytes. Old generations are retained conservatively until
//! explicit cache cleanup; no age-based deletion can invalidate a live reader.
//!
//! Reuse requires current route admission, permitted freshness posture,
//! matching tree digest and matching captured attribute/conversion inputs.
//! Missing/unknown state recaptures online and refuses offline. Initial
//! objects receive full fsck before a reusable marker is published; fetched
//! objects are checked at receipt. The cache is not publisher authentication
//! or a hostile-local-writer boundary.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::RegistryError;

/// The format of the durable snapshot. A state file carrying any other
/// value is not read: its directory is re-created from the endpoint.
pub(super) const SCHEMA: u32 = 2;

/// What one durable snapshot holds, and under what it may be reused.
///
/// `deny_unknown_fields`, like every other state file vibe keeps: a field a
/// newer vibe added is a format this one does not understand, and the
/// honest answer is a fresh capture rather than a partial read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeSnapshotState {
    /// Must be [`SCHEMA`].
    pub schema: u32,
    /// The normalized canonical registry url this cache belongs to.
    pub url: String,
    /// The reference as the registry declared it (`main`, a tag, an oid).
    pub reference: String,
    /// The namespace the reference resolved to (`refs/heads/main`).
    pub refname: String,
    /// The candidate that actually served — the canonical url unless a
    /// `[[mirror]]` or remap row did (PROP-066 `#provenance`).
    pub served_url: String,
    /// The object id the endpoint advertised at `refname`: exactly what a
    /// later `ls-remote` is compared against.
    pub object: String,
    /// The peeled commit the tree holds.
    pub commit: String,
    /// Immutable snapshot generation, local to this cache directory.
    pub generation: String,
    /// The per-file sha256 witness of `tree`, re-checked on every use.
    pub tree_digest: String,
    /// The admission inputs the staging probe read when this tree was
    /// admitted (`GitBackend::native_admission_inputs`).
    pub admission: String,
    /// When the endpoint was last asked, for the freshness window.
    pub checked_at: String,
}

/// One registry's durable snapshot directory.
pub(super) struct NativeCache {
    root: PathBuf,
    url: String,
    normalized: String,
}

/// Held while a cache is created or refreshed, so two vibe processes do not
/// write the same objects or the same tree at once.
///
/// The lock is advisory and exclusive, as the home's remap lock is
/// (`vibe_core::remap::consent`), and it is **blocking**: a second process
/// racing the first wants the first's result, not a refusal. All state validation and publication take this lock.
pub(super) struct CacheLock {
    _file: fs::File,
}

impl NativeCache {
    /// The cache of `url`, under the same `<cache_root>/<key16>` the
    /// ordinary transport keys its clone by, so one registry has one
    /// directory whichever transport reads it.
    pub(super) fn at(cache_root: &Path, url: &str) -> Result<Self, RegistryError> {
        let normalized = crate::registry_cache::normalize_url(url);
        let mut digest = Sha256::new();
        digest.update(normalized.as_bytes());
        let key = digest
            .finalize()
            .iter()
            .fold(String::new(), |mut acc, byte| {
                use std::fmt::Write;
                let _ = write!(&mut acc, "{byte:02x}");
                acc
            });
        let root = cache_root.join(&key[..16]).join("native");
        fs::create_dir_all(&root).map_err(|source| RegistryError::Io {
            path: root.clone(),
            source,
        })?;
        Ok(NativeCache {
            root,
            url: url.to_owned(),
            normalized,
        })
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    /// The verified checkout the registry reads.
    pub(super) fn tree(&self, state: &NativeSnapshotState) -> PathBuf {
        self.root.join(&state.generation).join("tree")
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("native.toml")
    }

    /// Take the exclusive write lock of this cache.
    pub(super) fn lock(&self) -> Result<CacheLock, RegistryError> {
        let path = self.root.join("native.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| RegistryError::Io {
                path: path.clone(),
                source,
            })?;
        loop {
            crate::git_backend::transfers::invocation_control()
                .checkpoint()
                .map_err(|_| {
                    RegistryError::Git(crate::GitError::ContactRefused(
                        crate::ContactRefusal::OperationCancelled,
                    ))
                })?;
            match file.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) => {
                    std::thread::sleep(std::time::Duration::from_millis(50))
                }
                Err(std::fs::TryLockError::Error(source)) => {
                    return Err(RegistryError::Io { path, source });
                }
            }
        }
        Ok(CacheLock { _file: file })
    }

    /// The recorded state, or `None` when there is none this vibe reads:
    /// an absent file, an unparsable one, another schema, or one about
    /// another url. Every one of those is answered by capturing afresh, so
    /// none of them is an error the caller has to handle.
    pub(super) fn state(&self, reference: &str) -> Option<NativeSnapshotState> {
        let raw = fs::read_to_string(self.state_path()).ok()?;
        let state: NativeSnapshotState = toml::from_str(&raw)
            .inspect_err(
                |error| tracing::debug!(target: "vibe_registry::monorepo", %error, "ignoring an unreadable native snapshot state"),
            )
            .ok()?;
        let mut components = Path::new(&state.generation).components();
        let local_generation = matches!(components.next(), Some(std::path::Component::Normal(_)))
            && components.next().is_none()
            && state.generation.starts_with("snapshot-");
        (state.schema == SCHEMA
            && state.url == self.normalized
            && state.reference == reference
            && local_generation)
            .then_some(state)
    }

    /// Record what the cache now holds. Written last, after the tree is in
    /// place: a state file naming a tree that is not there yet would be a
    /// claim the next process would check and reject, but checking costs a
    /// whole capture and the ordering costs nothing.
    pub(super) fn write_state(&self, state: &NativeSnapshotState) -> Result<(), RegistryError> {
        let path = self.state_path();
        let raw = toml::to_string_pretty(state).map_err(|error| RegistryError::MalformedMeta {
            path: path.clone(),
            reason: error.to_string(),
        })?;
        use std::io::Write;
        let mut staged =
            tempfile::NamedTempFile::new_in(&self.root).map_err(|source| RegistryError::Io {
                path: self.root.clone(),
                source,
            })?;
        staged
            .write_all(raw.as_bytes())
            .map_err(|source| RegistryError::Io {
                path: path.clone(),
                source,
            })?;
        staged.persist(&path).map_err(|error| RegistryError::Io {
            path,
            source: error.error,
        })?;
        Ok(())
    }

    /// Note that the endpoint was asked now, without touching anything else
    /// the state records. A failure here loses a freshness window, never
    /// correctness, so it is not raised.
    pub(super) fn touch(&self, state: &NativeSnapshotState) {
        let mut refreshed = state.clone();
        refreshed.checked_at = vibe_core::timestamp::now_utc();
        let _ = self.write_state(&refreshed);
    }

    /// The canonical url this cache belongs to, as declared.
    pub(super) fn url(&self) -> &str {
        &self.url
    }
}

/// Whether `state` was checked against its endpoint within `ttl_secs`.
///
/// `>=` rather than `>`, exactly as the ordinary transport's freshness does,
/// so a TTL of zero always asks even when both timestamps land in the same
/// second.
pub(super) fn within_window(state: &NativeSnapshotState, ttl_secs: u64) -> bool {
    let Some(previous) = vibe_core::timestamp::parse_unix_utc(&state.checked_at) else {
        return false;
    };
    let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return false;
    };
    now.as_secs().saturating_sub(previous) < ttl_secs
}

/// The per-file witness of a directory tree: every path, its mode and the
/// sha256 of its bytes, hashed into one value.
///
/// This is the same question `materialisation::witness` asks of a staged
/// checkout, asked of a checkout that outlived the process which staged it.
/// It reads every byte of the tree, which is why it is worth doing: the
/// alternative it replaces is a clone, an object walk, a path probe, a
/// checkout and two copies of the same tree.
pub(super) fn tree_digest(tree: &Path) -> Result<Option<String>, RegistryError> {
    if !tree.is_dir() {
        return Ok(None);
    }
    let mut entries = std::collections::BTreeMap::new();
    for entry in walkdir::WalkDir::new(tree).follow_links(false) {
        let entry = entry.map_err(|error| RegistryError::MalformedMeta {
            path: tree.to_path_buf(),
            reason: format!("the retained snapshot tree cannot be read: {error}"),
        })?;
        let relative = entry
            .path()
            .strip_prefix(tree)
            .map_err(|_| RegistryError::MalformedMeta {
                path: tree.to_path_buf(),
                reason: "a retained snapshot path escapes its tree".into(),
            })?
            .to_path_buf();
        let metadata = fs::symlink_metadata(entry.path()).map_err(|source| RegistryError::Io {
            path: entry.path().to_path_buf(),
            source,
        })?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::MetadataExt;
            metadata.mode()
        };
        #[cfg(not(unix))]
        let mode = u32::from(metadata.permissions().readonly());
        let content = if metadata.is_file() {
            let mut file = fs::File::open(entry.path()).map_err(|source| RegistryError::Io {
                path: entry.path().to_path_buf(),
                source,
            })?;
            let mut digest = Sha256::new();
            std::io::copy(&mut file, &mut digest).map_err(|source| RegistryError::Io {
                path: entry.path().to_path_buf(),
                source,
            })?;
            digest.finalize().to_vec()
        } else if metadata.file_type().is_symlink() {
            fs::read_link(entry.path())
                .map_err(|source| RegistryError::Io {
                    path: entry.path().to_path_buf(),
                    source,
                })?
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if metadata.is_dir() {
            Vec::new()
        } else {
            return Err(RegistryError::MalformedMeta {
                path: entry.path().to_path_buf(),
                reason: "a retained snapshot holds a special file".into(),
            });
        };
        entries.insert(relative, (mode, content));
    }
    let mut digest = Sha256::new();
    digest.update(b"vibe-native-snapshot-tree/1\0");
    for (path, (mode, content)) in &entries {
        digest.update(path.as_os_str().as_encoded_bytes());
        digest.update([0]);
        digest.update(mode.to_le_bytes());
        digest.update(content);
    }
    Ok(Some(hex(&digest.finalize())))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, byte| {
        use std::fmt::Write;
        let _ = write!(&mut acc, "{byte:02x}");
        acc
    })
}

#[cfg(test)]
#[path = "native_cache_tests.rs"]
mod tests;
