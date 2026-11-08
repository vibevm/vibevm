//! Shared source/store identity gates for registry fetch.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");
use super::*;

/// The one entry-under-pin verification gate (PROP-010 §2.7,
/// mismatch-is-named): re-hash the store entry and, when it no longer
/// matches the lockfile pin, name the package and the entry. Shared by
/// the fetch path's `AlreadyPresent` branch and the store-backed
/// resolution's fetch short-circuit, so both apply the SAME gate, not
/// a copy. `pub(crate)` for the multi-registry resolver's offline
/// module; `Group`/version arrive by reference so the error carries
/// the identity verbatim.
pub(crate) fn verify_store_entry_against_pin(
    entry: &Path,
    expected: &str,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<(), RegistryError> {
    let actual = compute_content_hash(entry)?;
    if actual != expected {
        return Err(RegistryError::StoreEntryMismatch {
            detail: Box::new(crate::error::StoreEntryMismatchDetail {
                group: group.clone(),
                name: name.to_string(),
                version: version.clone(),
                path: entry.to_path_buf(),
                expected: expected.to_string(),
                actual,
            }),
        });
    }
    Ok(())
}

/// A cheap, stable `content_hash` for an `in-place` package — `sha256` of the
/// resolved commit, not a tree walk (PROP-022 §2.4/§2.5). The lockfile's
/// `content_hash` field is non-optional, so an in-place slot still records a
/// well-formed `sha256:<hex>`; identity, though, is the `resolved_commit`.
/// An empty commit (a backend that reported none) hashes the empty string —
/// deterministic and harmless, since in-place requires a real git source.
pub(crate) fn commit_content_hash(commit: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(commit.as_bytes());
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{b:02x}");
        s
    });
    format!("sha256:{hex}")
}
