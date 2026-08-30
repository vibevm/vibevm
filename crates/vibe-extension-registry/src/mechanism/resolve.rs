//! §3.1's resolution and override law, as a pure function over one collected
//! registry.
//!
//! ```text
//! 1. an exact `provider` pin on the target wins;
//! 2. otherwise the host-owned `[mechanisms]` route wins;
//! 3. otherwise the shipped builtin default wins;
//! 4. otherwise resolution fails and lists the installed candidates.
//! ```
//!
//! What the law is *for* is the sentence that follows it in §3.1: "Installing a
//! dependency never lets it seize a logical key: there is no implicit
//! short-name discovery, 'highest version wins' or filesystem-order fallback."
//! So there is deliberately no fifth step and no tie-break — an installed
//! provider that no pin and no route names is INERT, whatever it calls itself.
//! Nothing here reads the filesystem, orders versions, or executes a provider.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::fmt;

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{MechanismKey, MechanismRoutes, ProviderPin};

use super::{MechanismRegistry, MechanismRegistryRow};

/// How many candidate identities a refusal spells before it summarises the
/// rest. A candidate list exists to be read by a human repairing a manifest;
/// an unbounded one in a world with hundreds of providers is a wall, not a
/// diagnostic.
const CANDIDATE_PREVIEW: usize = 8;

/// Which of §3.1's steps named the selected provider.
///
/// ```
/// use vibe_extension_registry::SelectionStep;
///
/// assert_eq!(SelectionStep::TargetPin.to_string(), "an exact `provider` pin on the target");
/// assert_ne!(SelectionStep::HostRoute, SelectionStep::BuiltinDefault);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStep {
    /// Step 1 — the target pinned this exact provider.
    TargetPin,
    /// Step 2 — the host routed this logical key to this exact provider.
    HostRoute,
    /// Step 3 — nothing selected, so the shipped default answered.
    BuiltinDefault,
}

impl fmt::Display for SelectionStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TargetPin => "an exact `provider` pin on the target",
            Self::HostRoute => "the host-owned `[mechanisms]` route",
            Self::BuiltinDefault => "the shipped builtin default",
        })
    }
}

/// One resolved routing decision: the selected row, the step that selected it,
/// and the builtin default it displaced, if any.
///
/// The displaced default is carried because §3.1 requires the registry display
/// and the lifecycle narration to show it, and because it is the honest proof
/// that a replacement replaced something: the builtin row is still collected,
/// still queryable, and simply NOT selected.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, Copy)]
pub struct MechanismSelection<'registry> {
    row: &'registry MechanismRegistryRow,
    via: SelectionStep,
    displaced_default: Option<&'registry MechanismRegistryRow>,
}

impl<'registry> MechanismSelection<'registry> {
    /// The selected provider row.
    #[must_use]
    pub const fn row(&self) -> &'registry MechanismRegistryRow {
        self.row
    }

    /// The step of §3.1 that selected it.
    #[must_use]
    pub const fn via(&self) -> SelectionStep {
        self.via
    }

    /// The shipped builtin default this selection displaced, when the winner
    /// is not the default itself.
    #[must_use]
    pub const fn displaced_default(&self) -> Option<&'registry MechanismRegistryRow> {
        self.displaced_default
    }
}

