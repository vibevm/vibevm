//! The typed refusals of the `[deploy]` section.
//!
//! Every refusal names its table, its field and the bounded offending value,
//! so a reader can go straight to the authored line without the message
//! echoing an attacker-sized string back at them. The `spec://` citation is
//! spelled LITERALLY in every template — the conform gate reads the
//! `#[error]` text itself and does not follow a `const` interpolation (the
//! lesson the durable-world adapter's error cell paid for first).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use specmark::spec;

/// Why a `[deploy]` section refuses. Values are bounded at the point of
/// construction, so no variant can echo a giant authored string.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let error = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[deploy.target]]\nid = \"-lead\"\nartifact = \"x.exe\"\n",
///     "mechanism = \"deploy:vibe-bin\"\n",
/// ))
/// .unwrap_err();
/// let message = error.to_string();
/// assert!(message.contains("is not a portable token"));
/// assert!(message.contains("spec://"), "the refusal cites its law");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub enum DeployError {
    #[error(
        "[[deploy.target]] field `id` value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: respell `id` in the portable-token grammar)"
    )]
    TargetIdNotPortable { value: String },

    #[error(
        "[[deploy.target]] `{target}` field `artifact` value {value} is not a portable token; `artifact` names one declared artifact output id \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: reference a declared output id in the portable-token grammar)"
    )]
    ArtifactIdNotPortable { target: String, value: String },

    #[error(
        "[[deploy.target]] `{target}` field `mechanism` value `{key}` has role `{actual}`; deploy targets select the `deploy:` family only \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; \
         fix: use a `deploy:` mechanism key)"
    )]
    MechanismFamily {
        target: String,
        key: String,
        actual: String,
    },

    #[error(
        "duplicate [[deploy.target]] field `id` value {value}; deploy target ids are unique within the manifest \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: rename one of the colliding targets)"
    )]
    DuplicateTargetId { value: String },

    #[error(
        "[[deploy.target]] `{target}` field `artifact` value {artifact} names no declared artifact output; a deploy target reconciles exactly one produced artifact \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: declare the artifact under [artifacts] or correct the reference)"
    )]
    UnknownArtifact { target: String, artifact: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` artifact value {value} is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: reference deploy target ids in the portable-token grammar)"
    )]
    DependencyIdNotPortable { target: String, value: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` lists `{dependency}` more than once \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: list each dependency once)"
    )]
    DuplicateDependency { target: String, dependency: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` lists itself; the target graph must be acyclic \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: remove the self-dependency)"
    )]
    SelfDependency { target: String },

    #[error(
        "[[deploy.target]] `{target}` field `depends_on` references unknown target `{dependency}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: declare the dependency target or correct the reference)"
    )]
    UnknownDependency { target: String, dependency: String },

    #[error(
        "deploy target graph is cyclic: {cycle} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: break the depends_on cycle)"
    )]
    DependsOnCycle { cycle: String },

    #[error(
        "[deploy.profiles] key `{name}` is not a portable token (nonempty lowercase alphanumerics, `-`, `.`) \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: respell the profile name in the portable-token grammar)"
    )]
    ProfileNameNotPortable { name: String },

    #[error(
        "[deploy.profiles.{name}] field `targets` is empty; a profile is a nonempty ordered selection \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: list the profile's deploy targets in order)"
    )]
    EmptyProfileTargets { name: String },

    #[error(
        "[deploy.profiles.{name}] field `targets` lists `{target}` more than once \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: list each target once)"
    )]
    DuplicateProfileTarget { name: String, target: String },

    #[error(
        "[deploy.profiles.{name}] field `targets` references unknown deploy target `{target}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: declare the target or correct the profile)"
    )]
    UnknownProfileTarget { name: String, target: String },

    #[error(
        "[deploy.profiles.{name}] selects target `{target}` whose dependency `{dependency}` is not included in the profile \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: add the dependency to the profile or drop the dependent target)"
    )]
    MissingDependencyInProfile {
        name: String,
        target: String,
        dependency: String,
    },

    #[error(
        "[deploy] field `default_profile` value `{name}` names no declared profile \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
         fix: declare the profile or correct `default_profile`)"
    )]
    UnknownDefaultProfile { name: String },
}
