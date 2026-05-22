//! `by-cap/<slug>.jsonl` + `by-purl/<slug>.jsonl` — inverted-index
//! files that let consumers fetch every package providing a given
//! capability (or describing a given upstream PURL) with a single
//! HTTP GET. PROP-005 §2.4 / §2.13 layout.
//!
//! The slug encoding is filesystem-safe and reversible — for both
//! capability and PURL strings, the same three punctuation
//! characters get mapped: `:` and `/` and `@` → `--`. PROP-005
//! §2.4 originally documented `:` as preserved in PURL slugs; we
//! escape it for Windows-compat (NTFS reserves `:` for ADS / drive
//! letters), and the unified rule keeps the encoder + decoder
//! trivial. The original shape is reconstructible by replacing
//! every `--` back to its original character — but since the slug
//! is only ever a filesystem-safe lookup key (never reversed by
//! consumers; the canonical capability/purl lives inside the
//! file's lines), the reversibility detail is informational only.
//!
//! Each line is one JSON record sorted by `(group, name, version)`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::{Deserialize, Serialize};
use vibe_core::Group;
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::index::persistence::{atomic_write, sha256_of_bytes};
use crate::types::{PackageKind, VersionEntry};

pub const BY_CAP_DIRNAME: &str = "by-cap";
pub const BY_PURL_DIRNAME: &str = "by-purl";

// ---------------------------------------------------------------------------
// Slug helpers
// ---------------------------------------------------------------------------

pub fn capability_slug(capability: &str) -> String {
    fs_safe_slug(capability)
}

pub fn purl_slug(purl: &str) -> String {
    fs_safe_slug(purl)
}

fn fs_safe_slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            ':' | '/' | '@' => out.push_str("--"),
            other => out.push(other),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Wire records
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRow {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurlRow {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub purl: String,
    /// Where the PURL was declared on the entry — `"package"` if the
    /// VersionEntry's top-level `describes` matched, `"subskill"` if
    /// the match came from a SubskillEntry's describes.
    pub binding_site: BindingSite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingSite {
    Package,
    Subskill,
}

// ---------------------------------------------------------------------------
// Aggregation: walk all VersionEntry's and bucket per-slug.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct InvertedView {
    pub by_capability: BTreeMap<String, Vec<CapabilityRow>>,
    pub by_purl: BTreeMap<String, Vec<PurlRow>>,
}

impl InvertedView {
    pub fn from_entries<'a, I: IntoIterator<Item = &'a VersionEntry>>(entries: I) -> Self {
        let mut by_capability: BTreeMap<String, Vec<CapabilityRow>> = BTreeMap::new();
        let mut by_purl: BTreeMap<String, Vec<PurlRow>> = BTreeMap::new();
        for entry in entries {
            for cap in &entry.provides.capabilities {
                let slug = capability_slug(cap);
                by_capability.entry(slug).or_default().push(CapabilityRow {
                    kind: entry.kind,
                    group: entry.group.clone(),
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    capability: cap.clone(),
                });
            }
            if let Some(purl) = &entry.describes {
                let slug = purl_slug(purl);
                by_purl.entry(slug).or_default().push(PurlRow {
                    kind: entry.kind,
                    group: entry.group.clone(),
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    purl: purl.clone(),
                    binding_site: BindingSite::Package,
                });
            }
            for sub in &entry.subskills {
                if let Some(purl) = &sub.describes {
                    let slug = purl_slug(purl);
                    by_purl.entry(slug).or_default().push(PurlRow {
                        kind: entry.kind,
                        group: entry.group.clone(),
                        name: entry.name.clone(),
                        version: entry.version.clone(),
                        purl: purl.clone(),
                        binding_site: BindingSite::Subskill,
                    });
                }
            }
        }
        // Deterministic line order per file: sort by the PROP-008 §2.2
        // identity `(group, name, version)`, then the row identifier.
        for rows in by_capability.values_mut() {
            rows.sort_by(|a, b| {
                (&a.group, a.name.as_str(), &a.version, a.capability.as_str()).cmp(&(
                    &b.group,
                    b.name.as_str(),
                    &b.version,
                    b.capability.as_str(),
                ))
            });
        }
        for rows in by_purl.values_mut() {
            rows.sort_by(|a, b| {
                (&a.group, a.name.as_str(), &a.version, a.binding_site as u8).cmp(&(
                    &b.group,
                    b.name.as_str(),
                    &b.version,
                    b.binding_site as u8,
                ))
            });
        }
        InvertedView {
            by_capability,
            by_purl,
        }
    }
}

// ---------------------------------------------------------------------------
// On-disk write/read
// ---------------------------------------------------------------------------

pub fn by_cap_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(BY_CAP_DIRNAME)
}