/// Why one logical key could not be routed to a provider.
///
/// Each variant names what was asked for and what the world actually holds,
/// because every one of them is a repairable manifest state rather than a
/// program bug.
///
/// The selected identity is boxed for the reason the durable adapter boxes its
/// own inner error: a `ProviderPin` carries a coordinate's two owned strings,
/// and an unboxed copy would put that weight on every `Result` selection
/// returns (`clippy::result_large_err`). It reads as a `ProviderPin` at every
/// use site.
///
/// ```
/// use vibe_core::manifest::MechanismKey;
/// use vibe_extension_registry::MechanismResolutionError;
///
/// let key: MechanismKey = "build:zig".parse().unwrap();
/// let refusal = MechanismResolutionError::NoProvider {
///     key,
///     candidates: "none installed".to_owned(),
/// };
/// assert!(refusal.to_string().contains("no target pin"));
/// assert!(refusal.to_string().contains("PROP-054#ONE-MACHINE"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MechanismResolutionError {
    /// A pin or a route named a provider this world does not install.
    #[error(
        "`{key}` selects `{pin}` through {via}, which is not installed in this world; \
         candidates for `{key}`: {candidates} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: require the package that declares `{pin}`, then run \
         `vibe install` — or correct the identity)"
    )]
    UninstalledProvider {
        key: MechanismKey,
        pin: Box<ProviderPin>,
        via: SelectionStep,
        candidates: String,
    },

    /// A pin or a route named an installed provider that services a different
    /// capability. Selection is not a rename: a provider answers the key it
    /// declares, or it answers nothing.
    #[error(
        "`{key}` selects `{pin}` through {via}, but that provider services `{provides}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: align the key with the provider's `role`/`name`, or \
         select a provider that declares `{key}`)"
    )]
    CapabilityMismatch {
        key: MechanismKey,
        pin: Box<ProviderPin>,
        via: SelectionStep,
        provides: MechanismKey,
    },

    /// A pin or a route named a provider the host disabled. Selecting it
    /// anyway would make the disable list advisory.
    #[error(
        "`{key}` selects `{pin}` through {via}, but the host disabled that provider \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: remove `{pin}` from `[extensions].disable`, or select \
         another provider)"
    )]
    DisabledProvider {
        key: MechanismKey,
        pin: Box<ProviderPin>,
        via: SelectionStep,
    },

    /// Nothing pinned, nothing routed, and the engine ships no default for
    /// this key. An installed provider that merely *calls itself* by this
    /// key's name does not qualify — that is the whole point of the law.
    #[error(
        "no provider is selected for `{key}`: no target pin, no `[mechanisms]` route, and no \
         shipped builtin default; candidates for `{key}`: {candidates} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: pin one on the target, or route `{key}` to one of the \
         candidates in `[mechanisms]` — an installed provider is never selected implicitly)"
    )]
    NoProvider {
        key: MechanismKey,
        candidates: String,
    },
}

