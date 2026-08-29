//! The owner-scoped transform plan value family (PROP-054
//! `#TRANSFORM-PLAN-IDENTITY`, frozen by
//! `campaigns/packages-2026-09/R4-TRANSFORM-PLAN-ABI-v0.1.md`).
//!
//! T1 lands the lossless neutral effective-configuration tree and its
//! canonical `ConfigDigest`. The plan, seed, provider, implementation and
//! selector values arrive with the later R4.1 atoms. Nothing here parses,
//! renders, or round-trips TOML: the workspace lowerer lowers effective
//! `ExtensionConfig` rows into this tree, and identity is defined on the tree
//! alone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

pub(crate) mod config;

#[cfg(test)]
mod config_tests;
