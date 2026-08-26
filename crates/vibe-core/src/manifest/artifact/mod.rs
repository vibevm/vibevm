//! `[[artifacts.build]]` / `[[artifacts.package]]` — desired artifact
//! producers.
//!
//! The manifest declares *desired* targets; the run carries actual artifact
//! records. Build and package targets form a DAG over stable artifact ids
//! (output rows), phase-forward: package may consume build outputs, build
//! can never consume package or deploy. This cell is pure grammar and
//! validation — it executes nothing and writes no artifact state.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

mod wire;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::declarant_path::{declarant_path_error, declarant_path_pattern};
use super::extension::ExtensionConfig;
use super::mechanism::{MechanismKey, MechanismRole, ProviderPin, is_portable_token};
use super::plane::assert_acyclic;

pub(crate) use wire::ArtifactsWire;

const ARTIFACT_REGISTRY: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY";

/// One producer input — a strict tagged-one-of inline row: exactly
/// `{ path = "…" }` or `{ artifact = "…" }`. The draft's ambiguous bare
/// strings are resolved structurally; path-versus-id is never guessed from
/// text.
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

/// One produced artifact row — `id` plus an open portable `kind` token.
/// `kind` stays open so future ecosystems need no phase-law change; no path
/// is runtime identity.
///
/// ```
/// use vibe_core::manifest::ArtifactOutput;
///
/// let output = ArtifactOutput { id: "helper.exe".into(), kind: "executable".into() };
/// assert_eq!(output.kind, "executable");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactOutput {
    pub id: String,
    pub kind: String,
}

/// One desired build or package target. The phase family comes from the
/// array the row lives in (`artifacts.build` / `artifacts.package`), which
/// makes an invalid phase unrepresentable.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let manifest = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
///     "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
/// )).unwrap();
/// assert_eq!(manifest.artifacts.as_ref().unwrap().build[0].id, "helper");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactTarget {
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

impl ArtifactTarget {
    /// Validate the row's own shape (no graph context).
    pub fn validate(&self, role: MechanismRole) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err(format!(
                "[[artifacts.{}]] field `id` value `{}` is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) ({ARTIFACT_REGISTRY})",
                role, self.id,
            ));
        }
        if self.mechanism.role() != role {
            return Err(format!(
                "[[artifacts.{role}]] `{}` field `mechanism` value `{}` has role `{}`; the mechanism key's role must equal the target's phase family ({ARTIFACT_REGISTRY})",
                self.id,
                self.mechanism,
                self.mechanism.role(),
            ));
        }
        if self.outputs.is_empty() {
            return Err(format!(
                "[[artifacts.{role}]] `{}` field `outputs` is empty; a desired target must declare at least one produced artifact id ({ARTIFACT_REGISTRY})",
                self.id,
            ));
        }
        for output in &self.outputs {
            if !is_portable_token(&output.id) {
                return Err(format!(
                    "[[artifacts.{role}]] `{}` field `outputs` id value `{}` is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) ({ARTIFACT_REGISTRY})",
                    self.id, output.id,
                ));
            }
            if !is_portable_token(&output.kind) {
                return Err(format!(
                    "[[artifacts.{role}]] `{}` output `{}` field `kind` value `{}` is not a portable token; kinds are an open vocabulary, but a portable one ({ARTIFACT_REGISTRY})",
                    self.id, output.id, output.kind,
                ));
            }
        }
        if let Some(inputs) = &self.inputs {
            for input in inputs {
                match input {
                    // The one glob-bearing authored surface. `*`/`**` are
                    // syntax here; every literal segment still answers to the
                    // full declarant-path law — no escape, no colon, no device
                    // stem, no spelling that means something different on
                    // another host.
                    ArtifactInput::Path { path } => {
                        if let Err(fault) = declarant_path_pattern(path) {
                            return Err(declarant_path_error(
                                &format!("[[artifacts.{role}]]"),
                                &self.id,
                                "inputs",
                                path,
                                fault,
                                ARTIFACT_REGISTRY,
                            ));
                        }
                    }
                    // A ref is judged for shape here and for existence in the
                    // graph pass, so an unspellable id says so instead of
                    // arriving as a misleading "unknown artifact ``".
                    ArtifactInput::Artifact { artifact } if !is_portable_token(artifact) => {
                        return Err(format!(
                            "[[artifacts.{role}]] `{}` field `inputs` artifact value `{artifact}` is not a portable token (nonempty lowercase alphanumerics, `-`, `.`); an artifact ref names a declared output id ({ARTIFACT_REGISTRY})",
                            self.id,
                        ));
                    }
                    ArtifactInput::Artifact { .. } => {}
                }
            }
        }
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
    pub build: Vec<ArtifactTarget>,
    pub package: Vec<ArtifactTarget>,
}

