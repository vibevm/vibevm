//! One atomic selection/managed-binding update under the incumbent reference owner.
use super::*;

impl Store {
    /// Apply one exact metadata subject, never sequential default/binding writes.
    pub fn select_reference_scope(
        &self,
        selected: &SecretId,
        replace_managed: bool,
        expected_digest: &str,
    ) -> Result<ReferenceIndex, Error> {
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
                .is_some_and(|p| p.iter().any(|p| p == "ai"))
        {
            return Err(Error::RefusedUnknown);
        }
        let dir = root
            .private_child("ai", false)
            .map_err(|_| Error::RefusedUnknown)?
            .ok_or(Error::RefusedUnknown)?;
        let snapshot = read(&dir)?;
        if snapshot.index_sha256 != expected_digest {
            return Err(Error::Conflict);
        }
        let row = snapshot.index.items.get(selected).ok_or(Error::NotFound)?;
        if !store.items.contains_key(&row.secret_ref) {
            return Err(Error::RefusedUnknown);
        }
        let original = snapshot.index;
        let mut proposed = original.clone();
        proposed.schema = proposed.schema.max(2);
        proposed.selection = Some(ReferenceSelection::Selected {
            connection: selected.clone(),
        });
        if replace_managed && let Some(bindings) = proposed.bindings.as_mut() {
            for (harness, binding) in bindings {
                if matches!(harness.as_str(), "pi" | "qwen") {
                    *binding = ReferenceBinding::InheritDefault;
                }
            }
        }
        if proposed == original {
            return Ok(original);
        }
        proposed.store_revision = proposed
            .store_revision
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let bytes = encode(&proposed)?;
        project
            .write_atomic_in_with_mode(&dir, "index.toml", bytes.as_bytes(), Some(0o600))
            .map_err(|_| Error::Publication)?;
        Ok(proposed)
    }
}
