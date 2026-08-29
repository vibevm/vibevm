//! The owner-scoped transform plan value family (PROP-054
//! `#TRANSFORM-PLAN-IDENTITY`, frozen by
//! `campaigns/packages-2026-09/R4-TRANSFORM-PLAN-ABI-v0.1.md`).
//!
//! T1 landed the lossless neutral effective-configuration tree and its
//! canonical `ConfigDigest`; T2 lands the typed plan family — seed,
//! provider, implementation, entry, plan — with the refusal law and the
//! exact canonical digests. Nothing here parses, renders, or round-trips
//! TOML: the workspace lowerer lowers effective `ExtensionConfig` rows into
//! the config tree, and identity is defined on the typed values alone. The
//! family stays `pub(crate)` until T10's workspace adapter becomes the
//! first cross-crate consumer; schedule insertion, behavior lookup and
//! mutation arrive with the later R4.1 atoms.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

pub(crate) mod behavior;
pub(crate) mod config;
pub(crate) mod plan;
pub(crate) mod plan_digest;
pub(crate) mod plan_validate;
pub(crate) mod registry;
pub(crate) mod schedule;

#[cfg(test)]
pub(crate) mod carriage;
#[cfg(test)]
mod config_tests;
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
mod registry_fence_tests;
#[cfg(test)]
pub(crate) mod registry_test_support;
#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod schedule_execution_tests;
#[cfg(test)]
mod schedule_execution_vehicles;
#[cfg(test)]
mod schedule_fence_tests;
#[cfg(test)]
mod schedule_tests;
