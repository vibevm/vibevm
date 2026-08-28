//! Deterministic TOML persistence for the adoption-facts registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#model");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::scan::SourceFileWitness;
use crate::{FactEntry, RegistryError};

const SCHEMA: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    schema: u32,
    #[serde(default, rename = "fact")]
    facts: Vec<FactEntry>,
}

/// A loaded project registry, keyed and iterated by full address.
#[derive(Debug, Default)]
pub struct Registry {
    entries: BTreeMap<String, FactEntry>,
    sources: BTreeMap<String, PathBuf>,
}

impl Registry {
    /// Load every TOML source under `<project_root>/vibefacts/`.
    /// An absent directory is the valid empty registry.
    ///
    /// A thin wrapper over [`Registry::load_with_witnesses`]: the parsed
    /// registry without the per-file witnesses.
    pub fn load(project_root: &Path) -> Result<Self, RegistryError> {
        Ok(Self::load_with_witnesses(project_root)?.registry)
    }

    /// The R7.5 A2a one-read snapshot: the loaded registry plus one
    /// [`SourceFileWitness`] for each actual `vibefacts/*.toml` file,
    /// sorted and unique by project-relative path. Every file is read
    /// exactly once as raw bytes; the parse consumes those SAME bytes
    /// whose witness is returned, so a consumer can name the registry
    /// bytes it observed without a second read and without any entry
    /// text crossing the boundary. Malformed UTF-8/TOML/schema/entries
    /// remain the existing typed errors — the central ruling is that a
    /// malformed registry aborts the caller, so no partial snapshot is
    /// owed.
    pub fn load_with_witnesses(project_root: &Path) -> Result<RegistrySnapshot, RegistryError> {
        let home = project_root.join(vibe_core::layout::current_vibefacts_root());
        if !home.exists() {
            return Ok(RegistrySnapshot {
                registry: Self::default(),
                witnesses: Vec::new(),
            });
        }
        if !home.is_dir() {
            return Err(RegistryError::InvalidRegistryHome { path: home });
        }

        let mut paths = Vec::new();
        let entries = fs::read_dir(&home).map_err(|source| RegistryError::Io {
            path: home.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| RegistryError::Io {
                path: home.clone(),
                source,
            })?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                paths.push(path);
            }
        }
        paths.sort();

        let mut registry = Self::default();
        let mut witnesses = Vec::with_capacity(paths.len());
        for path in paths {
            // The ONE raw read: witness first, then parse the same bytes.
            let raw = fs::read(&path).map_err(|source| RegistryError::Io {
                path: path.clone(),
                source,
            })?;
            let relative = path.strip_prefix(project_root).map_err(|_| {
                RegistryError::Invariant(format!(
                    "registry source `{}` escaped project root `{}`",
                    path.display(),
                    project_root.display()
                ))
            })?;
            let relative = vibe_core::machine_json_path(relative);
            witnesses.push(SourceFileWitness::of(relative, &raw));
            for fact in parse_file_bytes(&path, &raw)? {
                let expected = fact.registry_file_name()?;
                if path
                    .file_name()
                    .is_none_or(|name| name != expected.as_str())
                {
                    return Err(RegistryError::InvalidEntry {
                        address: fact.address,
                        reason: format!(
                            "entry belongs in `{expected}`, not `{}`",
                            path.file_name()
                                .map(|name| name.to_string_lossy())
                                .unwrap_or_default()
                        ),
                    });
                }
                if let Some(first) = registry.sources.get(&fact.address) {
                    return Err(RegistryError::DuplicateRegistryAddress {
                        address: fact.address,
                        first: first.clone(),
                        second: path,
                    });
                }
                registry.sources.insert(fact.address.clone(), path.clone());
                registry.entries.insert(fact.address.clone(), fact);
            }
        }
        Ok(RegistrySnapshot {
            registry,
            witnesses,
        })
    }

    pub fn entries(&self) -> impl Iterator<Item = &FactEntry> {
        self.entries.values()
    }

    pub fn get(&self, address: &str) -> Option<&FactEntry> {
        self.entries.get(address)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert or replace one entry in its deterministic source file.
    pub fn upsert(&mut self, project_root: &Path, entry: FactEntry) -> Result<(), RegistryError> {
        entry.validate()?;
        let target = project_root
            .join(vibe_core::layout::current_vibefacts_root())
            .join(entry.registry_file_name()?);
        if let Some(previous) = self.sources.get(&entry.address)
            && previous != &target
        {
            return Err(RegistryError::Invariant(format!(
                "address `{}` moved from `{}` to `{}`",
                entry.address,
                previous.display(),
                target.display()
            )));
        }

        let mut facts: BTreeMap<String, FactEntry> = self
            .entries
            .values()
            .filter(|fact| self.sources.get(&fact.address) == Some(&target))
            .map(|fact| (fact.address.clone(), fact.clone()))
            .collect();
        facts.insert(entry.address.clone(), entry.clone());
        write_file(&target, facts.into_values())?;

        self.sources.insert(entry.address.clone(), target);
        self.entries.insert(entry.address.clone(), entry);
        Ok(())
    }

    /// Remove one entry. The source file, and then the registry directory,
    /// are removed when they become empty.
    pub fn remove(&mut self, project_root: &Path, address: &str) -> Result<bool, RegistryError> {
        let Some(source) = self.sources.get(address).cloned() else {
            return Ok(false);
        };
        let remaining: Vec<FactEntry> = self
            .entries
            .values()
            .filter(|fact| fact.address != address)
            .filter(|fact| self.sources.get(&fact.address) == Some(&source))
            .cloned()
            .collect();
        if remaining.is_empty() {
            fs::remove_file(&source).map_err(|error| RegistryError::Io {
                path: source.clone(),
                source: error,
            })?;
        } else {
            write_file(&source, remaining)?;
        }
        self.entries.remove(address);
        self.sources.remove(address);
        remove_empty_home(&project_root.join(vibe_core::layout::current_vibefacts_root()))?;
        Ok(true)
    }
}

