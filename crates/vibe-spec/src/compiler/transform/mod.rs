//! The owner-scoped transform plan value family (PROP-054
//! `#TRANSFORM-PLAN-IDENTITY`, frozen by
//! `campaigns/packages-2026-09/R4-TRANSFORM-PLAN-ABI-v0.1.md`).
//!
//! T1 landed the lossless neutral effective-configuration tree and its
//! canonical `ConfigDigest`; T2 lands the typed plan family — seed,
//! provider, implementation, entry, plan — with the refusal law and the
//! exact canonical digests. Nothing here parses, renders, or round-trips
//! TOML: identity is defined on the typed values alone.
//!
//! T10B closed the family's one open seam — the workspace adapter, its
//! promised first cross-crate consumer. `lowering` is that entry: borrowed
//! kernel compile rows in, one owner-scoped plan out, with `config_lowering`
//! owning the effective-configuration half. Only [`plan::TransformPlan`]
//! itself widened to `pub`; every other member of the family, and every
//! constructor, stayed exactly where T2 put it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

pub(crate) mod behavior;
pub(crate) mod config;
pub(crate) mod config_lowering;
pub(crate) mod emitted_reconstruction;
pub(crate) mod fault;
pub(crate) mod lane_admission;
pub(crate) mod lowering;
pub(crate) mod plan;
pub(crate) mod plan_digest;
pub(crate) mod plan_validate;
pub(crate) mod registry;
pub(crate) mod schedule;
pub(crate) mod selector_admission;

#[cfg(test)]
pub(crate) mod carriage;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod lowering_e2e_tests;
#[cfg(test)]
mod lowering_tests;
#[cfg(test)]
mod lowering_worlds;
#[cfg(test)]
mod plan_digest_tests;
#[cfg(test)]
mod plan_fence_tests;
#[cfg(test)]
mod plan_refusal_tests;
#[cfg(test)]
mod plan_test_support;
#[cfg(test)]
mod plan_tests;
#[cfg(test)]
mod plan_visibility_fence_tests;
#[cfg(test)]
mod registry_fence_tests;
#[cfg(test)]
pub(crate) mod registry_test_support;
#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod schedule_emitted_tests;
#[cfg(test)]
mod schedule_execution_tests;
#[cfg(test)]
mod schedule_execution_vehicles;
#[cfg(test)]
mod schedule_fence_tests;
#[cfg(test)]
mod schedule_lane_tests;
#[cfg(test)]
mod schedule_lane_vehicles;
#[cfg(test)]
mod schedule_selector_tests;
#[cfg(test)]
mod schedule_selector_vehicles;
#[cfg(test)]
mod schedule_selector_worlds;
#[cfg(test)]
mod schedule_separator_tests;
#[cfg(test)]
mod schedule_tests;
#[cfg(test)]
mod selector_admission_tests;
#[cfg(test)]
mod transform_cells_fence_tests;
