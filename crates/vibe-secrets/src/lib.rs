//! Generic plaintext-at-rest keyring; references are not authorization.
//! Values never have a public read, Display or serialized metadata surface.
//! This protects filesystem access, not unrestricted same-user processes.
#![deny(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vibe_safefs::{Pinned, Project};
mod id;
mod mutations;
mod references;
mod roster;
pub use id::SecretId;
pub use mutations::{Expected, MetadataSnapshot, ReferenceGuard, ReferenceInventory};
pub use references::{
    CodexNativePreview, PiNativePreview, QwenNativePreview, ReferenceBinding, ReferenceIndex, ReferenceRow, ReferenceSelection, ReferenceSnapshot,
};

const VALUE_LIMIT: usize = 1024 * 1024;
const INDEX_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("private keyring storage is unavailable or failed protection checks")]
    Unavailable,
    #[error("invalid keyring identifier, metadata or bounded value")]
    Invalid,
    #[error("keyring item not found")]
    NotFound,
    #[error("keyring revision changed; refresh metadata and retry")]
    Conflict,
    #[error("keyring item has live consumer references")]
    RefusedReferenced { consumers: Vec<String> },
    #[error("complete protected reference inventory is unavailable")]
    RefusedUnknown,
    #[error(
        "keyring publication failed; previous selected revision is retained unless the index publication completed"
    )]
    Publication,
}

/// Owned explicit input. No serialization, raw-value accessor or derived Debug.
pub struct ProtectedValue(Vec<u8>);
impl ProtectedValue {
    pub fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.len() > VALUE_LIMIT {
            return Err(Error::Invalid);
        }
        Ok(Self(bytes))
    }
}
impl std::fmt::Debug for ProtectedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProtectedValue([REDACTED])")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemMetadata {
    pub current_revision: SecretId,
    pub kind: String,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: u32,
    store_revision: u64,
    #[serde(default)]
    items: BTreeMap<SecretId, ItemMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reference_protocol: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    participants: Option<Vec<String>>,
}
impl Default for Index {
    fn default() -> Self {
        Self {
            schema: 1,
            store_revision: 0,
            items: BTreeMap::new(),
            reference_protocol: None,
            participants: None,
        }
    }
}

/// Root is injected by a trusted composition boundary, never inferred from cwd.
#[derive(Debug)]
pub struct Store {
    root: PathBuf,
}
impl Store {
    /// Select exactly `<effective user settings>/secrets`; no fallback or discovery.
    pub fn at_settings(settings: &Path) -> Result<Self, Error> {
        if !settings.is_absolute() {
            return Err(Error::Invalid);
        }
        Ok(Self {
            root: settings.join("secrets"),
        })
    }

    fn open(&self, create: bool) -> Result<Option<(Project, Pinned)>, Error> {
        Ok(self
            .open_recording(create)?
            .map(|(project, root, _)| (project, root)))
    }

    fn open_recording(&self, create: bool) -> Result<Option<(Project, Pinned, bool)>, Error> {
        let parent = self.root.parent().ok_or(Error::Invalid)?;
        let settings = Project::open(parent).map_err(|_| Error::Unavailable)?;
        settings
            .root_dir()
            .map_err(|_| Error::Unavailable)?
            .verify_user_owned_parent()
            .map_err(|_| Error::Unavailable)?;
        let Some((root, created)) = settings
            .root_dir()
            .map_err(|_| Error::Unavailable)?
            .private_child_recording("secrets", create)
            .map_err(|_| Error::Unavailable)?
        else {
            return Ok(None);
        };
        let project = Project::open(root.path()).map_err(|_| Error::Unavailable)?;
        if project.root_identity().map_err(|_| Error::Unavailable)?
            != root.identity().map_err(|_| Error::Unavailable)?
        {
            return Err(Error::Unavailable);
        }
        Ok(Some((project, root, created)))
    }

    fn load(_project: &Project, root: &Pinned) -> Result<Index, Error> {
        if root
            .verify_private_leaf("index.toml", INDEX_LIMIT as u64)
            .map_err(|_| Error::Unavailable)?
            .is_none()
        {
            return Ok(Index::default());
        }
        let bytes = root
            .read_private_metadata("index.toml", INDEX_LIMIT)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let index: Index = toml::from_str(std::str::from_utf8(&bytes).map_err(|_| Error::Invalid)?)
            .map_err(|_| Error::Invalid)?;
        roster::validate_format(&index)?;
        for (id, item) in &index.items {
            validate_metadata(&item.kind, &item.label)?;
            let leaf = open_revision(root, id, &item.current_revision, false)?;
            if leaf
                .verify_private_leaf("value", VALUE_LIMIT as u64)
                .map_err(|_| Error::Unavailable)?
                .is_none()
            {
                return Err(Error::Unavailable);
            }
        }
        Ok(index)
    }

