use std::collections::HashSet;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_wire::generated::native::e1::manifest::{Manifest, ManifestExtension};
use vibe_wire::generated::native::e1::reply::Reply;

use crate::error::NativeLoadError;
use crate::{MANIFEST_CAP, scalar_preview};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-SCHEMA-VERSIONED");

pub(crate) fn select_manifest(
    bytes: &[u8],
    extension_id: &str,
    expected_point: ExtensionPoint,
    expected_schema: Option<u32>,
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
    let selected = manifest
        .extensions
        .iter()
        .find(|extension| extension.id == extension_id)
        .ok_or_else(|| NativeLoadError::MissingExtensionId {
            path: path.to_owned(),
            id: scalar_preview(extension_id),
        })?;
    admit_selected(
        selected,
        extension_id,
        expected_point,
        expected_schema,
        path,
    )
}

fn admit_selected(
    selected: &ManifestExtension,
    extension_id: &str,
    expected_point: ExtensionPoint,
    expected_schema: Option<u32>,
    path: &str,
) -> Result<(), NativeLoadError> {
    let actual_point = selected.point.parse::<ExtensionPoint>().map_err(|_| {
        NativeLoadError::InvalidExtensionPoint {
            path: path.to_owned(),
            id: scalar_preview(extension_id),
            point: scalar_preview(&selected.point),
        }
    })?;
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

fn option_schema(value: Option<u32>) -> String {
    value.map_or_else(|| "absent".to_owned(), |value| value.to_string())
}

fn json_reason(error: &serde_json::Error) -> String {
    format!("JSON at line {}, column {}", error.line(), error.column())
}
