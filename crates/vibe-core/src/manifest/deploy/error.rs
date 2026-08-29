//! The typed refusals of the `[deploy]` section.
//!
//! Every refusal names its table, its field and the bounded offending value,
//! so a reader can go straight to the authored line without the message
//! echoing an attacker-sized string back at them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

const DEPLOY_TARGETS: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS";
const KEY_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE";
const PORTABLE_TOKEN: &str = "a portable token (nonempty lowercase alphanumerics, `-`, `.`)";

/// Why a `[deploy]` section refuses. Values are bounded at the point of
/// construction, so no variant can echo a giant authored string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeployError {
    #[error(
        "[[deploy.target]] field `id` value {value} is not a portable token: expected {PORTABLE_TOKEN} ({DEPLOY_TARGETS})"
    )]
    TargetIdNotPortable { value: String },

    #[error(
        "[[deploy.target]] `{target}` field `artifact` value {value} is not a portable token; `artifact` names one declared artifact output id ({DEPLOY_TARGETS})"
    )]
    ArtifactIdNotPortable { target: String, value: String },

    #[error(
        "[[deploy.target]] `{target}` field `mechanism` value `{key}` has role `{actual}`; deploy targets select the `deploy:` family only ({KEY_GRAMMAR})"
    )]
    MechanismFamily {
        target: String,
        key: String,
        actual: String,
    },

    #[error(
        "duplicate [[deploy.target]] field `id` value {value}; deploy target ids are unique within the manifest ({DEPLOY_TARGETS})"
    )]
    DuplicateTargetId { value: String },

    #[error(
        "[[deploy.target]] `{target}` field `artifact` value {artifact} names no declared artifact output; a deploy target reconciles exactly one produced artifact ({DEPLOY_TARGETS})"
    )]
    UnknownArtifact { target: String, artifact: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` artifact value {value} is not a portable token: expected {PORTABLE_TOKEN} ({DEPLOY_TARGETS})"
    )]
    DependencyIdNotPortable { target: String, value: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` lists `{dependency}` more than once ({DEPLOY_TARGETS})"
    )]
    DuplicateDependency { target: String, dependency: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` lists itself; the target graph must be acyclic ({DEPLOY_TARGETS})"
    )]
    SelfDependency { target: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` references unknown target `{dependency}` ({DEPLOY_TARGETS})"
    )]
    UnknownDependency { target: String, dependency: String },

    #[error(
        "deploy target graph is cyclic: {cycle} (violates {DEPLOY_TARGETS}; fix: break the depends_on cycle)"
    )]
    DependsOnCycle { cycle: String },

    #[error(
        "[deploy.profiles] key `{name}` is not a portable token: expected {PORTABLE_TOKEN} ({DEPLOY_TARGETS})"
    )]
    ProfileNameNotPortable { name: String },

    #[error(
        "[deploy.profiles.{name}] field `targets` is empty; a profile is a nonempty ordered selection ({DEPLOY_TARGETS})"
    )]
    EmptyProfileTargets { name: String },

    #[error(
        "[deploy.profiles.{name}] field `targets` lists `{target}` more than once ({DEPLOY_TARGETS})"
    )]
    DuplicateProfileTarget { name: String, target: String },

    #[error(
        "[deploy.profiles.{name}] field `targets` references unknown deploy target `{target}` ({DEPLOY_TARGETS})"
    )]
    UnknownProfileTarget { name: String, target: String },

    #[error(
        "[deploy.profiles.{name}] selects target `{target}` whose dependency `{dependency}` is not included in the profile ({DEPLOY_TARGETS})"
    )]
    MissingDependencyInProfile {
        name: String,
        target: String,
        dependency: String,
    },

    #[error(
        "[deploy] field `default_profile` value `{name}` names no declared profile ({DEPLOY_TARGETS})"
    )]
    UnknownDefaultProfile { name: String },
}
