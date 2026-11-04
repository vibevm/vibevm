//! Values of the step template grammar (`PROP-065#templates`) for one run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#templates");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{StepPhase, StepVar, expand_step_template};

use crate::error::{AppError, StepError};

use super::params::ParamValue;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_secret_templates_refuse_without_echoing_values() {
        let params = [ParamValue {
            id: "token".into(),
            value: "SYN_TEMPLATE_CANARY".into(),
            secret: true,
        }];
        let vars = Vars::planning(
            Path::new("/work"),
            Path::new("/pkg"),
            Path::new("/home"),
            &[],
            &params,
        );
        let error = expand("$param:token", &vars, "test", "prompt").unwrap_err();
        assert!(error.to_string().contains("secret"));
        assert!(!error.to_string().contains("SYN_TEMPLATE_CANARY"));
    }

    #[test]
    fn escaped_dollars_and_returned_public_values_are_not_reparsed() {
        let params = [ParamValue {
            id: "public".into(),
            value: "$param:token".into(),
            secret: false,
        }];
        let vars = Vars::planning(
            Path::new("/work"),
            Path::new("/pkg"),
            Path::new("/home"),
            &[],
            &params,
        );
        assert_eq!(
            expand("$$param:token $param:public", &vars, "test", "prompt").unwrap(),
            "$param:token $param:token"
        );
    }
}

/// What `$work`, `$sources`, `$out`, `$package`, `$vibe_home`, `$app_root`,
/// `$app_bin`,
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
    expand_mode(template, vars, step, field, Expansion::Run)
}

/// Only finish-phase root paths may remain unavailable in a preview.
pub(crate) fn preview(
    template: &str,
    vars: &Vars<'_>,
    step: &str,
    phase: StepPhase,
) -> Result<String, AppError> {
    let mode = if phase == StepPhase::Finish {
        Expansion::FinishPreview
    } else {
        Expansion::Run
    };
    expand_mode(template, vars, step, "prompt", mode)
}

#[derive(Clone, Copy)]
enum Expansion {
    Run,
    FinishPreview,
}

fn expand_mode(
    template: &str,
    vars: &Vars<'_>,
    step: &str,
    field: &'static str,
    mode: Expansion,
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
            None if matches!(mode, Expansion::FinishPreview) => Ok("$app_root".into()),
            None => Err("`$app_root` is known only to finish steps".into()),
        },
        StepVar::Named("app_bin") => match vars.app_root {
            Some(root) => path(root.join("bin")),
            None if matches!(mode, Expansion::FinishPreview) => Ok("$app_bin".into()),
            None => Err("`$app_bin` is known only to finish steps".into()),
        },
        StepVar::Feature(id) => Ok(vars.features.iter().any(|f| f == id).to_string()),
        StepVar::Param(id) => vars
            .params
            .iter()
            .find(|param| param.id == id)
            .ok_or_else(|| format!("parameter `{id}` has no value"))
            .and_then(|param| if param.secret {
                Err(format!("parameter `{id}` is secret and cannot be interpolated into a framework template"))
            } else { Ok(param.value.clone()) }),
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
