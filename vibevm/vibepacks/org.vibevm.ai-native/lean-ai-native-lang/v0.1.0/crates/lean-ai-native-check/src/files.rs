use crate::{Diagnostic, FileBinding};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn relative(path: &Path) -> Result<String, Diagnostic> {
    let text = path
        .to_str()
        .ok_or_else(|| Diagnostic::new("invalid_path", "path must be UTF-8"))?;
    if text.is_empty()
        || text.contains(['\\', ':'])
        || path.is_absolute()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
        || text
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(Diagnostic::new(
            "invalid_path",
            "expected a contained, normalized relative path",
        ));
    }
    Ok(text.to_owned())
}

pub(crate) fn read_relative(root: &Path, path: &Path) -> Result<Vec<u8>, Diagnostic> {
    relative(path)?;
    let mut joined = root.to_path_buf();
    for part in path.components() {
        joined.push(part);
        let metadata = fs::symlink_metadata(&joined)
            .map_err(|e| Diagnostic::new("input_unavailable", e.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(Diagnostic::new(
                "invalid_path",
                "input links are not supported",
            ));
        }
    }
    let before =
        fs::metadata(&joined).map_err(|e| Diagnostic::new("input_unavailable", e.to_string()))?;
    if !before.is_file() || before.len() > 4 * 1024 * 1024 {
        return Err(Diagnostic::new(
            "input_limit",
            "input must be a regular file of at most 4 MiB",
        ));
    }
    let first =
        fs::read(&joined).map_err(|e| Diagnostic::new("input_unavailable", e.to_string()))?;
    let second =
        fs::read(&joined).map_err(|e| Diagnostic::new("input_unavailable", e.to_string()))?;
    let after = fs::symlink_metadata(&joined)
        .map_err(|e| Diagnostic::new("input_unstable", e.to_string()))?;
    if first != second
        || before.len() != after.len()
        || after.file_type().is_symlink()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(Diagnostic::new(
            "input_unstable",
            "input moved or changed during capture",
        ));
    }
    Ok(first)
}

pub(crate) fn binding(path: &str, bytes: &[u8]) -> FileBinding {
    FileBinding {
        path: path.to_owned(),
        sha256: hash(bytes),
    }
}

/// Conservative dependency closure: every shipped bin/lib file, including
/// compiled libraries and native runtime, not merely the version string.
pub(crate) fn distribution_hash(root: &Path) -> Result<String, Diagnostic> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(String, String)>) -> Result<(), Diagnostic> {
        let meta = fs::symlink_metadata(path)
            .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
        let name = path
            .strip_prefix(root)
            .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        if meta.file_type().is_symlink() {
            let target = path
                .canonicalize()
                .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
            if !target.starts_with(root.join("bin")) && !target.starts_with(root.join("lib")) {
                return Err(Diagnostic::new(
                    "toolchain_unavailable",
                    "toolchain link leaves its bin/lib closure",
                ));
            }
            let spelling = fs::read_link(path)
                .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
            rows.push((name, format!("link:{}", spelling.to_string_lossy())));
        } else if meta.is_dir() {
            rows.push((name, "directory".into()));
            for entry in fs::read_dir(path)
                .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?
            {
                visit(
                    root,
                    &entry
                        .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?
                        .path(),
                    rows,
                )?;
            }
        } else if meta.is_file() {
            let mut file = fs::File::open(path)
                .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
            let mut h = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let n = file
                    .read(&mut buffer)
                    .map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
                if n == 0 {
                    break;
                }
                h.update(&buffer[..n]);
            }
            rows.push((name, format!("file:{:x}", h.finalize())));
        } else {
            return Err(Diagnostic::new(
                "toolchain_unavailable",
                "toolchain contains a non-regular entry",
            ));
        }
        Ok(())
    }
    let mut rows = Vec::new();
    visit(root, &root.join("bin"), &mut rows)?;
    visit(root, &root.join("lib"), &mut rows)?;
    rows.sort();
    let mut digest = Sha256::new();
    digest.update(b"lean-ai-native:distribution:v1\0");
    for (path, content) in rows {
        for item in [path, content] {
            digest.update((item.len() as u64).to_be_bytes());
            digest.update(item.as_bytes());
        }
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}
