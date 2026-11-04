//! Validate actual plan identity before constructing a recipe/shell environment.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-ENV-IDENTITY");

use crate::error::{AppError, ResolveError};
use crate::steps::{Plan, step_env_name};
use vibe_core::manifest::StepEnvironment;

pub(super) fn prepare(plan: &Plan) -> Result<Vec<(String, String)>, AppError> {
    if !plan
        .steps
        .iter()
        .any(|s| s.step.kind.receives_step_environment())
    {
        return Ok(Vec::new());
    }
    let refuse = |error| ResolveError::Manifest {
        package: plan.package.key(),
        reason: format!(
            "plan environment projection for installer option `{}`: {error}",
            plan.option
        ),
    };
    StepEnvironment::Feature
        .validate(plan.declared.iter().map(|f| f.id.as_str()))
        .map_err(refuse)?;
    StepEnvironment::Parameter
        .validate(plan.parameters.iter().map(|p| p.id.as_str()))
        .map_err(refuse)?;
    Ok(plan
        .declared
        .iter()
        .map(|feature| {
            (
                step_env_name("VIBE_FEATURE_", &feature.id),
                if plan.features.contains(&feature.id) {
                    "1"
                } else {
                    "0"
                }
                .to_owned(),
            )
        })
        .chain(
            plan.parameters
                .iter()
                .map(|param| (step_env_name("VIBE_PARAM_", &param.id), param.value.clone())),
        )
        .collect())
}
