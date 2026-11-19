//! Continuation reuses the ordinary held commit/hash verifier.
use super::*;
impl LockedSelection {
    /// Verify continued payload before projecting held evidence.
    pub fn verify_continued(&self, cached: &mut CachedPackage) -> Result<(), RegistryError> {
        self.verify(&cached.content_hash, cached.resolved_commit.as_deref())?;
        self.apply_evidence(cached);
        Ok(())
    }
    pub(crate) fn verify_continued_in_place(
        &self,
        prepared: crate::PreparedInPlace,
    ) -> Result<crate::PreparedInPlace, RegistryError> {
        self.verify(prepared.content_hash(), Some(prepared.resolved_commit()))?;
        prepared.with_locked_evidence(self)
    }
}