    /// Read-only metadata snapshot; never creates a root, lock or last-used file.
    pub fn list(&self) -> Result<BTreeMap<SecretId, ItemMetadata>, Error> {
        let Some((project, root)) = self.open(false)? else {
            return Ok(BTreeMap::new());
        };
        Ok(Self::load(&project, &root)?.items)
    }
    pub fn details(&self, id: &SecretId) -> Result<ItemMetadata, Error> {
        self.list()?.remove(id).ok_or(Error::NotFound)
    }
    pub fn create(
        &self,
        kind: &str,
        label: &str,
        value: ProtectedValue,
    ) -> Result<SecretId, Error> {
        validate_metadata(kind, label)?;
        let (project, root, created) = self.open_recording(true)?.ok_or(Error::Unavailable)?;
        let _inventory = root
            .private_lock("inventory.lock")
            .map_err(|_| Error::Unavailable)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let mut index = Self::load(&project, &root)?;
        if created
            && root
                .private_entries_are_only(&["inventory.lock", "store.lock"])
                .map_err(|_| Error::Unavailable)?
        {
            index.schema = 2;
            index.reference_protocol = Some(1);
            index.participants = Some(Vec::new());
            let bytes = Self::prepare_index(&index)?;
            project
                .write_atomic_in_with_mode(&root, "index.toml", bytes.as_bytes(), Some(0o600))
                .map_err(|_| Error::Publication)?;
        }
        let id = SecretId::generate()?;
        if index.items.contains_key(&id) {
            return Err(Error::Conflict);
        }
        let items = root
            .private_child("items", true)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        items
            .create_private_child_exclusive(id.as_str())
            .map_err(|_| Error::Conflict)?;
        let revision = SecretId::generate()?;
        Self::publish(
            &project,
            &root,
            &mut index,
            &id,
            ItemMetadata {
                current_revision: revision,
                kind: kind.into(),
                label: label.into(),
            },
            &value,
        )?;
        Ok(id)
    }
    pub fn rotate(
        &self,
        id: &SecretId,
        expected: &SecretId,
        value: ProtectedValue,
    ) -> Result<SecretId, Error> {
        let (project, root) = self.open(false)?.ok_or(Error::NotFound)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let mut index = Self::load(&project, &root)?;
        let current = index.items.get(id).ok_or(Error::NotFound)?.clone();
        if &current.current_revision != expected {
            return Err(Error::Conflict);
        }
        let revision = SecretId::generate()?;
        Self::publish(
            &project,
            &root,
            &mut index,
            id,
            ItemMetadata {
                current_revision: revision.clone(),
                ..current
            },
            &value,
        )?;
        Ok(revision)
    }
    fn publish(
        project: &Project,
        root: &Pinned,
        index: &mut Index,
        id: &SecretId,
        metadata: ItemMetadata,
        value: &ProtectedValue,
    ) -> Result<(), Error> {
        let mut proposed = index.clone();
        proposed.store_revision = proposed
            .store_revision
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        proposed.items.insert(id.clone(), metadata.clone());
        let bytes = Self::prepare_index(&proposed)?;
        let leaf = open_revision(root, id, &metadata.current_revision, true)?;
        project
            .publish_new_in_with_mode(&leaf, "value", &value.0, Some(0o600))
            .map_err(|_| Error::Publication)?;
        leaf.verify_private_leaf("value", VALUE_LIMIT as u64)
            .map_err(|_| Error::Unavailable)?;
        Self::commit_index(project, root, index, proposed, &bytes)
    }

    fn prepare_index(proposed: &Index) -> Result<String, Error> {
        let bytes = toml::to_string(proposed).map_err(|_| Error::Invalid)?;
        #[cfg(test)]
        let limit = INDEX_WRITE_LIMIT.with(|limit| limit.get());
        #[cfg(not(test))]
        let limit = INDEX_LIMIT;
        if bytes.len() > limit {
            return Err(Error::Invalid);
        }
        let checked: Index = toml::from_str(&bytes).map_err(|_| Error::Invalid)?;
        roster::validate_format(&checked)?;
        if checked.store_revision != proposed.store_revision {
            return Err(Error::Invalid);
        }
        Ok(bytes)
    }

    fn commit_index(
        project: &Project,
        root: &Pinned,
        index: &mut Index,
        proposed: Index,
        bytes: &str,
    ) -> Result<(), Error> {
        #[cfg(test)]
        if FAIL_BEFORE_INDEX.with(|flag| flag.replace(false)) {
            return Err(Error::Publication);
        }

        project
            .write_atomic_in_with_mode(root, "index.toml", bytes.as_bytes(), Some(0o600))
            .map_err(|_| Error::Publication)?;
        *index = proposed;
        Ok(())
    }
}
fn validate_metadata(kind: &str, label: &str) -> Result<(), Error> {
    if kind.is_empty()
        || kind.len() > 128
        || label.is_empty()
        || label.len() > 256
        || kind.chars().chain(label.chars()).any(char::is_control)
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn open_revision(
    root: &Pinned,
    id: &SecretId,
    revision: &SecretId,
    create: bool,
) -> Result<Pinned, Error> {
    let mut dir = root
        .private_child("items", create)
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    dir = dir
        .private_child(id.as_str(), create)
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    dir = if create {
        dir.create_private_child_exclusive(revision.as_str())
            .map_err(|_| Error::Conflict)?
    } else {
        dir.private_child(revision.as_str(), false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
    };
    Ok(dir)
}

#[cfg(test)]
thread_local! { static FAIL_BEFORE_INDEX: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
#[cfg(test)]
mod tests;

#[cfg(test)]
thread_local! { static INDEX_WRITE_LIMIT: std::cell::Cell<usize> = const { std::cell::Cell::new(INDEX_LIMIT) }; }

#[cfg(test)]
mod tests_index_limit;

#[cfg(all(test, unix))]
mod roster_tests;
