//! Sole local connection metadata owner; no provider execution or secret access.
#![deny(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use vibe_secrets::{Expected, SecretId};

/// The connection-metadata layer's one error enum. Every message is fixed
/// text: it names the violated rule and a fix, never an endpoint or a value.
///
/// ```
/// use vibe_connections::{Error, Metadata};
///
/// // Plain http is admitted only towards a loopback address.
/// let metadata = Metadata {
///     label: "Synthetic".into(),
///     endpoint: "http://provider.invalid/v1".into(),
///     api_kind: "openai-chat-completions".into(),
///     default_model: "synthetic".into(),
/// };
/// let error = metadata.validate().unwrap_err();
/// assert!(matches!(error, Error::Invalid));
/// assert!(error.to_string().contains("TS-KEYRING-CONSUMER"));
/// assert!(!error.to_string().contains("provider.invalid"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER")]
pub enum Error {
    #[error(
        "invalid nonsecret connection metadata \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: give a bounded label and model, a supported API kind and harness, and an \
          https or loopback http endpoint without credentials, query or fragment)"
    )]
    Invalid,
    #[error(
        "connection metadata serialization failed \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; \
          fix: remove the connection whose stored nonsecret metadata is missing or \
          unreadable and add it again)"
    )]
    Metadata,
    #[error(transparent)]
    Store(#[from] vibe_secrets::Error),
}
/// The nonsecret description of one AI connection. The credential is not
/// here: a connection points at a keyring item by reference.
///
/// ```
/// use vibe_connections::Metadata;
///
/// let metadata = Metadata {
///     label: "Local gateway".into(),
///     endpoint: "http://127.0.0.1:8080/v1".into(),
///     api_kind: "openai-chat-completions".into(),
///     default_model: "synthetic".into(),
/// };
/// assert!(metadata.validate().is_ok());
///
/// // Credentials never travel inside the endpoint.
/// let leaking = Metadata {
///     endpoint: "https://user:password@provider.invalid/v1".into(),
///     ..metadata
/// };
/// assert!(leaking.validate().is_err());
/// ```
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
        if !matches!(
            self.api_kind.as_str(),
            "openai-chat-completions" | "openai-responses" | "anthropic-messages-bearer"
        ) || self.endpoint.len() > 4096
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
/// One stored connection: its metadata, its own revision and the keyring
/// item it refers to. It never carries the referenced value.
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::os::unix::fs::PermissionsExt;
/// # let settings = tempfile::tempdir()?;
/// # std::fs::set_permissions(settings.path(), std::fs::Permissions::from_mode(0o700))?;
/// use vibe_connections::{Metadata, Store};
/// use vibe_secrets::ProtectedValue;
///
/// let keyring = vibe_secrets::Store::at_settings(settings.path())?;
/// let secret = keyring.create("api-key", "Synthetic", ProtectedValue::new(b"synthetic".to_vec())?)?;
/// let store = Store::at_settings(settings.path())?;
/// let id = store.add(
///     Metadata {
///         label: "Local gateway".into(),
///         endpoint: "http://127.0.0.1:8080/v1".into(),
///         api_kind: "openai-chat-completions".into(),
///         default_model: "synthetic".into(),
///     },
///     &secret,
/// )?;
/// let connection = store.details(&id)?;
/// assert_eq!(connection.secret_ref, secret);
/// assert_eq!(connection.metadata.label, "Local gateway");
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub current_revision: SecretId,
    pub secret_ref: SecretId,
    pub metadata: Metadata,
}
/// Every stored connection together with the store revision they were read
/// at; that revision is the token a later mutation must present.
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::os::unix::fs::PermissionsExt;
/// # let settings = tempfile::tempdir()?;
/// # std::fs::set_permissions(settings.path(), std::fs::Permissions::from_mode(0o700))?;
/// use vibe_connections::{Metadata, Store};
/// use vibe_secrets::ProtectedValue;
///
/// let keyring = vibe_secrets::Store::at_settings(settings.path())?;
/// let secret = keyring.create("api-key", "Synthetic", ProtectedValue::new(b"synthetic".to_vec())?)?;
/// let store = Store::at_settings(settings.path())?;
/// let id = store.add(
///     Metadata {
///         label: "Local gateway".into(),
///         endpoint: "http://127.0.0.1:8080/v1".into(),
///         api_kind: "openai-chat-completions".into(),
///         default_model: "synthetic".into(),
///     },
///     &secret,
/// )?;
/// let snapshot = store.list()?;
/// assert_eq!(snapshot.items.len(), 1);
/// assert!(snapshot.items.contains_key(&id));
/// // The token observed with the rows guards the next mutation.
/// let selected = store.set_default(&id, snapshot.store_revision)?;
/// assert!(selected.store_revision > snapshot.store_revision);
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub store_revision: u64,
    pub items: BTreeMap<SecretId, Connection>,
}
/// The connection store under one explicitly given settings directory. It
/// owns nonsecret metadata only and reads no keyring value.
///
/// ```
/// use std::path::Path;
/// use vibe_connections::{Error, Store};
///
/// // The settings root is injected and absolute, never inferred.
/// assert!(matches!(
///     Store::at_settings(Path::new("relative/settings")),
///     Err(Error::Store(vibe_secrets::Error::Invalid))
/// ));
/// ```
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::os::unix::fs::PermissionsExt;
/// # let settings = tempfile::tempdir()?;
/// # std::fs::set_permissions(settings.path(), std::fs::Permissions::from_mode(0o700))?;
/// use vibe_connections::{Metadata, Store};
/// use vibe_secrets::ProtectedValue;
///
/// let keyring = vibe_secrets::Store::at_settings(settings.path())?;
/// let secret = keyring.create("api-key", "Synthetic", ProtectedValue::new(b"synthetic".to_vec())?)?;
/// let store = Store::at_settings(settings.path())?;
/// let id = store.add(
///     Metadata {
///         label: "Local gateway".into(),
///         endpoint: "http://127.0.0.1:8080/v1".into(),
///         api_kind: "openai-chat-completions".into(),
///         default_model: "synthetic".into(),
///     },
///     &secret,
/// )?;
/// assert_eq!(store.details(&id)?.metadata.default_model, "synthetic");
/// // Reading never publishes: the keyring item is still the only value.
/// assert_eq!(keyring.list()?.len(), 1);
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
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

