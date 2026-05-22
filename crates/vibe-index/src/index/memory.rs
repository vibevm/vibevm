//! In-memory index — `Index` struct + persistence orchestration.
//!
//! The index is keyed on the PROP-008 §2.2 package identity
//! `(group, name)`; `kind` is metadata and keys nothing. Slice 2 wired
//! the read/write pipeline for the three core file types (`repomd.json`,
//! `primary.jsonl`, the per-name `by-name` candidate sets); slice 4
//! layered in `by-cap` / `by-purl` / inverted text search; slice 5 the
//! HTTP server.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::{by_name, inverted, primary, repomd};
use crate::types::{
    NameEntry, NamingConvention, PackageEntry, Repomd, RepomdFileEntry, VersionEntry,
};

/// In-RAM index key — the `(group, name)` package identity (PROP-008
/// §2.2). `kind` is metadata and no longer keys anything.
pub type PkgKey = (Group, String);

const SCHEMA_VERSION: u32 = 1;

/// In-RAM index. Single-source-of-truth when the server is running;
/// loaded from disk on CLI invocation.
#[derive(Debug, Clone)]
pub struct Index {
    pub schema_version: u32,
    pub registry: String,
    pub registry_url: String,
    pub naming: NamingConvention,
    pub generator: String,
    pub generated_at: DateTime<Utc>,
    pub by_pkgref: BTreeMap<PkgKey, PackageEntry>,
}

impl Index {
    /// Build an empty index for `registry` rooted at `registry_url`.
    pub fn new(
        registry: impl Into<String>,
        registry_url: impl Into<String>,
        naming: NamingConvention,
    ) -> Self {
        Index {
            schema_version: SCHEMA_VERSION,
            registry: registry.into(),
            registry_url: registry_url.into(),
            naming,
            generator: default_generator(),
            generated_at: Utc::now(),
            by_pkgref: BTreeMap::new(),
        }
    }

    /// Insert (or replace) `entry`'s package version. The host
    /// `PackageEntry` is created on first insert. `latest_stable` is
    /// recomputed via [`PackageEntry::finalise`].
    pub fn upsert(&mut self, entry: VersionEntry) {
        let key = (entry.group.clone(), entry.name.clone());
        let pkg = self.by_pkgref.entry(key).or_insert_with(|| {
            PackageEntry::new(entry.group.clone(), entry.name.clone(), entry.indexed_at)
        });
        pkg.versions.retain(|v| v.version != entry.version);
        pkg.versions.push(entry);
        pkg.finalise();
    }

    /// Drop one specific version. Returns `true` iff the version was
    /// present. Empty packages stay in the map (zero-version package
    /// rows are valid; consumers that want to prune them call
    /// [`Index::remove_package`]).
    pub fn remove_version(&mut self, group: &Group, name: &str, version: &semver::Version) -> bool {
        let key = (group.clone(), name.to_string());
        let Some(pkg) = self.by_pkgref.get_mut(&key) else {
            return false;
        };
        let before = pkg.versions.len();
        pkg.versions.retain(|v| &v.version != version);
        let removed = pkg.versions.len() < before;
        if removed {
            pkg.finalise();
        }
        removed
    }

    /// Drop every version of a package.
    pub fn remove_package(&mut self, group: &Group, name: &str) -> bool {
        self.by_pkgref
            .remove(&(group.clone(), name.to_string()))
            .is_some()
    }

    /// One package by its exact `(group, name)` identity.
    pub fn get(&self, group: &Group, name: &str) -> Option<&PackageEntry> {
        self.by_pkgref.get(&(group.clone(), name.to_string()))
    }

    /// Every package sharing the bare `name`, across all groups — the
    /// short-name candidate set (PROP-008 §2.6 / §2.7). `by_pkgref`
    /// iterates in `(group, name)` order, so the result is group-sorted.
    pub fn candidates_for(&self, name: &str) -> Vec<&PackageEntry> {
        self.by_pkgref.values().filter(|p| p.name == name).collect()
    }

    pub fn package_count(&self) -> u32 {
        self.by_pkgref.len() as u32
    }

    pub fn version_count(&self) -> u32 {
        self.by_pkgref
            .values()
            .map(|p| p.versions.len() as u32)
            .sum()
    }

    /// Iterate every (group, name, version) entry in deterministic order.
    pub fn iter_versions(&self) -> impl Iterator<Item = &VersionEntry> {
        self.by_pkgref.values().flat_map(|p| p.versions.iter())
    }

