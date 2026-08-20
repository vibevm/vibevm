//! `repomd.json` — the per-index manifest. Modelled after RPM's
//! `repomd.xml`. PROP-005 §2.4 pins the schema.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout");

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specmark::spec;

use super::kinds::NamingConvention;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout",
    r = 1
)]
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
    /// data directory (excluding `state/`). Both entry kinds carry a
    /// `kind` tag on the wire: file entries are `kind: "file"` +
    /// size + sha256; directory entries are `kind: "directory"` +
    /// entries count. Keys are POSIX-style relative paths
    /// (`primary.jsonl`, `by-name`, etc.).
    pub files: BTreeMap<String, RepomdFileEntry>,
}

impl Repomd {
    pub const SCHEMA_VERSION: u32 = 1;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RepomdFileEntry {
    Directory {
        entries: u32,
    },
    File {
        /// Serialised as a **canonical decimal string** — the owner's
        /// standing rule for integers wider than 32 bits (2026-08-20):
        /// JTD has no 64-bit integer, so the string is the only wire
        /// form the schema can describe truthfully. See
        /// [`crate::types::wire_decimal`] and `formats/breaks/003.md`.
        #[serde(with = "crate::types::wire_decimal")]
        size: u64,
        sha256: String,
    },
}

impl RepomdFileEntry {
    pub fn directory(entries: u32) -> Self {
        RepomdFileEntry::Directory { entries }
    }

    pub fn file(size: u64, sha256: impl Into<String>) -> Self {
        RepomdFileEntry::File {
            size,
            sha256: sha256.into(),
        }
    }
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
    #[verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout",
        r = 1
    )]
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
        assert!(json.contains("\"kind\":\"file\""));
        // The size rides the wire as a canonical decimal STRING
        // (formats/breaks/003.md): integers wider than 32 bits are not
        // JSON numbers here.
        assert!(json.contains("\"size\":\"99\""));
        assert!(json.contains("\"sha256\":\"sha256:deadbeef\""));
    }

    #[test]
    fn parses_real_world_shape() {
        let json = r#"{
            "primary.jsonl": { "kind": "file", "size": "184522", "sha256": "sha256:abc" },
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

    #[test]
    fn untagged_file_shape_is_rejected_not_guessed() {
        // The pre-Ф1.5 wire shape: a file entry with no `kind` tag. Under
        // `untagged` serde silently matched it to the `File` arm; the
        // symmetric tag makes the absence an error instead.
        let json = r#"{ "size": "184522", "sha256": "sha256:abc" }"#;
        let parsed = serde_json::from_str::<RepomdFileEntry>(json);
        assert!(parsed.is_err());
    }

    /// The canonical spellings round-trip: the bare zero, the first
    /// value above 2^32 (the old `uint32` ceiling this change retires),
    /// and the top of the range — each as the exact decimal string.
    #[test]
    fn size_wire_canon_round_trips_across_the_u64_range() {
        for size in [0u64, 4_294_967_296, u64::MAX] {
            let entry = RepomdFileEntry::file(size, "sha256:abc");
            let json = serde_json::to_string(&entry).unwrap();
            assert!(
                json.contains(&format!("\"size\":\"{size}\"")),
                "u64 {size} must ride the wire as its exact decimal string: {json}"
            );
            let back: RepomdFileEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(entry, back);
        }
    }

    /// One number, one spelling: leading zeros, a sign, an empty
    /// string, and non-digits are all refusals — never a coercion into
    /// a plausible value (PROP-044 law 1).
    #[test]
    fn size_wire_refuses_non_canonical_strings() {
        for raw in ["\"007\"", "\"-1\"", "\"+42\"", "\"\"", "\"abc\"", "\" 42\""] {
            let json = format!(r#"{{"kind":"file","size":{raw},"sha256":"sha256:abc"}}"#);
            let parsed = serde_json::from_str::<RepomdFileEntry>(&json);
            assert!(
                parsed.is_err(),
                "a non-canonical size string must be refused, not coerced: {raw}"
            );
        }
    }

    /// The break itself: a JSON NUMBER where the string form is now
    /// expected — the pre-2026-08-20 wire — is refused loudly by the
    /// hand-written reader, exactly as the break note announces.
    #[test]
    fn size_wire_refuses_a_json_number() {
        let json = r#"{"kind":"file","size":123,"sha256":"sha256:abc"}"#;
        let parsed = serde_json::from_str::<RepomdFileEntry>(json);
        assert!(
            parsed.is_err(),
            "the old numeric wire form must be refused, not coerced"
        );
    }
}