/// Resolve one logical mechanism key to exactly one collected provider row.
///
/// `key` is §3.1's `(role, logical_name)` pair in its one typed spelling;
/// `target_pin` is the target's own exact `provider` pin, if it authored one;
/// `host_routes` is the host's `[mechanisms]` table. Selection is pure and
/// total over the registry: it reads no manifest, orders no versions, and
/// executes nothing.
///
/// ```
/// use vibe_core::manifest::{MechanismKey, MechanismRoutes, ProviderPin};
/// use vibe_extension_registry::{SelectionStep, collect_mechanisms, resolve_mechanism};
/// # use vibe_extension_registry::{
/// #     DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
/// #     HostExtensionSource, HostIdentity, HostProvider,
/// # };
/// # use vibe_core::manifest::{ExtensionHandler, ExtensionsControl, MechanismDecl, MechanismFreshness, MechanismRole};
/// # use vibe_core::{ContentHash, Group, PackageKind, PackageName};
/// # use std::path::PathBuf;
/// # let plugin = MechanismDecl {
/// #     id: "cargo-v2".into(),
/// #     role: MechanismRole::Build,
/// #     name: "cargo".into(),
/// #     handler: ExtensionHandler::Native { crate_dir: Some(PathBuf::from("crates/p")), prebuilt: None },
/// #     protocol: 1,
/// #     config_schema: PathBuf::from("schemas/cargo-build-v1.jtd.json"),
/// #     freshness: MechanismFreshness::Provider,
/// # };
/// # let world = ExtensionWorld {
/// #     installed: vec![DependencyExtensionSource {
/// #         provider: DependencyProvider {
/// #             id: DependencyProviderId::new(
/// #                 Group::parse("org.example").unwrap(),
/// #                 PackageName::parse("build-tools").unwrap(),
/// #             ),
/// #             root: PathBuf::from("vibedeps/build-tools"),
/// #             version: "1.0.0".into(),
/// #             kind: PackageKind::Tool,
/// #             content_hash: ContentHash::parse("sha256:aa").unwrap(),
/// #         },
/// #         declarations: Vec::new(),
/// #         controls: ExtensionsControl::default(),
/// #         mechanisms: vec![plugin],
/// #     }],
/// #     host: HostExtensionSource {
/// #         provider: HostProvider {
/// #             identity: HostIdentity::ungrouped_project("demo"),
/// #             root: PathBuf::from("."),
/// #             version: "0.1.0".into(),
/// #             kind: None,
/// #             content_hash: None,
/// #         },
/// #         declarations: Vec::new(),
/// #         controls: ExtensionsControl::default(),
/// #         mechanisms: Vec::new(),
/// #     },
/// #     effective_stack: None,
/// # };
/// let registry = collect_mechanisms(&world).unwrap();
/// let key: MechanismKey = "build:cargo".parse().unwrap();
/// let plugin: ProviderPin = "org.example/build-tools#cargo-v2".parse().unwrap();
///
/// // Installed, but nothing selects it: the shipped default still answers.
/// let default = resolve_mechanism(&registry, &key, None, &MechanismRoutes::default()).unwrap();
/// assert_eq!(default.via(), SelectionStep::BuiltinDefault);
/// assert_eq!(default.row().pin().to_string(), "org.vibevm/vibe#cargo");
///
/// // The host routes the logical key, and the plugin displaces the default.
/// let mut routes = MechanismRoutes::default();
/// routes.insert(key.clone(), plugin.clone());
/// let routed = resolve_mechanism(&registry, &key, None, &routes).unwrap();
/// assert_eq!(routed.via(), SelectionStep::HostRoute);
/// assert_eq!(routed.row().pin(), &plugin);
/// assert_eq!(
///     routed.displaced_default().unwrap().pin().to_string(),
///     "org.vibevm/vibe#cargo",
/// );
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
pub fn resolve_mechanism<'registry>(
    registry: &'registry MechanismRegistry,
    key: &MechanismKey,
    target_pin: Option<&ProviderPin>,
    host_routes: &MechanismRoutes,
) -> Result<MechanismSelection<'registry>, MechanismResolutionError> {
    // 1. an exact `provider` pin on the target wins.
    if let Some(pin) = target_pin {
        return select(registry, key, pin, SelectionStep::TargetPin);
    }
    // 2. otherwise the host-owned `[mechanisms]` route wins.
    if let Some(pin) = host_routes.get(&key.to_string()) {
        return select(registry, key, pin, SelectionStep::HostRoute);
    }
    // 3. otherwise the shipped builtin default wins.
    if let Some(row) = registry.builtin_default(key) {
        return Ok(MechanismSelection {
            row,
            via: SelectionStep::BuiltinDefault,
            displaced_default: None,
        });
    }
    // 4. otherwise resolution fails and lists the installed candidates.
    Err(MechanismResolutionError::NoProvider {
        key: key.clone(),
        candidates: candidates(registry, key),
    })
}

/// Turn one exact identity — however it was named — into a selection, or into
/// the typed refusal that says why that identity cannot answer this key.
fn select<'registry>(
    registry: &'registry MechanismRegistry,
    key: &MechanismKey,
    pin: &ProviderPin,
    via: SelectionStep,
) -> Result<MechanismSelection<'registry>, MechanismResolutionError> {
    let Some(row) = registry.find(pin) else {
        return Err(MechanismResolutionError::UninstalledProvider {
            key: key.clone(),
            pin: Box::new(pin.clone()),
            via,
            candidates: candidates(registry, key),
        });
    };
    if &row.key != key {
        return Err(MechanismResolutionError::CapabilityMismatch {
            key: key.clone(),
            pin: Box::new(pin.clone()),
            via,
            provides: row.key.clone(),
        });
    }
    if !row.is_enabled() {
        return Err(MechanismResolutionError::DisabledProvider {
            key: key.clone(),
            pin: Box::new(pin.clone()),
            via,
        });
    }
    Ok(MechanismSelection {
        row,
        via,
        displaced_default: registry
            .builtin_default(key)
            .filter(|default| default.pin != row.pin),
    })
}

/// The installed candidates for one key, bounded and in collection order.
fn candidates(registry: &MechanismRegistry, key: &MechanismKey) -> String {
    let identities: Vec<String> = registry
        .candidates(key)
        .map(|row| row.pin.to_string())
        .collect();
    if identities.is_empty() {
        return "none installed".to_owned();
    }
    let kept = identities.len().min(CANDIDATE_PREVIEW);
    let listed = identities[..kept].join(", ");
    if kept == identities.len() {
        listed
    } else {
        format!("{listed}, and {} more", identities.len() - kept)
    }
}
