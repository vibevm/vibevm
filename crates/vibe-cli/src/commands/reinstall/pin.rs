//! The lock pins bytes: a reinstall reproduces `vibe.lock` or refuses.
//!
//! Every refetch hands its source the locked `content_hash`, but only the git
//! registry's mirror walk treats that hash as a filter, and even it returns
//! the last disagreeing answer when every mirror disagrees. The
//! local-directory registry, an override and a git source ignore it. So the
//! one gate that holds for every source kind sits here, after the fetch has
//! put the new bytes in the machine store and before anything reaches
//! `vibedeps/`: the hash the source produced must be the hash the lock pins.
//! A package whose source moved on is `vibe update --reselect-source`'s
//! business: a plain update keeps the locked source and its pinned bytes
//! (PROP-073 `#UPDATE-SOURCE-RESELECTION`), and only the explicit reselection
//! re-resolves and re-records.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#lockfile-schema");

use vibe_core::manifest::LockedPackage;
use vibe_registry::CachedPackage;

/// A refetched package whose bytes no longer hash to its lock row.
#[derive(Debug, thiserror::Error)]
#[error(
    "lock_hash_mismatch: the source of `{group}/{name}@{version}` now hashes to {actual}, \
     but vibe.lock pins {expected}; nothing was copied into vibedeps/ \
     (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#lockfile-schema; \
      fix: `vibe update {group}/{name} --reselect-source`)"
)]
pub(crate) struct LockHashMismatch {
    pub(crate) group: String,
    pub(crate) name: String,
    pub(crate) version: semver::Version,
    pub(crate) expected: String,
    pub(crate) actual: String,
}

impl LockHashMismatch {
    /// The stable code the JSON envelope and the human refusal carry.
    pub(crate) const CODE: &'static str = "lock_hash_mismatch";
}

/// Refuse a refetch whose bytes differ from what the lock pins. The refusal
/// travels as `anyhow::Error` (both callers want one, and the struct is too
/// large for a bare `Err`); the JSON envelope downcasts it for its code.
pub(super) fn require_locked_bytes(
    locked: &LockedPackage,
    cached: &CachedPackage,
) -> anyhow::Result<()> {
    if cached.content_hash == locked.content_hash.as_str() {
        return Ok(());
    }
    Err(LockHashMismatch {
        group: locked.group.to_string(),
        name: locked.name.to_string(),
        version: locked.version.clone(),
        expected: locked.content_hash.as_str().to_string(),
        actual: cached.content_hash.clone(),
    }
    .into())
}

/// The locked-source replay's own held-hash refusal (PROP-073) is this
/// command's `lock_hash_mismatch`: the same fact, so it carries the same
/// stable code and fix. Every other resolver failure passes through.
pub(super) fn held_hash_refusal(error: vibe_registry::RegistryError) -> anyhow::Error {
    match error {
        vibe_registry::RegistryError::LockedContentMismatch { detail } => LockHashMismatch {
            group: detail.group,
            name: detail.name,
            version: detail.version,
            expected: detail.expected,
            actual: detail.actual,
        }
        .into(),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mismatch() -> LockHashMismatch {
        LockHashMismatch {
            group: "org.example".into(),
            name: "my-skills".into(),
            version: semver::Version::new(0, 1, 0),
            expected: "sha256:aaaa".into(),
            actual: "sha256:bbbb".into(),
        }
    }

    #[test]
    fn the_refusal_leads_with_its_code_and_names_both_hashes_and_the_fix() {
        let text = mismatch().to_string();
        assert!(text.starts_with("lock_hash_mismatch: "), "{text}");
        assert!(text.contains("org.example/my-skills@0.1.0"), "{text}");
        assert!(text.contains("sha256:aaaa") && text.contains("sha256:bbbb"));
        assert!(
            text.contains("`vibe update org.example/my-skills --reselect-source`"),
            "{text}"
        );
    }

    #[test]
    fn the_human_refusal_reads_the_code_and_the_fix_command() {
        let refusal = crate::output::refusal_of(&anyhow::Error::from(mismatch()));
        assert_eq!(refusal.code.as_deref(), Some(LockHashMismatch::CODE));
        assert!(
            refusal
                .fix
                .iter()
                .any(|line| line == "vibe update org.example/my-skills --reselect-source"),
            "{:?}",
            refusal.fix
        );
    }
}
