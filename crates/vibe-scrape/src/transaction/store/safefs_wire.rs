use vibe_wire::generated::scrape::e2::retirement_checkpoint::{
    CleanupIntent as CleanupIntentWire, EntryKind as SafefsEntryKindWire,
    EntryState as SafefsEntryStateWire, RetirementCheckpoint as RetirementWire,
    SafefsManifest as SafefsManifestWire, TreeEntry as SafefsTreeEntryWire,
};
use vibe_wire::generated::scrape::e3::transaction_owner::TransactionOwner as OwnerWire;

fn safefs_entry_state_to_wire(value: &EntryState) -> SafefsEntryStateWire {
    SafefsEntryStateWire {
        kind: match value.kind {
            EntryStateKind::File => SafefsEntryKindWire::File,
            EntryStateKind::Directory => SafefsEntryKindWire::Directory,
        },
        sha256: value.sha256.clone(),
        bytes: value.bytes,
        unix_mode: value.unix_mode,
        identity: value.identity.as_str().to_owned(),
    }
}

fn safefs_entry_state_from_wire(
    value: SafefsEntryStateWire,
) -> Result<EntryState, TransactionError> {
    Ok(EntryState {
        kind: match value.kind {
            SafefsEntryKindWire::File => EntryStateKind::File,
            SafefsEntryKindWire::Directory => EntryStateKind::Directory,
        },
        sha256: value.sha256,
        bytes: value.bytes,
        unix_mode: value.unix_mode,
        identity: EntryIdentity::from_token(&value.identity)
            .map_err(|error| store_error(format!("invalid retirement entry identity: {error:#}")))?,
    })
}

fn safefs_manifest_to_wire(value: &SafefsTreeManifest) -> SafefsManifestWire {
    SafefsManifestWire {
        digest: value.digest.clone(),
        entries: value
            .entries
            .iter()
            .map(|entry| SafefsTreeEntryWire {
                path: entry.path.clone(),
                state: safefs_entry_state_to_wire(&entry.state),
            })
            .collect(),
    }
}

fn safefs_manifest_from_wire(
    value: SafefsManifestWire,
) -> Result<SafefsTreeManifest, TransactionError> {
    if value.entries.len() > MAX_TREE_ENTRIES {
        return Err(store_error("retirement manifest exceeds entry bound"));
    }
    let manifest = SafefsTreeManifest {
        digest: value.digest,
        entries: value
            .entries
            .into_iter()
            .map(|entry| {
                Ok(SafefsTreeEntry {
                    path: entry.path,
                    state: safefs_entry_state_from_wire(entry.state)?,
                })
            })
            .collect::<Result<_, TransactionError>>()?,
    };
    validate_safefs_manifest(&manifest)?;
    Ok(manifest)
}

fn cleanup_intent_to_wire(value: &CleanupIntent) -> CleanupIntentWire {
    CleanupIntentWire {
        intent_token: value.intent_token.clone(),
        progress_key: value.progress_key.clone(),
        path: value.path.clone(),
        expected: safefs_entry_state_to_wire(&value.expected),
        root: value.root,
    }
}

fn cleanup_intent_from_wire(value: CleanupIntentWire) -> Result<CleanupIntent, TransactionError> {
    Ok(CleanupIntent {
        intent_token: value.intent_token,
        progress_key: value.progress_key,
        path: value.path,
        expected: safefs_entry_state_from_wire(value.expected)?,
        root: value.root,
    })
}
