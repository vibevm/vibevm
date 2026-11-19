//! Ephemeral planner-owned answers. Never serialized or included in error chains.
use super::*;

pub(crate) enum PreparedInstall {
    Ready(Box<Plan>),
    NeedsNode(Box<PreparedSelection>),
}

pub(crate) struct PreparedSelection {
    pub option: InstallerOptionDecl,
    pub profile: Option<String>,
    pub features: Vec<String>,
    pub parameters: Vec<ParamValue>,
    pub configuration: Option<crate::configuration::ConfigurationRecord>,
    pub secret_origins:
        BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>,
    pub secret_provided: Vec<String>,
    pub license: Option<PlanLicense>,
    pub command: String,
    source_hash: String,
    license_hash: Option<String>,
    package: PackageCoordinate,
}

impl std::fmt::Debug for PreparedSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedSelection")
            .field("package", &self.package)
            .field("option", &self.option.id)
            .field("answers", &"[protected ephemeral selection]")
            .finish_non_exhaustive()
    }
}

impl PreparedSelection {
    pub(super) fn capture(
        input: &PlanInput<'_>,
        answers: PreparedAnswers,
        command: String,
        witnessed_source: String,
    ) -> Result<Self, AppError> {
        let source_hash = source_hash(input.resolved)?;
        if source_hash != witnessed_source {
            return Err(crate::ResolveError::Manifest {
                package: input.resolved.package.key(),
                reason: "source changed while collecting prepared answers".into(),
            }
            .into());
        }
        let license_hash = license_hash(answers.license.as_ref())?;
        Ok(Self {
            option: input.option.clone(),
            package: input.resolved.package.clone(),
            profile: answers.profile,
            features: answers.features,
            parameters: answers.parameters,
            configuration: answers.configuration,
            secret_origins: answers.secret_origins,
            secret_provided: answers.secret_provided,
            license: answers.license,
            command,
            source_hash,
            license_hash,
        })
    }

    /// Consume the one ephemeral selection; host facts are rebuilt by the caller.
    pub(crate) fn continue_plan(self, input: &PlanInput<'_>) -> Result<Plan, AppError> {
        self.revalidate(input.resolved)?;
        if input.option != &self.option {
            return Err(crate::ResolveError::Manifest {
                package: input.resolved.package.key(),
                reason: "prepared installer option changed".into(),
            }
            .into());
        }
        let parameters = self
            .parameters
            .iter()
            .map(|p| (p.id.clone(), p.value.clone()))
            .collect::<Vec<_>>();
        let previous_values = self
            .parameters
            .iter()
            .filter(|p| !p.secret)
            .map(|p| (p.id.clone(), p.value.clone()))
            .collect::<BTreeMap<_, _>>();
        let previous = Previous {
            secret_origins: &self.secret_origins,
            configuration: self.configuration.as_ref(),
            features: &self.features,
            parameters: &previous_values,
            license: self.license.as_ref().map(|l| l.name.as_str()),
            ask: false,
        };
        // Preserve the resolved set explicitly, including exclusions from a profile.
        let features = FeatureRequest {
            enable: self.features.clone(),
            disable: self
                .option
                .features
                .iter()
                .filter(|f| !self.features.contains(&f.id))
                .map(|f| f.id.clone())
                .collect(),
            profile: self.profile.clone(),
        };
        let continued = PlanInput {
            option: &self.option,
            request: &features,
            parameters: &parameters,
            previous: Some(previous),
            prompts: Prompts::default(),
            attended: false,
            ..input.clone()
        };
        let plan = plan(&continued)?;
        if self
            .secret_provided
            .iter()
            .any(|id| !plan.secret_provided.contains(id))
        {
            return Err(crate::ResolveError::Manifest {
                package: input.resolved.package.key(),
                reason: "prepared secret provision changed; new input required".into(),
            }
            .into());
        }
        Ok(plan)
    }

    /// Revalidate terms and full canonical source before a single continuation.
    pub(crate) fn revalidate(&self, resolved: &ResolvedPackage) -> Result<(), AppError> {
        let same = resolved.package == self.package
            && resolved.options.iter().any(|o| o == &self.option)
            && source_hash(resolved)? == self.source_hash
            && license_hash(self.license.as_ref())? == self.license_hash;
        if !same {
            return Err(crate::ResolveError::Manifest {
                package: resolved.package.key(),
                reason:
                    "prepared runtime remediation input or license changed; new consent required"
                        .into(),
            }
            .into());
        }
        Ok(())
    }
}

pub(super) struct PreparedAnswers {
    pub profile: Option<String>,
    pub features: Vec<String>,
    pub parameters: Vec<ParamValue>,
    pub configuration: Option<crate::configuration::ConfigurationRecord>,
    pub secret_origins:
        BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>,
    pub secret_provided: Vec<String>,
    pub license: Option<PlanLicense>,
}
pub(super) fn source_hash(resolved: &ResolvedPackage) -> Result<String, AppError> {
    vibe_registry::compute_content_hash(&resolved.root).map_err(|_| {
        crate::ResolveError::Manifest {
            package: resolved.package.key(),
            reason: "cannot witness prepared package source".into(),
        }
        .into()
    })
}
fn license_hash(license: Option<&PlanLicense>) -> Result<Option<String>, AppError> {
    use sha2::{Digest, Sha256};
    license
        .map(|l| {
            std::fs::read(&l.file)
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
                .map_err(|_| {
                    crate::StepError::LicenseUnreadable {
                        key: "prepared selection".into(),
                        file: l.file.clone(),
                        reason: "cannot witness license terms".into(),
                    }
                    .into()
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests;
