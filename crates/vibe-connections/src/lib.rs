//! Sole local connection metadata owner; no provider execution or secret access.
#![deny(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use vibe_secrets::{Expected, SecretId};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid nonsecret connection metadata")]
    Invalid,
    #[error("connection metadata serialization failed")]
    Metadata,
    #[error(transparent)]
    Store(#[from] vibe_secrets::Error),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub label: String,
    pub endpoint: String,
    pub api_kind: String,
    pub default_model: String,
}
impl Metadata {
    pub fn validate(&self) -> Result<(), Error> {
        for text in [&self.label, &self.default_model] {
            if text.is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
                return Err(Error::Invalid);
            }
        }
        if self.api_kind != "openai-chat-completions"
            || self.endpoint.len() > 4096
            || self.endpoint.chars().any(char::is_control)
        {
            return Err(Error::Invalid);
        }
        let url = url::Url::parse(&self.endpoint).map_err(|_| Error::Invalid)?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
        {
            return Err(Error::Invalid);
        }
        let local = match url.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if url.scheme() != "https" && !(url.scheme() == "http" && local) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub current_revision: SecretId,
    pub secret_ref: SecretId,
    pub metadata: Metadata,
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub store_revision: u64,
    pub items: BTreeMap<SecretId, Connection>,
}
pub struct Store(vibe_secrets::Store, std::path::PathBuf);
impl Store {
    pub fn at_settings(settings: &Path) -> Result<Self, Error> {
        Ok(Self(
            vibe_secrets::Store::at_settings(settings)?,
            settings.to_path_buf(),
        ))
    }
    pub fn list(&self) -> Result<Snapshot, Error> {
        let snapshot = self.0.reference_snapshot("ai")?;
        let mut items = BTreeMap::new();
        for (id, row) in snapshot.index.items {
            let bytes = snapshot.metadata.get(&id).ok_or(Error::Metadata)?;
            let metadata: Metadata =
                toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Metadata)?)
                    .map_err(|_| Error::Metadata)?;
            metadata.validate()?;
            items.insert(
                id,
                Connection {
                    current_revision: row.current_revision,
                    secret_ref: row.secret_ref,
                    metadata,
                },
            );
        }
        Ok(Snapshot {
            store_revision: snapshot.index.store_revision,
            items,
        })
    }
    pub fn details(&self, id: &SecretId) -> Result<Connection, Error> {
        self.list()?
            .items
            .remove(id)
            .ok_or_else(|| vibe_secrets::Error::NotFound.into())
    }
    pub fn add(&self, metadata: Metadata, secret: &SecretId) -> Result<SecretId, Error> {
        metadata.validate()?;
        let bytes = toml::to_string(&metadata).map_err(|_| Error::Metadata)?;
        Ok(self.0.create_reference("ai", secret, bytes.as_bytes())?)
    }
    pub fn remove(&self, id: &SecretId, expected: &Expected) -> Result<(), Error> {
        Ok(self.0.remove_reference("ai", id, expected)?)
    }
}

#[derive(Debug, Serialize)]
pub struct DefaultSelection {
    pub store_revision: u64,
    pub connection: Option<SecretId>,
}
fn selection(index: vibe_secrets::ReferenceIndex) -> DefaultSelection {
    DefaultSelection {
        store_revision: index.store_revision,
        connection: match index.selection {
            Some(vibe_secrets::ReferenceSelection::Selected { connection }) => Some(connection),
            _ => None,
        },
    }
}
impl Store {
    pub fn default_selection(&self) -> Result<DefaultSelection, Error> {
        let snapshot = self.0.reference_snapshot("ai")?;
        // Validate exactly the metadata snapshot whose selection is returned.
        for bytes in snapshot.metadata.values() {
            let metadata: Metadata =
                toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Metadata)?)
                    .map_err(|_| Error::Metadata)?;
            metadata.validate()?;
        }
        Ok(selection(snapshot.index))
    }
    pub fn set_default(&self, id: &SecretId, expected: u64) -> Result<DefaultSelection, Error> {
        Ok(selection(self.0.select_reference(
            "ai",
            Some(id),
            expected,
        )?))
    }
    pub fn clear_default(&self, expected: u64) -> Result<DefaultSelection, Error> {
        Ok(selection(self.0.select_reference("ai", None, expected)?))
    }
}

/// Metadata compatibility only; never an authentication or native launch grant.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum MetadataCapability {
    Compatible {
        protocol: &'static str,
        scope: &'static str,
        evidence: &'static str,
    },
    Unsupported,
    Unknown,
}
pub fn metadata_capability(harness: &str) -> MetadataCapability {
    match harness {
        "pi" => MetadataCapability::Compatible {
            protocol: "openai-chat-completions",
            scope: "stored-selection-only",
            evidence: "Pi 1.5.2 Linux proxy fixture",
        },
        "qwen" => MetadataCapability::Compatible {
            protocol: "openai-chat-completions",
            scope: "stored-selection-only",
            evidence: "Qwen 0.27.2 Linux proxy fixture",
        },
        "claude" => MetadataCapability::Unsupported,
        _ => MetadataCapability::Unknown,
    }
}
#[derive(Debug, Serialize)]
pub struct BindingObservation {
    pub harness: String,
    pub store_revision: u64,
    pub mode: String,
    pub connection: Option<SecretId>,
    pub resolution: String,
    pub capability: MetadataCapability,
}
impl Store {
    pub fn binding(&self, harness: &str) -> Result<BindingObservation, Error> {
        if !matches!(
            harness,
            "claude" | "codex" | "opencode" | "qwen" | "hermes" | "pi"
        ) {
            return Err(Error::Invalid);
        }
        let snapshot = self.0.reference_snapshot("ai")?;
        for bytes in snapshot.metadata.values() {
            let metadata: Metadata =
                toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Metadata)?)
                    .map_err(|_| Error::Metadata)?;
            metadata.validate()?;
        }
        let binding = snapshot
            .index
            .bindings
            .as_ref()
            .and_then(|map| map.get(harness));
        let (mode, connection) = match binding {
            None => ("native", None),
            Some(vibe_secrets::ReferenceBinding::InheritDefault) => (
                "inherit-default",
                match &snapshot.index.selection {
                    Some(vibe_secrets::ReferenceSelection::Selected { connection }) => {
                        Some(connection.clone())
                    }
                    _ => None,
                },
            ),
            Some(vibe_secrets::ReferenceBinding::Connection { connection }) => {
                ("connection", Some(connection.clone()))
            }
        };
        Ok(BindingObservation {
            harness: harness.into(),
            store_revision: snapshot.index.store_revision,
            mode: mode.into(),
            resolution: if mode == "native" {
                "unmanaged"
            } else if connection.is_some() {
                "metadata-only"
            } else {
                "unavailable: select a default connection"
            }
            .into(),
            connection,
            capability: metadata_capability(harness),
        })
    }
    pub fn set_binding(
        &self,
        harness: &str,
        binding: Option<vibe_secrets::ReferenceBinding>,
        expected: u64,
    ) -> Result<BindingObservation, Error> {
        if binding.is_some()
            && !matches!(
                metadata_capability(harness),
                MetadataCapability::Compatible { .. }
            )
        {
            return Err(Error::Invalid);
        }
        self.0.bind_reference("ai", harness, binding, expected)?;
        self.binding(harness)
    }
}

mod setup;
pub use setup::{SetupPreview, SetupRow};
