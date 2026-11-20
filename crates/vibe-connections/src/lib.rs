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
pub struct Store(vibe_secrets::Store);
impl Store {
    pub fn at_settings(settings: &Path) -> Result<Self, Error> {
        Ok(Self(vibe_secrets::Store::at_settings(settings)?))
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
