//! In-memory index — `Index` struct + persistence orchestration.
//!
//! The index is keyed on the PROP-008 §2.2 package identity
//! `(group, name)`; `kind` is metadata and keys nothing. Slice 2 wired
//! the read/write pipeline for the three core file types (`repomd.json`,
//! `primary.jsonl`, the per-name `by-name` candidate sets); slice 4
//! layered in `by-cap` / `by-purl` / inverted text search; slice 5 the
//! HTTP server.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::index::persistence::atomic_write;
use crate::index::quarantine::{Quarantined, missing_capabilities};
use crate::index::{by_name, inverted, primary, repomd};
use crate::types::{
    NameEntry, NamingConvention, PackageEntry, Repomd, RepomdFileEntry, Tombstone, VersionEntry,
};
use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::hello::e1::hello::{Handshake, World};

/// In-RAM index key — the `(group, name)` package identity (PROP-008
/// §2.2). `kind` is metadata and no longer keys anything.
pub type PkgKey = (Group, String);

const SCHEMA_VERSION: u32 = 1;

/// The clock, as an input. A writer never calls `now()`: one state must
/// produce one byte sequence, or "rebuild and compare" measures nothing.
pub struct WriteCtx {
    pub at: DateTime<Utc>,
}

/// In-RAM index. Single-source-of-truth when the server is running;
/// loaded from disk on CLI invocation.
///
/// The index is keyed on the `(group, name)` identity (PROP-008 §2.2);
/// `kind` is metadata and keys nothing. A fresh index is empty; `upsert`
/// adds a version, `get` finds it by identity, `remove_version` drops it:
///
/// ```
/// use vibe_index::index::Index;
/// use vibe_index::types::{NamingConvention, PackageKind, VersionEntry};
///
/// let mut idx = Index::new(
///     "vibespecs",
///     "https://github.com/vibespecs",
///     NamingConvention::Fqdn,
///     "2026-05-06T12:00:00Z".parse().unwrap(),
/// );
/// let group = "org.vibevm".parse().unwrap();
/// assert!(idx.get(&group, "wal").is_none());
///
/// idx.upsert(VersionEntry::minimal(
///     PackageKind::Flow,
///     "org.vibevm".parse().unwrap(),
///     "wal",
///     "0.1.0".parse().unwrap(),
///     "2026-05-06T12:00:00Z".parse().unwrap(),
/// ));
/// assert_eq!(idx.package_count(), 1);
/// assert_eq!(idx.version_count(), 1);
///
/// let removed = idx.remove_version(&group, "wal", &"0.1.0".parse().unwrap());
/// assert!(removed);
/// assert_eq!(idx.version_count(), 0); // the version is gone; the row stays
/// ```
#[derive(Debug, Clone)]
pub struct Index {
    pub schema_version: u32,
    pub registry: String,
    pub registry_url: String,
    pub naming: NamingConvention,
    pub generator: String,
    pub generated_at: DateTime<Utc>,
    pub by_pkgref: BTreeMap<PkgKey, PackageEntry>,
    /// The reader's RECORD of the versions it refuses to act on —
    /// their `must_understand` names a capability it lacks (PROP-044
    /// §4.5). A record, not a store: the entries themselves stay in
    /// `by_pkgref`, and answering surfaces ask
    /// `crate::index::quarantine::is_usable` rather than consulting
    /// this list. In memory only — never serialised into any catalog
    /// file.
    pub quarantined: Vec<Quarantined>,
    /// Per-name tombstones (PROP-044 §2) — in memory only; `write_to`
    /// projects them back onto the by-name `NameEntry` it builds.
    pub tombstones: BTreeMap<String, Tombstone>,
}

impl Index {
    /// Build an empty index for `registry` rooted at `registry_url`,
    /// stamped `at`. The clock enters here, at the edge — callers pass
    /// the command's single clock reading; the writer modules never
    /// call the clock themselves.
    pub fn new(
        registry: impl Into<String>,
        registry_url: impl Into<String>,
        naming: NamingConvention,
        at: DateTime<Utc>,
    ) -> Self {
        Index {
            schema_version: SCHEMA_VERSION,
            registry: registry.into(),
            registry_url: registry_url.into(),
            naming,
            generator: default_generator(),
            generated_at: at,
            by_pkgref: BTreeMap::new(),
            quarantined: Vec::new(),
            tombstones: BTreeMap::new(),
        }
    }

