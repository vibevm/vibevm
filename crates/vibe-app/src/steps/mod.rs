//! The step engine (`PROP-065`): the planner, the runner over a kept work
//! directory with `steps.json`, one executor per step kind, the run records
//! and the wizard inputs (feature tree, profiles, parameters, license,
//! sizes). Every installer option reaches it as steps — the method forms
//! through the sugar mapping of `vibe_core::manifest::steps_of`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#principle");

mod agent;
mod configuration_view;
mod env;
mod envelope;
mod features;
mod kinds;
mod params;
mod plan;
mod plan_view;
mod resume;
mod run;
mod runs;
mod sizes;
mod state;
mod template;
mod work;

pub(crate) use agent::mode_of as agent_mode;
pub(crate) use env::StepPaths;
pub use features::{FeatureRequest, FeatureResolution, Implied, ImpliedRule, resolve_features};
pub(crate) use kinds::{MigrationEnv, StepContext, run as run_step};
pub(crate) use params::admit_license;
pub use params::{LicensePrompt, ParamValue, ParameterPrompt, PlanLicense, step_env_name};
pub use plan::{FeatureChooser, HostFacts, Plan, PlannedDependency, PlannedStep, RequiredBy};
pub(crate) use plan::{PlanInput, Previous, Prompts, RequirementResolver, plan};
pub(crate) use run::{Run, RunFlags};
pub(crate) use runs::{RunOutcome, RunRecord};
pub use runs::{prune_runs, runs};
pub use state::Start;
pub(crate) use state::{StepState, StepsFile};
pub(crate) use template::Vars as StepVars;
pub use work::{clean_import, clean_work, human_bytes, work, work_import};

#[cfg(test)]
mod tests;