/// The default connection, if one is selected, and the store revision the
/// answer was read at.
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::os::unix::fs::PermissionsExt;
/// # let settings = tempfile::tempdir()?;
/// # std::fs::set_permissions(settings.path(), std::fs::Permissions::from_mode(0o700))?;
/// use vibe_connections::{Metadata, Store};
/// use vibe_secrets::ProtectedValue;
///
/// let keyring = vibe_secrets::Store::at_settings(settings.path())?;
/// let secret = keyring.create("api-key", "Synthetic", ProtectedValue::new(b"synthetic".to_vec())?)?;
/// let store = Store::at_settings(settings.path())?;
/// let id = store.add(
///     Metadata {
///         label: "Local gateway".into(),
///         endpoint: "http://127.0.0.1:8080/v1".into(),
///         api_kind: "openai-chat-completions".into(),
///         default_model: "synthetic".into(),
///     },
///     &secret,
/// )?;
/// assert_eq!(store.default_selection()?.connection, None);
/// let revision = store.list()?.store_revision;
/// let selected = store.set_default(&id, revision)?;
/// assert_eq!(selected.connection, Some(id));
/// let cleared = store.clear_default(selected.store_revision)?;
/// assert_eq!(cleared.connection, None);
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
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
        self.default_scope_preview(id, DefaultScope::DefaultOnly)?;
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
///
/// ```
/// use vibe_connections::{MetadataCapability, metadata_capability};
///
/// match metadata_capability("pi") {
///     MetadataCapability::Compatible { protocol, scope, .. } => {
///         assert_eq!(protocol, "openai-chat-completions");
///         assert_eq!(scope, "stored-selection-only");
///     }
///     other => panic!("pi has recorded evidence, got {other:?}"),
/// }
/// assert_eq!(metadata_capability("claude"), MetadataCapability::Unsupported);
/// ```
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
fn metadata_supports(harness: &str, api_kind: &str) -> bool {
    matches!(metadata_capability(harness), MetadataCapability::Compatible { protocol, .. } if protocol == api_kind)
}
fn snapshot_metadata(
    snapshot: &vibe_secrets::ReferenceSnapshot,
    id: &SecretId,
) -> Result<Metadata, Error> {
    let bytes = snapshot.metadata.get(id).ok_or(Error::Metadata)?;
    let metadata: Metadata =
        toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Metadata)?)
            .map_err(|_| Error::Metadata)?;
    metadata.validate()?;
    Ok(metadata)
}
/// What stored connection metadata is known to be compatible with, per
/// harness. A harness without recorded evidence is `Unknown`, not refused.
///
/// ```
/// use vibe_connections::{MetadataCapability, metadata_capability};
///
/// assert!(matches!(
///     metadata_capability("qwen"),
///     MetadataCapability::Compatible { protocol: "openai-chat-completions", .. }
/// ));
/// assert_eq!(metadata_capability("unheard-of"), MetadataCapability::Unknown);
/// ```
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
/// How one harness is bound to a connection right now. `resolution` says
/// whether the binding is usable; it grants nothing by itself.
///
/// ```
/// # #[cfg(unix)]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::os::unix::fs::PermissionsExt;
/// # let settings = tempfile::tempdir()?;
/// # std::fs::set_permissions(settings.path(), std::fs::Permissions::from_mode(0o700))?;
/// use vibe_connections::{Metadata, Store};
/// use vibe_secrets::ProtectedValue;
///
/// let keyring = vibe_secrets::Store::at_settings(settings.path())?;
/// let secret = keyring.create("api-key", "Synthetic", ProtectedValue::new(b"synthetic".to_vec())?)?;
/// let store = Store::at_settings(settings.path())?;
/// let id = store.add(
///     Metadata {
///         label: "Local gateway".into(),
///         endpoint: "http://127.0.0.1:8080/v1".into(),
///         api_kind: "openai-chat-completions".into(),
///         default_model: "synthetic".into(),
///     },
///     &secret,
/// )?;
/// // Unbound, a harness keeps its own native configuration.
/// let native = store.binding("pi")?;
/// assert_eq!((native.mode.as_str(), native.resolution.as_str()), ("native", "unmanaged"));
///
/// let bound = store.set_binding(
///     "pi",
///     Some(vibe_secrets::ReferenceBinding::Connection { connection: id.clone() }),
///     native.store_revision,
/// )?;
/// assert_eq!(bound.mode, "connection");
/// assert_eq!(bound.connection, Some(id));
/// assert_eq!(bound.resolution, "metadata-only");
/// # Ok(())
/// # }
/// # #[cfg(not(unix))]
/// # fn main() {}
/// ```
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
        if !vibe_secrets::known_reference_harness(harness) {
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
            } else if connection.as_ref().is_some_and(|id| {
                snapshot_metadata(&snapshot, id)
                    .is_ok_and(|m| !metadata_supports(harness, &m.api_kind))
            }) {
                "unavailable: incompatible connection API"
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
        if binding.is_some() && !vibe_secrets::reference_binding_supported(harness) {
            return Err(Error::Invalid);
        }
        if let Some(binding) = &binding {
            let target = match binding {
                vibe_secrets::ReferenceBinding::Connection { connection } => {
                    Some(connection.clone())
                }
                vibe_secrets::ReferenceBinding::InheritDefault => {
                    self.default_selection()?.connection
                }
            };
            if let Some(target) = target
                && !metadata_supports(harness, &self.details(&target)?.metadata.api_kind)
            {
                return Err(Error::Invalid);
            }
        }
        self.0.bind_reference("ai", harness, binding, expected)?;
        self.binding(harness)
    }
}

mod setup;
pub use setup::{SetupPreview, SetupRow};
mod default_scope;
pub use default_scope::{DefaultScope, DefaultScopePreview, DefaultScopeRow};
mod native_settings;
pub use native_settings::{
    ClaudeSettingsAdapter, CodexSettingsAdapter, HermesSettingsAdapter, NativeSettingsAdapter,
    NativeSettingsPreview, OpenCodeSettingsAdapter, PiSettingsAdapter, QwenSettingsAdapter,
};
