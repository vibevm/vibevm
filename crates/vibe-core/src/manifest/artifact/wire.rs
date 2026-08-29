//! Strict serde intermediates for the authored `[artifacts]` section, in the
//! amended A1 spelling: tagged one-of inputs (`{ path = "…" }` |
//! `{ artifact = "…" }`) in both families, an optional exact `provider` pin
//! on every target, a closed lowercase `kind`, an optional opaque `select`
//! table, and `workdir` defaulting to `"."` on build targets.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use serde::{Deserialize, Serialize};

use super::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget,
    ArtifactsSection,
};
use crate::manifest::extension::ExtensionConfig;
use crate::manifest::mechanism::{MechanismKey, MechanismRole, ProviderPin};

const ARTIFACT_REGISTRY: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY";
const KEY_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";

fn default_workdir() -> String {
    ".".to_string()
}

fn is_default_workdir(workdir: &str) -> bool {
    workdir == "."
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactsWire {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    build: Vec<BuildTargetWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    package: Vec<PackageTargetWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildTargetWire {
    id: String,
    mechanism: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(
        default = "default_workdir",
        skip_serializing_if = "is_default_workdir"
    )]
    workdir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inputs: Option<Vec<toml::Table>>,
    outputs: Vec<OutputWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<toml::Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageTargetWire {
    id: String,
    mechanism: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inputs: Option<Vec<toml::Table>>,
    outputs: Vec<OutputWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<toml::Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputWire {
    id: String,
    kind: ArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    select: Option<toml::Table>,
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
        // A document that will not read back can never be written: the same
        // validator runs before the wire conversion.
        section.validate().map_err(|error| error.to_string())?;
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

fn parse_mechanism(
    family: MechanismRole,
    id: &str,
    mechanism: &str,
) -> Result<MechanismKey, String> {
    mechanism.parse::<MechanismKey>().map_err(|error| {
        format!(
            "[[artifacts.{family}]] `{id}` field `mechanism` value `{mechanism}` is invalid: {error} ({KEY_GRAMMAR})"
        )
    })
}

fn parse_provider(family: MechanismRole, id: &str, value: &str) -> Result<ProviderPin, String> {
    ProviderPin::parse(value).map_err(|error| {
        format!(
            "[[artifacts.{family}]] `{id}` field `provider` value `{value}` is invalid: {error}; an exact pin is `<group>/<package>#<id>` ({KEY_GRAMMAR})"
        )
    })
}

fn inputs_from_tables(
    family: MechanismRole,
    id: &str,
    rows: Vec<toml::Table>,
) -> Result<Vec<ArtifactInput>, String> {
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| input_from_table(family, id, index, row))
        .collect()
}

impl TryFrom<BuildTargetWire> for ArtifactBuildTarget {
    type Error = String;

    fn try_from(wire: BuildTargetWire) -> Result<Self, Self::Error> {
        let inputs = wire
            .inputs
            .map(|rows| inputs_from_tables(MechanismRole::Build, &wire.id, rows))
            .transpose()?;
        let provider = wire
            .provider
            .map(|value| parse_provider(MechanismRole::Build, &wire.id, &value))
            .transpose()?;
        Ok(Self {
            mechanism: parse_mechanism(MechanismRole::Build, &wire.id, &wire.mechanism)?,
            provider,
            id: wire.id,
            workdir: wire.workdir,
            inputs,
            outputs: wire
                .outputs
                .into_iter()
                .map(|output| ArtifactOutput {
                    id: output.id,
                    kind: output.kind,
                    select: output.select.map(ExtensionConfig::from_table),
                })
                .collect(),
            config: wire.config.map(ExtensionConfig::from_table),
        })
    }
}

impl TryFrom<ArtifactBuildTarget> for BuildTargetWire {
    type Error = String;

    fn try_from(target: ArtifactBuildTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            id: target.id,
            mechanism: target.mechanism.to_string(),
            provider: target.provider.map(|pin| pin.to_string()),
            workdir: target.workdir,
            inputs: target
                .inputs
                .map(|rows| rows.into_iter().map(input_to_table).collect()),
            outputs: target
                .outputs
                .into_iter()
                .map(|output| OutputWire {
                    id: output.id,
                    kind: output.kind,
                    select: output.select.map(ExtensionConfig::into_table),
                })
                .collect(),
            config: target.config.map(ExtensionConfig::into_table),
        })
    }
}

impl TryFrom<PackageTargetWire> for ArtifactPackageTarget {
    type Error = String;

    fn try_from(wire: PackageTargetWire) -> Result<Self, Self::Error> {
        let inputs = wire
            .inputs
            .map(|rows| inputs_from_tables(MechanismRole::Package, &wire.id, rows))
            .transpose()?;
        let provider = wire
            .provider
            .map(|value| parse_provider(MechanismRole::Package, &wire.id, &value))
            .transpose()?;
        Ok(Self {
            mechanism: parse_mechanism(MechanismRole::Package, &wire.id, &wire.mechanism)?,
            provider,
            id: wire.id,
            inputs,
            outputs: wire
                .outputs
                .into_iter()
                .map(|output| ArtifactOutput {
                    id: output.id,
                    kind: output.kind,
                    select: output.select.map(ExtensionConfig::from_table),
                })
                .collect(),
            config: wire.config.map(ExtensionConfig::from_table),
        })
    }
}

impl TryFrom<ArtifactPackageTarget> for PackageTargetWire {
    type Error = String;

    fn try_from(target: ArtifactPackageTarget) -> Result<Self, Self::Error> {
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
                .map(|output| OutputWire {
                    id: output.id,
                    kind: output.kind,
                    select: output.select.map(ExtensionConfig::into_table),
                })
                .collect(),
            config: target.config.map(ExtensionConfig::into_table),
        })
    }
}

/// One input row is a strict tagged-one-of inline table: exactly `path` or
/// exactly `artifact`, never both, never neither, never anything else.
fn input_from_table(
    family: MechanismRole,
    target_id: &str,
    index: usize,
    row: toml::Table,
) -> Result<ArtifactInput, String> {
    let table = format!("[[artifacts.{family}]]");
    let refuse = |reason: String| {
        Err(format!(
            "{table} `{target_id}` field `inputs` row {index} {reason} ({ARTIFACT_REGISTRY})"
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
                path: spelling.into(),
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