    /// Persist the index to `data_dir` atomically. Writes
    /// `primary.jsonl` and every `by-name/<name>.json` candidate set,
    /// then stamps `repomd.json` last so partial views are always
    /// consistent against an older manifest until the new one lands.
    pub fn write_to(&self, data_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(data_dir).map_err(|e| Error::Io {
            path: data_dir.to_path_buf(),
            message: e.to_string(),
        })?;

        // Drop existing by-name / by-cap / by-purl directories before
        // rewriting. Simplest correct approach: clear before rewrite,
        // so removed packages do not leave stale files behind. The
        // incremental-reindex path (slice 7) does its own per-package
        // diff for the by-name dir; here we still scorched-earth the
        // inverted indices because they regenerate cheaply from the
        // already-loaded entries.
        clear_by_name(data_dir)?;
        inverted::clear_dir(&inverted::by_cap_dir(data_dir))?;
        inverted::clear_dir(&inverted::by_purl_dir(data_dir))?;

        // Write primary.jsonl + primary.jsonl.gz.
        let mut entries: Vec<VersionEntry> = self.iter_versions().cloned().collect();
        let (primary_meta, primary_gz_meta) = primary::write(data_dir, &mut entries)?;

        let mut files: BTreeMap<String, RepomdFileEntry> = BTreeMap::new();
        files.insert(
            primary::FILENAME.into(),
            RepomdFileEntry::file(primary_meta.size, primary_meta.sha256),
        );
        files.insert(
            primary::FILENAME_GZ.into(),
            RepomdFileEntry::file(primary_gz_meta.size, primary_gz_meta.sha256),
        );

        // Write every by-name candidate-set file. Each holds every
        // `(group, name)` package sharing one bare name (PROP-008 §2.8).
        // `by_pkgref` iterates in `(group, name)` order, so each name's
        // candidates arrive group-sorted; `finalise` re-sorts defensively.
        let mut by_name_files: BTreeMap<String, NameEntry> = BTreeMap::new();
        for pkg in self.by_pkgref.values() {
            by_name_files
                .entry(pkg.name.clone())
                .or_insert_with(|| NameEntry::new(pkg.name.clone(), self.generated_at))
                .packages
                .push(pkg.clone());
        }
        for name_entry in by_name_files.values_mut() {
            name_entry.finalise();
            let written = by_name::write(data_dir, name_entry)?;
            files.insert(
                written.relative_path,
                RepomdFileEntry::file(written.size, written.sha256),
            );
        }
        files.insert(
            by_name::DIRNAME.into(),
            RepomdFileEntry::directory(by_name::entry_count(data_dir)),
        );

        // Build the inverted views and emit by-cap/<slug>.jsonl +
        // by-purl/<slug>.jsonl. PROP-005 §2.4.
        let view = inverted::InvertedView::from_entries(self.iter_versions());
        for (slug, rows) in &view.by_capability {
            let written = inverted::write_capability(data_dir, slug, rows)?;
            files.insert(
                written.relative_path,
                RepomdFileEntry::file(written.size, written.sha256),
            );
        }
        for (slug, rows) in &view.by_purl {
            let written = inverted::write_purl(data_dir, slug, rows)?;
            files.insert(
                written.relative_path,
                RepomdFileEntry::file(written.size, written.sha256),
            );
        }
        files.insert(
            inverted::BY_CAP_DIRNAME.into(),
            RepomdFileEntry::directory(inverted::entry_count_capability(data_dir)),
        );
        files.insert(
            inverted::BY_PURL_DIRNAME.into(),
            RepomdFileEntry::directory(inverted::entry_count_purl(data_dir)),
        );

        // Stamp the manifest.
        let manifest = Repomd {
            schema_version: Repomd::SCHEMA_VERSION,
            registry: self.registry.clone(),
            registry_url: self.registry_url.clone(),
            naming: self.naming,
            generated_at: Utc::now(),
            generator: self.generator.clone(),
            package_count: self.package_count(),
            version_count: self.version_count(),
            files,
        };
        repomd::write(data_dir, &manifest)
    }

    /// Load an index from `data_dir`. The on-disk shape is the source
    /// of truth for the in-memory copy; missing files surface as
    /// errors. Each `by-name/<name>.json` candidate set is flattened
    /// back into the `(group, name)`-keyed map.
    pub fn load_from(data_dir: &Path) -> Result<Self> {
        let manifest = repomd::read(data_dir)?;
        let name_entries = by_name::read_all(data_dir)?;
        let mut by_pkgref: BTreeMap<PkgKey, PackageEntry> = BTreeMap::new();
        for name_entry in name_entries {
            for mut pkg in name_entry.packages {
                pkg.finalise();
                by_pkgref.insert((pkg.group.clone(), pkg.name.clone()), pkg);
            }
        }
        Ok(Index {
            schema_version: manifest.schema_version,
            registry: manifest.registry,
            registry_url: manifest.registry_url,
            naming: manifest.naming,
            generator: manifest.generator,
            generated_at: manifest.generated_at,
            by_pkgref,
        })
    }
}

