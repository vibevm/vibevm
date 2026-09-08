//! Immutable pass-tier planning beside the staged [`TransformPlan`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

pub(crate) mod fault;
pub(crate) mod lowering;
pub(crate) mod plan;

pub(crate) use plan::PassPlan;

#[cfg(test)]
#[path = "pass_tier/tests.rs"]
mod tests;

use crate::compiler::transform::plan::TransformProvider;
