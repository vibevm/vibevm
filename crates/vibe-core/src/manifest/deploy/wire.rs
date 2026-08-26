//! Strict serde intermediates for the authored `[deploy]` section.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{DeployProfile, DeploySection, DeployTarget};
use crate::manifest::extension::ExtensionConfig;
use crate::manifest::mechanism::{MechanismKey, ProviderPin};

const KEY_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeploySectionWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_profile: Option<String>,
    #[serde(default, rename = "target", skip_serializing_if = "Vec::is_empty")]
    targets: Vec<DeployTargetWire>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    profiles: IndexMap<String, DeployProfileWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeployTargetWire {
    id: String,
    artifact: String,
    mechanism: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    depends_on: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<toml::Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeployProfileWire {
    targets: Vec<String>,
}

impl TryFrom<DeploySectionWire> for DeploySection {
    type Error = String;

    fn try_from(wire: DeploySectionWire) -> Result<Self, Self::Error> {
        Ok(Self {
            default_profile: wire.default_profile,
            targets: wire
                .targets
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, String>>()?,
            profiles: wire
                .profiles
                .into_iter()
                .map(|(name, profile)| {
                    Ok((
                        name,
                        DeployProfile {
                            targets: profile.targets,
                        },
                    ))
                })
                .collect::<Result<IndexMap<_, _>, String>>()?,
        })
    }
}

impl TryFrom<DeploySection> for DeploySectionWire {
    type Error = String;

    fn try_from(section: DeploySection) -> Result<Self, Self::Error> {
        for target in &section.targets {
            target.validate()?;
        }
        Ok(Self {
            default_profile: section.default_profile,
            targets: section
                .targets
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, String>>()?,
            profiles: section
                .profiles
                .into_iter()
                .map(|(name, profile)| {
                    (
                        name,
                        DeployProfileWire {
                            targets: profile.targets,
                        },
                    )
                })
                .collect(),
        })
    }
}

impl TryFrom<DeployTargetWire> for DeployTarget {
    type Error = String;

    fn try_from(wire: DeployTargetWire) -> Result<Self, Self::Error> {
        let mechanism = wire.mechanism.parse::<MechanismKey>().map_err(|error| {
            format!(
                "[[deploy.target]] `{}` field `mechanism` value `{}` is invalid: {error} ({KEY_GRAMMAR})",
                wire.id, wire.mechanism,
            )
        })?;
        let provider = wire
            .provider
            .map(|value| {
                ProviderPin::parse(&value).map_err(|error| {
                    format!(
                        "[[deploy.target]] `{}` field `provider` value `{value}` is invalid: {error}; an exact pin is `<group>/<package>#<id>` ({KEY_GRAMMAR})",
                        wire.id,
                    )
                })
            })
            .transpose()?;
        Ok(Self {
            id: wire.id,
            artifact: wire.artifact,
            mechanism,
            provider,
            depends_on: wire.depends_on,
            config: wire.config.map(ExtensionConfig::from_table),
        })
    }
}

impl TryFrom<DeployTarget> for DeployTargetWire {
    type Error = String;

    fn try_from(target: DeployTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            id: target.id,
            artifact: target.artifact,
            mechanism: target.mechanism.to_string(),
            provider: target.provider.map(|pin| pin.to_string()),
            depends_on: target.depends_on,
            config: target.config.map(ExtensionConfig::into_table),
        })
    }
}
