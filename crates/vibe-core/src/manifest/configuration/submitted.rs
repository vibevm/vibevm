//! Submitted root originals and project-scoped drift binding; no input authority.
use super::{
    ConfigurationDecl, ConfigurationIntentDecl, PackageConfiguration, PackageConfigurationError,
};
use crate::{PackageKind, PackageRef, manifest::Requires};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Durable original, attributed to an existing root, not authored input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmittedInput {
    pub original: ConfigurationIntentDecl,
    pub requirement: PackageRef,
    pub project_witness: String,
    pub expected_source: SubmittedSource,
}

/// Original preview expectation, not the subsequently selected lock row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmittedSource {
    pub registry: String,
    pub content_hash: crate::ContentHash,
}

/// Bind supplied canonical root bytes and exact manifest bytes. This performs
/// no canonicalization/IO and grants no consent or source authentication.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#ORIGINAL-INTENT")]
pub fn submitted_project_witness(root: &Path, manifest: &[u8]) -> Result<String, String> {
    if !root.is_absolute() {
        return Err("submitted project root must be canonical and absolute".into());
    }
    let mut hash = Sha256::new();
    let mut part = |bytes: &[u8]| {
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    };
    part(b"vibe-ordinary-submitted-project/1");
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        part(b"unix-native-bytes");
        part(root.as_os_str().as_bytes());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        part(b"windows-native-utf16le");
        let bytes = root
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        part(&bytes);
    }
    #[cfg(not(any(unix, windows)))]
    {
        return Err("submitted project native path encoding unsupported".into());
    }
    part(manifest);
    Ok(format!(
        "ordinary-submitted-project/1:sha256:{:x}",
        hash.finalize()
    ))
}

impl PackageConfiguration {
    /// Re-evaluate submitted history without relabeling it authored or reading
    /// a project. The actual host verifies witness/root/source separately.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
    pub(crate) fn replay_parts(
        &self,
        package: String,
        kind: PackageKind,
    ) -> Result<Self, PackageConfigurationError> {
        let mut effective = self.consumers.clone();
        if !self.submitted_inputs.is_empty() {
            let bad = |reason: &str| PackageConfigurationError::Invalid {
                package: package.clone(),
                reason: reason.into(),
            };
            if kind != PackageKind::Flow
                || self.submitted_inputs.len() != 1
                || !self.consumers.is_empty()
                || !self.forwarded_inputs.is_empty()
            {
                return Err(bad(
                    "submitted history requires one standalone root and no authored or forwarded input",
                ));
            }
            let Some((owner, input)) = self.submitted_inputs.first_key_value() else {
                return Err(bad("submitted history carries no root input"));
            };
            if owner != "./vibe.toml"
                || input.requirement.qualified_name() != package
                || !input.requirement.version.is_exact_pin()
            {
                return Err(bad(
                    "submitted history owner or exact root requirement disagrees",
                ));
            }
            let digest = input
                .project_witness
                .strip_prefix("ordinary-submitted-project/1:sha256:")
                .ok_or_else(|| bad("invalid submitted project witness"))?;
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err(bad("invalid submitted project witness"));
            }
            let integer_profile = integer_profile_shape(&self.definition);
            if !integer_profile {
                submitted_shape(&self.definition, &self.declarations).map_err(&bad)?;
            }
            let original = &input.original;
            if (original.profile.is_some() && !integer_profile)
                || (integer_profile
                    && original.profile.as_deref() != Some(self.definition.profiles[0].id.as_str()))
                || original.default_features
                || !original.features.is_empty()
                || original.policy != crate::configuration::ChoicePolicy::RequireSelection
            {
                return Err(bad(
                    "submitted original is outside explicit preview answer scope",
                ));
            }
            effective.insert(owner.clone(), original.clone());
        }
        let mut result = Self::evaluate_parts(
            package,
            kind,
            &self.definition,
            &self.declarations,
            &effective,
            &self.forwarded_inputs,
        )?;
        result.consumers = self.consumers.clone();
        result.submitted_inputs = self.submitted_inputs.clone();
        Ok(result)
    }
}

fn submitted_shape(
    definition: &ConfigurationDecl,
    requires: &Requires,
) -> Result<(), &'static str> {
    use super::{ConfigurationEffect, ConfigurationFeatureVisibility, ConfigurationGroupDecl};
    if definition.groups.len() != 1
        || definition.parameters.len() != 1
        || !definition.profiles.is_empty()
    {
        return Err("submitted definition is outside preview scope");
    }
    let ConfigurationGroupDecl::Choice {
        cardinality: crate::configuration::Cardinality::ExactlyOne,
        members,
        defaults: None,
        guards,
        ..
    } = &definition.groups[0]
    else {
        return Err("submitted choice is outside preview scope");
    };
    let p = &definition.parameters[0];
    if !guards.is_empty()
        || p.kind != crate::manifest::ParameterType::Bool
        || !p.required
        || p.secret
        || p.default.is_some()
        || definition.features.len() != members.len()
        || !requires.configurations.is_empty()
        || definition.features.iter().any(|f| {
            f.visibility != ConfigurationFeatureVisibility::Public
                || f.parent.is_some()
                || f.required
                || f.default
                || !f.requires.is_empty()
                || !f.guards.is_empty()
                || !members.contains(&f.id)
                || f.effects.iter().any(|e| {
                    matches!(
                        e,
                        ConfigurationEffect::Forward { .. } | ConfigurationEffect::Subskill { .. }
                    )
                })
        })
    {
        return Err("submitted definition is outside preview scope");
    }
    Ok(())
}

/// Exact public integer/preset vertical, not permission for arbitrary profiles.
pub fn integer_profile_shape(definition: &ConfigurationDecl) -> bool {
    use super::{ConfigurationFeatureVisibility, ConfigurationGroupDecl};
    if definition.parameters.len() != 1
        || definition.profiles.len() != 1
        || definition.groups.len() != 1
    {
        return false;
    }
    let p = &definition.parameters[0];
    let profile = &definition.profiles[0];
    let ConfigurationGroupDecl::Choice {
        cardinality: crate::configuration::Cardinality::ExactlyOne,
        defaults: None,
        guards,
        members,
        ..
    } = &definition.groups[0]
    else {
        return false;
    };
    p.kind == crate::manifest::ParameterType::Integer
        && p.required
        && !p.secret
        && p.default.is_none()
        && p.min.is_some()
        && p.max.is_some()
        && profile.parameters.len() == 1
        && profile.parameters.contains_key(&p.id)
        && profile.features.is_empty()
        && profile.selections.is_empty()
        && guards.is_empty()
        && definition.features.len() == members.len()
        && definition.features.iter().all(|f| {
            f.visibility == ConfigurationFeatureVisibility::Public
                && f.parent.is_none()
                && !f.required
                && !f.default
                && f.requires.is_empty()
                && f.guards.is_empty()
                && members.contains(&f.id)
                && f.effects.iter().all(|e| {
                    !matches!(
                        e,
                        super::ConfigurationEffect::Forward { .. }
                            | super::ConfigurationEffect::Subskill { .. }
                    )
                })
        })
}
