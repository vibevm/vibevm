use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vibe_publish::release_manifest::{
    DISTRIBUTION_SOURCE_MAX_DEPTH, DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES,
    DISTRIBUTION_SOURCE_MAX_PATH_BYTES,
};

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn atomic_replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state");
    let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(".{name}.{}-{nonce}.tmp", std::process::id()));
    atomic_replace_at(path, &temp, bytes)
}

pub(super) fn atomic_replace_at(path: &Path, temp: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut owned = false;
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp)?;
        owned = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temp, path)
    })();
    if result.is_err() && owned {
        let _ = fs::remove_file(temp);
    }
    result
}

pub(crate) fn open_regular_no_follow(path: &Path) -> io::Result<(fs::File, fs::Metadata)> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "linux")]
        options.custom_flags(0x2_0000);
        #[cfg(target_os = "macos")]
        options.custom_flags(0x100);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    reject_link_or_special(path, &metadata)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` is not a regular file", path.display()),
        ));
    }
    Ok((file, metadata))
}

pub(super) fn read_bounded_utf8(path: &Path, maximum: u64) -> io::Result<Option<String>> {
    let (mut file, metadata) = match open_regular_no_follow(path) {
        Ok(opened) => opened,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if metadata.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file exceeds its {maximum}-byte bound"),
        ));
    }
    let mut text = String::with_capacity(metadata.len() as usize);
    file.read_to_string(&mut text)?;
    if text.len() as u64 != metadata.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "file changed size while reading",
        ));
    }
    Ok(Some(text))
}

pub(super) fn preflight_file_leaf(root: &Path, path: &Path) -> io::Result<()> {
    guard_mutation_path(root, path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "destination leaf is not a regular file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

pub(super) fn lexical_path_eq(left: &Path, right: &Path) -> bool {
    if !absolute_normal_path(left) || !absolute_normal_path(right) {
        return false;
    }
    if cfg!(windows) {
        left.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&right.to_string_lossy().replace('/', "\\"))
    } else {
        left == right
    }
}

pub(super) fn absolute_normal_path(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let Some(raw) = path.to_str() else {
        return false;
    };
    let relative = raw.trim_start_matches(['/', '\\']);
    !relative.is_empty()
        && !relative
            .split(['/', '\\'])
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        && path
            .components()
            .all(|component| !matches!(component, Component::CurDir | Component::ParentDir))
}

pub(super) fn guard_existing_tree(path: &Path) -> io::Result<()> {
    const RUNTIME_ENTRY_OVERHEAD: u64 = 1024;
    const RUNTIME_DEPTH_OVERHEAD: usize = 16;
    const RUNTIME_PATH_OVERHEAD: usize = 1024;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    reject_link_or_special(path, &metadata)?;
    if !metadata.file_type().is_dir() {
        return Ok(());
    }
    let mut pending = vec![(path.to_path_buf(), 0_usize)];
    let mut visited = 0_u64;
    while let Some((directory, depth)) = pending.pop() {
        if depth > DISTRIBUTION_SOURCE_MAX_DEPTH + RUNTIME_DEPTH_OVERHEAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "mutation tree exceeds the maximum directory depth",
            ));
        }
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            visited += 1;
            if visited > DISTRIBUTION_SOURCE_MAX_MATERIALIZED_ENTRIES + RUNTIME_ENTRY_OVERHEAD {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mutation tree exceeds the maximum entry count",
                ));
            }
            let child = entry.path();
            let relative_bytes = child
                .strip_prefix(path)
                .unwrap_or(&child)
                .to_string_lossy()
                .len();
            if relative_bytes > DISTRIBUTION_SOURCE_MAX_PATH_BYTES + RUNTIME_PATH_OVERHEAD {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mutation tree exceeds the maximum relative path length",
                ));
            }
            let metadata = fs::symlink_metadata(&child)?;
            reject_link_or_special(&child, &metadata)?;
            if metadata.file_type().is_dir() {
                pending.push((child, depth + 1));
            }
        }
    }
    Ok(())
}

pub(super) fn guard_mutation_path(root: &Path, target: &Path) -> io::Result<()> {
    let lexical_relative = target.strip_prefix(root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("target is outside store root `{}`", root.display()),
        )
    })?;
    if !normal_relative(lexical_relative) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target has a non-normal relative path component",
        ));
    }
    let root = std::path::absolute(root)?;
    let target = std::path::absolute(target)?;
    let relative = target.strip_prefix(&root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("target is outside store root `{}`", root.display()),
        )
    })?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target has a non-normal relative path component",
        ));
    }

    let canonical_root = ensure_trusted_root(&root)?;
    let mut cursor = root.clone();
    let components = relative.components().collect::<Vec<_>>();
    let mut nearest_existing = root;
    for (index, component) in components.iter().enumerate() {
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) => {
                reject_link_or_special(&cursor, &metadata)?;
                if index + 1 < components.len() && !metadata.file_type().is_dir() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotADirectory,
                        format!("ancestor `{}` is not a directory", cursor.display()),
                    ));
                }
                nearest_existing = cursor.clone();
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => return Err(error),
        }
    }
    let resolved = nearest_existing.canonicalize()?;
    if !resolved.starts_with(&canonical_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "nearest existing ancestor `{}` resolves outside trusted root `{}`",
                resolved.display(),
                canonical_root.display()
            ),
        ));
    }
    Ok(())
}

fn normal_relative(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return true;
    }
    let Some(raw) = path.to_str() else {
        return false;
    };
    !raw.split(['/', '\\'])
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn ensure_trusted_root(root: &Path) -> io::Result<PathBuf> {
    let mut cursor = root.to_path_buf();
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) => {
                reject_link_or_special(&cursor, &metadata)?;
                if !metadata.file_type().is_dir() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotADirectory,
                        format!(
                            "trusted root ancestor `{}` is not a directory",
                            cursor.display()
                        ),
                    ));
                }
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let name = cursor.file_name().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "no existing root ancestor")
                })?;
                missing.push(name.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::NotFound, "no existing root ancestor")
                    })?
                    .to_path_buf();
            }
            Err(error) => return Err(error),
        }
    }

    for component in missing.into_iter().rev() {
        cursor.push(component);
        match fs::create_dir(&cursor) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let metadata = fs::symlink_metadata(&cursor)?;
        reject_link_or_special(&cursor, &metadata)?;
        if !metadata.file_type().is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!(
                    "created root component `{}` is not a directory",
                    cursor.display()
                ),
            ));
        }
    }

    let metadata = fs::symlink_metadata(root)?;
    reject_link_or_special(root, &metadata)?;
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("store root `{}` is not a directory", root.display()),
        ));
    }
    root.canonicalize()
}

fn reject_link_or_special(path: &Path, metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.file_type().is_symlink() || is_windows_reparse(metadata) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("`{}` is a symlink or reparse point", path.display()),
        ));
    }
    if !metadata.file_type().is_file() && !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` is a special filesystem object", path.display()),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_windows_reparse(_metadata: &fs::Metadata) -> bool {
    false
}
