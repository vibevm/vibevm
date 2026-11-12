//! Core types for vibevm.
//!
//! This crate holds the pieces every other vibevm crate depends on:
//! - Package identity: [`PackageRef`], [`PackageKind`], [`VersionSpec`].
//! - Capability identity: [`CapabilityRef`] — abstract interfaces a package
//!   can `provide` and another package can `require` (PROP-002 §2.9).
//! - Lifecycle vocabulary: [`lifecycle::Phase`] and
//!   [`lifecycle::ExtensionPoint`].
//! - Manifest schemas: [`manifest::Manifest`], [`manifest::Lockfile`].
//! - Typed-value tags exchanged between task-graph nodes: [`values::ValueTag`].
//!
//! Spec: `VIBEVM-SPEC.md` §4, §5.3, §7.

#![forbid(unsafe_code)]
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability");

pub mod cancellation;
pub mod capabilities;
pub mod capability_ref;
pub mod channel;
pub mod configuration;
pub mod content_hash;
pub mod download_headers;
pub mod error;
pub mod global_registry;
pub mod host_owner;
pub mod index_checkout;
pub mod judgment;
pub mod layout;
pub mod layout_registry;
pub mod lifecycle;
pub mod lock_provenance;
pub mod manifest;
pub mod net;
pub mod package_ref;
pub mod progress;
pub mod provenance;
pub mod rel_path;
pub mod remap;
pub mod settings;
pub mod site_profile;
pub mod timestamp;
pub mod toml_preserve;
pub mod trust;
pub mod user_config;
pub mod values;
pub mod version_select;
pub mod visibility;
pub mod wire_decimal;

pub use capability_ref::{CapabilityName, CapabilityNamespace, CapabilityRef};
pub use content_hash::ContentHash;
pub use error::{Error, Result, SafeTomlParseError};
pub use global_registry::{
    EffectiveRegistryConfig, GlobalRegistryConfig, GlobalRegistryError, GlobalRegistrySource,
    PROJECT_LOCAL_REGISTRY_FILE, RegistryLayers, RegistryScope, default_registries,
    ensure_default_global_registry, merge_effective, project_local_registry_path, url_is_local,
};
pub use host_owner::{HOST_OWNER, HostOwner, HostSegmentError};
pub use index_checkout::IndexCheckoutSection;
pub use package_ref::{ChannelName, Group, PackageKind, PackageName, PackageRef, VersionSpec};
pub use provenance::{RoutePublication, SourceUrl, TraceId};
pub use rel_path::{RelPath, machine_json_path};
