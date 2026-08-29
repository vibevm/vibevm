//! The typed refusals of the `[artifacts]` section.
//!
//! Every refusal names its table, its field and the bounded offending value,
//! so a reader can go straight to the authored line without the message
//! echoing an attacker-sized string back at them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use crate::manifest::mechanism::MechanismRole;

const ARTIFACT_REGISTRY: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY";

/// Why an `[artifacts]` section refuses. Values are bounded
/// ([`bounded_value`](super::plane::bounded_value)) at the point of
/// construction, so no variant can echo a giant authored string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactsError {
    #[error(
        "[[artifacts.{family}]] field `id` value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) ({ARTIFACT_REGISTRY})"
    )]
    TargetIdNotPortable {
        family: MechanismRole,
        value: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `outputs` id value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) ({ARTIFACT_REGISTRY})"
    )]
    OutputIdNotPortable {
        family: MechanismRole,
        target: String,
        value: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` artifact value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`); an artifact ref names a declared output id ({ARTIFACT_REGISTRY})"
    )]
    InputIdNotPortable {
        family: MechanismRole,
        target: String,
        value: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `mechanism` value `{key}` has role `{actual}`; the mechanism key's role must equal the target's phase family ({ARTIFACT_REGISTRY})"
    )]
    MechanismFamily {
        family: MechanismRole,
        target: String,
        key: String,
        actual: MechanismRole,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `outputs` is empty; a desired target must declare at least one produced artifact id ({ARTIFACT_REGISTRY})"
    )]
    EmptyOutputs {
        family: MechanismRole,
        target: String,
    },

    #[error(
        "[[artifacts.build]] `{target}` field `workdir` value {value} must be `.` or a nonempty declarant-root-relative forward-slashed path: {reason} ({ARTIFACT_REGISTRY})"
    )]
    WorkdirFault {
        target: String,
        value: String,
        reason: &'static str,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` value {value} must be a nonempty declarant-root-relative glob pattern with forward slashes: {reason} ({ARTIFACT_REGISTRY})"
    )]
    InputPatternFault {
        family: MechanismRole,
        target: String,
        value: String,
        reason: &'static str,
    },

    #[error(
        "duplicate [[artifacts.{family}]] field `id` value {value}; artifact target ids and output artifact ids are globally unique in the document ({ARTIFACT_REGISTRY})"
    )]
    DuplicateTargetId {
        family: MechanismRole,
        value: String,
    },

    #[error(
        "duplicate artifact id {value} ({detail}); artifact target ids and output artifact ids are globally unique in the document ({ARTIFACT_REGISTRY})"
    )]
    DuplicateOutputId { value: String, detail: String },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` references unknown artifact {input}; artifact refs name a declared output id ({ARTIFACT_REGISTRY})"
    )]
    UnknownInputArtifact {
        family: MechanismRole,
        target: String,
        input: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` references artifact {input} produced by phase `{producer_family}`; edges are phase-forward — package may consume build, build cannot consume package or deploy ({ARTIFACT_REGISTRY})"
    )]
    PhaseBackwardEdge {
        family: MechanismRole,
        target: String,
        input: String,
        producer_family: MechanismRole,
    },

    #[error(
        "artifact target graph is cyclic: {cycle} (violates {ARTIFACT_REGISTRY}; fix: break the cycle — artifact inputs form a DAG)"
    )]
    Cycle { cycle: String },
}
