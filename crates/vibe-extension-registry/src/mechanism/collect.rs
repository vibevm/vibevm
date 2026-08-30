//! The mechanism collection walk — three sources, one registry.
//!
//! The walk is the extension collector's twin: the same world, the same lock
//! order, the same host-control surface. It is a separate entry only because
//! it yields a separate registry; nothing here re-reads a manifest, and no
//! second ordering rule exists.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::BTreeMap;

use specmark::spec;
use vibe_core::manifest::{MechanismDecl, ProviderPin};

use super::builtin::{RESERVED_OWNER, builtin_mechanism_source};
use super::{
    MechanismProvider, MechanismRegistry, MechanismRegistryRow, host_owner, mechanism_key,
    mechanism_pin, owner_of_dependency,
};
use crate::CollectionError;
use crate::model::ExtensionWorld;

/// Collect the mechanism plane of one owner-scoped world.
///
/// The engine's own [`builtin_mechanism_source`] is appended FIRST and
/// unconditionally, then every installed package in the caller-supplied lock
/// order, then the selected host. Host disables apply last, exactly as they do
/// to extension rows.
///
/// Borrowing rather than consuming is deliberate: the two planes are collected
/// from ONE snapshot, so a caller that wants both calls this first and then
/// hands the same world to [`collect_extensions`](crate::collect_extensions).
///
/// Pure: no filesystem, environment, resolver or process access, and no
/// provider is executed or fingerprinted.
///
/// ```
/// use vibe_core::manifest::MechanismKey;
/// use vibe_extension_registry::collect_mechanisms;
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
/// let key: MechanismKey = "build:cargo".parse().unwrap();
///
/// let builtin = registry.builtin_default(&key).unwrap();
/// assert_eq!(builtin.pin().to_string(), "org.vibevm/vibe#cargo");
/// assert_eq!(builtin.protocol(), 1);
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn collect_mechanisms(world: &ExtensionWorld) -> Result<MechanismRegistry, CollectionError> {
    let mut rows = Vec::new();
    let mut by_pin = BTreeMap::new();

    // 1. The engine's own source, ALWAYS and first. A builtin is an ordinary
    //    row; the only thing special about it is that nobody supplies it.
    let builtins = builtin_mechanism_source();
    for (declaration_ordinal, declaration) in builtins.declarations().iter().enumerate() {
        let row = MechanismRegistryRow {
            pin: mechanism_pin(builtins.owner(), &declaration.id)?,
            key: mechanism_key(builtins.owner(), declaration)?,
            provider: MechanismProvider::Builtin,
            declaration: declaration.clone(),
            provider_ordinal: None,
            declaration_ordinal,
            disabled: false,
        };
        push_unique(&mut rows, &mut by_pin, row)?;
    }

    // 2. Every installed package, in ROOT LOCK ORDER and in no other.
    for (provider_ordinal, source) in world.installed.iter().enumerate() {
        let owner = owner_of_dependency(&source.provider).to_string();
        collect_source(
            &owner,
            &source.mechanisms,
            &MechanismProvider::Dependency(source.provider.clone()),
            Some(provider_ordinal),
            &mut rows,
            &mut by_pin,
        )?;
    }

    // 3. The selected host's own declarations.
    match host_owner(&world.host.provider.identity).map(|owner| owner.to_string()) {
        Some(owner) => collect_source(
            &owner,
            &world.host.mechanisms,
            &MechanismProvider::Host(world.host.provider.clone()),
            None,
            &mut rows,
            &mut by_pin,
        )?,
        // A pure virtual workspace owns no provider identity, so it can route
        // and select but never declare — the `[[extension]]` precedent, and
        // vibe-core's own `[[mechanism]]` law.
        None => {
            if let Some(declaration) = world.host.mechanisms.first() {
                return Err(CollectionError::VirtualHostMechanism {
                    id: declaration.id.clone(),
                });
            }
        }
    }

    apply_mechanism_disables(world, &by_pin, &mut rows)?;

    Ok(MechanismRegistry { rows, by_pin })
}

