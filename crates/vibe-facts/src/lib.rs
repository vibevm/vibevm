//! Consumer-owned adoption facts for a vibevm project.
//!
//! The registry is stored in `vibefacts/`: host-spec facts in
//! `spec.toml`, package facts in one `<group>.<name>.toml` file per
//! source. This crate owns the model and persistence; CLI and check
//! surfaces remain thin adapters over it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#model");

pub mod overlay;
mod store;
pub mod sync;

use std::fmt;
use std::path::{Path, PathBuf};

use progress_core::model::{Stage, State};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use specmark::spec;
use thiserror::Error;

pub use overlay::{PackageOverlay, overlay_file_hash};
pub use store::Registry;

/// A validated progress-core stage/state pair as it appears on the TOML wire.
///
/// ```
/// use vibe_facts::FactStatus;
///
/// let status = FactStatus::parse("impl/done").unwrap();
/// assert_eq!(status.to_string(), "impl/done");
/// assert!(FactStatus::parse("impl/finished").is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FactStatus {
    stage: Stage,
    state: State,
}

impl FactStatus {
    pub fn new(stage: Stage, state: State) -> Self {
        Self { stage, state }
    }

    /// Parse through progress-core's closed vocabularies.
    pub fn parse(value: &str) -> Result<Self, RegistryError> {
        let Some((stage, state)) = value.split_once('/') else {
            return Err(RegistryError::InvalidStatus {
                value: value.to_string(),
            });
        };
        if state.contains('/') {
            return Err(RegistryError::InvalidStatus {
                value: value.to_string(),
            });
        }
        let Some(stage) = Stage::parse(stage) else {
            return Err(RegistryError::InvalidStatus {
                value: value.to_string(),
            });
        };
        let Some(state) = State::parse(state) else {
            return Err(RegistryError::InvalidStatus {
                value: value.to_string(),
            });
        };
        Ok(Self { stage, state })
    }

    pub fn stage(self) -> Stage {
        self.stage
    }

    pub fn state(self) -> State {
        self.state
    }
}

impl fmt::Display for FactStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.stage, self.state)
    }
}

impl Serialize for FactStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for FactStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(de::Error::custom)
    }
}

/// Which source owns a registry entry.
///
/// ```
/// use vibe_facts::FactOrigin;
///
/// assert_eq!(FactOrigin::Spec.to_string(), "spec");
/// assert_eq!(FactOrigin::Package.to_string(), "package");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactOrigin {
    Package,
    Spec,
}

impl fmt::Display for FactOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package => f.write_str("package"),
            Self::Spec => f.write_str("spec"),
        }
    }
}