/// The one-read registry snapshot returned by
/// [`Registry::load_with_witnesses`]: the parsed registry beside the
/// witnesses of the exact raw bytes it was parsed from. The witnesses
/// are metadata only — no entry text rides along.
#[derive(Debug)]
pub struct RegistrySnapshot {
    /// The parsed, validated registry — exactly what [`Registry::load`]
    /// returns.
    pub registry: Registry,
    /// One witness per actual `vibefacts/*.toml` file read, sorted by
    /// project-relative path.
    pub witnesses: Vec<SourceFileWitness>,
}

/// The reading wrapper the store's own tests use: one raw read, then the
/// shared one-read parse core.
#[cfg(test)]
pub(crate) fn read_file(path: &Path) -> Result<Vec<FactEntry>, RegistryError> {
    let raw = fs::read(path).map_err(|source| RegistryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_file_bytes(path, &raw)
}

/// Parse registry entries from the SAME raw bytes a caller witnessed —
/// the one-read core both [`read_file`] and
/// [`Registry::load_with_witnesses`] feed. Invalid UTF-8 keeps the
/// existing typed shape it had through `read_to_string`: an
/// [`RegistryError::Io`] with an `InvalidData` source.
fn parse_file_bytes(path: &Path, raw: &[u8]) -> Result<Vec<FactEntry>, RegistryError> {
    let text = std::str::from_utf8(raw).map_err(|source| {
        let io = std::io::Error::new(std::io::ErrorKind::InvalidData, source);
        RegistryError::Io {
            path: path.to_path_buf(),
            source: io,
        }
    })?;
    let wire: RegistryFile = toml::from_str(text).map_err(|source| RegistryError::TomlRead {
        path: path.to_path_buf(),
        source,
    })?;
    if wire.schema != SCHEMA {
        return Err(RegistryError::UnsupportedSchema {
            path: path.to_path_buf(),
            schema: wire.schema,
        });
    }
    let mut seen = BTreeSet::new();
    for fact in &wire.facts {
        fact.validate()?;
        if !seen.insert(fact.address.clone()) {
            return Err(RegistryError::DuplicateAddress {
                path: path.to_path_buf(),
                address: fact.address.clone(),
            });
        }
    }
    Ok(wire.facts)
}

pub(crate) fn write_file(
    path: &Path,
    facts: impl IntoIterator<Item = FactEntry>,
) -> Result<(), RegistryError> {
    let mut sorted = BTreeMap::new();
    for fact in facts {
        fact.validate()?;
        let address = fact.address.clone();
        if sorted.insert(address.clone(), fact).is_some() {
            return Err(RegistryError::DuplicateAddress {
                path: path.to_path_buf(),
                address,
            });
        }
    }
    let wire = RegistryFile {
        schema: SCHEMA,
        facts: sorted.into_values().collect(),
    };
    let mut text = toml::to_string_pretty(&wire)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, text).map_err(|source| RegistryError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn remove_empty_home(home: &Path) -> Result<(), RegistryError> {
    if !home.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(home).map_err(|source| RegistryError::Io {
        path: home.to_path_buf(),
        source,
    })?;
    if entries.next().is_none() {
        fs::remove_dir(home).map_err(|source| RegistryError::Io {
            path: home.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}
