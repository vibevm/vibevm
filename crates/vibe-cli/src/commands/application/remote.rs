//! Mutable application-source proxy resolution with immutable observations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#sources");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_core::manifest::ApplicationSourceDecl;
use vibe_core::progress::Progress;
use vibe_registry::{GitBackend, ShellGit, compute_portable_content_hash};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedApplicationSource {
    pub registry_root: PathBuf,
    pub url: String,
    pub tracked_ref: String,
    pub resolved_commit: String,
    pub source_tree: String,
}

pub fn resolve_remote_source(
    settings_root: &Path,
    declaration: &ApplicationSourceDecl,
    offline: bool,
    progress: &Progress,
) -> Result<ResolvedApplicationSource> {
    let sources = settings_root.join("applications").join("sources");
    fs::create_dir_all(&sources).context("creating application source cache")?;
    let sources = canonical_directory(&sources, "application source cache")?;
    let entry = sources.join(cache_key(&declaration.url, &declaration.tracked_ref));
    let git = ShellGit::new()
        .anonymized_for_public()
        .unwrap_or_else(|| std::sync::Arc::new(ShellGit::new()));
    if entry.join(".git").is_dir() {
        if !offline {
            let fetch = progress.task("Fetching tracked application source");
            fetch.detail(format!("tracked ref: {}", declaration.tracked_ref));
            match git
                .update(&entry, &declaration.tracked_ref)
                .context("refreshing tracked application source")
            {
                Ok(()) => fetch.finish(),
                Err(error) => {
                    fetch.fail("application source fetch failed");
                    return Err(error);
                }
            }
        } else {
            let cached = progress.task("Using cached application source");
            cached.skip("offline mode");
        }
    } else {
        if offline {
            bail!("--offline: tracked application source is not cached");
        }
        let pending = sources.join(format!(
            ".pending-{}-{}",
            std::process::id(),
            cache_key(&declaration.url, &declaration.tracked_ref)
        ));
        if pending.exists() {
            fs::remove_dir_all(&pending).context("removing incomplete application source")?;
        }
        let clone = progress.task("Cloning tracked application source");
        clone.detail(format!("tracked ref: {}", declaration.tracked_ref));
        if let Err(error) = git
            .bootstrap_embedded(&declaration.url, &declaration.tracked_ref, &pending)
            .context("cloning tracked application source")
        {
            clone.fail("application source clone failed");
            return Err(error);
        }
        fs::rename(&pending, &entry).context("publishing tracked application source")?;
        clone.finish();
    }
    if git.working_tree_dirty(&entry)? {
        bail!("cached application source is dirty; refusing mutable local bytes");
    }
    let inspect = progress.task("Inspecting resolved application source");
    let resolved_commit = git
        .head_commit(&entry)?
        .ok_or_else(|| anyhow::anyhow!("application source has no observed commit"))?;
    let source_tree =
        compute_portable_content_hash(&entry).context("hashing resolved application source")?;
    inspect.detail(format!(
        "resolved commit: {}",
        &resolved_commit[..12.min(resolved_commit.len())]
    ));
    inspect.finish();
    let candidate = entry.join(&declaration.registry_path);
    let registry_root = canonical_directory(&candidate, "external application registry")?;
    if !registry_root.starts_with(&entry) || registry_root == entry {
        bail!("external application registry escapes or equals its source root");
    }
    Ok(ResolvedApplicationSource {
        registry_root,
        url: declaration.url.clone(),
        tracked_ref: declaration.tracked_ref.clone(),
        resolved_commit,
        source_tree,
    })
}

fn cache_key(url: &str, tracked_ref: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-application-source/1\0");
    hash.update(url.as_bytes());
    hash.update(b"\0");
    hash.update(tracked_ref.as_bytes());
    format!("{:x}", hash.finalize())[..24].to_string()
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading {label} `{}`", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("{label} is not a real directory");
    }
    fs::canonicalize(path)
        .map(crate::commands::init::strip_unc_public)
        .with_context(|| format!("resolving {label} `{}`", path.display()))
}
