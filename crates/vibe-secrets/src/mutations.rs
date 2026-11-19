//! Metadata mutations share the immutable-value owner's lock and publication.
use super::*;

/// One metadata-only snapshot: tokens and rows are observed together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataSnapshot {
    pub store_revision: u64,
    pub items: BTreeMap<SecretId, ItemMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub store_revision: u64,
    pub current_revision: SecretId,
}

/// Trusted composition must attest the COMPLETE registered consumer roster.
/// Acquire before the store lock, hold through publication. All reference writers
/// must take the same inventory lock first and check the item still exists before
/// inserting a binding. No callback may reenter Store while its lock is held.
/// An explicitly attested empty roster is valid; missing inventory is unknown.
/// Implementations must not discover consumers by scanning HOME or the filesystem.
pub trait ReferenceInventory {
    fn acquire(&self) -> Result<Box<dyn ReferenceGuard + '_>, Error>;
}

/// Protection stays live until this guard is dropped, including after publication.
/// Query returns only registered nonsecret consumer IDs, never paths or values.
pub trait ReferenceGuard {
    fn consumers(&self, id: &SecretId) -> Result<Vec<String>, Error>;
}

impl Store {
    pub fn snapshot(&self) -> Result<MetadataSnapshot, Error> {
        let index = match self.open(false)? {
            Some((project, root)) => Self::load(&project, &root)?,
            None => Index::default(),
        };
        Ok(MetadataSnapshot {
            store_revision: index.store_revision,
            items: index.items,
        })
    }

    /// Kind and value identity remain immutable. A token-valid no-op does not write.
    pub fn edit_label(
        &self,
        id: &SecretId,
        expected: &Expected,
        label: &str,
    ) -> Result<MetadataSnapshot, Error> {
        let (project, root) = self.open(false)?.ok_or(Error::NotFound)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let mut index = Self::load(&project, &root)?;
        let current = check_expected(&index, id, expected)?;
        validate_metadata(&current.kind, label)?;
        if current.label != label {
            let mut proposed = index.clone();
            proposed.items.get_mut(id).ok_or(Error::NotFound)?.label = label.into();
            advance(&mut proposed)?;
            let bytes = Self::prepare_index(&proposed)?;
            Self::commit_index(&project, &root, &mut index, proposed, &bytes)?;
        }
        Ok(MetadataSnapshot {
            store_revision: index.store_revision,
            items: index.items,
        })
    }

    /// Logical deletion only; immutable private paths stay reserved, not erased.
    pub fn remove(
        &self,
        id: &SecretId,
        expected: &Expected,
        inventory: Option<&dyn ReferenceInventory>,
    ) -> Result<MetadataSnapshot, Error> {
        let guard = inventory
            .ok_or(Error::RefusedUnknown)?
            .acquire()
            .map_err(|_| Error::RefusedUnknown)?;
        // Collect BEFORE store lock: callbacks cannot reenter while it is held.
        // The acquired guard protects this complete inventory until we return.
        let mut consumers = guard.consumers(id).map_err(|_| Error::RefusedUnknown)?;
        if consumers.len() > 4096
            || consumers.iter().any(|id| {
                id.is_empty()
                    || id.len() > 128
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-:".contains(&b))
            })
        {
            return Err(Error::RefusedUnknown);
        }
        consumers.sort();
        consumers.dedup();
        let (project, root) = self.open(false)?.ok_or(Error::NotFound)?;
        let _lock = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let mut index = Self::load(&project, &root)?;
        check_expected(&index, id, expected)?;
        if !consumers.is_empty() {
            return Err(Error::RefusedReferenced { consumers });
        }
        let mut proposed = index.clone();
        proposed.items.remove(id);
        advance(&mut proposed)?;
        let bytes = Self::prepare_index(&proposed)?;
        Self::commit_index(&project, &root, &mut index, proposed, &bytes)?;
        Ok(MetadataSnapshot {
            store_revision: index.store_revision,
            items: index.items,
        })
    }
}

fn check_expected<'a>(
    index: &'a Index,
    id: &SecretId,
    expected: &Expected,
) -> Result<&'a ItemMetadata, Error> {
    let item = index.items.get(id).ok_or(Error::NotFound)?;
    if index.store_revision != expected.store_revision
        || item.current_revision != expected.current_revision
    {
        return Err(Error::Conflict);
    }
    Ok(item)
}
fn advance(index: &mut Index) -> Result<(), Error> {
    index.store_revision = index.store_revision.checked_add(1).ok_or(Error::Invalid)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
