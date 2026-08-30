//! The agent-plugin adapter's structured config — §6.2's inputs,
//! validated at `plan`.
//!
//! Strict, like every table this project owns. Two members and no more:
//! the plugin `source` tree, and the `place` map that says where each
//! DECLARED input lands inside the produced plugin. `place` exists because
//! §6.2 fixes the plugin's shape — root `plugin.json`, `skills/<name>/
//! SKILL.md`, optional `mcp.json`, "only valid reverse-domain
//! client-extension directories" — so a consumed artifact cannot be
//! dropped somewhere the shape does not admit, and the engine will not
//! guess a destination for it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use vibe_core::manifest::ExtensionConfig;

use crate::mechanism::MechanismError;
use crate::mechanism::contain::checked_relative;
use crate::mechanism::error::preview;

/// The engine-owned members a target may not set, each with the reason.
const ENGINE_OWNED: [(&str, &str); 3] = [
    (
        "output",
        "the distributable's placement is engine-owned (§3.2: a provider cannot mint an output \
         path); the plugin IS the target's own package directory",
    ),
    (
        "output_dir",
        "the distributable's placement is engine-owned (§3.2: a provider cannot mint an output \
         path)",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The validated `config` table of one `package:agent-plugin` target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentPluginConfig {
    /// The plugin source tree, project-relative and canonical.
    pub(crate) source: String,
    /// Declared-input identity -> plugin-relative destination, in
    /// declaration order.
    pub(crate) place: Vec<(String, String)>,
}

impl AgentPluginConfig {
    /// Read and validate one target's `config` table.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, MechanismError> {
        let mut source: Option<String> = None;
        let mut place: Vec<(String, String)> = Vec::new();
        if let Some(config) = config {
            for (member, value) in config.as_table() {
                if let Some((_, reason)) = ENGINE_OWNED.iter().find(|(name, _)| name == member) {
                    return Err(MechanismError::Config {
                        target: target.to_owned(),
                        member: preview(member),
                        reason: (*reason).to_owned(),
                    });
                }
                match member.as_str() {
                    "source" => source = Some(relative_member(target, "source", value)?),
                    "place" => place = placements(target, value)?,
                    _ => {
                        return Err(MechanismError::Config {
                            target: target.to_owned(),
                            member: preview(member),
                            reason: "unknown member; the agent-plugin config is `source` and \
                                     `place`"
                                .to_owned(),
                        });
                    }
                }
            }
        }
        let source = source.ok_or_else(|| MechanismError::Config {
            target: target.to_owned(),
            member: "source".to_owned(),
            reason: "required; name the plugin source directory holding `plugin.json`".to_owned(),
        })?;
        Ok(Self { source, place })
    }
}

/// One `place` table: declared-input identity -> plugin-relative
/// destination inside a client-extension directory.
fn placements(target: &str, value: &toml::Value) -> Result<Vec<(String, String)>, MechanismError> {
    let table = value.as_table().ok_or_else(|| MechanismError::Config {
        target: target.to_owned(),
        member: "place".to_owned(),
        reason: format!("expected a table, found {}", preview(value.type_str())),
    })?;
    let mut placements = Vec::with_capacity(table.len());
    for (name, destination) in table {
        let spelled = destination.as_str().ok_or_else(|| MechanismError::Config {
            target: target.to_owned(),
            member: format!("place.{}", preview(name)),
            reason: format!(
                "expected a string destination, found {}",
                preview(destination.type_str())
            ),
        })?;
        let relative = checked_relative(spelled).map_err(|fault| MechanismError::Config {
            target: target.to_owned(),
            member: format!("place.{}", preview(name)),
            reason: format!("`{}` is unusable: {}", preview(spelled), fault.reason()),
        })?;
        let mut segments = relative.split('/');
        let head = segments.next().unwrap_or_default();
        if segments.next().is_none() {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: format!("place.{}", preview(name)),
                reason: "a placed file lives INSIDE a reverse-domain client-extension directory, \
                         so its destination has at least two components"
                    .to_owned(),
            });
        }
        if !is_reverse_domain(head) {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: format!("place.{}", preview(name)),
                reason: format!(
                    "`{}` is not a valid reverse-domain client-extension directory; §6.2 admits \
                     `plugin.json`, `skills/<name>/SKILL.md`, an optional `mcp.json` and \
                     reverse-domain directories, and nothing else",
                    preview(head)
                ),
            });
        }
        placements.push((name.clone(), relative));
    }
    Ok(placements)
}

/// One canonical relative path member.
fn relative_member(
    target: &str,
    member: &str,
    value: &toml::Value,
) -> Result<String, MechanismError> {
    let text = value.as_str().ok_or_else(|| MechanismError::Config {
        target: target.to_owned(),
        member: member.to_owned(),
        reason: format!("expected a string, found {}", preview(value.type_str())),
    })?;
    checked_relative(text).map_err(|fault| MechanismError::Config {
        target: target.to_owned(),
        member: member.to_owned(),
        reason: format!("`{}` is unusable: {}", preview(text), fault.reason()),
    })
}

/// The reverse-domain grammar §6.2 admits for a client-extension
/// directory: at least two dot-separated labels, each lowercase
/// alphanumeric with inner hyphens.
pub(crate) fn is_reverse_domain(value: &str) -> bool {
    let labels: Vec<&str> = value.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    labels.iter().all(|label| {
        let bytes = label.as_bytes();
        if bytes.is_empty() || bytes.len() > 63 {
            return false;
        }
        let ends_well = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        ends_well(bytes[0])
            && ends_well(bytes[bytes.len() - 1])
            && bytes
                .iter()
                .all(|byte| ends_well(*byte) || *byte == b'-' || *byte == b'_')
    })
}