impl ArtifactsSection {
    /// Whether the section can be omitted entirely.
    pub fn is_empty(&self) -> bool {
        self.build.is_empty() && self.package.is_empty()
    }

    /// Every produced artifact id, in declaration order.
    pub fn output_ids(&self) -> BTreeSet<String> {
        self.all_targets()
            .flat_map(|(_, target)| target.outputs.iter())
            .map(|output| output.id.clone())
            .collect()
    }

    /// All targets with their phase family, build first.
    pub fn all_targets(&self) -> impl Iterator<Item = (MechanismRole, &ArtifactTarget)> {
        std::iter::zip(std::iter::repeat(MechanismRole::Build), &self.build).chain(std::iter::zip(
            std::iter::repeat(MechanismRole::Package),
            &self.package,
        ))
    }

    /// Validate rows, global identity uniqueness, phase-forward edges, and
    /// DAG acyclicity. Returns the artifact-id producer index the deploy
    /// section validates its references against.
    pub fn validate(&self) -> Result<BTreeMap<String, MechanismRole>, String> {
        let mut target_ids: BTreeSet<&str> = BTreeSet::new();
        let mut artifact_ids: BTreeSet<&str> = BTreeSet::new();
        let mut producers: BTreeMap<String, (MechanismRole, String)> = BTreeMap::new();
        for (role, target) in self.all_targets() {
            target.validate(role)?;
            if !target_ids.insert(target.id.as_str()) {
                return Err(format!(
                    "duplicate [[artifacts.{role}]] field `id` value `{}`; artifact target ids and output artifact ids are globally unique in the document ({ARTIFACT_REGISTRY})",
                    target.id,
                ));
            }
            if artifact_ids.contains(target.id.as_str()) {
                return Err(format!(
                    "duplicate artifact id `{}` ([[artifacts.{role}]] target id collides with a declared output id); artifact target ids and output artifact ids are globally unique in the document ({ARTIFACT_REGISTRY})",
                    target.id,
                ));
            }
            for output in &target.outputs {
                if !artifact_ids.insert(output.id.as_str())
                    || target_ids.contains(output.id.as_str())
                {
                    return Err(format!(
                        "duplicate artifact id `{}` (output of [[artifacts.{role}]] `{}`); artifact target ids and output artifact ids are globally unique in the document ({ARTIFACT_REGISTRY})",
                        output.id, target.id,
                    ));
                }
                producers.insert(output.id.clone(), (role, target.id.clone()));
            }
        }

        // Phase-forward edge law + unknown-ref refusal, then acyclicity.
        let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (role, target) in self.all_targets() {
            let Some(inputs) = &target.inputs else {
                continue;
            };
            for input in inputs {
                let Some(referenced) = input.artifact_ref() else {
                    continue;
                };
                let Some((producer_phase, producer_id)) = producers.get(referenced) else {
                    return Err(format!(
                        "[[artifacts.{role}]] `{}` field `inputs` references unknown artifact `{}`; artifact refs name a declared output id ({ARTIFACT_REGISTRY})",
                        target.id, referenced,
                    ));
                };
                if *producer_phase != MechanismRole::Build && *producer_phase != role {
                    return Err(format!(
                        "[[artifacts.{role}]] `{}` field `inputs` references artifact `{}` produced by phase `{producer_phase}`; edges are phase-forward — package may consume build, build cannot consume package or deploy ({ARTIFACT_REGISTRY})",
                        target.id, referenced,
                    ));
                }
                edges
                    .entry(target.id.as_str())
                    .or_default()
                    .push(producer_id.as_str());
            }
        }
        assert_acyclic(
            &edges,
            "artifact target graph",
            ARTIFACT_REGISTRY,
            "break the cycle — artifact inputs form a DAG",
        )?;
        Ok(producers
            .into_iter()
            .map(|(id, (role, _))| (id, role))
            .collect())
    }
}
