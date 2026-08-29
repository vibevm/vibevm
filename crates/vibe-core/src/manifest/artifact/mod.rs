//! `[[artifacts.build]]` / `[[artifacts.package]]` — desired artifact
//! producers, in the amended A1 spelling (freeze of 2026-08-29, amended at
//! A1 acceptance the same day).
//!
//! The manifest declares *desired* targets; the run carries actual artifact
//! records (a later atom's wire shapes). Inputs keep the LANDED tagged
//! one-of shape — exactly `{ path = "…" }` or `{ artifact = "…" }`, in both
//! families — so path-versus-id is never guessed from text, a package may
//! carry raw files beside consumed artifacts, and build→build chaining stays
//! expressible under the incumbent phase-forward law (package may consume
//! build outputs; build never consumes package). This cell is pure grammar
//! and validation — it executes nothing and writes no artifact state.
//!
//! Ids (targets, outputs, artifact refs) obey the mechanism plane's ONE
//! grammar, the portable token; `kind` is the closed lowercase vocabulary
//! (superseding the landed open-kind decision); an exact `provider` pin
//! stays on every mechanism target.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

mod error;
mod wire;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
#[path = "tests_validation.rs"]
mod tests_validation;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::declarant_path::{declarant_path, declarant_path_pattern};
use super::extension::ExtensionConfig;
use super::mechanism::{MechanismKey, MechanismRole, ProviderPin, is_portable_token};
use super::plane::{assert_acyclic, bounded_value};

pub use error::ArtifactsError;
pub(crate) use wire::ArtifactsWire;

/// The closed produced-artifact vocabulary. Growing it is a spec amendment,
/// not a serde default — the §4 registry law validates records against
/// exactly this set, which is why it supersedes the landed open-kind
/// decision.
///
/// ```
/// use vibe_core::manifest::ArtifactKind;
///
/// assert_eq!(ArtifactKind::Executable.as_str(), "executable");
/// assert_eq!(ArtifactKind::AgentPlugin.as_str(), "agent-plugin");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
    Executable,
    Archive,
    File,
    Directory,
    Skill,
    #[serde(rename = "agent-plugin")]
    AgentPlugin,
}

impl ArtifactKind {
    /// The exact lowercase wire spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::Archive => "archive",
            Self::File => "file",
            Self::Directory => "directory",
            Self::Skill => "skill",
            Self::AgentPlugin => "agent-plugin",
        }
    }
}

impl fmt::Display for ArtifactKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One producer input — a strict tagged-one-of inline row: exactly
/// `{ path = "…" }` or `{ artifact = "…" }`. Path-versus-id is never guessed
/// from text.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::manifest::ArtifactInput;
///
/// let source = ArtifactInput::Path { path: PathBuf::from("Cargo.toml") };
/// let consumed = ArtifactInput::Artifact { artifact: "helper.exe".into() };
/// assert_ne!(source, consumed);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactInput {
    Path { path: PathBuf },
    Artifact { artifact: String },
}

impl ArtifactInput {
    /// The referenced artifact id, when this row consumes an artifact.
    pub fn artifact_ref(&self) -> Option<&str> {
        match self {
            Self::Path { .. } => None,
            Self::Artifact { artifact } => Some(artifact),
        }
    }
}

/// One produced artifact row — `id`, closed `kind`, and an optional opaque
/// provider `select` table (the same table newtype `config` uses).
///
/// ```
/// use vibe_core::manifest::{ArtifactKind, ArtifactOutput};
///
/// let output = ArtifactOutput {
///     id: "vibe-helper.exe".into(),
///     kind: ArtifactKind::Executable,
///     select: None,
/// };
/// assert_eq!(output.kind.as_str(), "executable");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactOutput {
    pub id: String,
    pub kind: ArtifactKind,
    /// Opaque provider selection table, preserved without interpretation.
    pub select: Option<ExtensionConfig>,
}

