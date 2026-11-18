//! Cache freshness and captured admission inputs.
use super::*;

impl NativeState {
    /// What the selected endpoint advertises for `reference`: the object id
    /// at its tip and the ref namespace it was found in, read with one
    /// `ls-remote` and nothing else.
    ///
    /// This is the whole network cost of deciding whether a durable cache
    /// is current: the advertised id is the same id a prepared source
    /// records, so the comparison needs no objects at all. A reference
    /// already spelled as an object id is answered without contact.
    pub(in crate::git_backend::shell) fn advertised(
        &self,
        address: &str,
        reference: &str,
        anonymous: bool,
        registry_auth: Option<vibe_core::manifest::AuthKind>,
    ) -> Result<(String, String), GitError> {
        if full_oid(reference) {
            return Ok((reference.to_owned(), reference.to_owned()));
        }
        let remote = self
            .profile
            .capture_for_request(address, anonymous, registry_auth)?;
        let remote = self.profile.relocate(remote)?;
        let context = Context::new(&self.profile)?;
        let advertised =
            context.run(&["ls-remote", "--", remote.address()], None, Some(&remote))?;
        let refs = text(advertised.stdout)?;
        advertised_tip(&refs, reference).ok_or_else(|| GitError::RefNotFound {
            url: remote.address().to_owned(),
            refname: reference.into(),
        })
    }
    /// An opaque digest of the attribute inputs a staging probe reads, for
    /// an owner that remembers a verdict **across** processes: a durable
    /// snapshot records this value and is reused only while it still holds
    /// (PROP-073 `#NATIVE-MATERIALISATION`).
    pub(in crate::git_backend::shell) fn admission_inputs(&self) -> Result<[u8; 32], GitError> {
        use sha2::{Digest, Sha256};
        let captured = self.profile.capture_context(None)?;
        let mut digest = Sha256::new();
        digest.update(b"vibe-native-admission-inputs/1\0");
        digest.update(captured.probe_inputs());
        digest.update(self.profile.installation_witness());
        Ok(digest.finalize().into())
    }
}
