//! The template snapshot a mirror writes into a registry's `_index/`, and
//! a client inside a closed network renders from (PROP-075
//! `#MIRROR-TEMPLATE-SNAPSHOT`, `#MIRROR-SNAPSHOT-LAYOUT`).
//!
//! ```text
//! <registry>/_index/templates/<group>/<name>/snapshot.toml
//! <registry>/_index/templates/<group>/<name>/versions.json
//! <registry>/_index/templates/<group>/<name>/digests/<version>
//! ```
//!
//! **The layout lives here, written and read from one place**, because the
//! mirror and the client must not disagree about it: the mirror builds the
//! three files, the client primes its own cache from them, and both use
//! the same path functions and the same record type.
//!
//! `versions.json` is deliberately the **client cache's own envelope**
//! ([`CachedVersionList`]) with the record array restricted to the
//! selected versions — so priming is a copy, not a translation, and a
//! client parses the snapshot with the same `select` the template
//! declares. Because `document` is filtered, its `sha256` is recomputed
//! over the filtered bytes: carrying the upstream hash across would make
//! the envelope describe bytes the snapshot does not have.
//!
//! **A client never revalidates a snapshot and never contacts upstream
//! from one.** A snapshot is a frozen version list by construction, which
//! is the whole point in a closed network; `vibe app versions` reports its
//! `[sync] last` so a user can see how old the list is and ask the
//! operator to re-run the mirror.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vibe_core::Group;

use super::cache::{CachedVersionList, TemplateCache, digest_of};
use super::error::TemplateError;

/// The directory under a registry's `_index/` that holds every snapshot.
pub const SNAPSHOT_SUBDIR: &str = "templates";

/// The only `snapshot.toml` schema this vibe reads or writes.
pub const SCHEMA: u32 = 1;

/// `_index/templates/<group>/<name>/`, relative to the registry root.
///
/// ```
/// use vibe_core::Group;
/// use vibe_registry::templates::snapshot;
///
/// let group = Group::parse("org.vibevm.node")?;
/// assert_eq!(
///     snapshot::relative_dir(&group, "node"),
///     "_index/templates/org.vibevm.node/node"
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn relative_dir(group: &Group, name: &str) -> String {
    format!(
        "{}/{SNAPSHOT_SUBDIR}/{group}/{name}",
        vibe_core::layout_registry::INDEX_DIR
    )
}

/// The three registry-relative paths of one package's snapshot.
pub fn relative_record(group: &Group, name: &str) -> String {
    format!("{}/snapshot.toml", relative_dir(group, name))
}

pub fn relative_version_list(group: &Group, name: &str) -> String {
    format!("{}/versions.json", relative_dir(group, name))
}

pub fn relative_digest(group: &Group, name: &str, version: &semver::Version) -> String {
    format!("{}/digests/{version}", relative_dir(group, name))
}

/// `snapshot.toml` — what the mirror snapshotted, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub schema: u32,
    pub source: SnapshotSource,
    pub sync: SnapshotSync,
    #[serde(default, rename = "version")]
    pub versions: Vec<SnapshotVersion>,
}

/// `[source]` — the template coordinate and revision, and the two urls
/// **as the template declares them, before remap**: a client comparing
/// its own declaration against the snapshot must see the same spelling,
/// and a remapped url is this mirror operator's route, not the template's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotSource {
    /// `<group>/<name>` of the template package.
    pub package: String,
    /// The template package's own version — a revision, never a produced
    /// version.
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_list_url: Option<String>,
    /// The digest url **template** over the version variables.
    pub digest_url: String,
}

/// `[sync]` — the writing vibe and the first and last sync time, exactly
/// as the mirror record's own `[sync]` carries them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotSync {
    pub vibe: String,
    pub first: String,
    pub last: String,
}

/// One `[[version]]` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotVersion {
    pub version: String,
    /// SHA-256 of the digest document's bytes. A changed one for an
    /// already-snapshotted version is the alarm of
    /// `#TEMPLATE-LOCK-RECORD` raised at mirror time.
    pub digest_sha256: String,
    /// The full render the mirror computed while it was there, so a
    /// client inside the network can compare without rendering first and
    /// an operator sees a diff in the commit.
    pub rendered_sha256: String,
}

impl SnapshotRecord {
    /// A fresh record for `package`, with `first` and `last` both `now`.
    pub fn new(
        package: String,
        revision: String,
        version_list_url: Option<String>,
        digest_url: String,
        vibe: String,
        now: String,
    ) -> Self {
        SnapshotRecord {
            schema: SCHEMA,
            source: SnapshotSource {
                package,
                revision,
                version_list_url,
                digest_url,
            },
            sync: SnapshotSync {
                vibe,
                first: now.clone(),
                last: now,
            },
            versions: Vec::new(),
        }
    }

