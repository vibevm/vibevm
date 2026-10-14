//! Registered immutable derivative intent, linked to the incumbent edit journal.
use super::*;
use vibe_wire::generated::{
    derivative_rebase_journal::DerivativeRebaseJournal as Record,
    derivative_result::DerivativeResult,
};
pub(super) const CAP: usize = 8 * 1024 * 1024;
pub(super) fn path(id: &str) -> Result<String, EditError> {
    journal::path(id)?;
    Ok(format!(".vibe/derivative-rebases/{id}.json"))
}
pub(super) fn identity(record: &Record) -> Result<String, EditError> {
    let mut copy = record.clone();
    copy.id.clear();
    let mut bytes = b"derivative-rebase-journal/1\0".to_vec();
    bytes.extend(serde_json::to_vec(&copy).map_err(|_| EditError::InvalidJournal)?);
    Ok(journal::raw_hash(&bytes))
}
pub(super) fn result(raw: &str) -> Result<DerivativeResult, EditError> {
    let r: DerivativeResult = serde_json::from_str(raw).map_err(|_| EditError::JournalShape)?;
    r.checked_binding()
        .map_err(|_| EditError::JournalIntegrity)?;
    Ok(r)
}
pub(super) fn validate(
    authority: &DocumentEditAuthority,
    r: &Record,
) -> Result<journal::Journal, EditError> {
    if r.schema != 1 || r.recipe != "derivative-rebase-journal/1" {
        return Err(EditError::JournalEpoch);
    }
    if identity(r)? != r.id {
        return Err(EditError::JournalIntegrity);
    }
    vibe_safefs::ensure_safe_component(&r.result_name).map_err(|_| EditError::AuthorityRefused)?;
    let old = result(&r.previous_result)?;
    let new = result(&r.desired_result)?;
    let old_binding = old
        .checked_binding()
        .map_err(|_| EditError::JournalIntegrity)?;
    let new_binding = new
        .checked_binding()
        .map_err(|_| EditError::JournalIntegrity)?;
    if old_binding.base().owner.group != new_binding.base().owner.group
        || old_binding.base().owner.name != new_binding.base().owner.name
        || old_binding.base().owner.kind != new_binding.base().owner.kind
        || old_binding.base().descriptor != new_binding.base().descriptor
        || old_binding.local().owner != new_binding.local().owner
        || old_binding.local().resource != new_binding.local().resource
        || old_binding.base().resource != new_binding.base().resource
    {
        return Err(EditError::JournalIntegrity);
    }
    let intent = journal::decode(authority, r.edit_intent.as_bytes())?;
    if intent.files.len() != 1
        || intent.files[0].before != old.binding_toml.as_bytes()
        || intent.files[0].after != new.binding_toml.as_bytes()
        || intent.phase != journal::Phase::Prepared
    {
        return Err(EditError::JournalIntegrity);
    }
    Ok(intent)
}
pub(super) fn load(authority: &DocumentEditAuthority, id: &str) -> Result<Record, EditError> {
    let raw = authority
        .host
        .project
        .read_file_bounded(&path(id)?, CAP)
        .map_err(|_| EditError::InvalidJournal)?
        .ok_or(EditError::RecoveryRequired)?;
    let r: Record = serde_json::from_slice(&raw).map_err(|_| EditError::JournalShape)?;
    if r.id != id {
        return Err(EditError::JournalIntegrity);
    }
    validate(authority, &r)?;
    Ok(r)
}
pub(super) fn chain(authority: &DocumentEditAuthority, r: &Record) -> Result<(), EditError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut current = r.clone();
    for _ in 0..128 {
        let Some(id) = current.prior.as_deref() else {
            return Ok(());
        };
        if !seen.insert(id.to_owned()) {
            return Err(EditError::JournalIntegrity);
        }
        let previous = load(authority, id)?;
        if previous.desired_result != current.previous_result {
            return Err(EditError::JournalIntegrity);
        }
        current = previous;
    }
    Err(EditError::JournalIntegrity)
}
pub(super) fn retain(authority: &DocumentEditAuthority, r: &Record) -> Result<bool, EditError> {
    validate(authority, r)?;
    chain(authority, r)?;
    let raw = serde_json::to_vec(r).map_err(|_| EditError::InvalidJournal)?;
    if raw.len() > CAP {
        return Err(EditError::InvalidJournal);
    }
    let p = &authority.host.project;
    if let Some(found) = p
        .read_file_bounded(&path(&r.id)?, CAP)
        .map_err(|_| EditError::RecoveryRequired)?
    {
        if found != raw {
            return Err(EditError::ThirdState);
        }
    } else {
        let dir = p
            .dir(&[".vibe", "derivative-rebases"], true)
            .map_err(|_| EditError::RecoveryRequired)?;
        p.publish_new_in(&dir, &format!("{}.json", r.id), &raw)
            .map_err(|_| EditError::RecoveryRequired)?;
    }
    let mut reduced = false;
    for components in [
        &[".vibe", "derivative-rebases"][..],
        &[".vibe"][..],
        &[][..],
    ] {
        let dir = p
            .dir(components, false)
            .map_err(|_| EditError::RecoveryRequired)?;
        let durability = dir.sync_directory();
        if matches!(durability, vibe_safefs::DirectoryDurability::Failed(_)) {
            return Err(EditError::RecoveryRequired);
        }
        reduced |= durability != vibe_safefs::DirectoryDurability::Synced;
    }
    if p.read_file_bounded(&path(&r.id)?, CAP)
        .map_err(|_| EditError::RecoveryRequired)?
        .as_deref()
        != Some(raw.as_slice())
    {
        return Err(EditError::RecoveryRequired);
    }
    Ok(reduced)
}
