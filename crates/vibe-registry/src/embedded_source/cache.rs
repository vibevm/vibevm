//! Exact-pin cache hydration, authentication, and contained file reads.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction");

use super::*;

pub(super) fn ensure_at(
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

pub(super) fn verify_entry(
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

pub(super) fn require_real_directory(path: &Path) -> Result<(), EmbeddedSourceError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(EmbeddedSourceError::InvalidCache { path: path.into() });
    }
    Ok(())
}

pub(super) fn require_real_file(path: &Path) -> Result<(), EmbeddedSourceError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(EmbeddedSourceError::InvalidCache { path: path.into() });
    }
    Ok(())
}

pub(super) fn require_link_free_tree(root: &Path) -> Result<(), EmbeddedSourceError> {
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

/// Where one embedded source's hydrated tree lives.
///
/// The key is the **content hash alone** (`PROP-066#locator`): identity is
/// content and location is a replaceable detail, so the same tree fetched
/// from a mirror has to hit the entry the public host filled. Keying on
/// the URL and the commit as well — the `v1` salt — meant a remap row
/// could never reuse anything, which is the one thing a closed network
/// has least of. The old entries are not migrated and simply miss; the
/// cache is rebuildable, and a stale `v1` directory costs one re-fetch.
pub(super) fn entry_path(
    root: &Path,
    _url: &str,
    _commit: &str,
    content_hash: &ContentHash,
) -> PathBuf {
    let mut key = Sha256::new();
    key.update(b"vibe-embedded-source-v2\0");
    key.update(content_hash.as_str().as_bytes());
    root.join(hex(key.finalize()))
}

pub(super) fn hex_digest(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

pub(super) fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    use std::fmt::Write;
    bytes.into_iter().fold(String::new(), |mut rendered, byte| {
        let _ = write!(&mut rendered, "{byte:02x}");
        rendered
    })
}

pub(super) fn read_regular_contained(
    root: &Path,
    relative: &Path,
) -> Result<Vec<u8>, EmbeddedSourceError> {
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

pub(super) fn io(path: &Path, source: std::io::Error) -> EmbeddedSourceError {
    EmbeddedSourceError::Io {
        path: path.to_path_buf(),
        source,
    }
}