    /// The row of `version`, if it is snapshotted.
    pub fn version(&self, version: &semver::Version) -> Option<&SnapshotVersion> {
        let spelling = version.to_string();
        self.versions.iter().find(|row| row.version == spelling)
    }

    /// Add or replace rows, keeping the others, sorted by version.
    ///
    /// A re-run adds versions and leaves existing ones untouched; a
    /// caller that *replaces* a row has already compared its
    /// `digest_sha256` and raised the alarm if it moved.
    pub fn merge(&mut self, rows: Vec<SnapshotVersion>) {
        for row in rows {
            self.versions.retain(|old| old.version != row.version);
            self.versions.push(row);
        }
        self.versions.sort_by(|a, b| {
            let parsed = |row: &SnapshotVersion| semver::Version::parse(&row.version).ok();
            (parsed(a), a.version.clone()).cmp(&(parsed(b), b.version.clone()))
        });
    }

    pub fn render(&self) -> Result<String, TemplateError> {
        let body = toml::to_string(self).map_err(|error| TemplateError::Declaration {
            package: self.source.package.clone(),
            reason: format!("serialising the template snapshot: {error}"),
        })?;
        Ok(format!(
            "# Written by `vibe registry mirror` (PROP-075 #MIRROR-SNAPSHOT-LAYOUT): the\n\
             # external inputs of template `{}` this registry carries, so a client inside\n\
             # a closed network renders from them with no upstream request at all.\n{body}",
            self.source.package
        ))
    }
}

/// One package's snapshot inside a registry tree.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub record: SnapshotRecord,
    dir: PathBuf,
}

impl Snapshot {
    /// Read the snapshot of `<group>/<name>` out of a registry tree;
    /// `Ok(None)` when the registry carries none.
    ///
    /// A snapshot whose `schema` is not [`SCHEMA`], or whose record does
    /// not parse, reads as **absent** rather than fatal. A snapshot is a
    /// cache of someone else's bytes: a client that cannot read one must
    /// fall back to its own cache and the network, which is a degraded
    /// answer, while a refusal would make the whole registry unusable for
    /// every package — including the ones with no template at all.
    pub fn read(
        registry_root: &Path,
        group: &Group,
        name: &str,
    ) -> Result<Option<Self>, TemplateError> {
        let dir = registry_root.join(relative_dir(group, name));
        let path = dir.join("snapshot.toml");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(TemplateError::Io {
                    path,
                    reason: error.to_string(),
                });
            }
        };
        let Ok(record) = toml::from_str::<SnapshotRecord>(&text) else {
            tracing::warn!(
                target: "vibe_registry::templates",
                "{group}/{name}: the template snapshot at `{}` does not parse; ignored",
                path.display()
            );
            return Ok(None);
        };
        if record.schema != SCHEMA {
            tracing::warn!(
                target: "vibe_registry::templates",
                "{group}/{name}: the template snapshot at `{}` has schema {}, this vibe reads \
                 {SCHEMA}; ignored",
                path.display(),
                record.schema
            );
            return Ok(None);
        }
        Ok(Some(Snapshot { record, dir }))
    }

    /// When the mirror last wrote this snapshot — the `[sync] last`
    /// `vibe app versions` reports so a user can see how old the list is.
    pub fn synced(&self) -> &str {
        &self.record.sync.last
    }

    /// Copy this snapshot's inputs into `cache`, so every later read goes
    /// through the ordinary cache path and **no reader learns that the
    /// bytes came from a registry**.
    ///
    /// That is the whole mechanism by which a network-denied client
    /// renders a snapshotted version: `version_document` and
    /// `digest_document` find a cache hit and never reach the transport.
    /// Existing cache entries are left alone — a digest document is
    /// immutable, so a local copy and a snapshot copy of one version
    /// cannot honestly differ, and if they did, overwriting would erase
    /// the evidence the alarm of `#TEMPLATE-LOCK-RECORD` exists to raise.
    ///
    /// Returns how many files it placed.
    pub fn prime(&self, cache: &TemplateCache) -> Result<usize, TemplateError> {
        let mut placed = 0usize;
        if let (None, Some(envelope)) = (cache.read_version_list()?, self.version_list()?) {
            cache.write_version_list(&envelope)?;
            placed += 1;
        }
        for row in &self.record.versions {
            let Ok(version) = semver::Version::parse(&row.version) else {
                continue;
            };
            if cache.read_digest_document(&version)?.is_some() {
                continue;
            }
            let path = self.dir.join("digests").join(&row.version);
            let document = match std::fs::read_to_string(&path) {
                Ok(document) => document,
                // A row whose digest file is missing is a snapshot the
                // mirror did not finish. The honest answer is the
                // ordinary cache miss for that one version, not a
                // refusal for the package.
                Err(_) => continue,
            };
            if digest_of(document.as_bytes()) != row.digest_sha256 {
                tracing::warn!(
                    target: "vibe_registry::templates",
                    "{}: the snapshot's digest document for {version} does not hash to the \
                     `digest_sha256` its row records; not primed",
                    self.record.source.package
                );
                continue;
            }
            cache.write_digest_document(&version, &document)?;
            placed += 1;
        }
        Ok(placed)
    }

    /// The snapshot's `versions.json`, in the client cache's own envelope.
    pub fn version_list(&self) -> Result<Option<CachedVersionList>, TemplateError> {
        let path = self.dir.join("versions.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(TemplateError::Io {
                    path,
                    reason: error.to_string(),
                });
            }
        };
        match serde_json::from_str::<CachedVersionList>(&text) {
            Ok(envelope) if digest_of(envelope.document.as_bytes()) == envelope.sha256 => {
                Ok(Some(envelope))
            }
            // A `sha256` that does not describe the `document` beside it
            // is a snapshot written by something that filtered the
            // records and forgot to recompute the hash. Reading it would
            // put bytes in the cache under a provenance that is not
            // theirs, so it reads as absent.
            Ok(_) => {
                tracing::warn!(
                    target: "vibe_registry::templates",
                    "{}: the snapshot's `versions.json` carries a `sha256` that does not hash \
                     its own `document`; ignored",
                    self.record.source.package
                );
                Ok(None)
            }
            Err(error) => {
                tracing::warn!(
                    target: "vibe_registry::templates",
                    "{}: the snapshot's `versions.json` does not parse ({error}); ignored",
                    self.record.source.package
                );
                Ok(None)
            }
        }
    }
}

