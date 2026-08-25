//! Deterministic lifecycle vocabulary and request chaining for vibe.
//!
//! The default lifecycle is a fixed nine-phase ritual. Requesting a phase
//! includes every phase before it; the independent clean lifecycle can be
//! prepended to that chain.
//!
//! ```
//! use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase};
//!
//! assert_eq!(
//!     LifecycleRequest::Clean {
//!         then: Some(Phase::Build),
//!     }
//!     .steps(),
//!     vec![
//!         LifecycleStep::Clean,
//!         LifecycleStep::Default(Phase::Validate),
//!         LifecycleStep::Default(Phase::Install),
//!         LifecycleStep::Default(Phase::Generate),
//!         LifecycleStep::Default(Phase::Build),
//!     ],
//! );
//! ```
//!
//! The vocabulary remains available from this crate for compatibility even
//! though its single owner is now `vibe_core::lifecycle`:
//!
//! ```
//! use vibe_lifecycle::{CompilePoint, ExtensionPoint, Phase, PhasePoint};
//!
//! let phase = PhasePoint::Default(Phase::Build);
//! let point = ExtensionPoint::Phase(phase);
//! let core_point: vibe_core::lifecycle::ExtensionPoint = point;
//! assert_eq!(core_point.to_string(), "phase:build");
//! assert_eq!("compile:pass".parse(), Ok(CompilePoint::Pass));
//! ```
//!
//! Collection is pure: callers hand the crate an already selected installed
//! world in canonical lock order, then query the retained registry for a
//! subject-specific execution plan.
//!
//! ```
//! use std::path::PathBuf;
//! use vibe_core::{ContentHash, Group, PackageKind, PackageName};
//! use vibe_core::manifest::{ExtensionDecl, ExtensionHandler, ExtensionsControl};
//! use vibe_lifecycle::{
//!     DependencyExtensionSource, DependencyProvider, DependencyProviderId,
//!     ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
//!     SelectorSubject, collect_extensions,
//! };
//!
//! let id = DependencyProviderId::new(
//!     Group::parse("org.demo").unwrap(),
//!     PackageName::parse("tools").unwrap(),
//! );
//! let declaration = ExtensionDecl {
//!     id: "announce".into(),
//!     point: "phase:build".parse().unwrap(),
//!     handler: ExtensionHandler::Builtin { name: "log".into() },
//!     config: None, auto: None, inputs: None, applies_to: None,
//!     compiler_internals: None, pass: None, when: None,
//! };
//! let world = ExtensionWorld {
//!     installed: vec![DependencyExtensionSource {
//!         provider: DependencyProvider {
//!             id, root: PathBuf::from("vibedeps/tools"), version: "1.0.0".into(),
//!             kind: PackageKind::Tool,
//!             content_hash: ContentHash::parse("sha256:aa").unwrap(),
//!         },
//!         declarations: vec![declaration],
//!     }],
//!     host: HostExtensionSource {
//!         provider: HostProvider {
//!             identity: HostIdentity::ungrouped_project("demo"), root: PathBuf::from("."),
//!             version: "0.1.0".into(), kind: None, content_hash: None,
//!         },
//!         declarations: Vec::new(), controls: ExtensionsControl::default(),
//!     },
//!     effective_stack: None,
//! };
//! let registry = collect_extensions(world).unwrap();
//! let point = "phase:build".parse().unwrap();
//! assert_eq!(registry.plan(point, SelectorSubject::unscoped()).len(), 1);
//! ```

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

mod chain;
mod execution;
mod registry;

pub use chain::{LifecycleRequest, LifecycleStep, inclusive_chain};
pub use execution::{
    BuiltinRegistry, ContributionOutcome, DispatchBatch, DispatchError, ExecutionSession,
    RunMetadata,
};
pub use registry::{
    CollectionError, CollectionNotice, ContributionTier, DependencyExtensionSource,
    DependencyProvider, DependencyProviderId, ExecutableContribution, ExecutablePlan,
    ExtensionProvider, ExtensionRegistry, ExtensionRegistryRow, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, RegistryView, SelectorSubject,
    collect_extensions,
};
pub use vibe_core::lifecycle::{
    CompilePoint, CompilePointParseError, DEFAULT_PHASES, ExtensionPoint, ExtensionPointParseError,
    Phase, PhaseParseError, PhasePoint, PhasePointParseError, SlotPoint, SlotPointParseError,
};
