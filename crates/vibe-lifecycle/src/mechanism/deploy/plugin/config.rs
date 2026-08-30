//! Strict `{ name = "portable-kebab" }` client-plugin deploy config.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_core::manifest::{ExtensionConfig, SkillDecl};

use crate::mechanism::error::{DeployProviderError, preview};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginDeployConfig {
    pub(crate) name: String,
}

impl PluginDeployConfig {
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, DeployProviderError> {
        let mut name = None;
        if let Some(config) = config {
            for (member, value) in config.as_table() {
                if member != "name" {
                    return Err(DeployProviderError::Config {
                        target: target.to_owned(),
                        member: preview(member),
                        reason: "unknown member; client choice, home, paths, marketplaces and environment are provider/engine authority, so this config is exactly `name`".to_owned(),
                    });
                }
                let toml::Value::String(value) = value else {
                    return Err(DeployProviderError::Config {
                        target: target.to_owned(),
                        member: "name".to_owned(),
                        reason: format!("expected a string, found {}", value.type_str()),
                    });
                };
                if !SkillDecl::valid_name(value) {
                    return Err(DeployProviderError::Config {
                        target: target.to_owned(),
                        member: "name".to_owned(),
                        reason: format!(
                            "`{}` is not the shared portable lowercase-kebab name (1..64 bytes, one safe path component)",
                            preview(value)
                        ),
                    });
                }
                name = Some(value.clone());
            }
        }
        name.map(|name| Self { name })
            .ok_or_else(|| DeployProviderError::Config {
                target: target.to_owned(),
                member: "name".to_owned(),
                reason: "required with no default; the client plugin coordinate is explicit"
                    .to_owned(),
            })
    }
}
