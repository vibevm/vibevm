//! Semantic version admission around the strict generated ordinary envelope.
use crate::generated::ordinary_install_answers::OrdinaryInstallAnswers;
use serde::de::Error as _;
pub fn parse(bytes: &[u8]) -> Result<OrdinaryInstallAnswers, serde_json::Error> {
    let input: OrdinaryInstallAnswers = serde_json::from_slice(bytes)?;
    if input.protocol != "vibe-ordinary-install-answers/1" {
        return Err(serde_json::Error::custom(
            "unsupported ordinary answer protocol",
        ));
    }
    Ok(input)
}
