//! The incremental cache and atomic file IO (PROP-043 §7.1).
//!
//! Every write in this crate goes through `write_atomic` (tmp + rename),
//! so a killed process never leaves a torn JSON on disk.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#cache");

use crate::doc::ParsedDoc;
use crate::rollup::DocRollup;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Schema 2: the fact amendment — `DocRollup` counts facts
/// (paragraphs + list items + table cells), not paragraphs.
pub const CACHE_SCHEMA: u32 = 2;

/// One observed file's record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub content_hash: String,
    pub rollup: DocRollup,
    pub marker_count: usize,
    pub unit_count: usize,
    pub issue_count: usize,
    /// Campaign fields (verdicts etc.) merge in during phases C–E; absent
    /// until then.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub campaign: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cache {
    pub schema: u32,
    pub updated_at: String,
    /// repo-relative `/`-separated path → record; BTreeMap keeps the
    /// serialized form stably sorted (clean diffs).
    pub files: BTreeMap<String, FileRecord>,
}

impl Cache {
    pub fn load(path: &Path) -> Result<Cache> {
        if !path.exists() {
            return Ok(Cache {
                schema: CACHE_SCHEMA,
                ..Cache::default()
            });
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// Crash-tolerant load: a torn or corrupt cache (power loss mid-write)
    /// degrades to an empty cache plus a warning — the cache is derived
    /// acceleration (PROP-043 §7.5) and must never kill a session.
    pub fn load_tolerant(path: &Path) -> (Cache, Option<String>) {
        match Cache::load(path) {
            Ok(c) => (c, None),
            Err(e) => (
                Cache {
                    schema: CACHE_SCHEMA,
                    ..Cache::default()
                },
                Some(format!(
                    "cache at {} was unreadable ({e:#}); rebuilt from scratch",
                    path.display()
                )),
            ),
        }
    }

    pub fn store(&self, path: &Path) -> Result<()> {
        let body = serde_json::to_string_pretty(self)?;
        write_atomic(path, body.as_bytes())
    }

    /// True when the cached record for `path` is current for `hash`.
    pub fn is_current(&self, path: &str, hash: &str) -> bool {
        self.files
            .get(path)
            .map(|r| r.content_hash == hash)
            .unwrap_or(false)
    }

    pub fn upsert(&mut self, doc: &ParsedDoc, rollup: &DocRollup) {
        let campaign = self
            .files
            .get(&doc.path)
            .map(|r| r.campaign.clone())
            .unwrap_or_default();
        self.files.insert(
            doc.path.clone(),
            FileRecord {
                content_hash: doc.content_hash.clone(),
                rollup: rollup.clone(),
                marker_count: doc.markers.len(),
                unit_count: doc.units.len(),
                issue_count: doc.issues.len(),
                campaign,
            },
        );
    }

    pub fn touch(&mut self) {
        self.updated_at = now_utc();
    }
}

/// RFC-3339 UTC timestamp (seconds precision) for `updated_at` fields.
pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// tmp + fsync + rename: the poller never sees half a file, and a power
/// cut never leaves a zero-length rename (the fsync-before-rename lesson,
/// paid for live on 2026-07-24).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp~");
    {
        let mut f =
            std::fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        f.write_all(bytes)
            .with_context(|| format!("writing {}", tmp.display()))?;
        f.sync_all()
            .with_context(|| format!("syncing {}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_cache_degrades_to_empty_with_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache.json");
        // A zero-length file — exactly what a power cut after rename but
        // before data flush leaves behind.
        std::fs::write(&path, b"").expect("write");
        let (c, warn) = Cache::load_tolerant(&path);
        assert!(c.files.is_empty());
        assert!(warn.expect("warning").contains("rebuilt from scratch"));
    }

    #[test]
    fn cache_round_trips_and_tracks_currency() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache.json");
        let mut c = Cache {
            schema: CACHE_SCHEMA,
            ..Cache::default()
        };
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        let r = crate::rollup::rollup_doc(&doc);
        c.upsert(&doc, &r);
        c.touch();
        c.store(&path).expect("store");
        let back = Cache::load(&path).expect("load");
        assert!(back.is_current("a.md", &doc.content_hash));
        assert!(!back.is_current("a.md", "deadbeef"));
    }
}