pub fn by_purl_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(BY_PURL_DIRNAME)
}

pub fn capability_file(data_dir: &Path, slug: &str) -> PathBuf {
    by_cap_dir(data_dir).join(format!("{slug}.jsonl"))
}

pub fn purl_file(data_dir: &Path, slug: &str) -> PathBuf {
    by_purl_dir(data_dir).join(format!("{slug}.jsonl"))
}

#[derive(Debug, Clone)]
pub struct WrittenInvertedFile {
    pub relative_path: String,
    pub size: u64,
    pub sha256: String,
}

pub fn write_capability(
    data_dir: &Path,
    slug: &str,
    rows: &[CapabilityRow],
) -> Result<WrittenInvertedFile> {
    let bytes = serialise_rows(rows.iter(), serde_json::to_string)?;
    let path = capability_file(data_dir, slug);
    atomic_write(&path, &bytes)?;
    Ok(WrittenInvertedFile {
        relative_path: format!("{BY_CAP_DIRNAME}/{slug}.jsonl"),
        size: bytes.len() as u64,
        sha256: sha256_of_bytes(&bytes),
    })
}

pub fn write_purl(data_dir: &Path, slug: &str, rows: &[PurlRow]) -> Result<WrittenInvertedFile> {
    let bytes = serialise_rows(rows.iter(), serde_json::to_string)?;
    let path = purl_file(data_dir, slug);
    atomic_write(&path, &bytes)?;
    Ok(WrittenInvertedFile {
        relative_path: format!("{BY_PURL_DIRNAME}/{slug}.jsonl"),
        size: bytes.len() as u64,
        sha256: sha256_of_bytes(&bytes),
    })
}

fn serialise_rows<'a, T: 'a, I, F>(rows: I, ser: F) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> serde_json::Result<String>,
{
    let mut out = Vec::new();
    for row in rows {
        let line = ser(row)
            .map_err(|e| Error::Malformed(format!("could not serialise inverted row: {e}")))?;
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

pub fn entry_count_capability(data_dir: &Path) -> u32 {
    count_jsonl(&by_cap_dir(data_dir))
}

pub fn entry_count_purl(data_dir: &Path) -> u32 {
    count_jsonl(&by_purl_dir(data_dir))
}

fn count_jsonl(dir: &Path) -> u32 {
    if !dir.is_dir() {
        return 0;
    }
    WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().and_then(|s| s.to_str()) == Some("jsonl")
        })
        .count() as u32
}