/// Collect one collected (never engine-minted) source's declarations, under
/// that source's canonical owner spelling.
fn collect_source(
    owner: &str,
    declarations: &[MechanismDecl],
    provider: &MechanismProvider,
    provider_ordinal: Option<usize>,
    rows: &mut Vec<MechanismRegistryRow>,
    by_pin: &mut BTreeMap<ProviderPin, usize>,
) -> Result<(), CollectionError> {
    if owner == RESERVED_OWNER {
        // Impersonation of the engine's own identity, refused at the plane
        // that would otherwise key a stranger's row as a shipped default. A
        // package merely NAMED that coordinate declares nothing and takes
        // nothing, so the refusal is on the declaration, not on the name.
        if let Some(declaration) = declarations.first() {
            return Err(CollectionError::ReservedMechanismOwner {
                owner: owner.to_owned(),
                id: declaration.id.clone(),
            });
        }
    }
    for (declaration_ordinal, declaration) in declarations.iter().enumerate() {
        declaration
            .validate()
            .map_err(|reason| CollectionError::InvalidMechanism {
                owner: owner.to_owned(),
                id: declaration.id.clone(),
                reason,
            })?;
        let row = MechanismRegistryRow {
            pin: mechanism_pin(owner, &declaration.id)?,
            key: mechanism_key(owner, declaration)?,
            provider: provider.clone(),
            declaration: declaration.clone(),
            provider_ordinal,
            declaration_ordinal,
            disabled: false,
        };
        push_unique(rows, by_pin, row)?;
    }
    Ok(())
}

/// Retain one row under its provider-qualified key, refusing a second claim on
/// that exact identity.
fn push_unique(
    rows: &mut Vec<MechanismRegistryRow>,
    by_pin: &mut BTreeMap<ProviderPin, usize>,
    row: MechanismRegistryRow,
) -> Result<(), CollectionError> {
    if let Some(first_index) = by_pin.get(&row.pin).copied() {
        return Err(CollectionError::DuplicateMechanismKey {
            pin: row.pin.clone(),
            first: declaration_site(&rows[first_index]),
            second: declaration_site(&row),
        });
    }
    by_pin.insert(row.pin.clone(), rows.len());
    rows.push(row);
    Ok(())
}

fn declaration_site(row: &MechanismRegistryRow) -> String {
    match (&row.provider, row.provider_ordinal) {
        (MechanismProvider::Builtin, _) => format!(
            "the engine's builtin source at declaration index {}",
            row.declaration_ordinal
        ),
        (provider, Some(lock)) => format!(
            "dependency {provider} at lock index {lock}, declaration index {}",
            row.declaration_ordinal
        ),
        (provider, None) => format!(
            "host {provider} at declaration index {}",
            row.declaration_ordinal
        ),
    }
}

/// Apply the host's `[extensions].disable` list to the mechanism rows it names.
///
/// One disable list governs both planes, so an entry that names no mechanism
/// row is not this walk's business — the extension collector owns the
/// unknown-key refusal, and refusing here too would make every extension
/// disable in the tree an error. An entry naming an engine-minted row IS this
/// walk's business: a builtin default is not the host's to switch off, exactly
/// as a synthetic `@vibe/` contribution is not.
fn apply_mechanism_disables(
    world: &ExtensionWorld,
    by_pin: &BTreeMap<ProviderPin, usize>,
    rows: &mut [MechanismRegistryRow],
) -> Result<(), CollectionError> {
    for key in &world.host.controls.disable {
        let Ok(pin) = key.as_str().parse::<ProviderPin>() else {
            continue;
        };
        let Some(index) = by_pin.get(&pin).copied() else {
            continue;
        };
        if rows[index].is_builtin() {
            return Err(CollectionError::ReservedMechanismControl { pin });
        }
        rows[index].disabled = true;
    }
    Ok(())
}

/// Every mechanism identity this world can key, rendered through the ONE pin
/// codec — the set the extension collector consults before calling a disable
/// key unknown.
///
/// A world is caller-constructed, so an identity that cannot be keyed simply
/// is not in the set: [`collect_mechanisms`] refuses that world by name, and a
/// disable naming an unkeyable row was never going to resolve anyway.
pub(crate) fn mechanism_disable_targets(world: &ExtensionWorld) -> Vec<String> {
    let mut targets = Vec::new();
    let sources = world
        .installed
        .iter()
        .map(|source| {
            (
                owner_of_dependency(&source.provider).to_string(),
                &source.mechanisms,
            )
        })
        .chain(
            host_owner(&world.host.provider.identity)
                .map(|owner| (owner.to_string(), &world.host.mechanisms)),
        );
    for (owner, declarations) in sources {
        for declaration in declarations {
            if let Ok(pin) = mechanism_pin(&owner, &declaration.id) {
                targets.push(pin.to_string());
            }
        }
    }
    targets
}
