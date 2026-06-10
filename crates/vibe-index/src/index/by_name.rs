//! `by-name/<name>.json` — the candidate-set file for one bare package
//! name (PROP-008 §2.8). A single HTTP GET fetches every `(group, name)`
//! package that shares the short name `<name>`, each with all its
//! versions. This is the layout that makes CLI short-name resolution
//! (PROP-008 §2.6) one round-trip per registry: the consumer reads the
//! whole candidate set at once and either resolves it (one candidate) or
//! reports a collision (more than one, PROP-008 §2.7).
//!
//! Before PROP-008 the layer keyed on the package's own `kind`
//! (`by-name/<kind>/<name>.json`). `kind` left package identity, so the
//! directory level is gone — `<name>` alone is the key.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#short-name");

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::index::persistence::{atomic_write, sha256_of_bytes};
use crate::types::NameEntry;

pub const DIRNAME: &str = "by-name";

pub fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join(DIRNAME)
}

pub fn file_path(data_dir: &Path, name: &str) -> PathBuf {
    dir(data_dir).join(format!("{name}.json"))
}

/// Serialise `entry` to pretty-printed JSON bytes (with trailing newline).
pub fn serialise(entry: &NameEntry) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(entry).map_err(|e| {
        Error::Malformed(format!(
            "could not serialise by-name entry for `{}` — {e}",
            entry.name
        ))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write(data_dir: &Path, entry: &NameEntry) -> Result<WrittenFile> {
    let bytes = serialise(entry)?;
    let path = file_path(data_dir, &entry.name);
    atomic_write(&path, &bytes)?;
    Ok(WrittenFile {
        relative_path: format!("{DIRNAME}/{}.json", entry.name),
        size: bytes.len() as u64,
        sha256: sha256_of_bytes(&bytes),
    })
}

pub fn read(data_dir: &Path, name: &str) -> Result<NameEntry> {
    let path = file_path(data_dir, name);
    let bytes = fs::read(&path).map_err(|e| Error::Io {
        path: path.clone(),
        message: e.to_string(),
    })?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<NameEntry> {
    serde_json::from_slice(bytes).map_err(|e| Error::Malformed(format!("by-name JSON: {e}")))
}

/// Walk `<data-dir>/by-name/` and return every name's candidate set.
/// Used by the load + verify paths.
pub fn read_all(data_dir: &Path) -> Result<Vec<NameEntry>> {
    let root = dir(data_dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(&root)
        .max_depth(1)
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

/// Count of `by-name/*.json` files — one per distinct bare name.
pub fn entry_count(data_dir: &Path) -> u32 {
    let root = dir(data_dir);
    if !root.is_dir() {
        return 0;
    }
    WalkDir::new(&root)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("json")
        })
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
    use crate::types::{PackageEntry, PackageKind, VersionEntry};
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;
    use vibe_core::Group;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    fn version_entry(group: Group, name: &str, version: &str) -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind: PackageKind::Flow,
            group,
            name: name.into(),
            version: version.parse().unwrap(),
            content_hash: "sha256:0".into(),
            source_url: "https://example.invalid/x.git".into(),
            source_ref: format!("v{version}"),
            resolved_commit: None,
            registry: "vibespecs".into(),
            workspace_origin: None,
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
            subskills: vec![],
            i18n: Default::default(),
            boot_snippet: None,
            files_count: 1,
            indexed_at: now(),
            indexed_by: "vibe-index 0.1.0-dev".into(),
        }
    }

    fn package(group: Group, name: &str, versions: &[&str]) -> PackageEntry {
        let mut pkg = PackageEntry::new(group.clone(), name, now());
        for v in versions {
            pkg.versions.push(version_entry(group.clone(), name, v));
        }
        pkg.finalise();
        pkg
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = tempdir().unwrap();
        let mut ne = NameEntry::new("wal", now());
        ne.packages.push(package(org(), "wal", &["0.1.0", "0.2.0"]));
        ne.finalise();
        let written = write(dir.path(), &ne).unwrap();
        assert_eq!(written.relative_path, "by-name/wal.json");
        let back = read(dir.path(), "wal").unwrap();
        assert_eq!(back.packages.len(), 1);
        assert_eq!(back.packages[0].versions.len(), 2);
        assert_eq!(
            back.packages[0].latest_stable.as_ref().unwrap().to_string(),
            "0.2.0"
        );
    }

    #[test]
    fn candidate_set_holds_multiple_groups() {
        // A short-name collision: two groups publish a package called
        // `wal`. The by-name file is the candidate set for both.
        let dir = tempdir().unwrap();
        let mut ne = NameEntry::new("wal", now());
        ne.packages.push(package(org(), "wal", &["0.1.0"]));
        ne.packages.push(package(
            Group::parse("com.acme").unwrap(),
            "wal",
            &["1.0.0"],
        ));
        ne.finalise();
        write(dir.path(), &ne).unwrap();
        let back = read(dir.path(), "wal").unwrap();
        assert_eq!(back.packages.len(), 2);
        // finalise sorts by group: com.acme before org.vibevm.
        assert_eq!(back.packages[0].group.as_str(), "com.acme");
        assert_eq!(back.packages[1].group.as_str(), "org.vibevm");
    }

    #[test]
    fn read_all_collects_every_name() {
        let dir = tempdir().unwrap();
        let mut wal = NameEntry::new("wal", now());
        wal.packages.push(package(org(), "wal", &["0.1.0"]));
        let mut commits = NameEntry::new("atomic-commits", now());
        commits
            .packages
            .push(package(org(), "atomic-commits", &["0.1.0"]));
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
