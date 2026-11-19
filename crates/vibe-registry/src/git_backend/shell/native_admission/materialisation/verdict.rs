//! What a staging verdict is *about*, so one already reached can be reused.
//!
//! The admission probe of `materialisation` refuses a payload that selects
//! `filter`, `ident` or `working-tree-encoding`. That refusal is a pure
//! function of three things — the tree it reads, the attribute and
//! conversion inputs the probe child is given, and the Git that answers —
//! so a tree already admitted under exactly those inputs need not be read
//! again (PROP-073 `#NATIVE-MATERIALISATION`). This file is the key that
//! says "exactly those inputs", and nothing else: every input the refusal
//! depends on goes into it, so a remembered verdict can only ever be reused
//! for a probe that would ask the same question. Split out of
//! `materialisation` to hold the file-length budget.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::super::*;
use super::CapturedContext;
use crate::git_backend::PayloadBytes;

impl NativeState {
    /// The identity of one staging verdict: the tree, the attribute inputs
    /// the probe child reads and the Git that answers it. Every input the
    /// refusal depends on is in the key, so a remembered verdict can only
    /// be reused for a probe that would ask the same question.
    pub(super) fn verdict_key(
        &self,
        owned: &OwnedRepository,
        captured: &CapturedContext,
        bytes: PayloadBytes,
        attr_source: bool,
    ) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(b"vibe-native-admission-verdict/1\0");
        digest.update(owned.revision.commit.as_bytes());
        digest.update([0]);
        digest.update(owned.revision.object.as_bytes());
        digest.update([0]);
        digest.update([
            u8::from(bytes == PayloadBytes::Verbatim),
            u8::from(attr_source),
        ]);
        for scope in [
            captured.system_attributes.as_slice(),
            captured.global_attributes.as_slice(),
            captured.info_attributes.as_slice(),
        ] {
            digest.update((scope.len() as u64).to_le_bytes());
            digest.update(scope);
        }
        // Exactly the entries `stage_owned` hands the probe child.
        for (key, value) in &captured.entries {
            if matches!(
                key.as_str(),
                "core.autocrlf"
                    | "core.eol"
                    | "core.safecrlf"
                    | "core.filemode"
                    | "core.symlinks"
                    | "core.ignorecase"
                    | "core.precomposeunicode"
            ) {
                digest.update(key.as_bytes());
                digest.update([0]);
                digest.update(value.as_bytes());
                digest.update([0]);
            }
        }
        digest.update(self.profile.installation_witness());
        digest.finalize().into()
    }
    pub(super) fn admitted(&self, verdict: &[u8; 32]) -> Result<bool, GitError> {
        Ok(self
            .admitted
            .lock()
            .map_err(|_| unsupported("admitted verdict lock poisoned"))?
            .contains(verdict))
    }
    pub(super) fn remember_admitted(&self, verdict: [u8; 32]) -> Result<(), GitError> {
        self.admitted
            .lock()
            .map_err(|_| unsupported("admitted verdict lock poisoned"))?
            .insert(verdict);
        Ok(())
    }
}
