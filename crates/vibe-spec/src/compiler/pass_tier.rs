//! Immutable pass-tier planning beside the staged [`TransformPlan`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

pub(crate) mod backend;
pub(crate) mod catalog;
pub(crate) mod execution;
pub(crate) mod fault;
pub(crate) mod frontend;
pub(crate) mod lowering;
pub(crate) mod native;
pub(crate) mod plan;
pub(crate) mod schedule;

pub use execution::PassTierCompileError;
pub(crate) use plan::PassPlan;

#[cfg(test)]
#[path = "pass_tier/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pass_tier/execution_tests.rs"]
mod execution_tests;

#[cfg(test)]
#[path = "pass_tier/frontend_discovery_tests.rs"]
mod frontend_discovery_tests;

#[cfg(test)]
#[path = "pass_tier/catalog_tests.rs"]
mod catalog_tests;

use crate::compiler::transform::plan::TransformProvider;
