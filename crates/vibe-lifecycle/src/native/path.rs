//! Canonical provider-root containment for native source and prebuilt paths.

use std::path::{Component, Path, PathBuf};

use crate::mechanism::contain::{digest_file, forward_slashed, prove_regular_file, relative_to};
use crate::mechanism::error::preview;

use super::provider::ProviderFacts;
use super::{NativeArtifactError, NativePlatform};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerifiedFile {
    pub(super) absolute: PathBuf,
    pub(super) relative: String,
    pub(super) digest: String,
    pub(super) bytes: u64,
}

pub(super) fn canonical_provider_root(
    provider: &ProviderFacts,
) -> Result<PathBuf, NativeArtifactError> {
    let root =
        provider
            .root()
            .canonicalize()
            .map_err(|error| NativeArtifactError::ProviderRoot {
                provider: provider.identity.clone(),
                path: preview(&forward_slashed(provider.root())),
                reason: error.to_string(),
            })?;
    let metadata = std::fs::metadata(&root).map_err(|error| NativeArtifactError::ProviderRoot {
        provider: provider.identity.clone(),
        path: preview(&forward_slashed(&root)),
        reason: error.to_string(),
    })?;
    if !metadata.is_dir() {
        return Err(NativeArtifactError::ProviderRoot {
            provider: provider.identity.clone(),
            path: preview(&forward_slashed(&root)),
            reason: "not a directory".to_owned(),
        });
    }
    Ok(root)
}

pub(super) fn source_crate(
    provider: &ProviderFacts,
    crate_dir: &Path,
) -> Result<(PathBuf, PathBuf), NativeArtifactError> {
    let root = canonical_provider_root(provider)?;
    let relative =
        relative_spelling(crate_dir).map_err(|reason| NativeArtifactError::CrateDirectory {
            provider: provider.identity.clone(),
            crate_dir: preview(&forward_slashed(crate_dir)),
            reason,
        })?;
    let candidate = join_components(&root, &relative);
    let canonical =
        candidate
            .canonicalize()
            .map_err(|error| NativeArtifactError::CrateDirectory {
                provider: provider.identity.clone(),
                crate_dir: preview(&relative),
                reason: error.to_string(),
            })?;
    if canonical.strip_prefix(&root).is_err() {
        return Err(NativeArtifactError::CrateDirectory {
            provider: provider.identity.clone(),
            crate_dir: preview(&relative),
            reason: "canonical path escapes the provider root".to_owned(),
        });
    }
    if !canonical.is_dir() {
        return Err(NativeArtifactError::CrateDirectory {
            provider: provider.identity.clone(),
            crate_dir: preview(&relative),
            reason: "not a directory".to_owned(),
        });
    }
    let manifest = canonical.join("Cargo.toml");
    prove_regular_file(&manifest).map_err(|fault| NativeArtifactError::CrateDirectory {
        provider: provider.identity.clone(),
        crate_dir: preview(&relative),
        reason: format!("Cargo.toml is unavailable: {}", fault.reason()),
    })?;
    Ok((root, manifest))
}

pub(super) fn prebuilt_file(
    extension: &str,
    provider: &ProviderFacts,
    authored: &Path,
    platform: NativePlatform,
) -> Result<VerifiedFile, NativeArtifactError> {
    let path_display = forward_slashed(authored);
    if !path_display.ends_with(platform.suffix()) {
        return Err(NativeArtifactError::PrebuiltSuffix {
            extension: extension.to_owned(),
            platform: platform.key().to_owned(),
            path: preview(&path_display),
            suffix: platform.suffix().to_owned(),
        });
    }
    let root = canonical_provider_root(provider)?;
    let relative =
        relative_spelling(authored).map_err(|reason| NativeArtifactError::PrebuiltUnavailable {
            extension: extension.to_owned(),
            platform: platform.key().to_owned(),
            path: preview(&path_display),
            reason,
        })?;
    let candidate = join_components(&root, &relative);
    let canonical = contained_regular(&candidate, &root).map_err(|reason| {
        NativeArtifactError::PrebuiltUnavailable {
            extension: extension.to_owned(),
            platform: platform.key().to_owned(),
            path: preview(&path_display),
            reason,
        }
    })?;
    digest_verified(canonical, &root).map_err(|reason| NativeArtifactError::PrebuiltUnavailable {
        extension: extension.to_owned(),
        platform: platform.key().to_owned(),
        path: preview(&path_display),
        reason,
    })
}

pub(super) fn cargo_reported_file(
    provider: &ProviderFacts,
    reported: &Path,
    target_root: &Path,
) -> Result<VerifiedFile, NativeArtifactError> {
    let canonical_root =
        target_root
            .canonicalize()
            .map_err(|error| NativeArtifactError::ReportedArtifact {
                provider: provider.identity.clone(),
                path: preview(&forward_slashed(target_root)),
                reason: error.to_string(),
            })?;
    let canonical = contained_regular(reported, &canonical_root).map_err(|reason| {
        NativeArtifactError::ReportedArtifact {
            provider: provider.identity.clone(),
            path: preview(&forward_slashed(reported)),
            reason,
        }
    })?;
    let provider_root = canonical_provider_root(provider)?;
    digest_verified(canonical, &provider_root).map_err(|reason| {
        NativeArtifactError::ReportedArtifact {
            provider: provider.identity.clone(),
            path: preview(&forward_slashed(reported)),
            reason,
        }
    })
}

pub(super) fn recorded_file(
    provider: &ProviderFacts,
    relative: &str,
    platform: NativePlatform,
) -> Result<VerifiedFile, String> {
    if !relative.ends_with(platform.suffix()) {
        return Err(format!(
            "recorded relative path does not have the exact `{}` suffix",
            platform.suffix()
        ));
    }
    let relative = relative_spelling(Path::new(relative))?;
    let root = canonical_provider_root(provider).map_err(|error| error.to_string())?;
    let candidate = join_components(&root, &relative);
    let canonical = contained_regular(&candidate, &root)?;
    digest_verified(canonical, &root)
}

pub(super) fn contained_regular(candidate: &Path, root: &Path) -> Result<PathBuf, String> {
    prove_regular_file(candidate).map_err(|fault| fault.reason())?;
    let canonical = candidate
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if canonical.strip_prefix(root).is_err() {
        return Err("canonical path escapes its owning root".to_owned());
    }
    Ok(canonical)
}

fn digest_verified(path: PathBuf, root: &Path) -> Result<VerifiedFile, String> {
    let relative = relative_to(&path, root)
        .ok_or_else(|| "canonical file has no contained relative identity".to_owned())?;
    let (digest, bytes) = digest_file(&path).map_err(|fault| fault.reason())?;
    Ok(VerifiedFile {
        absolute: path,
        relative,
        digest,
        bytes,
    })
}

pub(super) fn relative_spelling(path: &Path) -> Result<String, String> {
    if path.as_os_str().is_empty() {
        return Err("path names nothing".to_owned());
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| "path is not valid UTF-8".to_owned())?;
                if part.is_empty() {
                    return Err("path contains an empty component".to_owned());
                }
                parts.push(part);
            }
            Component::CurDir => return Err("path contains a `.` component".to_owned()),
            Component::ParentDir => return Err("path contains a `..` component".to_owned()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("path is absolute or drive-prefixed".to_owned());
            }
        }
    }
    if parts.is_empty() {
        return Err("path names nothing".to_owned());
    }
    Ok(parts.join("/"))
}

pub(super) fn join_components(root: &Path, relative: &str) -> PathBuf {
    let mut joined = root.to_path_buf();
    for component in relative.split('/') {
        joined.push(component);
    }
    joined
}
