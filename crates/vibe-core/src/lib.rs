//! Core types for vibevm.
//!
//! This crate holds the pieces every other vibevm crate depends on:
//! - Package identity: [`PackageRef`], [`PackageKind`], [`VersionSpec`].
//! - Manifest schemas: [`manifest::ProjectManifest`], [`manifest::PackageManifest`], [`manifest::Lockfile`].
//! - Typed-value tags exchanged between task-graph nodes: [`values::ValueTag`].
//!
//! Spec: `VIBEVM-SPEC.md` §4, §5.3, §7.

#![forbid(unsafe_code)]

pub mod error;
pub mod manifest;
pub mod package_ref;
pub mod timestamp;
pub mod values;

pub use error::{Error, Result};
pub use package_ref::{PackageKind, PackageRef, VersionSpec};
