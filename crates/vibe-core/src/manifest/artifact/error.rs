//! The typed refusals of the `[artifacts]` section.
//!
//! Every refusal names its table, its field and the bounded offending value,
//! so a reader can go straight to the authored line without the message
//! echoing an attacker-sized string back at them. The `spec://` citation is
//! spelled LITERALLY in every template — the conform gate reads the
//! `#[error]` text itself and does not follow a `const` interpolation (the
//! lesson the durable-world adapter's error cell paid for first).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use specmark::spec;

use crate::manifest::mechanism::MechanismRole;

/// Why an `[artifacts]` section refuses. Values are bounded
/// ([`bounded_value`](super::plane::bounded_value)) at the point of
/// construction, so no variant can echo a giant authored string.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let error = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[artifacts.build]]\nid = \"Bad Id\"\nmechanism = \"build:cargo\"\n",
///     "outputs = [{ id = \"x.exe\", kind = \"executable\" }]\n",
/// ))
/// .unwrap_err();
/// let message = error.to_string();
/// assert!(message.contains("is not a portable token"));
/// assert!(message.contains("spec://"), "the refusal cites its law");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub enum ArtifactsError {
    #[error(
        "[[artifacts.{family}]] field `id` value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: respell `id` in the portable-token grammar)"
    )]
    TargetIdNotPortable {
        family: MechanismRole,
        value: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `outputs` id value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: respell the output `id` in the portable-token grammar)"
    )]
    OutputIdNotPortable {
        family: MechanismRole,
        target: String,
        value: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` artifact value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`); an artifact ref names a declared output id \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: reference an output id spelled in the portable-token grammar)"
    )]
    InputIdNotPortable {
        family: MechanismRole,
        target: String,
        value: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `mechanism` value `{key}` has role `{actual}`; the mechanism key's role must equal the target's phase family \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: use a `{family}:` mechanism key or move the target to its own family's array)"
    )]
    MechanismFamily {
        family: MechanismRole,
        target: String,
        key: String,
        actual: MechanismRole,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `outputs` is empty; a desired target must declare at least one produced artifact id \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: declare the produced artifact ids in `outputs`)"
    )]
    EmptyOutputs {
        family: MechanismRole,
        target: String,
    },

    #[error(
        "[[artifacts.build]] `{target}` field `workdir` value {value} must be `.` or a nonempty declarant-root-relative forward-slashed path: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: respell `workdir` relative to the declaring root, forward-slashed)"
    )]
    WorkdirFault {
        target: String,
        value: String,
        reason: &'static str,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` value {value} must be a nonempty declarant-root-relative glob pattern with forward slashes: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: respell the `path` input as a declarant-root-relative glob)"
    )]
    InputPatternFault {
        family: MechanismRole,
        target: String,
        value: String,
        reason: &'static str,
    },

    #[error(
        "duplicate [[artifacts.{family}]] field `id` value {value}; artifact target ids and output artifact ids are globally unique in the document \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: rename one of the colliding ids)"
    )]
    DuplicateTargetId {
        family: MechanismRole,
        value: String,
    },

    #[error(
        "duplicate artifact id {value} ({detail}); artifact target ids and output artifact ids are globally unique in the document \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: rename one of the colliding ids)"
    )]
    DuplicateOutputId { value: String, detail: String },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` references unknown artifact {input}; artifact refs name a declared output id \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: declare the referenced output or correct the reference)"
    )]
    UnknownInputArtifact {
        family: MechanismRole,
        target: String,
        input: String,
    },

    #[error(
        "[[artifacts.{family}]] `{target}` field `inputs` references artifact {input} produced by phase `{producer_family}`; edges are phase-forward — package may consume build, build cannot consume package or deploy \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: produce the input in an earlier phase or consume it from a later target)"
    )]
    PhaseBackwardEdge {
        family: MechanismRole,
        target: String,
        input: String,
        producer_family: MechanismRole,
    },

    #[error(
        "artifact target graph is cyclic: {cycle} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; \
         fix: break the cycle — artifact inputs form a DAG)"
    )]
    Cycle { cycle: String },
}
