//! `primary.jsonl` — JSON Lines, one [`VersionEntry`] per line, sorted
//! by `(group, name, version)`. Plus `primary.jsonl.gz` — gzip
//! sibling for bandwidth-conscious consumers (PROP-005 §2.4). The
//! gzip output is deterministic (level 6, `mtime=0`, no filename in
//! the header) so its sha256 stays stable across machines for
//! identical input.

use std::io::Write;
use std::path::Path;

use flate2::Compression;
use flate2::write::GzEncoder;

use crate::error::{Error, Result};
use crate::index::persistence::{atomic_write, sha256_of_bytes};
use crate::types::VersionEntry;

pub const FILENAME: &str = "primary.jsonl";
pub const FILENAME_GZ: &str = "primary.jsonl.gz";

/// Serialise `entries` to JSONL bytes (newline-terminated, sorted).
/// `entries` are sorted in place by [`VersionEntry::sort_key`] so the
/// caller can pass any iteration order and the on-disk shape stays
/// deterministic.
pub fn serialise(entries: &mut [VersionEntry]) -> Result<Vec<u8>> {
    entries.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    let mut out = Vec::new();
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(|e| {
            Error::Malformed(format!(
                "could not serialise entry {}:{}@{} — {e}",
                entry.kind, entry.name, entry.version
            ))
        })?;
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

pub fn write(dir: &Path, entries: &mut [VersionEntry]) -> Result<(WrittenFile, WrittenFile)> {
    let bytes = serialise(entries)?;
    let path = dir.join(FILENAME);
    atomic_write(&path, &bytes)?;
    let plain = WrittenFile {
        size: bytes.len() as u64,
        sha256: sha256_of_bytes(&bytes),
    };
    let gz_bytes = gzip_deterministic(&bytes)?;
    let gz_path = dir.join(FILENAME_GZ);
    atomic_write(&gz_path, &gz_bytes)?;
    let gz = WrittenFile {
        size: gz_bytes.len() as u64,
        sha256: sha256_of_bytes(&gz_bytes),
    };
    Ok((plain, gz))
}

/// gzip-encode `bytes` deterministically: header `mtime=0`, no
/// filename, level 6 (the zlib default that gzip-1.x ships). Same
/// input → same output across machines so the sha256 in
/// `repomd.json` stays stable.
pub fn gzip_deterministic(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
    encoder
        .write_all(bytes)
        .map_err(|e| Error::Malformed(format!("gzip write: {e}")))?;
    encoder
        .finish()
        .map_err(|e| Error::Malformed(format!("gzip finish: {e}")))
}

pub fn read(dir: &Path) -> Result<Vec<VersionEntry>> {
    let path = dir.join(FILENAME);
    let bytes = std::fs::read(&path).map_err(|e| Error::Io {
        path: path.clone(),
        message: e.to_string(),
    })?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<Vec<VersionEntry>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| Error::Malformed(format!("primary.jsonl is not valid UTF-8: {e}")))?;
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: VersionEntry = serde_json::from_str(line).map_err(|e| {
            Error::Malformed(format!(
                "primary.jsonl line {} is malformed: {e}",
                lineno + 1
            ))
        })?;
        out.push(entry);
    }
    Ok(out)
}

pub struct WrittenFile {
    pub size: u64,
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PackageKind, VersionEntry};
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;
    use vibe_core::Group;

    fn entry(kind: PackageKind, name: &str, version: &str) -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind,
            group: Group::parse("org.vibevm").unwrap(),
            name: name.into(),
            version: version.parse().unwrap(),
            content_hash: format!("sha256:{name}{version}"),
            source_url: format!("https://example.invalid/{name}.git"),
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
            indexed_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            indexed_by: "vibe-index 0.1.0-dev".into(),
        }
    }

    #[test]
    fn round_trip_sorts_entries() {
        let mut entries = vec![
            entry(PackageKind::Flow, "wal", "0.2.0"),
            entry(PackageKind::Flow, "wal", "0.1.0"),
            entry(PackageKind::Flow, "atomic-commits", "0.1.0"),
        ];
        let bytes = serialise(&mut entries).unwrap();
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.len(), 3);
        // After sort: atomic-commits 0.1.0, wal 0.1.0, wal 0.2.0
        assert_eq!(parsed[0].name, "atomic-commits");
        assert_eq!(parsed[1].name, "wal");
        assert_eq!(parsed[1].version.to_string(), "0.1.0");
        assert_eq!(parsed[2].name, "wal");
        assert_eq!(parsed[2].version.to_string(), "0.2.0");
    }

    #[test]
    fn deterministic_byte_output() {
        let mut a = vec![
            entry(PackageKind::Flow, "wal", "0.1.0"),
            entry(PackageKind::Flow, "wal", "0.2.0"),
        ];
        let mut b = vec![
            entry(PackageKind::Flow, "wal", "0.2.0"),
            entry(PackageKind::Flow, "wal", "0.1.0"),
        ];
        assert_eq!(serialise(&mut a).unwrap(), serialise(&mut b).unwrap());
    }

    #[test]
    fn write_persists_on_disk() {
        let dir = tempdir().unwrap();
        let mut entries = vec![entry(PackageKind::Flow, "wal", "0.1.0")];
        let (plain, gz) = write(dir.path(), &mut entries).unwrap();
        assert!(plain.size > 0);
        assert!(plain.sha256.starts_with("sha256:"));
        assert!(gz.size > 0);
        assert!(gz.sha256.starts_with("sha256:"));
        let back = read(dir.path()).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "wal");
        assert!(dir.path().join("primary.jsonl.gz").is_file());
    }

    #[test]
    fn gzip_is_deterministic() {
        let bytes = b"vibevm primary.jsonl bytes go here\n";
        let a = gzip_deterministic(bytes).unwrap();
        let b = gzip_deterministic(bytes).unwrap();
        assert_eq!(
            a, b,
            "gzip output must be byte-identical for identical input"
        );
    }

    #[test]
    fn gzip_round_trips_to_original_bytes() {
        let original = b"line one\nline two\nline three\n";
        let compressed = gzip_deterministic(original).unwrap();
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn empty_lines_are_skipped() {
        let bytes = b"\n";
        let parsed = parse(bytes).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn malformed_line_surfaces_with_lineno() {
        let bytes = b"{\"not a valid entry\":true}\n";
        let err = parse(bytes).unwrap_err();
        match err {
            Error::Malformed(m) => assert!(m.contains("line 1")),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
