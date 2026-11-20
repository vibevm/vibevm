//! Fixed trusted participant reference index; metadata-only, no value access.
use super::*;
use sha2::{Digest, Sha256};
const LIMIT: usize = 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceRow {
    pub current_revision: SecretId,
    pub secret_ref: SecretId,
    pub metadata_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceIndex {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<ReferenceSelection>,
    pub schema: u32,
    pub store_revision: u64,
    pub items: BTreeMap<SecretId, ReferenceRow>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReferenceSelection {
    None,
    Selected { connection: SecretId },
}

#[derive(Debug, Clone)]
pub struct ReferenceSnapshot {
    pub index: ReferenceIndex,
    /// Bounded nonsecret consumer metadata only, never keyring value bytes.
    pub metadata: BTreeMap<SecretId, Vec<u8>>,
}
impl Default for ReferenceIndex {
    fn default() -> Self {
        Self {
            selection: None,
            schema: 1,
            store_revision: 0,
            items: BTreeMap::new(),
        }
    }
}
fn owner(id: &str) -> Result<(), Error> {
    if id != "ai" {
        return Err(Error::RefusedUnknown);
    }
    Ok(())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn revision(root: &Pinned, id: &SecretId, rev: &SecretId, create: bool) -> Result<Pinned, Error> {
    let connections = root
        .private_child("connections", create)
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    let dir = if create {
        connections
            .create_private_child_exclusive(id.as_str())
            .map_err(|_| Error::Conflict)?
    } else {
        connections
            .private_child(id.as_str(), false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
    };
    if create {
        dir.create_private_child_exclusive(rev.as_str())
            .map_err(|_| Error::Conflict)
    } else {
        dir.private_child(rev.as_str(), false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)
    }
}
pub(super) fn read(root: &Pinned) -> Result<ReferenceSnapshot, Error> {
    let bytes = root
        .read_private_metadata("index.toml", LIMIT)
        .map_err(|_| Error::RefusedUnknown)?
        .ok_or(Error::RefusedUnknown)?;
    let index: ReferenceIndex =
        toml::from_str(std::str::from_utf8(&bytes).map_err(|_| Error::RefusedUnknown)?)
            .map_err(|_| Error::RefusedUnknown)?;
    match (index.schema, &index.selection) {
        (1, None) | (2, Some(ReferenceSelection::None)) => {}
        (2, Some(ReferenceSelection::Selected { connection }))
            if index.items.contains_key(connection) => {}
        _ => return Err(Error::RefusedUnknown),
    }
    let mut metadata = BTreeMap::new();
    let mut total = 0usize;
    for (id, row) in &index.items {
        let dir = revision(root, id, &row.current_revision, false)?;
        let bytes = dir
            .read_private_metadata("config.toml", LIMIT)
            .map_err(|_| Error::RefusedUnknown)?
            .ok_or(Error::RefusedUnknown)?;
        total = total
            .checked_add(bytes.len())
            .ok_or(Error::RefusedUnknown)?;
        if total > 4 * LIMIT {
            return Err(Error::RefusedUnknown);
        }
        if row.metadata_sha256 != digest(&bytes) {
            return Err(Error::RefusedUnknown);
        }
        metadata.insert(id.clone(), bytes);
    }
    Ok(ReferenceSnapshot { index, metadata })
}
fn encode(index: &ReferenceIndex) -> Result<String, Error> {
    let bytes = toml::to_string(index).map_err(|_| Error::Invalid)?;
    if bytes.len() > LIMIT {
        return Err(Error::Invalid);
    }
    Ok(bytes)
}
impl Store {
    /// Observe a fixed trusted consumer inventory; a registered absent index is
    /// unknown. Never creates a root, locks or timestamps.
    pub fn reference_snapshot(&self, participant: &str) -> Result<ReferenceSnapshot, Error> {
        owner(participant)?;
        let Some((project, root)) = self.open(false)? else {
            return Ok(ReferenceSnapshot {
                index: ReferenceIndex::default(),
                metadata: BTreeMap::new(),
            });
        };
        let index = Self::load(&project, &root)?;
        let registered = index
            .participants
            .as_ref()
            .is_some_and(|p| p.iter().any(|p| p == participant));
        match root
            .private_child(participant, false)
            .map_err(|_| Error::RefusedUnknown)?
        {
            Some(dir) => {
                if !registered {
                    return Err(Error::RefusedUnknown);
                }
                read(&dir)
            }
            None if !registered => Ok(ReferenceSnapshot {
                index: ReferenceIndex::default(),
                metadata: BTreeMap::new(),
            }),
            None => Err(Error::RefusedUnknown),
        }
    }
    /// Trusted in-process writer constrained to a fixed descriptor, fresh
    /// consumer metadata and its authoritative reference row. This is not caller
    /// authentication or proof that supplied bytes are nonsecret. Trusted owner
    /// code must classify input; never expose this as a generic tool operation.
    /// No caller root/paths/value handle or callback is accepted.
    pub fn create_reference(
        &self,
        participant: &str,
        secret: &SecretId,
        metadata: &[u8],
    ) -> Result<SecretId, Error> {
        owner(participant)?;
        if metadata.is_empty() || metadata.len() > LIMIT {
            return Err(Error::Invalid);
        }
        let (project, root) = self.open(false)?.ok_or(Error::NotFound)?;
        let _inventory = root
            .private_lock("inventory.lock")
            .map_err(|_| Error::Unavailable)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let mut store = Self::load(&project, &root)?;
        if store.schema != 2 || store.reference_protocol != Some(1) {
            return Err(Error::RefusedUnknown);
        }
        if !store.items.contains_key(secret) {
            return Err(Error::NotFound);
        }
        let participants = store.participants.as_mut().ok_or(Error::RefusedUnknown)?;
        if participants.iter().any(|p| p != "ai") {
            return Err(Error::RefusedUnknown);
        }
        let registered = participants.iter().any(|p| p == participant);
        let dir = if registered {
            root.private_child(participant, false)
                .map_err(|_| Error::RefusedUnknown)?
                .ok_or(Error::RefusedUnknown)?
        } else {
            // Durable registration precedes any binding. Failure after this
            // point deliberately leaves an unknown owner until explicit repair.
            if root
                .private_child(participant, false)
                .map_err(|_| Error::RefusedUnknown)?
                .is_some()
            {
                return Err(Error::RefusedUnknown);
            }
            participants.push(participant.into());
            participants.sort();
            store.store_revision = store.store_revision.checked_add(1).ok_or(Error::Invalid)?;
            let bytes = Self::prepare_index(&store)?;
            project
                .write_atomic_in_with_mode(&root, "index.toml", bytes.as_bytes(), Some(0o600))
                .map_err(|_| Error::Publication)?;
            let dir = root
                .create_private_child_exclusive(participant)
                .map_err(|_| Error::Unavailable)?;
            let bytes = encode(&ReferenceIndex::default())?;
            project
                .publish_new_in_with_mode(&dir, "index.toml", bytes.as_bytes(), Some(0o600))
                .map_err(|_| Error::Publication)?;
            dir
        };
        let mut snapshot = read(&dir)?;
        let id = SecretId::generate()?;
        let revision_id = SecretId::generate()?;
        snapshot.index.store_revision = snapshot
            .index
            .store_revision
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        snapshot.index.items.insert(
            id.clone(),
            ReferenceRow {
                current_revision: revision_id.clone(),
                secret_ref: secret.clone(),
                metadata_sha256: digest(metadata),
            },
        );
        let total = snapshot
            .metadata
            .values()
            .try_fold(metadata.len(), |sum, bytes| sum.checked_add(bytes.len()))
            .ok_or(Error::Invalid)?;
        if total > 4 * LIMIT {
            return Err(Error::Invalid);
        }
        let bytes = encode(&snapshot.index)?;
        let leaf = revision(&dir, &id, &revision_id, true)?;
        project
            .publish_new_in_with_mode(&leaf, "config.toml", metadata, Some(0o600))
            .map_err(|_| Error::Publication)?;
        project
            .write_atomic_in_with_mode(&dir, "index.toml", bytes.as_bytes(), Some(0o600))
            .map_err(|_| Error::Publication)?;
        Ok(id)
    }
    /// Explicit unbinding retains owner registration, immutable config and secret.
    pub fn remove_reference(
        &self,
        participant: &str,
        id: &SecretId,
        expected: &Expected,
    ) -> Result<(), Error> {
        owner(participant)?;
        let (project, root) = self.open(false)?.ok_or(Error::NotFound)?;
        let _inventory = root
            .private_lock("inventory.lock")
            .map_err(|_| Error::Unavailable)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let store = Self::load(&project, &root)?;
        if store.schema != 2
            || store.reference_protocol != Some(1)
            || !store
                .participants
                .as_ref()
                .is_some_and(|p| p.iter().any(|p| p == participant))
        {
            return Err(Error::RefusedUnknown);
        }
        let dir = root
            .private_child(participant, false)
            .map_err(|_| Error::RefusedUnknown)?
            .ok_or(Error::RefusedUnknown)?;
        let mut snapshot = read(&dir)?;
        let row = snapshot.index.items.get(id).ok_or(Error::NotFound)?;
        if snapshot.index.store_revision != expected.store_revision
            || row.current_revision != expected.current_revision
        {
            return Err(Error::Conflict);
        }
        if matches!(&snapshot.index.selection, Some(ReferenceSelection::Selected { connection }) if connection == id)
        {
            return Err(Error::RefusedReferenced {
                consumers: vec!["ai:default".into()],
            });
        }
        snapshot.index.items.remove(id);
        snapshot.index.store_revision = snapshot
            .index
            .store_revision
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let bytes = encode(&snapshot.index)?;
        project
            .write_atomic_in_with_mode(&dir, "index.toml", bytes.as_bytes(), Some(0o600))
            .map_err(|_| Error::Publication)?;
        Ok(())
    }
    /// Select only metadata; never opts a harness into managed authentication.
    pub fn select_reference(
        &self,
        participant: &str,
        selected: Option<&SecretId>,
        expected_store: u64,
    ) -> Result<ReferenceIndex, Error> {
        owner(participant)?;
        let Some((project, root)) = self.open(false)? else {
            if expected_store == 0 && selected.is_none() {
                return Ok(ReferenceIndex::default());
            }
            return Err(Error::NotFound);
        };
        let _inventory = root
            .private_lock("inventory.lock")
            .map_err(|_| Error::Unavailable)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let store = Self::load(&project, &root)?;
        if store.schema != 2 || store.reference_protocol != Some(1) {
            return Err(Error::RefusedUnknown);
        }
        let registered = store
            .participants
            .as_ref()
            .is_some_and(|p| p.iter().any(|p| p == participant));
        if !registered {
            if root
                .private_child(participant, false)
                .map_err(|_| Error::RefusedUnknown)?
                .is_some()
            {
                return Err(Error::RefusedUnknown);
            }
            if expected_store == 0 && selected.is_none() {
                return Ok(ReferenceIndex::default());
            }
            return Err(Error::NotFound);
        }
        let dir = root
            .private_child(participant, false)
            .map_err(|_| Error::RefusedUnknown)?
            .ok_or(Error::RefusedUnknown)?;
        let mut snapshot = read(&dir)?;
        if snapshot.index.store_revision != expected_store {
            return Err(Error::Conflict);
        }
        if let Some(id) = selected
            && !snapshot.index.items.contains_key(id)
        {
            return Err(Error::NotFound);
        }
        let current = match &snapshot.index.selection {
            Some(ReferenceSelection::Selected { connection }) => Some(connection),
            _ => None,
        };
        if current == selected {
            return Ok(snapshot.index);
        }
        snapshot.index.schema = 2;
        snapshot.index.selection = Some(match selected {
            Some(id) => ReferenceSelection::Selected {
                connection: id.clone(),
            },
            None => ReferenceSelection::None,
        });
        snapshot.index.store_revision = snapshot
            .index
            .store_revision
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let bytes = encode(&snapshot.index)?;
        project
            .write_atomic_in_with_mode(&dir, "index.toml", bytes.as_bytes(), Some(0o600))
            .map_err(|_| Error::Publication)?;
        Ok(snapshot.index)
    }
}
