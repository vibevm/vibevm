//! `repomd.json` — the per-index manifest, written last on every
//! batch update so a reader chasing hashes always sees consistent
//! files.

use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::index::persistence::atomic_write;
use crate::types::Repomd;

pub const FILENAME: &str = "repomd.json";

pub fn serialise(r: &Repomd) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(r).map_err(|e| {
        Error::Malformed(format!(
            "could not serialise repomd for `{}`: {e}",
            r.registry
        ))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write(data_dir: &Path, r: &Repomd) -> Result<()> {
    let bytes = serialise(r)?;
    atomic_write(&data_dir.join(FILENAME), &bytes)
}

pub fn read(data_dir: &Path) -> Result<Repomd> {
    let path = data_dir.join(FILENAME);
    let bytes = fs::read(&path).map_err(|e| Error::Io {
        path: path.clone(),
        message: e.to_string(),
    })?;
    serde_json::from_slice(&bytes).map_err(|e| Error::Malformed(format!("repomd.json: {e}")))
}

pub fn exists(data_dir: &Path) -> bool {
    data_dir.join(FILENAME).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NamingConvention, RepomdFileEntry};
    use chrono::{DateTime, Utc};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn sample() -> Repomd {
        let mut files = BTreeMap::new();
        files.insert(
            "primary.jsonl".into(),
            RepomdFileEntry::file(123, "sha256:abc"),
        );
        Repomd {
            schema_version: Repomd::SCHEMA_VERSION,
            registry: "vibespecs".into(),
            registry_url: "https://github.com/vibespecs".into(),
            naming: NamingConvention::KindName,
            generated_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            generator: "vibe-index 0.1.0-dev".into(),
            package_count: 1,
            version_count: 1,
            files,
        }
    }

    #[test]
    fn round_trips_on_disk() {
        let dir = tempdir().unwrap();
        let r = sample();
        write(dir.path(), &r).unwrap();
        let back = read(dir.path()).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn exists_reports_correctly() {
        let dir = tempdir().unwrap();
        assert!(!exists(dir.path()));
        write(dir.path(), &sample()).unwrap();
        assert!(exists(dir.path()));
    }
}