pub fn clear_dir(dir: &Path) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| Error::Io {
            path: dir.to_path_buf(),
            message: e.to_string(),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        BootSnippetEntry, DeliveryMode, ProvidesEntry, SubskillEntry, VersionEntry,
    };
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn entry(
        kind: PackageKind,
        name: &str,
        version: &str,
        capabilities: &[&str],
        describes: Option<&str>,
        subskill_purls: &[&str],
    ) -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind,
            group: Group::parse("org.vibevm").unwrap(),
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
            describes: describes.map(|s| s.to_string()),
            compatibility: Default::default(),
            provides: ProvidesEntry {
                capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
            },
            requires: Default::default(),
            requires_any: vec![],
            obsoletes: Default::default(),
            conflicts: Default::default(),
            features: Default::default(),
            subskills: subskill_purls
                .iter()
                .map(|p| SubskillEntry {
                    path: format!("sub-{p}"),
                    delivery: DeliveryMode::Eager,
                    describes: Some((*p).to_string()),
                    description: None,
                    channels: vec![],
                })
                .collect(),
            i18n: Default::default(),
            boot_snippet: Some(BootSnippetEntry {
                source: format!("boot/{name}.md"),
                category: None,
            }),
            files_count: 1,
            indexed_at: now(),
            indexed_by: "test".into(),
        }
    }

    #[test]
    fn capability_slug_replaces_three_punctuation() {
        assert_eq!(
            capability_slug("ui:landing-page@0.3.0"),
            "ui--landing-page--0.3.0"
        );
        assert_eq!(
            capability_slug("interface:trace-discipline"),
            "interface--trace-discipline"
        );
        assert_eq!(capability_slug("a/b/c@1.0.0"), "a--b--c--1.0.0");
    }

    #[test]
    fn purl_slug_uses_filesystem_safe_encoding() {
        // Same encoding as capability_slug so the rule is uniform —
        // `:` is reserved on Windows for drive letters / NTFS ADS.
        assert_eq!(purl_slug("pkg:cargo/sqlx@0.8.0"), "pkg--cargo--sqlx--0.8.0");
        assert_eq!(
            purl_slug("pkg:npm/@scope/pkg@1"),
            "pkg--npm----scope--pkg--1"
        );
    }

    #[test]
    fn from_entries_buckets_per_capability() {
        let entries = vec![
            entry(
                PackageKind::Feat,
                "welcome",
                "0.3.0",
                &["ui:landing-page@0.3.0"],
                None,
                &[],
            ),
            entry(
                PackageKind::Feat,
                "wal",
                "0.1.0",
                &["interface:wal"],
                None,
                &[],
            ),
            entry(
                PackageKind::Stack,
                "rust",
                "0.1.0",
                &["interface:wal"],
                None,
                &[],
            ),
        ];
        let view = InvertedView::from_entries(&entries);
        let wal_slug = capability_slug("interface:wal");
        assert!(view.by_capability.contains_key(&wal_slug));
        assert_eq!(view.by_capability[&wal_slug].len(), 2);
    }

    #[test]
    fn from_entries_buckets_purls_with_binding_site() {
        let entries = vec![
            entry(
                PackageKind::Flow,
                "sqlx-skin",
                "0.1.0",
                &[],
                Some("pkg:cargo/sqlx@0.8.0"),
                &[],
            ),
            entry(
                PackageKind::Stack,
                "rust",
                "0.1.0",
                &[],
                None,
                &["pkg:cargo/sqlx@0.8.0"],
            ),
        ];
        let view = InvertedView::from_entries(&entries);
        let slug = purl_slug("pkg:cargo/sqlx@0.8.0");
        let rows = &view.by_purl[&slug];
        assert_eq!(rows.len(), 2);
        let pkg_row = rows.iter().find(|r| r.name == "sqlx-skin").unwrap();
        assert_eq!(pkg_row.binding_site, BindingSite::Package);
        let sub_row = rows.iter().find(|r| r.name == "rust").unwrap();
        assert_eq!(sub_row.binding_site, BindingSite::Subskill);
    }

    #[test]
    fn write_capability_round_trips_on_disk() {
        let dir = tempdir().unwrap();
        let rows = vec![CapabilityRow {
            kind: PackageKind::Feat,
            group: Group::parse("org.vibevm").unwrap(),
            name: "welcome".into(),
            version: "0.3.0".parse().unwrap(),
            capability: "ui:landing-page@0.3.0".into(),
        }];
        let written = write_capability(dir.path(), "ui--landing-page--0.3.0", &rows).unwrap();
        assert_eq!(
            written.relative_path,
            "by-cap/ui--landing-page--0.3.0.jsonl"
        );
        let content =
            std::fs::read_to_string(capability_file(dir.path(), "ui--landing-page--0.3.0"))
                .unwrap();
        assert!(content.contains("ui:landing-page"));
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn entry_count_returns_zero_when_dir_missing() {
        let dir = tempdir().unwrap();
        assert_eq!(entry_count_capability(dir.path()), 0);
        assert_eq!(entry_count_purl(dir.path()), 0);
    }
}
