//! Ordinary preview admission around generated optional object fields.
//! Serde's Option alone accepts null; the public nonnullable optional contract
//! treats only absence as unanswered, never an explicitly null submission.
use crate::generated::ordinary_configuration_preview_request::OrdinaryConfigurationPreviewRequest;
use serde::de::Error as _;

pub fn parse_request(
    value: serde_json::Value,
) -> Result<OrdinaryConfigurationPreviewRequest, serde_json::Error> {
    if value.get("input").is_some_and(serde_json::Value::is_null) {
        return Err(serde_json::Error::custom(
            "present ordinary preview input must be an object, not null",
        ));
    }
    serde_json::from_value(value)
}
