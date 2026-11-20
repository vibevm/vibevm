//! One reviewed setup subject over existing metadata, not installation.
use super::*;
use sha2::{Digest, Sha256};
use vibe_secrets::{ReferenceBinding, ReferenceSelection};
#[derive(Debug, Serialize)]
pub struct SetupRow {
    pub harness: String,
    pub old_mode: String,
    pub old_connection: Option<SecretId>,
    pub proposed_mode: String,
    pub effective_connection: SecretId,
    pub current_revision: SecretId,
}
#[derive(Debug, Serialize)]
pub struct SetupPreview {
    preview_id: String,
    #[serde(skip)]
    scope: std::path::PathBuf,
    pub store_revision: u64,
    pub rows: Vec<SetupRow>,
    #[serde(skip)]
    pub(crate) digest: String,
    #[serde(skip)]
    pub(crate) edits: BTreeMap<String, ReferenceBinding>,
}
impl SetupPreview {
    pub fn preview_id(&self) -> &str {
        &self.preview_id
    }
}
struct Frames(Sha256);
impl Frames {
    fn new() -> Self {
        let mut h = Sha256::new();
        h.update(b"vibevm.ai-setup/v1\0");
        Self(h)
    }
    fn bytes(&mut self, b: &[u8]) {
        self.0.update((b.len() as u64).to_be_bytes());
        self.0.update(b);
    }
    fn text(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn count(&mut self, n: usize) {
        self.0.update((n as u64).to_be_bytes());
    }
    fn optional(&mut self, s: Option<&SecretId>) {
        self.0.update([u8::from(s.is_some())]);
        if let Some(s) = s {
            self.text(s.as_str());
        }
    }
    fn finish(self) -> String {
        format!("ai-setup/1:sha256:{:x}", self.0.finalize())
    }
}
impl Store {
    /// Aliases are resolved by trusted CLI composition; binding includes each
    /// requested name and canonical result so a remap changes the subject.
    pub fn setup_preview(
        &self,
        aliases: &BTreeMap<String, String>,
        mode: ReferenceBinding,
    ) -> Result<SetupPreview, Error> {
        if aliases.is_empty() || aliases.len() > 6 {
            return Err(Error::Invalid);
        }
        let mut identities = std::collections::BTreeSet::new();
        for harness in aliases.values() {
            if !identities.insert(harness)
                || !matches!(
                    metadata_capability(harness),
                    MetadataCapability::Compatible { .. }
                )
            {
                return Err(Error::Invalid);
            }
        }
        let snapshot = self.0.reference_snapshot("ai")?;
        let target = match &mode {
            ReferenceBinding::Connection { connection } => connection.clone(),
            ReferenceBinding::InheritDefault => match &snapshot.index.selection {
                Some(ReferenceSelection::Selected { connection }) => connection.clone(),
                _ => return Err(vibe_secrets::Error::NotFound.into()),
            },
        };
        let target_row = snapshot
            .index
            .items
            .get(&target)
            .ok_or(vibe_secrets::Error::NotFound)?;
        for bytes in snapshot.metadata.values() {
            let metadata: Metadata =
                toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Metadata)?)
                    .map_err(|_| Error::Metadata)?;
            metadata.validate()?;
        }
        let canonical = self.1.canonicalize().map_err(|_| Error::Invalid)?;
        if canonical != self.1 {
            return Err(Error::Invalid);
        }
        let mut hash = Frames::new();
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            hash.text("unix-path-bytes");
            hash.bytes(canonical.as_os_str().as_bytes());
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            hash.text("windows-path-utf16le");
            let bytes: Vec<_> = canonical
                .as_os_str()
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect();
            hash.bytes(&bytes);
        }
        hash.text(&snapshot.index_sha256);
        hash.count(aliases.len());
        for (alias, harness) in aliases {
            hash.text(alias);
            hash.text(harness);
        }
        let mut edits = BTreeMap::new();
        let mut rows = Vec::new();
        for harness in identities {
            let old = snapshot
                .index
                .bindings
                .as_ref()
                .and_then(|map| map.get(harness));
            let (old_mode, old_connection) = match old {
                None => ("native", None),
                Some(ReferenceBinding::InheritDefault) => (
                    "inherit-default",
                    match &snapshot.index.selection {
                        Some(ReferenceSelection::Selected { connection }) => {
                            Some(connection.clone())
                        }
                        _ => None,
                    },
                ),
                Some(ReferenceBinding::Connection { connection }) => {
                    ("connection", Some(connection.clone()))
                }
            };
            let proposed = match mode {
                ReferenceBinding::InheritDefault => "inherit-default",
                ReferenceBinding::Connection { .. } => "connection",
            };
            hash.text(harness);
            hash.text(old_mode);
            hash.optional(old_connection.as_ref());
            hash.text(proposed);
            hash.text(target.as_str());
            hash.text(target_row.current_revision.as_str());
            hash.text(&target_row.metadata_sha256);
            rows.push(SetupRow {
                harness: harness.clone(),
                old_mode: old_mode.into(),
                old_connection,
                proposed_mode: proposed.into(),
                effective_connection: target.clone(),
                current_revision: target_row.current_revision.clone(),
            });
            edits.insert(harness.clone(), mode.clone());
        }
        Ok(SetupPreview {
            preview_id: hash.finish(),
            scope: canonical,
            store_revision: snapshot.index.store_revision,
            rows,
            digest: snapshot.index_sha256,
            edits,
        })
    }
    pub fn apply_setup(&self, preview: &SetupPreview, response: &str) -> Result<u64, Error> {
        if self.1.canonicalize().map_err(|_| Error::Invalid)? != preview.scope
            || response != preview.preview_id
        {
            return Err(vibe_secrets::Error::Conflict.into());
        }
        let result = self
            .0
            .bind_references_bulk("ai", &preview.edits, &preview.digest)?;
        Ok(result.store_revision)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_distinguishes_boundaries() {
        let mut a = Frames::new();
        a.text("ab");
        a.text("c");
        let mut b = Frames::new();
        b.text("a");
        b.text("bc");
        assert_ne!(a.finish(), b.finish());
    }
}