fn clear_by_name(data_dir: &Path) -> Result<()> {
    let dir = by_name::dir(data_dir);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| Error::Io {
            path: dir.clone(),
            message: e.to_string(),
        })?;
    }
    Ok(())
}

pub fn data_dir_state(data_dir: &Path) -> PathBuf {
    data_dir.join("state")
}

fn default_generator() -> String {
    format!("vibe-index {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PackageKind, VersionEntry};
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

    fn entry(kind: PackageKind, group: Group, name: &str, version: &str) -> VersionEntry {
        VersionEntry {
            schema_version: VersionEntry::SCHEMA_VERSION,
            kind,
            group,
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
            indexed_at: now(),
            indexed_by: "vibe-index 0.1.0-dev".into(),
        }
    }

    #[test]
    fn upsert_replaces_existing_version() {
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
        );
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        assert_eq!(idx.version_count(), 1);
    }

    #[test]
    fn remove_version_works() {
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
        );
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.2.0"));
        let v = "0.1.0".parse().unwrap();
        assert!(idx.remove_version(&org(), "wal", &v));
        assert_eq!(idx.version_count(), 1);
    }

    #[test]
    fn write_then_load_round_trips() {
        let tmp = tempdir().unwrap();
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
        );
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.2.0"));
        idx.upsert(entry(PackageKind::Flow, org(), "atomic-commits", "0.1.0"));
        idx.upsert(entry(PackageKind::Stack, org(), "rust-cli", "0.1.0"));
        idx.write_to(tmp.path()).unwrap();

        let back = Index::load_from(tmp.path()).unwrap();
        assert_eq!(back.registry, idx.registry);
        assert_eq!(back.registry_url, idx.registry_url);
        assert_eq!(back.naming, idx.naming);
        assert_eq!(back.package_count(), 3);
        assert_eq!(back.version_count(), 4);
        assert!(back.get(&org(), "wal").is_some());
    }

    #[test]
    fn candidate_set_collapses_a_shared_name_into_one_file() {
        // Two groups publish a package called `wal` — a short-name
        // collision. They land in one `by-name/wal.json` candidate set.
        let tmp = tempdir().unwrap();
        let acme = Group::parse("com.acme").unwrap();
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
        );
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        idx.upsert(entry(PackageKind::Feat, acme.clone(), "wal", "1.0.0"));
        idx.write_to(tmp.path()).unwrap();

        // One file, two candidates.
        assert!(by_name::file_path(tmp.path(), "wal").exists());
        let candidates = idx.candidates_for("wal");
        assert_eq!(candidates.len(), 2);

        let back = Index::load_from(tmp.path()).unwrap();
        assert_eq!(back.package_count(), 2);
        assert!(back.get(&org(), "wal").is_some());
        assert!(back.get(&acme, "wal").is_some());
    }

    #[test]
    fn write_creates_repomd_with_file_hashes() {
        let tmp = tempdir().unwrap();
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
        );
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        idx.write_to(tmp.path()).unwrap();
        let manifest = repomd::read(tmp.path()).unwrap();
        assert!(matches!(
            manifest.files.get("primary.jsonl"),
            Some(RepomdFileEntry::File { .. })
        ));
        assert!(matches!(
            manifest.files.get("by-name"),
            Some(RepomdFileEntry::Directory { .. })
        ));
        assert!(manifest.files.contains_key("by-name/wal.json"));
    }

    #[test]
    fn write_replaces_stale_by_name_files() {
        let tmp = tempdir().unwrap();
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
        );
        idx.upsert(entry(PackageKind::Flow, org(), "wal", "0.1.0"));
        idx.write_to(tmp.path()).unwrap();
        // Drop the package; the old file MUST be gone after rewrite.
        idx.remove_package(&org(), "wal");
        idx.upsert(entry(PackageKind::Flow, org(), "atomic-commits", "0.1.0"));
        idx.write_to(tmp.path()).unwrap();
        assert!(!by_name::file_path(tmp.path(), "wal").exists());
        assert!(by_name::file_path(tmp.path(), "atomic-commits").exists());
    }
}
