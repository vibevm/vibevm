//! The PROP-011 §2.6 mutable-source refresh over the write-once store —
//! split out of `store.rs` along the file-length seam.

use std::fs;
use std::path::Path;

use vibe_core::Group;

use crate::error::RegistryError;

use super::{InsertOutcome, insert_at, record_hash_at, recorded_hash_at, sidecar_path};

/// [`insert_at`] with the PROP-011 §2.6 mutable-source guard: when the
/// entry already exists but its bytes no longer match `expected_hash` —
/// the hash the caller just computed over the LIVE source — the stale
/// entry (and its sidecar) is replaced by a fresh insert. Same-version
/// content drift is real for local sources the author edits in place (a
/// version is NOT bumped on every edit, owner policy 2026-07-26), and a
/// write-once hit on stale bytes poisons every downstream consumer: the
/// lock records the fresh hash while the slots and the boot model read
/// the old manifest (the 2026-08-24 normal-flip regression). The check
/// costs one sidecar read on the hot path; an unrecorded entry pays one
/// hash to earn its record.
pub(crate) fn insert_current_at(
    root: &Path,
    src: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_hash: &str,
) -> Result<InsertOutcome, RegistryError> {
    match insert_at(root, src, group, name, version)? {
        InsertOutcome::AlreadyPresent(entry) => {
            let recorded = match recorded_hash_at(root, group, name, version)? {
                Some(line) => line,
                None => {
                    let computed = crate::shippable::compute_content_hash(&entry)?;
                    let _ = record_hash_at(root, group, name, version, &computed);
                    computed
                }
            };
            if recorded == expected_hash {
                return Ok(InsertOutcome::AlreadyPresent(entry));
            }
            fs::remove_dir_all(&entry).map_err(|source| RegistryError::Io {
                path: entry.clone(),
                source,
            })?;
            let sidecar = sidecar_path(root, group, name, version);
            if sidecar.exists() {
                let _ = fs::remove_file(&sidecar);
            }
            insert_at(root, src, group, name, version)
        }
        inserted => Ok(inserted),
    }
}
