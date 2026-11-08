//! Existing payload copying and removal honor the operation cancellation token.
use super::*;

pub(super) fn copy_dir(src: &Path, dst: &Path) -> Result<(), PublishError> {
    copy_dir_controlled(
        src,
        dst,
        &vibe_core::cancellation::CancellationToken::default(),
    )
}
pub(super) fn copy_dir_controlled(
    src: &Path,
    dst: &Path,
    cancellation: &vibe_core::cancellation::CancellationToken,
) -> Result<(), PublishError> {
    cancellation
        .checkpoint()
        .map_err(|error| PublishError::Git(error.to_string()))?;
    std::fs::create_dir_all(dst).map_err(|e| PublishError::Io {
        path: dst.to_path_buf(),
        message: format!("create_dir_all: {e}"),
    })?;
    for entry in walk_controlled(src, cancellation)? {
        cancellation
            .checkpoint()
            .map_err(|error| PublishError::Git(error.to_string()))?;
        let path = entry;
        let rel = path
            .strip_prefix(src)
            .map_err(|_| PublishError::Io {
                path: path.clone(),
                message: format!("walked path escaped its copy root `{}`", src.display()),
            })?
            .to_path_buf();
        if rel.components().any(|c| {
            c.as_os_str() == std::ffi::OsStr::new(".git")
                || c.as_os_str() == std::ffi::OsStr::new(".gitmodules")
        }) {
            continue;
        }
        let target = dst.join(&rel);
        if path.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| PublishError::Io {
                path: target.clone(),
                message: format!("create_dir_all: {e}"),
            })?;
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| PublishError::Io {
                    path: parent.to_path_buf(),
                    message: format!("create_dir_all: {e}"),
                })?;
            }
            process::copy_controlled(&path, &target, cancellation)?;
        }
    }
    Ok(())
}

pub(super) fn walk_controlled(
    root: &Path,
    cancellation: &vibe_core::cancellation::CancellationToken,
) -> Result<Vec<std::path::PathBuf>, PublishError> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        cancellation
            .checkpoint()
            .map_err(|error| PublishError::Git(error.to_string()))?;
        if p.is_dir() {
            let entries = std::fs::read_dir(&p).map_err(|e| PublishError::Io {
                path: p.clone(),
                message: format!("read_dir: {e}"),
            })?;
            for entry in entries {
                let entry = entry.map_err(|e| PublishError::Io {
                    path: p.clone(),
                    message: format!("read_dir entry: {e}"),
                })?;
                let path = entry.path();
                stack.push(path.clone());
                if path.is_file() {
                    out.push(path);
                }
            }
        } else if p.is_file() {
            out.push(p);
        }
    }
    Ok(out)
}

pub(super) fn clear_payload_tree(staging_path: &Path) -> Result<(), PublishError> {
    // This helper is intentionally destructive only inside the freshly
    // initialised temp repository. Requiring its `.git` sentinel makes an
    // accidental call against a source/package directory fail closed.
    if !staging_path.join(".git").is_dir() {
        return Err(PublishError::Git(format!(
            "refusing to clear publish staging directory without `.git`: {}",
            staging_path.display()
        )));
    }
    let entries = std::fs::read_dir(staging_path).map_err(|e| PublishError::Io {
        path: staging_path.to_path_buf(),
        message: format!("reading publish staging directory: {e}"),
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| PublishError::Io {
            path: staging_path.to_path_buf(),
            message: format!("reading publish staging entry: {e}"),
        })?;
        if entry.file_name() == std::ffi::OsStr::new(".git") {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| PublishError::Io {
            path: path.clone(),
            message: format!("reading file type: {e}"),
        })?;
        let result = if file_type.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        result.map_err(|e| PublishError::Io {
            path,
            message: format!("clearing previous release payload: {e}"),
        })?;
    }
    Ok(())
}
