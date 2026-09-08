use std::collections::HashSet;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{MechanismKey, MechanismRole};
use vibe_wire::behaviour::native_deploy::{self, DeployOperation};
use vibe_wire::generated::native::e1::deploy_reply::DeployReply;
use vibe_wire::generated::native::e1::manifest::{Manifest, ManifestExtension};
use vibe_wire::generated::native::e1::mechanism_manifest::{
    MechanismDescriptor, MechanismManifest, NativeMechanismRole,
};
use vibe_wire::generated::native::e1::reply::Reply;

use crate::error::NativeLoadError;
use crate::{MANIFEST_CAP, scalar_preview};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-SCHEMA-VERSIONED");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestFamily {
    Lifecycle,
    Compiler,
}

impl ManifestFamily {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Lifecycle => "lifecycle",
            Self::Compiler => "compiler",
        }
    }

    const fn of(point: ExtensionPoint) -> Self {
        match point {
            ExtensionPoint::Phase(_) | ExtensionPoint::Slot(_) => Self::Lifecycle,
            ExtensionPoint::Compile(_) => Self::Compiler,
        }
    }
}

pub(crate) fn select_manifest(
    bytes: &[u8],
    extension_id: &str,
    expected_point: ExtensionPoint,
    expected_schema: Option<u32>,
    expected_family: ManifestFamily,
    path: &str,
) -> Result<(), NativeLoadError> {
    if bytes.len() >= MANIFEST_CAP {
        return Err(NativeLoadError::ManifestTooLarge {
            path: path.to_owned(),
            cap: MANIFEST_CAP,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| NativeLoadError::ManifestUtf8 {
        path: path.to_owned(),
    })?;
    let manifest: Manifest =
        serde_json::from_str(text).map_err(|error| NativeLoadError::ManifestJson {
            path: path.to_owned(),
            reason: json_reason(&error),
        })?;
    let mut ids = HashSet::new();
    for extension in &manifest.extensions {
        if !ids.insert(extension.id.as_str()) {
            return Err(NativeLoadError::DuplicateExtensionId {
                path: path.to_owned(),
                id: scalar_preview(&extension.id),
            });
        }
    }
    let points = manifest
        .extensions
        .iter()
        .map(|extension| parse_point(extension, path))
        .collect::<Result<Vec<_>, _>>()?;
    for (extension, actual_point) in manifest.extensions.iter().zip(points.iter().copied()) {
        let actual_family = ManifestFamily::of(actual_point);
        if actual_family != expected_family {
            return Err(NativeLoadError::ManifestFamilyMismatch {
                path: path.to_owned(),
                id: scalar_preview(&extension.id),
                actual: actual_family.as_str(),
                expected: expected_family.as_str(),
            });
        }
    }
    let selected_index = manifest
        .extensions
        .iter()
        .position(|extension| extension.id == extension_id)
        .ok_or_else(|| NativeLoadError::MissingExtensionId {
            path: path.to_owned(),
            id: scalar_preview(extension_id),
        })?;
    admit_selected(
        &manifest.extensions[selected_index],
        points[selected_index],
        extension_id,
        expected_point,
        expected_schema,
        path,
    )
}

fn admit_selected(
    selected: &ManifestExtension,
    actual_point: ExtensionPoint,
    extension_id: &str,
    expected_point: ExtensionPoint,
    expected_schema: Option<u32>,
    path: &str,
) -> Result<(), NativeLoadError> {
    if actual_point != expected_point {
        return Err(NativeLoadError::ExtensionPointMismatch {
            path: path.to_owned(),
            id: scalar_preview(extension_id),
            actual: scalar_preview(&selected.point),
            expected: scalar_preview(&expected_point.to_string()),
        });
    }
    if selected.ir_schema != expected_schema {
        return Err(NativeLoadError::IrSchemaMismatch {
            path: path.to_owned(),
            id: scalar_preview(extension_id),
            actual: option_schema(selected.ir_schema),
            expected: option_schema(expected_schema),
        });
    }
    Ok(())
}

fn parse_point(
    extension: &ManifestExtension,
    path: &str,
) -> Result<ExtensionPoint, NativeLoadError> {
    extension
        .point
        .parse::<ExtensionPoint>()
        .map_err(|_| NativeLoadError::InvalidExtensionPoint {
            path: path.to_owned(),
            id: scalar_preview(&extension.id),
            point: scalar_preview(&extension.point),
        })
}

pub(crate) fn parse_reply(bytes: &[u8], path: &str) -> Result<Reply, NativeLoadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| NativeLoadError::ReplyUtf8 {
        path: path.to_owned(),
    })?;
    let reply: Reply = serde_json::from_str(text).map_err(|error| NativeLoadError::ReplyJson {
        path: path.to_owned(),
        reason: json_reason(&error),
    })?;
    if reply.envelope != 1 {
        return Err(NativeLoadError::ReplyEnvelope {
            path: path.to_owned(),
            actual: reply.envelope,
        });
    }
    Ok(reply)
}

