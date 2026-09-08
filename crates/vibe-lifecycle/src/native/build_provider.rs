//! Exact `build:cargo` provider selection and bootstrap admission.

use vibe_core::manifest::{ExtensionHandler, MechanismKey};
use vibe_extension_registry::{MechanismRegistryRow, resolve_mechanism};

use super::path::prebuilt_file;
use super::provider::mechanism_facts;
use super::{NativeArtifactError, NativeBuildExecution};

pub(super) struct SelectedBuildProvider<'registry> {
    pub(super) key: MechanismKey,
    pub(super) row: &'registry MechanismRegistryRow,
}

impl SelectedBuildProvider<'_> {
    pub(super) fn pin(&self) -> String {
        self.row.pin().to_string()
    }
}

pub(super) enum MechanismBuildTransport {
    Builtin { pin: String },
    ForeignSource { pin: String },
    ForeignPrebuilt { pin: String },
}

impl MechanismBuildTransport {
    pub(super) fn pin(&self) -> &str {
        match self {
            Self::Builtin { pin } | Self::ForeignSource { pin } | Self::ForeignPrebuilt { pin } => {
                pin
            }
        }
    }

    pub(super) fn admit_prepare(&self, platform: &str) -> Result<(), NativeArtifactError> {
        match self {
            Self::Builtin { .. } => Ok(()),
            Self::ForeignSource { pin } => Err(NativeArtifactError::BootstrapCycle {
                provider: pin.clone(),
                platform: platform.to_owned(),
            }),
            Self::ForeignPrebuilt { pin } => Err(NativeArtifactError::BuildAdapterNotConnected {
                provider: pin.clone(),
            }),
        }
    }
}

pub(super) fn select_build_provider<'registry>(
    execution: &NativeBuildExecution<'registry>,
) -> Result<SelectedBuildProvider<'registry>, NativeArtifactError> {
    let selected = resolve_build_provider(execution)?;
    admit_builtin(selected.row)?;
    Ok(selected)
}

pub(super) fn mechanism_build_provider(
    execution: &NativeBuildExecution<'_>,
) -> Result<MechanismBuildTransport, NativeArtifactError> {
    let selected = resolve_build_provider(execution)?;
    let pin = selected.pin();
    if selected.row.is_builtin() {
        admit_builtin(selected.row)?;
        return Ok(MechanismBuildTransport::Builtin { pin });
    }
    let ExtensionHandler::Native {
        crate_dir,
        prebuilt,
    } = selected.row.handler()
    else {
        return Err(NativeArtifactError::TransportNotLanded {
            provider: pin,
            kind: selected.row.handler().kind().to_owned(),
        });
    };
    let provider = mechanism_facts(selected.row)?;
    if let Some(path) = prebuilt
        .as_ref()
        .and_then(|paths| paths.get(execution.platform.key()))
    {
        prebuilt_file(&pin, &provider, path, execution.platform)?;
        return Ok(MechanismBuildTransport::ForeignPrebuilt { pin });
    }
    if crate_dir.is_some() {
        return Ok(MechanismBuildTransport::ForeignSource { pin });
    }
    let mut declared = prebuilt
        .iter()
        .flat_map(|paths| paths.keys())
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    if declared.is_empty() {
        declared.push("none".to_owned());
    }
    Err(NativeArtifactError::NoCurrentArtifact {
        extension: selected.key.to_string(),
        platform: execution.platform.key().to_owned(),
        declared: declared.join(", "),
    })
}

fn resolve_build_provider<'registry>(
    execution: &NativeBuildExecution<'registry>,
) -> Result<SelectedBuildProvider<'registry>, NativeArtifactError> {
    let key = "build:cargo".parse::<MechanismKey>().map_err(|error| {
        NativeArtifactError::MechanismSelection {
            reason: format!("engine-owned key is invalid: {error}"),
        }
    })?;
    let selection =
        resolve_mechanism(execution.registry, &key, None, execution.routes).map_err(|error| {
            NativeArtifactError::MechanismSelection {
                reason: error.to_string(),
            }
        })?;
    Ok(SelectedBuildProvider {
        key,
        row: selection.row(),
    })
}

fn admit_builtin(row: &MechanismRegistryRow) -> Result<(), NativeArtifactError> {
    let provider = row.pin().to_string();
    if !row.is_builtin() {
        return Err(NativeArtifactError::TransportNotLanded {
            provider,
            kind: row.handler().kind().to_owned(),
        });
    }
    match row.handler() {
        ExtensionHandler::Builtin { name } if name == "cargo" => Ok(()),
        ExtensionHandler::Builtin { name } => Err(NativeArtifactError::UnknownBuiltin {
            provider,
            name: name.clone(),
        }),
        handler => Err(NativeArtifactError::UnknownBuiltin {
            provider,
            name: handler.kind().to_owned(),
        }),
    }
}