/// One desired `[[artifacts.build]]` target — a producer of code artifacts
/// from source. `workdir` is A1's addition (default `"."`, the declarant-path
/// law); an invalid phase family is unrepresentable because the row lives in
/// the `build` array.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let manifest = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[artifacts.build]]\nid = \"vibe-helper\"\nmechanism = \"build:cargo\"\n",
///     "inputs = [{ path = \"Cargo.toml\" }, { path = \"crates/vibe-helper/**\" }]\n",
///     "outputs = [{ id = \"vibe-helper.exe\", kind = \"executable\",\n",
///     "  select = { package = \"vibe-helper\", bin = \"vibe-helper\" } }]\n",
/// )).unwrap();
/// let build = &manifest.artifacts.as_ref().unwrap().build[0];
/// assert_eq!(build.workdir, ".");
/// assert_eq!(build.outputs[0].kind.as_str(), "executable");
/// assert_eq!(
///     build.outputs[0].select.as_ref().unwrap().as_table()["bin"].as_str(),
///     Some("vibe-helper")
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactBuildTarget {
    pub id: String,
    pub mechanism: MechanismKey,
    /// Optional exact provider pin; routing resolution lands later.
    pub provider: Option<ProviderPin>,
    /// Working directory relative to the declaring manifest's root. The
    /// authored default is `"."` — the root itself, the one `.` spelling the
    /// declarant-path law would otherwise refuse as a segment.
    pub workdir: String,
    /// Authored presence is preserved: absent and `inputs = []` differ in
    /// nothing semantic today, but the distinction survives the round-trip.
    pub inputs: Option<Vec<ArtifactInput>>,
    pub outputs: Vec<ArtifactOutput>,
    pub config: Option<ExtensionConfig>,
}

impl ArtifactBuildTarget {
    /// Validate the row's own shape (no graph context).
    pub fn validate(&self) -> Result<(), ArtifactsError> {
        let family = MechanismRole::Build;
        validate_target_id(family, &self.id)?;
        validate_mechanism_family(family, &self.id, &self.mechanism)?;
        validate_inputs(family, &self.id, &self.inputs)?;
        validate_workdir(&self.id, &self.workdir)?;
        validate_outputs(family, &self.id, &self.outputs)?;
        Ok(())
    }
}

/// One desired `[[artifacts.package]]` target — a producer of portable
/// distributables from declared artifact ids (and raw files).
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let manifest = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[artifacts.build]]\nid = \"vibe-helper\"\nmechanism = \"build:cargo\"\n",
///     "inputs = [{ path = \"Cargo.toml\" }]\n",
///     "outputs = [{ id = \"vibe-helper.exe\", kind = \"executable\" }]\n",
///     "[[artifacts.package]]\nid = \"vibe-helper-windows\"\nmechanism = \"package:windows-zip\"\n",
///     "inputs = [{ artifact = \"vibe-helper.exe\" }]\n",
///     "outputs = [{ id = \"vibe-helper.zip\", kind = \"archive\" }]\n",
/// )).unwrap();
/// let package = &manifest.artifacts.as_ref().unwrap().package[0];
/// assert_eq!(package.inputs.as_ref().map(Vec::len), Some(1));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPackageTarget {
    pub id: String,
    pub mechanism: MechanismKey,
    /// Optional exact provider pin; routing resolution lands later.
    pub provider: Option<ProviderPin>,
    /// Authored presence is preserved: absent and `inputs = []` differ in
    /// nothing semantic today, but the distinction survives the round-trip.
    pub inputs: Option<Vec<ArtifactInput>>,
    pub outputs: Vec<ArtifactOutput>,
    pub config: Option<ExtensionConfig>,
}

impl ArtifactPackageTarget {
    /// Validate the row's own shape (no graph context).
    pub fn validate(&self) -> Result<(), ArtifactsError> {
        let family = MechanismRole::Package;
        validate_target_id(family, &self.id)?;
        validate_mechanism_family(family, &self.id, &self.mechanism)?;
        validate_inputs(family, &self.id, &self.inputs)?;
        validate_outputs(family, &self.id, &self.outputs)?;
        Ok(())
    }
}

/// The `[artifacts]` section — desired build and package producers.
///
/// ```
/// use vibe_core::manifest::ArtifactsSection;
///
/// assert!(ArtifactsSection::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactsSection {
    pub build: Vec<ArtifactBuildTarget>,
    pub package: Vec<ArtifactPackageTarget>,
}