    /// Insert (or replace) `entry`'s package version. The host
    /// `PackageEntry` is created on first insert. `latest_stable` is
    /// recomputed via [`PackageEntry::finalise`]. Returns `true` iff
    /// the state changed (F2-3): an entry equal to the one already
    /// stored under the same version number touches nothing — a
    /// mutation that changes nothing must not write, and must not
    /// publish, anything.
    pub fn upsert(&mut self, entry: VersionEntry) -> bool {
        let key = (entry.group.clone(), entry.name.clone());
        let pkg = self.by_pkgref.entry(key).or_insert_with(|| {
            PackageEntry::new(entry.group.clone(), entry.name.clone(), entry.indexed_at)
        });
        // F2-3 — equality on the whole value, not the version number:
        // a differing entry under the same number IS an update, while
        // an identical one is a no-op that leaves the map untouched.
        if pkg.versions.contains(&entry) {
            return false;
        }
        pkg.versions.retain(|v| v.version != entry.version);
        pkg.versions.push(entry);
        pkg.finalise();
        true
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

    /// Count EVERY version the index holds, quarantined included: this
    /// is the writer's number (it rides into `repomd.json`, which every
    /// reader parses). An ANSWERING surface must ask
    /// `crate::index::quarantine::usable_version_count` instead.
    pub fn version_count(&self) -> u32 {
        self.by_pkgref
            .values()
            .map(|p| p.versions.len() as u32)
            .sum()
    }

    /// Iterate every (group, name, version) entry in deterministic
    /// order — ALL of them, quarantined included: the writer's
    /// projections (`primary.jsonl`, the inverted views) must carry
    /// everything the journal holds. An ANSWERING surface must ask
    /// `crate::index::quarantine::usable_entries` instead.
    pub fn iter_versions(&self) -> impl Iterator<Item = &VersionEntry> {
        self.by_pkgref.values().flat_map(|p| p.versions.iter())
    }

    /// Persist the index to `data_dir` atomically. Writes
    /// `primary.jsonl` and every `by-name/<name>.json` candidate set,
    /// then stamps `repomd.json` so partial views are always consistent
    /// against an older manifest until the new one lands, and finally
    /// the eternal handshake `hello.json` — above every world, so it
    /// follows even the manifest (Р39). Every timestamped field — the
    /// manifest's `generated_at` and the by-name `NameEntry` labels —
    /// comes from `ctx.at`: same index, same `WriteCtx` ⇒
    /// byte-identical output (F2-1).
    pub fn write_to(&self, data_dir: &Path, ctx: &WriteCtx) -> Result<()> {
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
        // A name carrying ONLY a tombstone gets its file too — a name
        // that ever existed must answer, never fall silent (PROP-044 §2).
        let mut by_name_files: BTreeMap<String, NameEntry> = BTreeMap::new();
        for pkg in self.by_pkgref.values() {
            by_name_files
                .entry(pkg.name.clone())
                .or_insert_with(|| NameEntry::new(pkg.name.clone(), ctx.at))
                .packages
                .push(pkg.clone());
        }
        for (name, ts) in &self.tombstones {
            let slot = by_name_files
                .entry(name.clone())
                .or_insert_with(|| NameEntry::new(name.clone(), ctx.at));
            slot.tombstone = Some(ts.clone());
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

        // Stamp the manifest. The schema version comes from STATE, not
        // from this build's constant: a catalog this writer read keeps
        // the version it carried (F2-2) — only `Index::new`, an
        // artifact born from scratch, stamps the constant.
        let manifest = Repomd {
            schema_version: self.schema_version,
            registry: self.registry.clone(),
            registry_url: self.registry_url.clone(),
            naming: self.naming.clone(),
            generated_at: ctx.at,
            generator: self.generator.clone(),
            package_count: self.package_count(),
            version_count: self.version_count(),
            files,
        };
        repomd::write(data_dir, &manifest)?;

        // The eternal handshake lands last, after the manifest, and
        // stays OUTSIDE its `files` map (Р39): `repomd.json` is the
        // manifest of one world, while the handshake stands above
        // worlds and dispatches to them. Precedent for a root file the
        // map does not carry: `.gitignore` and `README.md` from `init`.
        write_handshake(data_dir)
    }

    /// Load an index from `data_dir`. The on-disk shape is the source
    /// of truth for the in-memory copy; missing files surface as
    /// errors. Each `by-name/<name>.json` candidate set is flattened
    /// back into the `(group, name)`-keyed map.
    ///
    /// Versions whose `must_understand` names a capability this build
    /// lacks are detected here — above the parsers, which stay pure
    /// bytes→types — and ENTER `by_pkgref` like every other version,
    /// with a WARN and a record in `quarantined`. Nothing is dropped:
    /// quarantine is the READER's judgement about a (record × build)
    /// pair, made at the point of use — an ANSWERING surface asks
    /// `crate::index::quarantine::is_usable` / the `usable_*`
    /// accessors and never serves such a version, while the WRITER
    /// path (`write_to`) deliberately does not ask, because the catalog
    /// is the projection of the journal and a reader's capabilities
    /// never shrink what is written. Tombstones ride the by-name files
    /// and are collected into the in-memory carrier.
    pub fn load_from(data_dir: &Path) -> Result<Self> {
        let manifest = repomd::read(data_dir)?;
        let name_entries = by_name::read_all(data_dir)?;
        let mut by_pkgref: BTreeMap<PkgKey, PackageEntry> = BTreeMap::new();
        let mut quarantined: Vec<Quarantined> = Vec::new();
        let mut tombstones: BTreeMap<String, Tombstone> = BTreeMap::new();
        for name_entry in name_entries {
            if let Some(ts) = name_entry.tombstone {
                tombstones.insert(name_entry.name.clone(), ts);
            }
            for mut pkg in name_entry.packages {
                // Record versions this reader cannot honour (PROP-044
                // §4.5): quarantine + WARN — but never drop them. The
                // version STAYS in `by_pkgref`: an ANSWERING surface
                // asks `quarantine::is_usable` and never serves it,
                // while the WRITER path projects everything the journal
                // holds, and a reader's capabilities never shrink what
                // is written.
                for v in &pkg.versions {
                    let missing = missing_capabilities(&v.must_understand);
                    if missing.is_empty() {
                        continue;
                    }
                    tracing::warn!(
                        group = %pkg.group,
                        name = %pkg.name,
                        version = %v.version,
                        missing = %missing.join(","),
                        "quarantined: must_understand names capabilities this build lacks"
                    );
                    quarantined.push(Quarantined {
                        group: pkg.group.clone(),
                        name: pkg.name.clone(),
                        version: v.version.clone(),
                        missing,
                    });
                }
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
            quarantined,
            tombstones,
        })
    }
}

/// Write the eternal handshake `hello.json` (PROP-044
/// `##ONE-ETERNAL-FILE`) — the one document whose keys never change
/// meaning, through which a client of any age finds the worlds that
/// currently exist. A projection of the FORMAT REGISTRY, not of the
/// journal (Р40): both numbers come from the generated `FormatId`,
/// neither from a literal, and no clock enters — the handshake is a
/// function of the registry and the binary, so two builds of one state
/// produce the same bytes.
///
/// The degenerate first implementation (Р43, `##RISK-OVERENGINEERING`):
/// one world, `path: "."`, every optional key absent. `vibe` is the
/// handshake format's OWN version — a different family from the world's
/// epoch (Р45), which is the catalog format family's number, read from
/// `index-repomd` because `repomd.json` is the world's manifest; the
/// epoch-agreement sentinel in vibe-wire holds that family to one value.
fn write_handshake(data_dir: &Path) -> Result<()> {
    let handshake = Handshake {
        vibe: format!("hello/{}", FormatId::Handshake.epoch()),
        worlds: vec![World {
            epoch: FormatId::IndexRepomd.epoch(),
            path: ".".to_string(),
            sunset: None,
        }],
        min_client: None,
        notice: None,
        successor: None,
    };
    let mut bytes = serde_json::to_vec(&handshake)
        .map_err(|e| Error::Malformed(format!("could not serialise the handshake: {e}")))?;
    bytes.push(b'\n');
    atomic_write(&data_dir.join("hello.json"), &bytes)
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

/// The generator label — `vibe-index <version>` — stamped into every
/// artifact this binary writes. Two consumers since the journal
/// landed: `Index::new` fills a fresh index's `generator` field with
/// it, and the CLI edge (`init`) reuses it as the journal record's
/// `actor`, so the one format string lives in exactly one place and
/// the catalog and the journal never disagree about who wrote them.
pub fn default_generator() -> String {
    format!("vibe-index {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
#[path = "memory/tests.rs"]
mod tests;
