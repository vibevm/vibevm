//! The index of a directory-layout registry (`monorepo`, `localdir`;
//! PROP-002 `#registry-index`, PROP-005 `#layout-registry`).
//!
//! A layout registry holds its packages at `<group>/<name>/v<version>/` and
//! its own index at `_index/` ([`vibe_core::layout_registry::INDEX_DIR`]):
//! an ordinary vibe-index data directory whose journal is committed with
//! it. The index is ONE projection, computed by [`project`], of two inputs:
//!
//! - **the tree** — which versions exist and what bytes they hold (a
//!   directory filled by hand is as real as a published one);
//! - **the journal** — the facts a tree cannot carry: the registry's
//!   identity, where a mirrored version came from (its `Published` entry),
//!   yanks and tombstones.
//!
//! A `Published` fact whose bytes no longer match the tree yields to the
//! tree; a version present only in the tree is projected from the tree
//! alone. A writer ([`refresh`]) appends its facts, projects, and persists
//! the result plus the listing it was computed from; a reader
//! ([`load_or_project`]) uses the persisted files while the listing still
//! matches and otherwise runs the very same [`project`] in memory.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout-registry");

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use vibe_core::layout_registry::{self as layout, IndexState, LayoutVersion};

use crate::error::{Error, Result};
use crate::index::Index;
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, project as fold, replay};
use crate::scanner::SkipNote;
use crate::scanner::tree_entry::{EntrySource, entry_from_tree};
use crate::types::{NamingConvention, VersionEntry};

/// What a reader or writer knows about the registry before reading its
/// journal: the name and url it configured. The journal's own
/// `Initialised` fact wins once it exists, so every reader projects the
/// same identity whatever it calls the registry locally.
#[derive(Debug, Clone)]
pub struct LayoutIdentity {
    pub registry: String,
    pub registry_url: String,
    /// The branch a `monorepo` serves (`source_ref` of an entry projected
    /// from the tree); `None` for a `localdir`, whose entries carry
    /// `v<version>`.
    pub branch: Option<String>,
}

/// One projection of a layout registry.
#[derive(Debug)]
pub struct LayoutProjection {
    pub index: Index,
    /// Version directories the projection could not read (no or a bad
    /// `vibe.toml`, a manifest naming another coordinate).
    pub skipped: Vec<SkipNote>,
}

/// Project the registry rooted at `root` from its tree and journal.
pub fn project(root: &Path, identity: &LayoutIdentity) -> Result<LayoutProjection> {
    let records = replay(&layout::journal_dir(root))?;
    project_records(root, identity, &records)
}

fn project_records(
    root: &Path,
    identity: &LayoutIdentity,
    records: &[JournalRecord],
) -> Result<LayoutProjection> {
    let initialised = records
        .iter()
        .find(|record| matches!(record.event, Event::Initialised { .. }))
        .cloned()
        .unwrap_or_else(|| JournalRecord {
            at: DateTime::UNIX_EPOCH,
            actor: default_generator(),
            event: Event::Initialised {
                registry: identity.registry.clone(),
                registry_url: identity.registry_url.clone(),
                naming: NamingConvention::Fqdn,
            },
        });
    let Event::Initialised {
        registry,
        registry_url,
        ..
    } = &initialised.event
    else {
        return Err(Error::Unprojectable(
            "the layout registry's identity record is not `Initialised`".into(),
        ));
    };

    let versions = layout::version_dirs(root).map_err(|e| io(root, e))?;
    let mut skipped = Vec::new();
    let mut tree: BTreeMap<(String, String, semver::Version), VersionEntry> = BTreeMap::new();
    for version in &versions {
        let dir = root.join(version.path());
        let source = EntrySource {
            source_url: format!("{registry_url}#{}", version.path()),
            source_ref: identity
                .branch
                .clone()
                .unwrap_or_else(|| format!("v{}", version.version)),
            resolved_commit: None,
            registry: registry.clone(),
            indexed_at: initialised.at,
            indexed_by: initialised.actor.clone(),
        };
        match entry_from_tree(&dir, source) {
            Ok(entry) if coordinate_matches(&entry, version) => {
                tree.insert(key_of(&entry), entry);
            }
            Ok(entry) => skipped.push(skip(
                version,
                format!(
                    "its vibe.toml names {}/{}@{}",
                    entry.group, entry.name, entry.version
                ),
            )),
            Err(error) => skipped.push(skip(version, error.to_string())),
        }
    }

    let recorded: std::collections::BTreeSet<_> = records
        .iter()
        .filter_map(|record| match &record.event {
            Event::Published { entry } => Some(key_of(entry)),
            _ => None,
        })
        .collect();

    let mut stream = vec![initialised.clone()];
    for (key, entry) in &tree {
        if !recorded.contains(key) {
            stream.push(published(&initialised, entry.clone()));
        }
    }
    for record in records {
        match &record.event {
            Event::Initialised { .. } | Event::EntrySetReplaced { .. } => {}
            Event::Published { entry } => {
                let Some(on_disk) = tree.get(&key_of(entry)) else {
                    continue;
                };
                let chosen = if on_disk.content_hash == entry.content_hash {
                    let mut kept = (**entry).clone();
                    kept.registry = registry.clone();
                    kept
                } else {
                    on_disk.clone()
                };
                stream.push(published(record, chosen));
            }
            _ => stream.push(record.clone()),
        }
    }
    Ok(LayoutProjection {
        index: fold(stream)?,
        skipped,
    })
}

