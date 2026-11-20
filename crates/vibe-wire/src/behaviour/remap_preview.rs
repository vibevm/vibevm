//! Strict request-local shape; values/keys never enter decode diagnostics.
use crate::generated::remap_preview_input::RemapPreviewInput;
use serde::de::Error as _;
pub fn parse(bytes: &[u8]) -> Result<RemapPreviewInput, serde_json::Error> {
    serde_json::from_slice(bytes).map_err(|_|serde_json::Error::custom("remap preview input: invalid shape; expected selections group records with exact intent and single-member array"))
}
