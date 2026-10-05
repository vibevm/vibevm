use super::wire::{Diagnostic, Result};
use lean_ai_native_check::FileBinding;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path},
};

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
pub(super) fn binding(path: &str, bytes: &[u8]) -> FileBinding {
    FileBinding {
        path: path.into(),
        sha256: hash(bytes),
    }
}
pub(super) fn relative(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .ok_or_else(|| Diagnostic::new("invalid_path", "non-UTF8 path"))?;
    if text.is_empty()
        || text.contains(['\\', ':'])
        || path.is_absolute()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
        || text
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(Diagnostic::new(
            "invalid_path",
            "expected a normalized relative path",
        ));
    }
    Ok(text.into())
}
pub(super) fn read(root: &Path, path: &Path) -> Result<Vec<u8>> {
    relative(path)?;
    let mut full = root.to_path_buf();
    for part in path.components() {
        full.push(part);
        if fs::symlink_metadata(&full)
            .map_err(input)?
            .file_type()
            .is_symlink()
        {
            return Err(Diagnostic::new(
                "invalid_path",
                "input links are unsupported",
            ));
        }
    }
    let meta = fs::metadata(&full).map_err(input)?;
    if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
        return Err(Diagnostic::new(
            "input_limit",
            "input must be a regular file at most 4 MiB",
        ));
    }
    let first = fs::read(&full).map_err(input)?;
    if first != fs::read(&full).map_err(input)? {
        return Err(Diagnostic::new(
            "inputs_unstable",
            "input changed while captured",
        ));
    }
    Ok(first)
}
pub(super) fn input(error: impl ToString) -> Diagnostic {
    Diagnostic::new("input_unavailable", error.to_string())
}

#[derive(Default)]
pub(super) struct Capture(pub BTreeMap<String, Vec<u8>>);
impl Capture {
    pub fn add(&mut self, root: &Path, path: &str) -> Result<Vec<u8>> {
        let bytes = read(root, Path::new(path))?;
        if let Some(old) = self.0.insert(path.into(), bytes.clone()) {
            if old != bytes {
                return Err(Diagnostic::new(
                    "inputs_unstable",
                    "input changed between captures",
                ));
            }
        }
        Ok(bytes)
    }
    pub fn recheck(&self, root: &Path) -> Result<()> {
        for (path, bytes) in &self.0 {
            if read(root, Path::new(path))? != *bytes {
                return Err(Diagnostic::new(
                    "inputs_unstable",
                    "input changed during fresh checks",
                ));
            }
        }
        Ok(())
    }
    pub fn materialize(&self, root: &Path) -> Result<()> {
        for (path, bytes) in &self.0 {
            let full = root.join(path);
            fs::create_dir_all(full.parent().expect("contained file")).map_err(input)?;
            fs::write(full, bytes).map_err(input)?;
        }
        Ok(())
    }
}

/// All parents must already exist and be real directories. Output is one
/// atomic replacement, never an incremental authoritative JSON write.
pub(super) fn atomic_write(root: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    relative(path)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new(""));
    let mut dir = root.to_path_buf();
    for part in parent.components() {
        dir.push(part);
        let meta = fs::symlink_metadata(&dir).map_err(input)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(Diagnostic::new(
                "invalid_path",
                "output parent must be an existing real directory",
            ));
        }
    }
    let destination = root.join(path);
    if let Ok(meta) = fs::symlink_metadata(&destination) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(Diagnostic::new(
                "invalid_path",
                "output must be a regular file",
            ));
        }
    }
    let mut file = tempfile::NamedTempFile::new_in(&dir).map_err(input)?;
    file.write_all(bytes).map_err(input)?;
    file.as_file().sync_all().map_err(input)?;
    file.persist(destination).map_err(|e| input(e.error))?;
    Ok(())
}

pub(super) fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(input)?;
    let mut digest = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let count = file.read(&mut buf).map_err(input)?;
        if count == 0 {
            break;
        }
        digest.update(&buf[..count]);
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}

pub(super) fn distribution_hash(root: &Path) -> Result<String> {
    fn visit(root: &Path, path: &Path, rows: &mut BTreeMap<String, String>) -> Result<()> {
        let meta = fs::symlink_metadata(path).map_err(input)?;
        let name = path
            .strip_prefix(root)
            .map_err(input)?
            .to_string_lossy()
            .replace('\\', "/");
        if meta.file_type().is_symlink() {
            let target = path.canonicalize().map_err(input)?;
            if !target.starts_with(root) {
                return Err(Diagnostic::new(
                    "rust_toolchain_unavailable",
                    "toolchain link leaves its closure",
                ));
            }
            rows.insert(
                name,
                format!("link:{}", fs::read_link(path).map_err(input)?.display()),
            );
        } else if meta.is_dir() {
            rows.insert(name, "directory".into());
            for entry in fs::read_dir(path).map_err(input)? {
                visit(root, &entry.map_err(input)?.path(), rows)?;
            }
        } else if meta.is_file() {
            rows.insert(name, file_hash(path)?);
        } else {
            return Err(Diagnostic::new(
                "rust_toolchain_unavailable",
                "non-regular toolchain entry",
            ));
        }
        Ok(())
    }
    let mut rows = BTreeMap::new();
    for dir in ["bin", "lib"] {
        visit(root, &root.join(dir), &mut rows)?;
    }
    Ok(hash(&serde_json::to_vec(&rows).map_err(input)?))
}
