//! Semantic version admission around the strict generated ordinary envelope.
use crate::generated::ordinary_install_answers::OrdinaryInstallAnswers;
use serde::de::Error as _;
pub fn parse(bytes: &[u8]) -> Result<OrdinaryInstallAnswers, serde_json::Error> {
    // This boundary precedes definition classification; serde errors may quote
    // caller-supplied values or unknown keys, so retain only a static schema.
    let input: OrdinaryInstallAnswers = serde_json::from_slice(bytes).map_err(|_| {
        serde_json::Error::custom(
            "ordinary install answers: invalid shape; expected protocol/package/project_witness strings, source registry/content_hash strings, input.selections string map and input.parameters boolean map",
        )
    })?;
    if input.protocol != "vibe-ordinary-install-answers/1" {
        return Err(serde_json::Error::custom(
            "unsupported ordinary answer protocol",
        ));
    }
    Ok(input)
}
