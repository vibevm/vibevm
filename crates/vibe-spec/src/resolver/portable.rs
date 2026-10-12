//! Physical source lookup through the shared portable component authority.
//! This is a read-side preflight, not a captured-byte or race-free read grant.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#PORTABLE-SOURCE-COMPONENTS");

use std::fs::{self, DirEntry};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use vibe_safefs::{ensure_no_follow_walk, identity_key, split_relative};

use super::ResolveError;

pub(super) fn split(path: &str) -> Result<(Vec<String>, String), ResolveError> {
    split_relative(path).map_err(|error| unsafe_path(path, error.to_string()))
}

pub(super) fn existing_directory(path: &Path) -> Result<bool, ResolveError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(metadata) if metadata.file_type().is_symlink() => Err(unsafe_path(
            &path.display().to_string(),
            "symlinked directory",
        )),
        Ok(_) => Err(unreadable(path, "not a directory")),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(unreadable(path, error.to_string())),
    }
}

/// Reach only exactly spelled, ordinary child directories. Compare siblings
/// before the next open so a case/NFC alias never silently chooses a host's
/// spelling. The selected root itself is supplied by the caller as trusted.
pub(super) fn directory(base: &Path, parents: &[String]) -> Result<Option<PathBuf>, ResolveError> {
    if !base.is_absolute() {
        return Err(unsafe_path(&base.display().to_string(), "root is relative"));
    }
    if !existing_directory(base)? {
        return Ok(None);
    }
    let mut current = base.to_path_buf();
    for part in parents {
        let entries = match entries(&current)? {
            Some(entries) => entries,
            None => return Err(disappeared(&current, "directory disappeared during lookup")),
        };
        let exact = exact_entry(&entries, part, &current)?;
        let Some(entry) = exact else {
            return Ok(None);
        };
        let next = entry.path();
        ensure_no_follow_walk(base, &next, false).map_err(|error| {
            match error.root_cause().downcast_ref::<std::io::Error>() {
                Some(io) if io.kind() == ErrorKind::NotFound => ResolveError::SourceUnstable {
                    path: next.display().to_string(),
                    reason: io.to_string(),
                },
                Some(io) => unreadable(&next, io.to_string()),
                None => unsafe_path(&next.display().to_string(), error.to_string()),
            }
        })?;
        let kind = entry
            .file_type()
            .map_err(|error| unreadable(&next, error.to_string()))?;
        if !kind.is_dir() {
            return Err(if kind.is_symlink() {
                unsafe_path(&next.display().to_string(), "symlinked directory")
            } else {
                unreadable(&next, "not a directory")
            });
        }
        current = next;
    }
    Ok(Some(current))
}

pub(super) fn entries(dir: &Path) -> Result<Option<Vec<DirEntry>>, ResolveError> {
    let iter = match fs::read_dir(dir) {
        Ok(iter) => iter,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unreadable(dir, error.to_string())),
    };
    let mut entries = Vec::new();
    for entry in iter {
        entries.push(entry.map_err(|error| unreadable(dir, error.to_string()))?);
    }
    Ok(Some(entries))
}

pub(super) fn exact_entry<'a>(
    entries: &'a [DirEntry],
    requested: &str,
    dir: &Path,
) -> Result<Option<&'a DirEntry>, ResolveError> {
    let mut exact = None;
    let key = identity_key(requested);
    for entry in entries {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if identity_key(name) == key {
            if name != requested {
                return Err(ResolveError::PortableSourceAlias {
                    requested: dir.join(requested).display().to_string(),
                    observed: entry.path().display().to_string(),
                });
            }
            exact = Some(entry);
        }
    }
    Ok(exact)
}

pub(super) fn checked_file(base: &Path, path: &Path) -> Result<bool, ResolveError> {
    let relative = path
        .strip_prefix(base)
        .map_err(|_| unsafe_path(&path.display().to_string(), "outside selected root"))?;
    let relative = relative
        .to_str()
        .ok_or_else(|| unsafe_path(&path.display().to_string(), "non-UTF-8 source name"))?;
    let (parents, name) = split(relative)?;
    let Some(dir) = directory(base, &parents)? else {
        return Ok(false);
    };
    let Some(entries) = entries(&dir)? else {
        return Err(disappeared(&dir, "directory disappeared before file check"));
    };
    let Some(entry) = exact_entry(&entries, &name, &dir)? else {
        return Ok(false);
    };
    let kind = entry
        .file_type()
        .map_err(|error| unreadable(path, error.to_string()))?;
    if kind.is_symlink() || !kind.is_file() {
        return Err(if kind.is_symlink() {
            unsafe_path(&path.display().to_string(), "symlinked source")
        } else {
            unreadable(path, "not a regular file")
        });
    }
    ensure_no_follow_walk(base, path, false).map_err(|error| {
        match error.root_cause().downcast_ref::<std::io::Error>() {
            Some(io) if io.kind() == ErrorKind::NotFound => ResolveError::SourceUnstable {
                path: path.display().to_string(),
                reason: io.to_string(),
            },
            Some(io) => unreadable(path, io.to_string()),
            None => unsafe_path(&path.display().to_string(), error.to_string()),
        }
    })?;
    Ok(true)
}

pub(super) fn unsafe_path(path: &str, reason: impl Into<String>) -> ResolveError {
    ResolveError::UnsafeSourcePath {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

pub(super) fn unreadable(path: &Path, reason: impl Into<String>) -> ResolveError {
    ResolveError::SourceUnreadable {
        path: path.display().to_string(),
        reason: reason.into(),
    }
}

pub(super) fn disappeared(path: &Path, reason: impl Into<String>) -> ResolveError {
    ResolveError::SourceUnstable {
        path: path.display().to_string(),
        reason: reason.into(),
    }
}
