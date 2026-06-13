//! Core types for vibevm.
//!
//! This crate holds the pieces every other vibevm crate depends on:
//! - Package identity: [`PackageRef`], [`PackageKind`], [`VersionSpec`].
//! - Capability identity: [`CapabilityRef`] — abstract interfaces a package
//!   can `provide` and another package can `require` (PROP-002 §2.9).
//! - Manifest schemas: [`manifest::Manifest`], [`manifest::Lockfile`].
//! - Typed-value tags exchanged between task-graph nodes: [`values::ValueTag`].
//!
//! Spec: `VIBEVM-SPEC.md` §4, §5.3, §7.

#![forbid(unsafe_code)]
specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#capability");

pub mod capability_ref;
pub mod error;
pub mod manifest;
pub mod package_ref;
pub mod rel_path;
pub mod timestamp;
pub mod user_config;
pub mod values;

pub use capability_ref::{CapabilityName, CapabilityNamespace, CapabilityRef};
pub use error::{Error, Result};
pub use package_ref::{Group, PackageKind, PackageName, PackageRef, VersionSpec};
pub use rel_path::RelPath;
