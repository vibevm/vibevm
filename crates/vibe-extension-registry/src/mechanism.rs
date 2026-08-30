//! The mechanism plane — provider rows beside the extension rows, in the ONE
//! registry kernel.
//!
//! A `[[mechanism]]` declaration is a *provider* for one logical capability of
//! the build/package/deploy plane; an `[[extension]]` declaration is a
//! contribution at a scheduled moment. They are different nouns on one machine:
//! the same provider identity, the same collection walk, the same world
//! snapshot, the same disable controls. What the mechanism plane adds is
//! **lookup** — a mechanism is inert until a target selects it, and selection is
//! the pure four-step law in [`resolve`].
//!
//! Three sources feed one [`MechanismRegistry`], in this order:
//!
//! 1. the engine's own [`builtin_mechanism_source`], which collection ALWAYS
//!    appends first — ordinary rows under the reserved `org.vibevm/vibe`
//!    identity, never a privileged branch in the resolver;
//! 2. every installed package's declarations, in the caller-supplied lock
//!    order;
//! 3. the selected host's own declarations.
//!
//! Nothing here executes a provider, reads a config schema, or touches the
//! filesystem: the plane, the routes and the selection law land at this atom,
//! and the provider protocol (plan/fingerprint/apply/verify/remove/recover)
//! lands with the first real provider.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

mod builtin;
mod collect;
mod resolve;

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{
    ExtensionHandler, MechanismDecl, MechanismKey, MechanismRole, ProviderOwner, ProviderPin,
};

use super::collect::CollectionError;
use super::model::{DependencyProvider, HostIdentity, HostProvider};

pub use builtin::{BuiltinMechanismSource, builtin_mechanism_source};
pub use collect::collect_mechanisms;
pub(super) use collect::mechanism_disable_targets;
pub use resolve::{MechanismResolutionError, MechanismSelection, SelectionStep, resolve_mechanism};

/// Who declared one mechanism row.
///
/// The third variant is what keeps §3's promise: a builtin is a *source*, not
/// a branch. Its rows carry the reserved `org.vibevm/vibe` identity and sit in
/// the same vector, under the same key law, as any package's.
///
/// ```
/// use vibe_extension_registry::MechanismProvider;
///
/// assert_eq!(MechanismProvider::Builtin.to_string(), "org.vibevm/vibe");
/// assert!(MechanismProvider::Builtin.is_builtin());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MechanismProvider {
    /// The engine itself, under the reserved provider identity.
    Builtin,
    /// A package from the installed lock-ordered world.
    Dependency(DependencyProvider),
    /// The selected host manifest.
    Host(HostProvider),
}

impl MechanismProvider {
    /// Whether this row is engine-owned and therefore immune to host controls.
    #[must_use]
    pub const fn is_builtin(&self) -> bool {
        matches!(self, Self::Builtin)
    }
}

impl fmt::Display for MechanismProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin => formatter.write_str(builtin::RESERVED_OWNER),
            Self::Dependency(provider) => provider.id.fmt(formatter),
            Self::Host(provider) => provider.identity.fmt(formatter),
        }
    }
}

/// One collected `[[mechanism]]` declaration and its effective control state.
///
/// A disabled row stays queryable — only [`resolve_mechanism`] refuses to
/// select it — for the same reason a disabled extension row stays retained:
/// the registry display must be able to say a provider is installed AND off.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone)]
pub struct MechanismRegistryRow {
    pub(super) pin: ProviderPin,
    pub(super) key: MechanismKey,
    pub(super) provider: MechanismProvider,
    pub(super) declaration: MechanismDecl,
    pub(super) provider_ordinal: Option<usize>,
    pub(super) declaration_ordinal: usize,
    pub(super) disabled: bool,
}

impl MechanismRegistryRow {
    /// The exact, group-qualified provider identity this row is keyed by —
    /// `<group>/<package>#<id>` or `__host__/<project>#<id>`. Never a short id:
    /// §3.1's law is that every stored and returned identity is qualified.
    #[must_use]
    pub const fn pin(&self) -> &ProviderPin {
        &self.pin
    }

    /// The logical capability this row services, `<role>:<name>`.
    #[must_use]
    pub const fn key(&self) -> &MechanismKey {
        &self.key
    }

    /// The row's role family.
    #[must_use]
    pub fn role(&self) -> MechanismRole {
        self.key.role()
    }

    /// The row's logical mechanism name — the `name` half of its key, which is
    /// a capability and never an identity.
    #[must_use]
    pub fn logical_name(&self) -> &str {
        self.key.name()
    }

    /// The complete authored declaration.
    #[must_use]
    pub const fn declaration(&self) -> &MechanismDecl {
        &self.declaration
    }

    /// How this provider is implemented.
    #[must_use]
    pub const fn handler(&self) -> &ExtensionHandler {
        &self.declaration.handler
    }

    /// The numbered provider protocol this row speaks.
    #[must_use]
    pub const fn protocol(&self) -> u32 {
        self.declaration.protocol
    }

    /// The declarant-relative path of this provider's config schema.
    #[must_use]
    pub fn config_schema(&self) -> &Path {
        &self.declaration.config_schema
    }

    /// Who declared this row.
    #[must_use]
    pub const fn provider(&self) -> &MechanismProvider {
        &self.provider
    }

    /// Whether the engine minted this row.
    #[must_use]
    pub const fn is_builtin(&self) -> bool {
        self.provider.is_builtin()
    }

    /// Lock ordinal for a package-declared row; absent for the builtin source
    /// and for the host.
    #[must_use]
    pub const fn provider_ordinal(&self) -> Option<usize> {
        self.provider_ordinal
    }