pub(crate) fn select_mechanism_manifest(
    bytes: &[u8],
    mechanism_id: &str,
    logical_key: &MechanismKey,
    path: &str,
) -> Result<MechanismDescriptor, NativeLoadError> {
    if bytes.len() >= MANIFEST_CAP {
        return Err(NativeLoadError::ManifestTooLarge {
            path: path.to_owned(),
            cap: MANIFEST_CAP,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| NativeLoadError::ManifestUtf8 {
        path: path.to_owned(),
    })?;
    let manifest: MechanismManifest =
        serde_json::from_str(text).map_err(|error| NativeLoadError::MechanismManifestJson {
            path: path.to_owned(),
            reason: json_reason(&error),
        })?;
    vibe_wire::behaviour::native_mechanism::validate_manifest(&manifest).map_err(|error| {
        NativeLoadError::MechanismManifestAdmission {
            path: path.to_owned(),
            reason: error.to_string(),
        }
    })?;
    let descriptor = manifest
        .mechanisms
        .into_iter()
        .find(|descriptor| descriptor.id == mechanism_id)
        .ok_or_else(|| NativeLoadError::MissingMechanismId {
            path: path.to_owned(),
            id: scalar_preview(mechanism_id),
        })?;
    if descriptor_role(&descriptor.role) != logical_key.role()
        || descriptor.name != logical_key.name()
    {
        return Err(NativeLoadError::MechanismManifestAdmission {
            path: path.to_owned(),
            reason: "selected descriptor role/name differs from the registry mechanism key"
                .to_owned(),
        });
    }
    Ok(descriptor)
}

fn descriptor_role(role: &NativeMechanismRole) -> MechanismRole {
    match role {
        NativeMechanismRole::Build => MechanismRole::Build,
        NativeMechanismRole::Package => MechanismRole::Package,
        NativeMechanismRole::Deploy => MechanismRole::Deploy,
        NativeMechanismRole::Acquire => MechanismRole::Acquire,
    }
}

pub(crate) fn parse_mechanism_reply(
    bytes: &[u8],
    expected: DeployOperation,
    mechanism_id: &str,
    path: &str,
) -> Result<DeployReply, NativeLoadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| NativeLoadError::ReplyUtf8 {
        path: path.to_owned(),
    })?;
    let reply: DeployReply =
        serde_json::from_str(text).map_err(|error| NativeLoadError::MechanismReplyJson {
            path: path.to_owned(),
            reason: json_reason(&error),
        })?;
    native_deploy::validate_reply(expected, &reply).map_err(|error| {
        NativeLoadError::MechanismReplyAdmission {
            path: path.to_owned(),
            id: scalar_preview(mechanism_id),
            reason: error.to_string(),
        }
    })?;
    Ok(reply)
}

fn option_schema(value: Option<u32>) -> String {
    value.map_or_else(|| "absent".to_owned(), |value| value.to_string())
}

fn json_reason(error: &serde_json::Error) -> String {
    format!("JSON at line {}, column {}", error.line(), error.column())
}
