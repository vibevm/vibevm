//! Strict serde intermediates for the authored `[artifacts]` section.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{ArtifactInput, ArtifactOutput, ArtifactTarget, ArtifactsSection};
use crate::manifest::extension::ExtensionConfig;
use crate::manifest::mechanism::{MechanismKey, ProviderPin};

const ARTIFACT_REGISTRY: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY";
const KEY_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactsWire {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    build: Vec<ArtifactTargetWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    package: Vec<ArtifactTargetWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactTargetWire {
    id: String,
    mechanism: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inputs: Option<Vec<toml::Table>>,
    outputs: Vec<ArtifactOutputWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<toml::Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactOutputWire {
    id: String,
    kind: String,
}

impl TryFrom<ArtifactsWire> for ArtifactsSection {
    type Error = String;

    fn try_from(wire: ArtifactsWire) -> Result<Self, Self::Error> {
        Ok(Self {
            build: wire
                .build
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, String>>()?,
            package: wire
                .package
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, String>>()?,
        })
    }
}

impl TryFrom<ArtifactsSection> for ArtifactsWire {
    type Error = String;

    fn try_from(section: ArtifactsSection) -> Result<Self, Self::Error> {
        for (role, target) in section.all_targets() {
            target.validate(role)?;
        }
        Ok(Self {
            build: section
                .build
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, String>>()?,
            package: section
                .package
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, String>>()?,
        })
    }
}

impl TryFrom<ArtifactTargetWire> for ArtifactTarget {
    type Error = String;

    fn try_from(wire: ArtifactTargetWire) -> Result<Self, Self::Error> {
        let mechanism = wire.mechanism.parse::<MechanismKey>().map_err(|error| {
            format!(
                "[[artifacts]] target `{}` field `mechanism` value `{}` is invalid: {error} ({KEY_GRAMMAR})",
                wire.id, wire.mechanism,
            )
        })?;
        let provider = wire
            .provider
            .map(|value| {
                ProviderPin::parse(&value).map_err(|error| {
                    format!(
                        "[[artifacts]] target `{}` field `provider` value `{value}` is invalid: {error}; an exact pin is `<group>/<package>#<id>` ({KEY_GRAMMAR})",
                        wire.id,
                    )
                })
            })
            .transpose()?;
        let inputs = wire
            .inputs
            .map(|rows| {
                rows.into_iter()
                    .enumerate()
                    .map(|(index, row)| input_from_table(&wire.id, index, row))
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?;
        Ok(Self {
            id: wire.id,
            mechanism,
            provider,
            inputs,
            outputs: wire
                .outputs
                .into_iter()
                .map(|output| ArtifactOutput {
                    id: output.id,
                    kind: output.kind,
                })
                .collect(),
            config: wire.config.map(ExtensionConfig::from_table),
        })
    }
}

impl TryFrom<ArtifactTarget> for ArtifactTargetWire {
    type Error = String;

    fn try_from(target: ArtifactTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            id: target.id,
            mechanism: target.mechanism.to_string(),
            provider: target.provider.map(|pin| pin.to_string()),
            inputs: target
                .inputs
                .map(|rows| rows.into_iter().map(input_to_table).collect()),
            outputs: target
                .outputs
                .into_iter()
                .map(|output| ArtifactOutputWire {
                    id: output.id,
                    kind: output.kind,
                })
                .collect(),
            config: target.config.map(ExtensionConfig::into_table),
        })
    }
}

/// One input row is a strict tagged-one-of inline table: exactly `path` or
/// exactly `artifact`, never both, never neither, never anything else.
fn input_from_table(
    target_id: &str,
    index: usize,
    row: toml::Table,
) -> Result<ArtifactInput, String> {
    let refuse = |reason: String| {
        Err(format!(
            "[[artifacts]] target `{target_id}` field `inputs` row {index} {reason} ({ARTIFACT_REGISTRY})"
        ))
    };
    match (row.contains_key("path"), row.contains_key("artifact")) {
        (true, true) => refuse(
            "carries both `path` and `artifact`; an input is exactly one of them".into(),
        ),
        (false, false) => refuse(
            "carries neither `path` nor `artifact`; an input is exactly `{ path = \"…\" }` or `{ artifact = \"id\" }`".into(),
        ),
        (true, false) if row.len() > 1 => refuse(
            "carries `path` plus unknown field(s); an input row is exactly `{ path = \"…\" }`".into(),
        ),
        (false, true) if row.len() > 1 => refuse(
            "carries `artifact` plus unknown field(s); an input row is exactly `{ artifact = \"id\" }`".into(),
        ),
        (true, false) => {
            let Some(toml::Value::String(spelling)) = row.get("path") else {
                return refuse("field `path` must be a string".into());
            };
            Ok(ArtifactInput::Path {
                path: PathBuf::from(spelling),
            })
        }
        (false, true) => {
            let Some(toml::Value::String(id)) = row.get("artifact") else {
                return refuse("field `artifact` must be a string".into());
            };
            Ok(ArtifactInput::Artifact {
                artifact: id.to_owned(),
            })
        }
    }
}

fn input_to_table(input: ArtifactInput) -> toml::Table {
    let mut table = toml::Table::new();
    match input {
        ArtifactInput::Path { path } => {
            table.insert(
                "path".into(),
                toml::Value::String(path.display().to_string()),
            );
        }
        ArtifactInput::Artifact { artifact } => {
            table.insert("artifact".into(), toml::Value::String(artifact));
        }
    }
    table
}
