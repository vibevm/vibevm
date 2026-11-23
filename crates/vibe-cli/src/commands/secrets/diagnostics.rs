//! Metadata-only enrichment after a typed keyring refusal; never retries a write.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use vibe_secrets::{Error, Expected, SecretId, Store};

pub(super) fn expected(
    store: &Store,
    id: &SecretId,
    expected: &Expected,
    error: Error,
) -> anyhow::Error {
    if error == Error::Conflict
        && let Ok(snapshot) = store.snapshot()
        && snapshot.items.contains_key(id)
        && snapshot.store_revision != expected.store_revision
    {
        // This read follows the refusal: it is not evidence of the revision
        // held by the rejecting mutation, nor of a per-item token mismatch.
        let message = format!(
            "keyring_stale: requested store revision {}; current observation is store revision {} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; \
             fix: `vibe secrets list` shows the current revisions to repeat the change against)",
            expected.store_revision, snapshot.store_revision
        );
        return anyhow::Error::new(error).context(message);
    }
    error.into()
}