/// Append `facts` to the registry's journal (an `Initialised` first when
/// the journal has none), project, and persist the index and the listing
/// it was computed from. Returns the projection written.
pub fn refresh(
    root: &Path,
    identity: &LayoutIdentity,
    facts: Vec<Event>,
    at: DateTime<Utc>,
) -> Result<LayoutProjection> {
    let journal = layout::journal_dir(root);
    let mut records = replay(&journal)?;
    let mut appended = Vec::new();
    if !records
        .iter()
        .any(|record| matches!(record.event, Event::Initialised { .. }))
    {
        appended.push(Event::Initialised {
            registry: identity.registry.clone(),
            registry_url: identity.registry_url.clone(),
            naming: NamingConvention::Fqdn,
        });
    }
    appended.extend(facts);
    for event in appended {
        let record = JournalRecord {
            at,
            actor: default_generator(),
            event,
        };
        append(&journal, &record)?;
        records.push(record);
    }
    let projection = project_records(root, identity, &records)?;
    persist(root, &projection, at)?;
    Ok(projection)
}

/// Write the projection's catalog into `_index/`, then the listing (last:
/// a listing that matches always vouches for files already written).
pub fn persist(root: &Path, projection: &LayoutProjection, at: DateTime<Utc>) -> Result<()> {
    let dir = layout::index_dir(root);
    projection.index.write_to(&dir, &WriteCtx { at })?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        // Only the journal of `state/` is the registry's shared truth; a
        // `vibe-index serve` run here must not commit its tokens or lock.
        crate::index::atomic_write(&ignore, b"/state/*\n!/state/journal/\n")?;
    }
    let listing = layout::current_listing(root).map_err(|e| io(root, e))?;
    crate::index::atomic_write(&dir.join(layout::LISTING_FILE), listing.as_bytes())
}

/// The registry's index for a reader: the persisted files while they are
/// [`IndexState::Current`], else the same [`project`] computed in memory.
/// Returns the index and the state the persisted files were found in.
pub fn load_or_project(root: &Path, identity: &LayoutIdentity) -> Result<(Index, IndexState)> {
    let state = layout::index_state(root).map_err(|e| io(root, e))?;
    if state == IndexState::Current {
        return Ok((Index::load_from(&layout::index_dir(root))?, state));
    }
    if let IndexState::Stale { reason } = &state {
        tracing::warn!(
            "the index of layout registry `{}` is stale ({reason}); computing it in memory",
            root.display()
        );
    }
    Ok((project(root, identity)?.index, state))
}

fn published(at: &JournalRecord, entry: VersionEntry) -> JournalRecord {
    JournalRecord {
        at: at.at,
        actor: at.actor.clone(),
        event: Event::Published {
            entry: Box::new(entry),
        },
    }
}

fn key_of(entry: &VersionEntry) -> (String, String, semver::Version) {
    (
        entry.group.as_str().to_string(),
        entry.name.clone(),
        entry.version.clone(),
    )
}

fn coordinate_matches(entry: &VersionEntry, version: &LayoutVersion) -> bool {
    entry.group == version.group && entry.name == version.name && entry.version == version.version
}

fn skip(version: &LayoutVersion, reason: String) -> SkipNote {
    tracing::warn!("skipped {}: {reason}", version.path());
    SkipNote {
        repo: format!("{}/{}", version.group, version.name),
        tag: Some(format!("v{}", version.version)),
        reason,
    }
}

fn io(path: &Path, error: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
#[path = "layout/tests.rs"]
mod tests;