/// One adoption claim in a registry file.
///
/// ```
/// use vibe_facts::{FactEntry, FactOrigin};
///
/// let entry = FactEntry::for_host(
///     "spec://org.vibevm.world/wal/flows/wal/WAL-SPEC#THE-LAW",
///     "org.example/demo",
///     None,
///     None,
/// )
/// .unwrap();
/// assert_eq!(entry.origin, FactOrigin::Package);
/// assert_eq!(entry.package.as_deref(), Some("org.vibevm.world/wal"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactEntry {
    pub address: String,
    pub origin: FactOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<FactStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl FactEntry {
    /// Construct an entry and derive its origin from the address coordinate.
    pub fn for_host(
        address: impl Into<String>,
        host_package: &str,
        status: Option<FactStatus>,
        comment: Option<String>,
    ) -> Result<Self, RegistryError> {
        let address = address.into();
        let package = package_from_address(&address)?;
        let (origin, package) = if package == host_package {
            (FactOrigin::Spec, None)
        } else {
            (FactOrigin::Package, Some(package))
        };
        let entry = Self {
            address,
            origin,
            package,
            status,
            comment,
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn source_package(&self) -> Result<String, RegistryError> {
        package_from_address(&self.address)
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        let address_package = package_from_address(&self.address)?;
        match (&self.origin, &self.package) {
            (FactOrigin::Spec, None) => Ok(()),
            (FactOrigin::Spec, Some(_)) => Err(RegistryError::InvalidEntry {
                address: self.address.clone(),
                reason: "origin `spec` must not carry `package`".to_string(),
            }),
            (FactOrigin::Package, Some(package)) if package == &address_package => Ok(()),
            (FactOrigin::Package, Some(package)) => Err(RegistryError::InvalidEntry {
                address: self.address.clone(),
                reason: format!(
                    "package `{package}` does not match address source `{address_package}`"
                ),
            }),
            (FactOrigin::Package, None) => Err(RegistryError::InvalidEntry {
                address: self.address.clone(),
                reason: "origin `package` requires `package`".to_string(),
            }),
        }
    }

    pub(crate) fn registry_file_name(&self) -> Result<String, RegistryError> {
        match self.origin {
            FactOrigin::Spec => Ok("spec.toml".to_string()),
            FactOrigin::Package => {
                let package =
                    self.package
                        .as_deref()
                        .ok_or_else(|| RegistryError::InvalidEntry {
                            address: self.address.clone(),
                            reason: "origin `package` requires `package`".to_string(),
                        })?;
                Ok(format!("{}.toml", package.replace('/', ".")))
            }
        }
    }
}

/// Read the host project's `<group>/<name>` coordinate from `vibe.toml`.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
/// )
/// .unwrap();
/// assert_eq!(vibe_facts::host_package(dir.path()).unwrap(), "org.example/demo");
/// ```
pub fn host_package(project_root: &Path) -> Result<String, RegistryError> {
    #[derive(Deserialize)]
    struct Manifest {
        project: Project,
    }
    #[derive(Deserialize)]
    struct Project {
        group: String,
        name: String,
    }

    let path = project_root.join("vibe.toml");
    let text = std::fs::read_to_string(&path).map_err(|source| RegistryError::Io {
        path: path.clone(),
        source,
    })?;
    let manifest: Manifest = toml::from_str(&text).map_err(|source| RegistryError::TomlRead {
        path: path.clone(),
        source,
    })?;
    if manifest.project.group.trim().is_empty() || manifest.project.name.trim().is_empty() {
        return Err(RegistryError::InvalidManifest {
            path,
            reason: "`[project].group` and `[project].name` must be non-empty".to_string(),
        });
    }
    Ok(format!(
        "{}/{}",
        manifest.project.group, manifest.project.name
    ))
}

/// Extract the `<group>/<name>` source coordinate from a full fact address.
///
/// ```
/// use vibe_facts::package_from_address;
///
/// assert_eq!(
///     package_from_address("spec://org.vibevm.core/vibevm/common/PROP-046#root").unwrap(),
///     "org.vibevm.core/vibevm"
/// );
/// assert!(package_from_address("spec://org.vibevm.core/vibevm#root").is_err());
/// ```
pub fn package_from_address(address: &str) -> Result<String, RegistryError> {
    let Some(rest) = address.strip_prefix("spec://") else {
        return Err(invalid_address(
            address,
            "address must start with `spec://`",
        ));
    };
    let Some((path, anchor)) = rest.split_once('#') else {
        return Err(invalid_address(address, "address must carry a `#anchor`"));
    };
    if anchor.is_empty() || anchor.contains('#') {
        return Err(invalid_address(
            address,
            "address must carry one non-empty anchor",
        ));
    }
    let mut parts = path.split('/');
    let Some(group) = parts.next().filter(|part| !part.is_empty()) else {
        return Err(invalid_address(address, "address group is missing"));
    };
    let Some(name) = parts.next().filter(|part| !part.is_empty()) else {
        return Err(invalid_address(address, "address package name is missing"));
    };
    let document: Vec<&str> = parts.collect();
    if document.is_empty() || document.iter().any(|part| part.is_empty()) {
        return Err(invalid_address(address, "address document path is missing"));
    }
    Ok(format!("{group}/{name}"))
}

fn invalid_address(address: &str, reason: &str) -> RegistryError {
    RegistryError::InvalidAddress {
        address: address.to_string(),
        reason: reason.to_string(),
    }
}

/// Every recoverable registry, schema, and sync failure.
///
/// ```
/// use vibe_facts::{FactStatus, RegistryError};
///
/// let err = FactStatus::parse("impl/finished").unwrap_err();
/// assert!(matches!(err, RegistryError::InvalidStatus { .. }));
/// assert!(err.to_string().contains("spec://org.vibevm.core/vibevm/common/PROP-046#model"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-046#model")]
pub enum RegistryError {
    #[error(
        "could not access `{}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: check the path exists and is readable)", path.display()
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "could not parse `{}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: repair the TOML against the closed [[fact]] schema)", path.display()
    )]
    TomlRead {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(
        "could not serialize registry TOML: {0} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: report this — emission of a validated registry cannot fail)"
    )]
    TomlWrite(#[from] toml::ser::Error),
    #[error(
        "unsupported registry schema {schema} in `{}` (expected 1) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: set `schema = 1` or upgrade vibe)", path.display()
    )]
    UnsupportedSchema { path: PathBuf, schema: u32 },
    #[error(
        "duplicate fact address `{address}` in `{}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: keep one [[fact]] per address)", path.display()
    )]
    DuplicateAddress { path: PathBuf, address: String },
    #[error(
        "fact address `{address}` appears in both `{}` and `{}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: one address has one source file — remove the stray copy)",
        first.display(), second.display()
    )]
    DuplicateRegistryAddress {
        address: String,
        first: PathBuf,
        second: PathBuf,
    },
    #[error(
        "invalid status `{value}`; expected a progress-core `<stage>/<state>` pair \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: use the progress-core vocabulary, e.g. `impl/done`)"
    )]
    InvalidStatus { value: String },
    #[error(
        "invalid fact address `{address}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: use the full `spec://<group>/<name>/<doc>#<anchor>` form)"
    )]
    InvalidAddress { address: String, reason: String },
    #[error(
        "invalid fact entry `{address}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: align `origin`/`package` with the address's source coordinate)"
    )]
    InvalidEntry { address: String, reason: String },
    #[error(
        "invalid project manifest `{}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: give vibe.toml a non-empty [project] group and name)", path.display()
    )]
    InvalidManifest { path: PathBuf, reason: String },
    #[error(
        "registry home `{}` exists but is not a directory \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: `vibefacts` at the project root must be a directory)", path.display()
    )]
    InvalidRegistryHome { path: PathBuf },
    #[error(
        "invalid spec marker in `{}` at line {line}: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#laws; \
          fix: repair the marker — the L2 sync reads the spec as authoritative)",
        path.display()
    )]
    SpecParse {
        path: PathBuf,
        line: usize,
        message: String,
    },
    #[error(
        "internal registry invariant failed: {0} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-046#model; \
          fix: report this — the registry reached a state its laws forbid)"
    )]
    Invariant(String),
}

#[cfg(test)]
mod tests;
