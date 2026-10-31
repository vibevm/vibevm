//! The step engine (`PROP-065`): the planner, the runner over a kept work
//! directory with `steps.json`, and one executor per step kind. Every
//! installer option reaches it as steps — the method forms through the
//! sugar mapping of `vibe_core::manifest::steps_of`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#principle");

mod kinds;
mod plan;
mod run;
mod state;
mod template;

pub use plan::{FeatureChooser, FeatureRequest, HostFacts, Plan, PlannedStep};
pub(crate) use plan::{PlanInput, plan};
pub(crate) use run::{Run, RunFlags};
pub use state::Start;

#[cfg(test)]
mod tests;