    /// Declaration ordinal within its own source.
    #[must_use]
    pub const fn declaration_ordinal(&self) -> usize {
        self.declaration_ordinal
    }

    /// Whether the exact pin appeared in the host disable list.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Whether this row may be selected. Mechanisms have no activation tier:
    /// a row is selectable unless a host disabled it, and inert until a target
    /// routes to it.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        !self.disabled
    }
}

/// The collected mechanism plane of one owner-scoped world.
///
/// Rows are retained in COLLECTION ORDER — builtins, then installed packages
/// in lock order, then the host — because that is the order a registry display
/// reads and the order the candidate list in a refusal is built from. There is
/// no second, "effective" order: mechanisms are not scheduled, so there is
/// nothing to sequence.
///
/// ```
/// use vibe_extension_registry::{builtin_mechanism_source, collect_mechanisms};
/// # use vibe_extension_registry::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};
/// # use vibe_core::manifest::ExtensionsControl;
/// # let world = ExtensionWorld {
/// #     installed: Vec::new(),
/// #     host: HostExtensionSource {
/// #         provider: HostProvider {
/// #             identity: HostIdentity::ungrouped_project("demo"),
/// #             root: std::path::PathBuf::from("."),
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
///
/// // A world with no declared provider is still the four shipped builtins.
/// assert_eq!(
///     registry.rows().len(),
///     builtin_mechanism_source().declarations().len(),
/// );
/// assert!(registry.rows().iter().all(|row| row.is_builtin()));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone)]
pub struct MechanismRegistry {
    pub(super) rows: Vec<MechanismRegistryRow>,
    pub(super) by_pin: BTreeMap<ProviderPin, usize>,
}

impl MechanismRegistry {
    /// Every collected row exactly once, in collection order and without loss.
    #[must_use]
    pub fn rows(&self) -> &[MechanismRegistryRow] {
        &self.rows
    }

    /// The row an exact provider identity names, if this world installs it.
    ///
    /// This is the ONE lookup a pin and a host route both go through, so the
    /// two selection steps can never disagree about what a pin means.
    #[must_use]
    pub fn find(&self, pin: &ProviderPin) -> Option<&MechanismRegistryRow> {
        self.by_pin.get(pin).map(|index| &self.rows[*index])
    }

    /// The shipped builtin default for one logical key, if the engine has one.
    #[must_use]
    pub fn builtin_default(&self, key: &MechanismKey) -> Option<&MechanismRegistryRow> {
        self.rows
            .iter()
            .find(|row| row.is_builtin() && &row.key == key)
    }

    /// Every row that services one logical key, in collection order — builtin
    /// and installed alike. Membership is NOT selection: an installed row that
    /// no pin and no route names is a candidate and stays inert.
    pub fn candidates<'registry>(
        &'registry self,
        key: &'registry MechanismKey,
    ) -> impl Iterator<Item = &'registry MechanismRegistryRow> + 'registry {
        self.rows.iter().filter(move |row| &row.key == key)
    }
}

/// The provider identity of one installed source, as the mechanism plane keys
/// it. Shared by the collector (to key rows and to detect impersonation), so a
/// row's key and an impersonation check can never be built from two different
/// renderings of one identity.
pub(super) fn owner_of_dependency(provider: &DependencyProvider) -> ProviderOwner {
    ProviderOwner::Package {
        group: provider.id.group().clone(),
        package: provider.id.name().clone(),
    }
}

/// The selected host's provider identity, or `None` for a pure virtual
/// workspace — which owns no coordinate and therefore declares no provider.
///
/// The two kinds are told apart structurally, exactly as [`HostIdentity`]
/// draws them, and the ungrouped project reaches its `__host__/<name>`
/// spelling through the one shared host-owner codec rather than a local
/// `format!`.
pub(super) fn host_owner(identity: &HostIdentity) -> Option<ProviderOwner> {
    match identity {
        HostIdentity::UngroupedProject(project) => Some(ProviderOwner::Host {
            project: project.clone(),
        }),
        HostIdentity::Coordinate(id) => Some(ProviderOwner::Package {
            group: id.group().clone(),
            package: id.name().clone(),
        }),
        HostIdentity::VirtualWorkspace => None,
    }
}

/// One declaration's exact, group-qualified provider identity.
///
/// `owner` is a canonical owner spelling — a typed [`ProviderOwner`]'s render
/// for a collected source, the reserved constant for the engine's own — and
/// the join is parsed back through the ONE `ProviderPin` grammar, so a row's
/// key is written in the same codec a host route and a target pin are. The
/// round trip succeeds for every canonical owner and every validated id; a
/// failure could only mean the declaration is unusable as an addressable
/// provider, which is exactly what the refusal it funnels into says.
pub(super) fn mechanism_pin(owner: &str, id: &str) -> Result<ProviderPin, CollectionError> {
    ProviderPin::parse(&format!("{owner}#{id}")).map_err(|error| {
        CollectionError::InvalidMechanism {
            owner: owner.to_owned(),
            id: id.to_owned(),
            reason: error.to_string(),
        }
    })
}

/// One declaration's logical capability key, `<role>:<name>`.
///
/// Parsed through the same grammar a `[mechanisms]` route key is written in,
/// so a route and a row can never disagree about what `build:cargo` means.
pub(super) fn mechanism_key(
    owner: &str,
    declaration: &MechanismDecl,
) -> Result<MechanismKey, CollectionError> {
    format!("{}:{}", declaration.role, declaration.name)
        .parse::<MechanismKey>()
        .map_err(|error| CollectionError::InvalidMechanism {
            owner: owner.to_owned(),
            id: declaration.id.clone(),
            reason: error.to_string(),
        })
}
