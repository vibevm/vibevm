//! Two environments of a step: the durable user environment an `env` step
//! changes (`PROP-065#env-step`), and the PATH and `VIBE_APP_USE` a step's
//! child receives from the installations it requires
//! (`PROP-065#app-dependencies`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#env-step");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies");

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{StepDecl, StepKind};

use crate::error::AppError;
use crate::error_lifecycle::LifecycleError;
use crate::exposure::Exposure;
use crate::index::{EnvEntry, EnvOp};
use crate::session::{SESSION_VARIABLE, SessionChoices};

use super::run::Run;
use super::template;

/// The exposures of the required installations: the option's for every
/// step, and each step's own.
#[derive(Debug, Clone, Default)]
pub(crate) struct StepPaths {
    pub option: Vec<Exposure>,
    pub steps: BTreeMap<String, Vec<Exposure>>,
}

impl StepPaths {
    /// `base` plus, when the step has requirements, a `PATH` that starts
    /// with their proxy and `bin` directories before `inherited`, and the
    /// `VIBE_APP_USE` choices that make the proxies pick them.
    pub fn child_env(
        &self,
        base: &[(String, String)],
        step: &str,
        inherited: Option<&OsStr>,
    ) -> Vec<(String, String)> {
        let exposures: Vec<&Exposure> = self
            .option
            .iter()
            .chain(self.steps.get(step).into_iter().flatten())
            .collect();
        let mut env = base.to_vec();
        if exposures.is_empty() {
            return env;
        }
        let mut front: Vec<PathBuf> = Vec::new();
        for exposure in &exposures {
            for dir in std::iter::once(&exposure.proxies).chain(exposure.bin.as_ref()) {
                if !front.contains(dir) {
                    front.push(dir.clone());
                }
            }
        }
        let rest: Vec<PathBuf> = inherited
            .map(|value| std::env::split_paths(value).collect())
            .unwrap_or_default();
        if let Ok(path) = std::env::join_paths(front.into_iter().chain(rest)) {
            env.push(("PATH".into(), path.to_string_lossy().into_owned()));
        }
        let mut choices = SessionChoices::default();
        for exposure in &exposures {
            if choices.apply(&exposure.choice()).is_err() {
                continue;
            }
        }
        env.push((SESSION_VARIABLE.into(), choices.render()));
        env
    }
}

impl Run<'_> {
    /// Apply one env step through the caller's user environment and return
    /// the entries it applied, values expanded (`$app_root`, `$app_bin`,
    /// `$vibe_home`).
    pub(super) fn env_step(
        &self,
        step: &StepDecl,
        app_root: &Path,
    ) -> Result<Vec<EnvEntry>, AppError> {
        let StepKind::Env {
            set,
            unset,
            path_prepend,
        } = &step.kind
        else {
            return Ok(Vec::new());
        };
        let key = self.plan.package.key();
        let vars = self.vars(Some(app_root));
        let entry = |op: EnvOp, name: &str, value: Option<String>| EnvEntry {
            step: step.id.clone(),
            op,
            name: name.to_owned(),
            value,
            previous: None,
            previous_kind: None,
        };
        let mut entries = Vec::new();
        for (name, value) in set {
            let value = template::expand(value, &vars, &step.id, "set")?;
            entries.push(entry(EnvOp::Set, name, Some(value)));
        }
        for name in unset {
            entries.push(entry(EnvOp::Unset, name, None));
        }
        for value in path_prepend {
            let value = template::expand(value, &vars, &step.id, "path_prepend")?;
            entries.push(entry(EnvOp::PathPrepend, "PATH", Some(value)));
        }
        let environment = self
            .ctx
            .environment
            .ok_or_else(|| LifecycleError::NoEnvironment {
                key: key.clone(),
                step: step.id.clone(),
            })?;
        environment.apply(&key, &entries).map_err(|reason| {
            LifecycleError::Environment {
                key,
                step: step.id.clone(),
                reason,
            }
            .into()
        })
    }
}
