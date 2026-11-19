//! Inputs of the retained attribute-admission verdict.
use super::*;

impl CapturedContext {
    /// The attribute and conversion inputs a staging probe child is given,
    /// as one byte string: the three attribute scopes and exactly the
    /// configuration entries `materialisation` forwards. An owner that
    /// remembers a staging verdict across processes keys it on this, so a
    /// changed `~/.config/git/attributes` or `core.autocrlf` invalidates
    /// what an earlier run admitted (PROP-073 `#NATIVE-MATERIALISATION`).
    pub(in crate::git_backend::shell::native_admission) fn probe_inputs(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for scope in [
            self.system_attributes.as_slice(),
            self.global_attributes.as_slice(),
            self.info_attributes.as_slice(),
        ] {
            bytes.extend_from_slice(&(scope.len() as u64).to_le_bytes());
            bytes.extend_from_slice(scope);
        }
        for (key, value) in &self.entries {
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
                bytes.extend_from_slice(key.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(value.as_bytes());
                bytes.push(0);
            }
        }
        bytes
    }
}
