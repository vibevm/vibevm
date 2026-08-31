//! Canonical provider-root containment for native source and prebuilt paths.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

const LOAD_IMAGE_DIR: [&str; 3] = [".vibe", "native-load", "e1"];
static LOAD_IMAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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

/// Publish one admitted source/prebuilt artifact as an immutable load image.
///
/// Images are derived cache, never ARTIFACT authority. Each lives below the
/// selected project's `.vibe/native-load/e1/<sha256>/` and is created without
/// replacement, so a process-global path-keyed loader can keep old handles
/// while rebuilt/replaced bytes receive a new canonical name.
pub(super) fn publish_load_image(
    selected_project_root: &Path,
    source: &Path,
    expected_digest: &str,
    expected_bytes: u64,
) -> Result<PathBuf, NativeArtifactError> {
    if !valid_lower_hex_64(expected_digest) {
        return Err(load_image_error(
            source,
            "artifact digest is not exactly 64 lowercase hex characters".to_owned(),
        ));
    }
    let source = source.canonicalize().map_err(|error| {
        load_image_error(source, format!("canonicalizing admitted artifact: {error}"))
    })?;
    let (actual_digest, actual_bytes) =
        digest_file(&source).map_err(|fault| load_image_error(&source, fault.reason()))?;
    if actual_digest != expected_digest || actual_bytes != expected_bytes {
        return Err(load_image_error(
            &source,
            "admitted artifact bytes changed before load-image publication".to_owned(),
        ));
    }
    let basename = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            load_image_error(&source, "artifact basename is not valid UTF-8".to_owned())
        })?;
    let root = selected_project_root.canonicalize().map_err(|error| {
        load_image_error(
            selected_project_root,
            format!("canonicalizing selected project root: {error}"),
        )
    })?;
    if !root.is_dir() {
        return Err(load_image_error(
            &root,
            "selected project root is not a directory".to_owned(),
        ));
    }
    let mut directory = root.clone();
    for component in LOAD_IMAGE_DIR.into_iter().chain([expected_digest]) {
        directory = ensure_load_directory(&root, &directory, component)?;
    }
    let destination = directory.join(basename);
    if std::fs::symlink_metadata(&destination).is_ok() {
        return verify_load_image(&root, &destination, expected_digest, expected_bytes);
    }

    let sequence = LOAD_IMAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = directory.join(format!(".pending-{}-{sequence}", std::process::id()));
    let published = publish_new_load_image(
        &source,
        &temporary,
        &destination,
        expected_digest,
        expected_bytes,
    );
    if let Err(error) = published {
        let _ = std::fs::remove_file(&temporary);
        if error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(load_image_error(
                &destination,
                format!("publishing immutable load image: {error}"),
            ));
        }
    }
    let _ = std::fs::remove_file(&temporary);
    verify_load_image(&root, &destination, expected_digest, expected_bytes)
}

fn ensure_load_directory(
    root: &Path,
    parent: &Path,
    component: &str,
) -> Result<PathBuf, NativeArtifactError> {
    let candidate = parent.join(component);
    match std::fs::create_dir(&candidate) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(load_image_error(
                &candidate,
                format!("creating cache directory: {error}"),
            ));
        }
    }
    let metadata = std::fs::symlink_metadata(&candidate)
        .map_err(|error| load_image_error(&candidate, error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(load_image_error(
            &candidate,
            "cache component is a link or not a directory".to_owned(),
        ));
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|error| load_image_error(&candidate, error.to_string()))?;
    if canonical.strip_prefix(root).is_err() {
        return Err(load_image_error(
            &candidate,
            "cache component escapes the selected project root".to_owned(),
        ));
    }
    Ok(canonical)
}

fn publish_new_load_image(
    source: &Path,
    temporary: &Path,
    destination: &Path,
    expected_digest: &str,
    expected_bytes: u64,
) -> std::io::Result<()> {
    let mut input = std::fs::File::open(source)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
    }
    output.sync_all()?;
    let (digest, bytes) =
        digest_file(temporary).map_err(|fault| std::io::Error::other(fault.reason()))?;
    if digest != expected_digest || bytes != expected_bytes {
        return Err(std::io::Error::other(
            "copied bytes do not match the admitted artifact",
        ));
    }
    std::fs::hard_link(temporary, destination)
}

fn verify_load_image(
    root: &Path,
    destination: &Path,
    expected_digest: &str,
    expected_bytes: u64,
) -> Result<PathBuf, NativeArtifactError> {
    let metadata = std::fs::symlink_metadata(destination)
        .map_err(|error| load_image_error(destination, error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(load_image_error(
            destination,
            "load image is a link or not a regular file".to_owned(),
        ));
    }
    let canonical = contained_regular(destination, root)
        .map_err(|reason| load_image_error(destination, reason))?;
    let (digest, bytes) =
        digest_file(&canonical).map_err(|fault| load_image_error(destination, fault.reason()))?;
    if digest != expected_digest || bytes != expected_bytes {
        return Err(load_image_error(
            destination,
            "existing load image bytes do not match the digest-addressed identity".to_owned(),
        ));
    }
    Ok(canonical)
}

fn load_image_error(path: &Path, reason: String) -> NativeArtifactError {
    NativeArtifactError::LoadImage {
        path: preview(&forward_slashed(path)),
        reason,
    }
}

fn valid_lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
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