impl ArtifactsSection {
    /// Whether the section can be omitted entirely.
    pub fn is_empty(&self) -> bool {
        self.build.is_empty() && self.package.is_empty()
    }

    /// Every produced artifact id, in declaration order.
    pub fn output_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        for target in &self.build {
            ids.extend(target.outputs.iter().map(|output| output.id.clone()));
        }
        for target in &self.package {
            ids.extend(target.outputs.iter().map(|output| output.id.clone()));
        }
        ids
    }

    /// Validate rows, global identity uniqueness, input resolution,
    /// acyclicity and the phase-forward edge law. Returns the artifact-id
    /// producer index the deploy section validates its references against.
    pub fn validate(&self) -> Result<BTreeMap<String, MechanismRole>, ArtifactsError> {
        let mut target_ids: BTreeSet<&str> = BTreeSet::new();
        let mut artifact_ids: BTreeSet<&str> = BTreeSet::new();
        let mut producers: BTreeMap<String, (MechanismRole, String)> = BTreeMap::new();
        for target in &self.build {
            target.validate()?;
            remember_target(
                MechanismRole::Build,
                &target.id,
                &mut target_ids,
                &artifact_ids,
            )?;
            remember_outputs(
                MechanismRole::Build,
                &target.id,
                &target.outputs,
                &target_ids,
                &mut artifact_ids,
                &mut producers,
            )?;
        }
        for target in &self.package {
            target.validate()?;
            remember_target(
                MechanismRole::Package,
                &target.id,
                &mut target_ids,
                &artifact_ids,
            )?;
            remember_outputs(
                MechanismRole::Package,
                &target.id,
                &target.outputs,
                &target_ids,
                &mut artifact_ids,
                &mut producers,
            )?;
        }

        // Resolve every `{ artifact }` input against the complete producer
        // set (a build row may appear after its consumer in the file), then
        // demand the consumer -> producer graph be a DAG — naming the cycle —
        // and only then judge edge direction: a build→package→build loop is
        // reported as the cycle it is, while a lone backward edge answers to
        // the phase-forward law.
        let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut references: Vec<(MechanismRole, String, String, MechanismRole)> = Vec::new();
        let rows: Vec<(MechanismRole, &str, &Option<Vec<ArtifactInput>>)> = self
            .build
            .iter()
            .map(|target| (MechanismRole::Build, target.id.as_str(), &target.inputs))
            .chain(
                self.package
                    .iter()
                    .map(|target| (MechanismRole::Package, target.id.as_str(), &target.inputs)),
            )
            .collect();
        for (family, target_id, inputs) in rows {
            let Some(inputs) = inputs else {
                continue;
            };
            for input in inputs {
                let Some(referenced) = input.artifact_ref() else {
                    continue;
                };
                let Some((producer_family, producer_id)) = producers.get(referenced) else {
                    return Err(ArtifactsError::UnknownInputArtifact {
                        family,
                        target: target_id.to_string(),
                        input: bounded_value(referenced),
                    });
                };
                references.push((
                    family,
                    target_id.to_string(),
                    referenced.to_string(),
                    *producer_family,
                ));
                edges
                    .entry(target_id)
                    .or_default()
                    .push(producer_id.as_str());
            }
        }
        assert_acyclic(&edges).map_err(|cycle| ArtifactsError::Cycle {
            cycle: cycle.join(" -> "),
        })?;
        for (family, target, input, producer_family) in references {
            if producer_family != MechanismRole::Build && producer_family != family {
                return Err(ArtifactsError::PhaseBackwardEdge {
                    family,
                    target,
                    input: bounded_value(&input),
                    producer_family,
                });
            }
        }
        Ok(producers
            .into_iter()
            .map(|(id, (role, _))| (id, role))
            .collect())
    }
}

