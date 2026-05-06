//! `by-name/<kind>/<name>.json` — cargo-sparse-style per-package
//! aggregate file. One HTTP GET fetches every version of one package.

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::index::persistence::{atomic_write, sha256_of_bytes};
use crate::types::{PackageEntry, PackageKind};

pub const DIRNAME: &str = "by-name";

pub fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join(DIRNAME)
}

pub fn file_path(data_dir: &Path, kind: PackageKind, name: &str) -> PathBuf {
    dir(data_dir).join(kind.as_str()).join(format!("{name}.json"))
}

/// Serialise `entry` to pretty-printed JSON bytes (with trailing newline).
pub fn serialise(entry: &PackageEntry) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(entry).map_err(|e| {
        Error::Malformed(format!(
            "could not serialise by-name entry {}:{} — {e}",
            entry.kind, entry.name
        ))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write(data_dir: &Path, entry: &PackageEntry) -> Result<WrittenFile> {
    let bytes = serialise(entry)?;
    let path = file_path(data_dir, entry.kind, &entry.name);
    atomic_write(&path, &bytes)?;
    Ok(WrittenFile {
        relative_path: format!("{DIRNAME}/{}/{}.json", entry.kind, entry.name),
        size: bytes.len() as u64,
        sha256: sha256_of_bytes(&bytes),
    })
}

pub fn read(data_dir: &Path, kind: PackageKind, name: &str) -> Result<PackageEntry> {
    let path = file_path(data_dir, kind, name);
    let bytes = fs::read(&path).map_err(|e| Error::Io {
        path: path.clone(),
        message: e.to_string(),
    })?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<PackageEntry> {
    serde_json::from_slice(bytes).map_err(|e| Error::Malformed(format!("by-name JSON: {e}")))
}

/// Walk `<data-dir>/by-name/` and return every package entry currently
/// on disk. Used by load + verify paths.
pub fn read_all(data_dir: &Path) -> Result<Vec<PackageEntry>> {
    let root = dir(data_dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(&root)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        out.push(parse(&bytes)?);
    }
    Ok(out)
}

pub fn entry_count(data_dir: &Path) -> u32 {
    let root = dir(data_dir);
    if !root.is_dir() {
        return 0;
    }
    WalkDir::new(&root)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .count() as u32
}

pub struct WrittenFile {
    pub relative_path: String,
    pub size: u64,
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeliveryMode, PackageKind, SubskillEntry, VersionEntry};
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn version(name: &str, version: &str) -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind: PackageKind::Flow,
            name: name.into(),
            version: version.parse().unwrap(),
            content_hash: "sha256:0".into(),
            source_url: "https://example.invalid/x.git".into(),
            source_ref: format!("v{version}"),
            resolved_commit: None,
            registry: "vibespecs".into(),
            license: None,
            authors: vec![],
            description: None,
            homepage: None,
            keywords: vec![],
            describes: None,
            compatibility: Default::default(),
            provides: Default::default(),
            requires: Default::default(),
            requires_any: vec![],
            obsoletes: Default::default(),
            conflicts: Default::default(),
            features: Default::default(),
            subskills: vec![SubskillEntry {
                path: "feature/x".into(),
                delivery: DeliveryMode::Eager,
                describes: None,
                description: Some("a subskill".into()),
                channels: vec!["manual".into()],
            }],
            i18n: Default::default(),
            boot_snippet: None,
            files_count: 1,
            indexed_at: now(),
            indexed_by: "vibe-index 0.1.0-dev".into(),
        }
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = tempdir().unwrap();
        let mut entry = PackageEntry::new(PackageKind::Flow, "wal", now());
        entry.versions.push(version("wal", "0.1.0"));
        entry.versions.push(version("wal", "0.2.0"));
        entry.finalise();
        let written = write(dir.path(), &entry).unwrap();
        assert_eq!(written.relative_path, "by-name/flow/wal.json");
        let back = read(dir.path(), PackageKind::Flow, "wal").unwrap();
        assert_eq!(back.versions.len(), 2);
        assert_eq!(back.latest_stable.unwrap().to_string(), "0.2.0");
    }

    #[test]
    fn read_all_collects_every_kind() {
        let dir = tempdir().unwrap();
        let mut wal = PackageEntry::new(PackageKind::Flow, "wal", now());
        wal.versions.push(version("wal", "0.1.0"));
        wal.finalise();
        let mut commits = PackageEntry::new(PackageKind::Flow, "atomic-commits", now());
        commits.versions.push(version("atomic-commits", "0.1.0"));
        commits.finalise();
        write(dir.path(), &wal).unwrap();
        write(dir.path(), &commits).unwrap();
        let all = read_all(dir.path()).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(entry_count(dir.path()), 2);
    }

    #[test]
    fn read_all_returns_empty_when_dir_missing() {
        let dir = tempdir().unwrap();
        let all = read_all(dir.path()).unwrap();
        assert!(all.is_empty());
        assert_eq!(entry_count(dir.path()), 0);
    }
}
