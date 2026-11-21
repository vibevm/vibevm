//! Default-scope preview over one incumbent reference-index snapshot.
use super::*;
use sha2::{Digest, Sha256};
use vibe_secrets::{ReferenceBinding, ReferenceSelection};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DefaultScope {
    DefaultOnly,
    ManagedOverrides,
}
#[derive(Debug, Serialize)]
pub struct DefaultScopeRow {
    pub harness: String,
    pub old_mode: String,
    pub old_connection: Option<SecretId>,
    pub proposed_mode: String,
    pub proposed_connection: Option<SecretId>,
}
#[derive(Debug, Serialize)]
pub struct DefaultScopePreview {
    pub protocol: &'static str,
    pub preview_id: String,
    pub scope: DefaultScope,
    pub old_default: Option<SecretId>,
    pub proposed_default: SecretId,
    pub rows: Vec<DefaultScopeRow>,
    #[serde(skip)]
    root: std::path::PathBuf,
    #[serde(skip)]
    digest: String,
    #[serde(skip)]
    authorized_target: SecretId,
    #[serde(skip)]
    authorized_scope: DefaultScope,
    #[serde(skip)]
    authorized_token: String,
}
impl Store {
    pub fn default_scope_preview(
        &self,
        target: &SecretId,
        scope: DefaultScope,
    ) -> Result<DefaultScopePreview, Error> {
        let snapshot = self.0.reference_snapshot("ai")?;
        if !snapshot.index.items.contains_key(target) {
            return Err(vibe_secrets::Error::NotFound.into());
        }
        for bytes in snapshot.metadata.values() {
            let metadata: Metadata =
                toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Metadata)?)
                    .map_err(|_| Error::Metadata)?;
            metadata.validate()?;
        }
        let old_default = match &snapshot.index.selection {
            Some(ReferenceSelection::Selected { connection }) => Some(connection.clone()),
            _ => None,
        };
        let mut rows = Vec::new();
        for harness in ["claude", "codex", "opencode", "qwen", "hermes", "pi"] {
            let binding = snapshot
                .index
                .bindings
                .as_ref()
                .and_then(|b| b.get(harness));
            let (old_mode, old_connection) = match binding {
                None => ("native", None),
                Some(ReferenceBinding::InheritDefault) => ("inherit-default", old_default.clone()),
                Some(ReferenceBinding::Connection { connection }) => {
                    ("connection", Some(connection.clone()))
                }
            };
            let replace = scope == DefaultScope::ManagedOverrides
                && binding.is_some()
                && matches!(
                    metadata_capability(harness),
                    MetadataCapability::Compatible { .. }
                );
            let proposed_mode = if replace { "inherit-default" } else { old_mode };
            let proposed_connection = if proposed_mode == "inherit-default" {
                Some(target.clone())
            } else {
                old_connection.clone()
            };
            if let Some(id) = &proposed_connection {
                let metadata = snapshot_metadata(&snapshot, id)?;
                if !metadata_supports(harness, &metadata.api_kind) {
                    return Err(Error::Invalid);
                }
            }
            rows.push(DefaultScopeRow {
                harness: harness.into(),
                old_mode: old_mode.into(),
                old_connection,
                proposed_mode: proposed_mode.into(),
                proposed_connection,
            });
        }
        let root = self.1.canonicalize().map_err(|_| Error::Invalid)?;
        if root != self.1 {
            return Err(Error::Invalid);
        }
        let mut hash = Sha256::new();
        hash.update(b"vibevm.ai-default-scope/v1\0");
        fn frame(hash: &mut Sha256, bytes: &[u8]) {
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            frame(&mut hash, root.as_os_str().as_bytes());
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let bytes: Vec<_> = root
                .as_os_str()
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect();
            frame(&mut hash, &bytes);
        }
        frame(&mut hash, snapshot.index_sha256.as_bytes());
        frame(
            &mut hash,
            match scope {
                DefaultScope::DefaultOnly => b"default-only",
                DefaultScope::ManagedOverrides => b"managed-overrides",
            },
        );
        frame(&mut hash, target.as_str().as_bytes());
        frame(
            &mut hash,
            old_default.as_ref().map_or("", SecretId::as_str).as_bytes(),
        );
        for row in &rows {
            for value in [
                row.harness.as_str(),
                row.old_mode.as_str(),
                row.old_connection.as_ref().map_or("", SecretId::as_str),
                row.proposed_mode.as_str(),
                row.proposed_connection
                    .as_ref()
                    .map_or("", SecretId::as_str),
            ] {
                frame(&mut hash, value.as_bytes());
            }
        }
        let token = format!("ai-default-scope/1:sha256:{:x}", hash.finalize());
        Ok(DefaultScopePreview {
            protocol: "vibe-ai-default-scope/1",
            preview_id: token.clone(),
            authorized_token: token,
            authorized_scope: scope,
            authorized_target: target.clone(),
            scope,
            old_default,
            proposed_default: target.clone(),
            rows,
            root,
            digest: snapshot.index_sha256,
        })
    }
    pub fn apply_default_scope(
        &self,
        preview: &DefaultScopePreview,
        response: &str,
    ) -> Result<u64, Error> {
        if response != preview.authorized_token
            || preview.preview_id != preview.authorized_token
            || preview.scope != preview.authorized_scope
            || preview.proposed_default != preview.authorized_target
            || self.1.canonicalize().map_err(|_| Error::Invalid)? != preview.root
        {
            return Err(vibe_secrets::Error::Conflict.into());
        }
        Ok(self
            .0
            .select_reference_scope(
                &preview.proposed_default,
                preview.scope == DefaultScope::ManagedOverrides,
                &preview.digest,
            )?
            .store_revision)
    }
}
