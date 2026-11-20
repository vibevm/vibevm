//! Ordinary preview admission around generated optional object fields.
//! Decode errors precede secret classification: never expose caller bytes or keys.
use crate::generated::ordinary_configuration_preview_request::{
    Input, OrdinaryConfigurationPreviewRequest,
};
use serde::de::Error as _;

fn invalid_input() -> serde_json::Error {
    serde_json::Error::custom(
        "ordinary preview input: invalid shape; expected input.selections string map and input.parameters boolean map",
    )
}

/// Strict generated decoding with a closed diagnostic, not a serde error chain.
pub fn parse_input(bytes: &[u8]) -> Result<Input, serde_json::Error> {
    serde_json::from_slice(bytes).map_err(|_| invalid_input())
}

pub fn parse_request(
    value: serde_json::Value,
) -> Result<OrdinaryConfigurationPreviewRequest, serde_json::Error> {
    if value.get("input").is_some_and(serde_json::Value::is_null) {
        return Err(serde_json::Error::custom(
            "present ordinary preview input must be an object, not null",
        ));
    }
    serde_json::from_value(value).map_err(|_| {
        serde_json::Error::custom(
            "ordinary preview request: invalid shape; expected package string and optional input object with selections string map and parameters boolean map",
        )
    })
}