/// A target id claims a globally unique slot and may not shadow a declared
/// output id.
fn remember_target<'id>(
    family: MechanismRole,
    id: &'id str,
    target_ids: &mut BTreeSet<&'id str>,
    artifact_ids: &BTreeSet<&'id str>,
) -> Result<(), ArtifactsError> {
    if !target_ids.insert(id) {
        return Err(ArtifactsError::DuplicateTargetId {
            family,
            value: bounded_value(id),
        });
    }
    if artifact_ids.contains(id) {
        return Err(ArtifactsError::DuplicateOutputId {
            value: bounded_value(id),
            detail: format!("[[artifacts.{family}]] target id collides with a declared output id"),
        });
    }
    Ok(())
}

/// Output ids are globally unique across both families and never collide
/// with a target id.
fn remember_outputs<'id>(
    family: MechanismRole,
    target: &'id str,
    outputs: &'id [ArtifactOutput],
    target_ids: &BTreeSet<&'id str>,
    artifact_ids: &mut BTreeSet<&'id str>,
    producers: &mut BTreeMap<String, (MechanismRole, String)>,
) -> Result<(), ArtifactsError> {
    for output in outputs {
        if !artifact_ids.insert(output.id.as_str()) || target_ids.contains(output.id.as_str()) {
            return Err(ArtifactsError::DuplicateOutputId {
                value: bounded_value(&output.id),
                detail: format!("output of [[artifacts.{family}]] `{target}`"),
            });
        }
        producers.insert(output.id.clone(), (family, target.to_string()));
    }
    Ok(())
}

fn validate_target_id(family: MechanismRole, id: &str) -> Result<(), ArtifactsError> {
    if !is_portable_token(id) {
        return Err(ArtifactsError::TargetIdNotPortable {
            family,
            value: bounded_value(id),
        });
    }
    Ok(())
}

fn validate_mechanism_family(
    family: MechanismRole,
    id: &str,
    mechanism: &MechanismKey,
) -> Result<(), ArtifactsError> {
    if mechanism.role() != family {
        return Err(ArtifactsError::MechanismFamily {
            family,
            target: id.to_string(),
            key: mechanism.to_string(),
            actual: mechanism.role(),
        });
    }
    Ok(())
}

/// Tagged input rows: `path` answers the one glob-bearing declarant-path law,
/// `artifact` answers the portable-token law so an unspellable ref refuses on
/// shape instead of arriving as a misleading "unknown artifact".
fn validate_inputs(
    family: MechanismRole,
    id: &str,
    inputs: &Option<Vec<ArtifactInput>>,
) -> Result<(), ArtifactsError> {
    let Some(inputs) = inputs else {
        return Ok(());
    };
    for input in inputs {
        match input {
            ArtifactInput::Path { path } => {
                if let Err(fault) = declarant_path_pattern(path) {
                    return Err(ArtifactsError::InputPatternFault {
                        family,
                        target: id.to_string(),
                        value: bounded_value(&path.display().to_string()),
                        reason: fault.reason(),
                    });
                }
            }
            ArtifactInput::Artifact { artifact } => {
                if !is_portable_token(artifact) {
                    return Err(ArtifactsError::InputIdNotPortable {
                        family,
                        target: id.to_string(),
                        value: bounded_value(artifact),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_workdir(target: &str, workdir: &str) -> Result<(), ArtifactsError> {
    // `.` is the authored default and names the declaring root itself; every
    // other spelling answers to the full literal declarant-path law.
    if workdir == "." {
        return Ok(());
    }
    if let Err(fault) = declarant_path(Path::new(workdir)) {
        return Err(ArtifactsError::WorkdirFault {
            target: target.to_string(),
            value: bounded_value(workdir),
            reason: fault.reason(),
        });
    }
    Ok(())
}

fn validate_outputs(
    family: MechanismRole,
    target: &str,
    outputs: &[ArtifactOutput],
) -> Result<(), ArtifactsError> {
    if outputs.is_empty() {
        return Err(ArtifactsError::EmptyOutputs {
            family,
            target: target.to_string(),
        });
    }
    for output in outputs {
        if !is_portable_token(&output.id) {
            return Err(ArtifactsError::OutputIdNotPortable {
                family,
                target: target.to_string(),
                value: bounded_value(&output.id),
            });
        }
    }
    Ok(())
}
