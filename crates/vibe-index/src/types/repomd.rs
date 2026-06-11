//! `repomd.json` — the per-index manifest. Modelled after RPM's
//! `repomd.xml`. PROP-005 §2.4 pins the schema.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#layout");

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specmark::spec;

use super::kinds::NamingConvention;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#layout", r = 1)]
pub struct Repomd {
    pub schema_version: u32,
    pub registry: String,
    pub registry_url: String,
    pub naming: NamingConvention,
    pub generated_at: DateTime<Utc>,
    pub generator: String,
    pub package_count: u32,
    pub version_count: u32,
    /// Path-keyed map of file or directory entries beneath the
    /// data directory (excluding `state/`). File entries carry size +
    /// sha256; directory entries carry kind: "directory" + entries
    /// count. Keys are POSIX-style relative paths
    /// (`primary.jsonl`, `by-name`, etc.).
    pub files: BTreeMap<String, RepomdFileEntry>,
}

impl Repomd {
    pub const SCHEMA_VERSION: u32 = 1;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepomdFileEntry {
    Directory {
        /// Always the literal string `"directory"`. Carrying this as
        /// a tag inside the directory variant lets serde's `untagged`
        /// matcher distinguish unambiguously.
        kind: DirectoryTag,
        entries: u32,
    },
    File {
        size: u64,
        sha256: String,
    },
}

impl RepomdFileEntry {
    pub fn directory(entries: u32) -> Self {
        RepomdFileEntry::Directory {
            kind: DirectoryTag::Directory,
            entries,
        }
    }

    pub fn file(size: u64, sha256: impl Into<String>) -> Self {
        RepomdFileEntry::File {
            size,
            sha256: sha256.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryTag {
    Directory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    fn sample_repomd() -> Repomd {
        let mut files = BTreeMap::new();
        files.insert(
            "primary.jsonl".into(),
            RepomdFileEntry::file(1024, "sha256:abc"),
        );
        files.insert("by-name".into(), RepomdFileEntry::directory(3));
        Repomd {
            schema_version: Repomd::SCHEMA_VERSION,
            registry: "vibespecs".into(),
            registry_url: "https://github.com/vibespecs".into(),
            naming: NamingConvention::KindName,
            generated_at: DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            generator: "vibe-index 0.1.0-dev".into(),
            package_count: 3,
            version_count: 5,
            files,
        }
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-index/PROP-005#layout", r = 1)]
    fn repomd_round_trips() {
        let r = sample_repomd();
        let json = serde_json::to_string(&r).unwrap();
        let back: Repomd = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn directory_serialises_with_kind_tag() {
        let entry = RepomdFileEntry::directory(42);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"kind\":\"directory\""));
        assert!(json.contains("\"entries\":42"));
    }

    #[test]
    fn file_serialises_with_size_and_sha256() {
        let entry = RepomdFileEntry::file(99, "sha256:deadbeef");
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"size\":99"));
        assert!(json.contains("\"sha256\":\"sha256:deadbeef\""));
        assert!(!json.contains("kind"));
    }

    #[test]
    fn parses_real_world_shape() {
        let json = r#"{
            "primary.jsonl": { "size": 184522, "sha256": "sha256:abc" },
            "by-name":       { "kind": "directory", "entries": 42 }
        }"#;
        let parsed: BTreeMap<String, RepomdFileEntry> = serde_json::from_str(json).unwrap();
        match &parsed["primary.jsonl"] {
            RepomdFileEntry::File { size, sha256 } => {
                assert_eq!(*size, 184522);
                assert_eq!(sha256, "sha256:abc");
            }
            _ => panic!("expected file"),
        }
        match &parsed["by-name"] {
            RepomdFileEntry::Directory { entries, .. } => assert_eq!(*entries, 42),
            _ => panic!("expected directory"),
        }
    }
}
