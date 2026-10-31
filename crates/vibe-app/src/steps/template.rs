//! Values of the step template grammar (`PROP-065#templates`) for one run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#templates");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{StepVar, expand_step_template};

use crate::error::{AppError, StepError};

use super::params::ParamValue;

/// What `$work`, `$sources`, `$out`, `$package`, `$vibe_home`, `$app_root`,
/// `$feature:ID` and `$param:ID` stand for.
pub(crate) struct Vars<'a> {
    pub work: PathBuf,
    pub package: &'a Path,
    pub vibe_home: &'a Path,
    /// Known only to `finish` steps.
    pub app_root: Option<&'a Path>,
    pub features: &'a [String],
    pub params: &'a [ParamValue],
}

impl<'a> Vars<'a> {
    pub fn planning(
        work: &Path,
        package: &'a Path,
        vibe_home: &'a Path,
        features: &'a [String],
        params: &'a [ParamValue],
    ) -> Self {
        Self {
            work: work.to_path_buf(),
            package,
            vibe_home,
            app_root: None,
            features,
            params,
        }
    }
}

/// Expand one template field of a step.
pub(crate) fn expand(
    template: &str,
    vars: &Vars<'_>,
    step: &str,
    field: &'static str,
) -> Result<String, AppError> {
    let path = |path: PathBuf| Ok(path.to_string_lossy().into_owned());
    expand_step_template(template, &mut |var| match var {
        StepVar::Named("work") => path(vars.work.clone()),
        StepVar::Named("sources") => path(vars.work.join("sources")),
        StepVar::Named("out") => path(vars.work.join("out")),
        StepVar::Named("package") => path(vars.package.to_path_buf()),
        StepVar::Named("vibe_home") => path(vars.vibe_home.to_path_buf()),
        StepVar::Named("app_root") => match vars.app_root {
            Some(root) => path(root.to_path_buf()),
            None => Err("`$app_root` is known only to finish steps".into()),
        },
        StepVar::Feature(id) => Ok(vars.features.iter().any(|f| f == id).to_string()),
        StepVar::Param(id) => vars
            .params
            .iter()
            .find(|param| param.id == id)
            .map(|param| param.value.clone())
            .ok_or_else(|| format!("parameter `{id}` has no value")),
        StepVar::Named(other) => Err(format!("unknown template variable `${other}`")),
    })
    .map_err(|reason| {
        StepError::Template {
            step: step.into(),
            field,
            reason,
        }
        .into()
    })
}
