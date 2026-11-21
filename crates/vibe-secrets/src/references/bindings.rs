//! Harness binding writes under the incumbent reference owner: one row or one
//! all-or-nothing set, each a single index publication.
use super::*;

impl Store {
    /// Trusted canonical metadata binding action; absence resets only this row.
    pub fn bind_reference(
        &self,
        participant: &str,
        harness: &str,
        binding: Option<ReferenceBinding>,
        expected: u64,
    ) -> Result<ReferenceIndex, Error> {
        owner(participant)?;
        if !known_reference_harness(harness) {
            return Err(Error::Invalid);
        }
        if binding.is_some() && !reference_binding_supported(harness) {
            return Err(Error::Invalid);
        }
        let Some((project, root)) = self.open(false)? else {
            if binding.is_none() && expected == 0 {
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
            if binding.is_none()
                && expected == 0
                && root
                    .private_child(participant, false)
                    .map_err(|_| Error::RefusedUnknown)?
                    .is_none()
            {
                return Ok(ReferenceIndex::default());
            }
            return Err(Error::RefusedUnknown);
        }
        let dir = root
            .private_child(participant, false)
            .map_err(|_| Error::RefusedUnknown)?
            .ok_or(Error::RefusedUnknown)?;
        let mut snapshot = read(&dir)?;
        if snapshot.index.store_revision != expected {
            return Err(Error::Conflict);
        }
        if let Some(ReferenceBinding::Connection { connection }) = &binding
            && !snapshot.index.items.contains_key(connection)
        {
            return Err(Error::NotFound);
        }
        if let Some(binding) = &binding {
            compatibility::binding_api(&snapshot, harness, binding)?;
        }
        let current = snapshot
            .index
            .bindings
            .as_ref()
            .and_then(|map| map.get(harness));
        if current == binding.as_ref() {
            return Ok(snapshot.index);
        }
        snapshot.index.schema = 3;
        if snapshot.index.selection.is_none() {
            snapshot.index.selection = Some(ReferenceSelection::None);
        }
        let map = snapshot.index.bindings.get_or_insert_with(BTreeMap::new);
        match binding {
            Some(value) => {
                map.insert(harness.into(), value);
            }
            None => {
                map.remove(harness);
            }
        }
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
    /// One trusted batch, not sequential per-agent mutation. The index snapshot
    /// is re-admitted under both locks before validating or publishing any row.
    pub fn bind_references_bulk(
        &self,
        participant: &str,
        edits: &BTreeMap<String, ReferenceBinding>,
        expected_digest: &str,
    ) -> Result<ReferenceIndex, Error> {
        owner(participant)?;
        if edits.is_empty() || edits.keys().any(|name| !reference_binding_supported(name)) {
            return Err(Error::Invalid);
        }
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
        let snapshot = read(&dir)?;
        if snapshot.index_sha256 != expected_digest {
            return Err(Error::Conflict);
        }
        for (harness, binding) in edits {
            let id = match binding {
                ReferenceBinding::Connection { connection } => connection,
                ReferenceBinding::InheritDefault => match &snapshot.index.selection {
                    Some(ReferenceSelection::Selected { connection }) => connection,
                    _ => return Err(Error::NotFound),
                },
            };
            let row = snapshot.index.items.get(id).ok_or(Error::NotFound)?;
            if !store.items.contains_key(&row.secret_ref) {
                return Err(Error::RefusedUnknown);
            }
            compatibility::binding_api(&snapshot, harness, binding)?;
        }
        let mut proposed = snapshot.index;
        let changed = edits
            .iter()
            .any(|(h, b)| proposed.bindings.as_ref().and_then(|m| m.get(h)) != Some(b));
        if !changed {
            return Ok(proposed);
        }
        proposed.schema = 3;
        if proposed.selection.is_none() {
            proposed.selection = Some(ReferenceSelection::None);
        }
        let bindings = proposed.bindings.get_or_insert_with(BTreeMap::new);
        for (h, b) in edits {
            bindings.insert(h.clone(), b.clone());
        }
        proposed.store_revision = proposed
            .store_revision
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let bytes = encode(&proposed)?;
        #[cfg(test)]
        if crate::FAIL_BEFORE_INDEX.with(|flag| flag.replace(false)) {
            return Err(Error::Publication);
        }
        project
            .write_atomic_in_with_mode(&dir, "index.toml", bytes.as_bytes(), Some(0o600))
            .map_err(|_| Error::Publication)?;
        Ok(proposed)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn bulk_interruption_and_invalid_last_row_preserve_both_indexes() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let store = Store::at_settings(dir.path()).unwrap();
        let key = store
            .create(
                "api-key",
                "Synthetic",
                ProtectedValue::new(b"SYNTHETIC".to_vec()).unwrap(),
            )
            .unwrap();
        let connection = store
            .create_reference("ai", &key, b"api_kind = 'openai-chat-completions'\n")
            .unwrap();
        let snapshot = store.reference_snapshot("ai").unwrap();
        let path = dir.path().join("secrets/ai/index.toml");
        let before = std::fs::read(&path).unwrap();
        let keys = std::fs::read(dir.path().join("secrets/index.toml")).unwrap();
        let edits = BTreeMap::from([
            (
                "pi".into(),
                ReferenceBinding::Connection {
                    connection: connection.clone(),
                },
            ),
            ("qwen".into(), ReferenceBinding::Connection { connection }),
        ]);
        crate::FAIL_BEFORE_INDEX.with(|f| f.set(true));
        assert_eq!(
            store.bind_references_bulk("ai", &edits, &snapshot.index_sha256),
            Err(Error::Publication)
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(
            std::fs::read(dir.path().join("secrets/index.toml")).unwrap(),
            keys
        );
        let mut invalid = edits;
        invalid.insert(
            "qwen".into(),
            ReferenceBinding::Connection {
                connection: SecretId::generate().unwrap(),
            },
        );
        assert_eq!(
            store.bind_references_bulk("ai", &invalid, &snapshot.index_sha256),
            Err(Error::NotFound)
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