#[cfg(all(test, unix))]
mod scope_tests {
    use super::*;
    #[test]
    fn identical_index_in_another_scope_does_not_accept_preview() {
        use std::os::unix::fs::PermissionsExt;
        fn temp() -> tempfile::TempDir {
            tempfile::Builder::new()
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap()
        }
        fn copy(source: &Path, target: &Path) {
            std::fs::create_dir(target).unwrap();
            std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o700)).unwrap();
            for entry in std::fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &target.join(entry.file_name()));
                } else {
                    std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
                }
            }
        }
        let a = temp();
        let b = temp();
        let secrets = vibe_secrets::Store::at_settings(a.path()).unwrap();
        let key = secrets
            .create(
                "api-key",
                "Synthetic",
                vibe_secrets::ProtectedValue::new(b"SYNTHETIC".to_vec()).unwrap(),
            )
            .unwrap();
        let store = Store::at_settings(a.path()).unwrap();
        let id = store
            .add(
                Metadata {
                    label: "Synthetic".into(),
                    endpoint: "http://127.0.0.1:32123/v1".into(),
                    api_kind: "openai-chat-completions".into(),
                    default_model: "synthetic".into(),
                },
                &key,
            )
            .unwrap();
        copy(&a.path().join("secrets"), &b.path().join("secrets"));
        let other = Store::at_settings(b.path()).unwrap();
        let aliases = BTreeMap::from([("pi".into(), "pi".into())]);
        let preview = store
            .setup_preview(&aliases, ReferenceBinding::Connection { connection: id })
            .unwrap();
        let before = std::fs::read(b.path().join("secrets/ai/index.toml")).unwrap();
        assert!(other.apply_setup(&preview, preview.preview_id()).is_err());
        assert_eq!(
            std::fs::read(b.path().join("secrets/ai/index.toml")).unwrap(),
            before
        );
    }
}