/// The envelope a snapshot writes: the upstream document restricted to
/// `keep`, with **`sha256` recomputed over the filtered bytes**.
///
/// `select` is the template's own JSON Pointer and `version_field` its
/// version key, so the filter reads the document exactly as the renderer
/// does. A document whose pointed value is not an array of objects is
/// left verbatim: there is nothing to filter, and refusing here would
/// refuse a source shape the renderer itself rejects with a better
/// message.
pub fn filtered_version_list(
    upstream: &CachedVersionList,
    select: &str,
    version_field: &str,
    strip_prefix: Option<&str>,
    keep: &[semver::Version],
) -> Result<CachedVersionList, TemplateError> {
    let kept: std::collections::BTreeSet<String> =
        keep.iter().map(semver::Version::to_string).collect();
    let mut document: serde_json::Value =
        serde_json::from_str(&upstream.document).map_err(|error| TemplateError::Declaration {
            package: upstream.source_url.clone(),
            reason: format!("the cached version list is not JSON: {error}"),
        })?;
    if let Some(array) = pointed_array(&mut document, select) {
        array.retain(|record| {
            record
                .get(version_field)
                .and_then(serde_json::Value::as_str)
                .map(|raw| normalise(raw, strip_prefix))
                .is_some_and(|version| kept.contains(&version))
        });
    }
    let filtered =
        serde_json::to_string(&document).map_err(|error| TemplateError::Declaration {
            package: upstream.source_url.clone(),
            reason: format!("re-serialising the filtered version list: {error}"),
        })?;
    Ok(CachedVersionList {
        source_url: upstream.source_url.clone(),
        sha256: digest_of(filtered.as_bytes()),
        fetched_at: upstream.fetched_at.clone(),
        document: filtered,
    })
}

/// The array the template's `select` points at, if it points at one. The
/// empty pointer is the document root, the one addition
/// `#TEMPLATE-SELECTOR` makes to RFC 6901.
fn pointed_array<'a>(
    document: &'a mut serde_json::Value,
    select: &str,
) -> Option<&'a mut Vec<serde_json::Value>> {
    let target = if select.is_empty() {
        document
    } else {
        document.pointer_mut(select)?
    };
    target.as_array_mut()
}

/// A source's own version spelling as canonical semver, or the spelling
/// itself when it does not parse — a record vibe cannot read is simply
/// not one of the kept versions.
fn normalise(raw: &str, strip_prefix: Option<&str>) -> String {
    let trimmed = strip_prefix
        .and_then(|prefix| raw.strip_prefix(prefix))
        .unwrap_or(raw);
    semver::Version::parse(trimmed)
        .map(|version| version.to_string())
        .unwrap_or_else(|_| trimmed.to_string())
}

#[cfg(test)]
#[path = "snapshot/tests.rs"]
mod tests;
