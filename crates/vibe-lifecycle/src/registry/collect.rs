//! Collection and host-control application over an owned effective world.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION");

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use specmark::spec;
use thiserror::Error;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{ExtensionKey, ExtensionUse};

use super::model::{
    ContributionTier, DependencyProvider, ExtensionProvider, ExtensionRegistry,
    ExtensionRegistryRow, ExtensionWorld, HostIdentity, HostProvider,
};
use super::selector::{CompiledSelector, SelectorCompileError};

/// Non-fatal facts surfaced while constructing the effective registry.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AUTO-BY-FAMILY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionNotice {
    /// A dependency authored `auto = true` at a compile point; host activation
    /// remains mandatory, so the authored value was ignored.
    InstalledCompileAutoIgnored { key: ExtensionKey },
}

impl CollectionNotice {
    /// The exact opaque contribution key associated with this notice.
    #[must_use]
    pub const fn key(&self) -> &ExtensionKey {
        match self {
            Self::InstalledCompileAutoIgnored { key } => key,
        }
    }
}

impl fmt::Display for CollectionNotice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InstalledCompileAutoIgnored { key } => write!(
                formatter,
                "installed compile contribution `{key}` authored `auto = true`; the value is ignored and explicit host activation is still required"
            ),
        }
    }
}

/// A hard error that prevents construction of an unambiguous registry.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CollectionError {
    /// A caller-constructed declaration violated the manifest grammar.
    #[error(
        "extension declaration `{key}` is invalid: {reason}; fix the provider manifest (spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR)"
    )]
    InvalidDeclaration { key: ExtensionKey, reason: String },

    /// Two structural declaration sites rendered the same public key.
    #[error(
        "extension key collision for `{key}`: {first} and {second} are distinct declaration sites; rename one declaration id (spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION)"
    )]
    DuplicateDeclarationKey {
        key: ExtensionKey,
        first: String,
        second: String,
    },

    /// A selector glob could not be compiled.
    #[error(
        "extension `{key}` has malformed `{field}` glob `{pattern}`: {reason}; fix the positive selector glob (spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR)"
    )]
    MalformedSelector {
        key: ExtensionKey,
        field: &'static str,
        pattern: String,
        reason: String,
    },

    /// The same exact dependency reference was activated twice.
    #[error(
        "duplicate [[extensions.use]] reference `{key}` at indices {first} and {duplicate}; remove the duplicate activation (spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION)"
    )]
    DuplicateUse {
        key: ExtensionKey,
        first: usize,
        duplicate: usize,
    },

    /// An activation named no declaration in the selected world.
    #[error(
        "unresolved [[extensions.use]] reference `{key}`; run `vibe install` to refresh providers, or correct the exact ref (spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION)"
    )]
    UnresolvedUse { key: ExtensionKey },

    /// Activations may target installed dependencies, never host declarations.
    #[error(
        "[[extensions.use]] reference `{key}` targets a host declaration; remove the activation because host declarations are already active (spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION)"
    )]
    UseTargetsHost { key: ExtensionKey },

    /// A disable key named no declaration in the selected world.
    #[error(
        "unknown [extensions].disable reference `{key}`; run `vibe install` to refresh providers, or correct or remove the exact key (spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION)"
    )]
    UnknownDisable { key: ExtensionKey },

    /// A pure virtual workspace attempted to supply a direct declaration.
    #[error(
        "pure virtual workspace cannot declare extension `{id}`; move it to a project/package manifest (spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR)"
    )]
    VirtualHostDeclaration { id: String },
}

/// Collect every declaration, apply host controls, and freeze effective order.
///
/// The installed vector is consumed exactly in its caller-supplied lock order.
/// This function performs no filesystem, environment, resolver, or workspace
/// access and never infers an effective stack.
pub fn collect_extensions(world: ExtensionWorld) -> Result<ExtensionRegistry, CollectionError> {
    let ExtensionWorld {
        installed,
        host,
        effective_stack,
    } = world;
    let mut rows = Vec::new();
    let mut row_by_key = BTreeMap::new();
    let mut notices = Vec::new();

    for (provider_ordinal, source) in installed.into_iter().enumerate() {
        let is_effective_stack = effective_stack.as_ref() == Some(&source.provider.id);
        for (declaration_ordinal, declaration) in source.declarations.into_iter().enumerate() {
            let key = dependency_key(&source.provider, &declaration.id);
            validate_declaration(&key, &declaration)?;
            let selector = compile_selector(&key, declaration.applies_to.as_ref())?;
            let is_phase = matches!(declaration.point, ExtensionPoint::Phase(_));
            let is_compile = matches!(declaration.point, ExtensionPoint::Compile(_));
            let natural_tier = if is_effective_stack && is_phase {
                ContributionTier::Preset
            } else {
                ContributionTier::Dependency
            };
            if is_compile && declaration.auto == Some(true) {
                notices.push(CollectionNotice::InstalledCompileAutoIgnored { key: key.clone() });
            }
            let row = ExtensionRegistryRow {
                key,
                provider: ExtensionProvider::Dependency(source.provider.clone()),
                effective_config: declaration.config.clone(),
                declaration,
                natural_tier,
                effective_tier: natural_tier,
                provider_ordinal: Some(provider_ordinal),
                declaration_ordinal,
                activation_ordinal: None,
                active_by_default: !is_compile,
                activated: false,
                disabled: false,
                selector,
            };
            push_unique_row(&mut rows, &mut row_by_key, row)?;
        }
    }

    let host_provider = host.provider;
    for (declaration_ordinal, declaration) in host.declarations.into_iter().enumerate() {
        let key = host_key(&host_provider, &declaration.id)?;
        validate_declaration(&key, &declaration)?;
        let selector = compile_selector(&key, declaration.applies_to.as_ref())?;
        let row = ExtensionRegistryRow {
            key,
            provider: ExtensionProvider::Host(host_provider.clone()),
            effective_config: declaration.config.clone(),
            declaration,
            natural_tier: ContributionTier::HostDeclaration,
            effective_tier: ContributionTier::HostDeclaration,
            provider_ordinal: None,
            declaration_ordinal,
            activation_ordinal: None,
            active_by_default: true,
            activated: false,
            disabled: false,
            selector,
        };
        push_unique_row(&mut rows, &mut row_by_key, row)?;
    }

    let mut activation_indices = Vec::with_capacity(host.controls.uses.len());
    apply_activations(
        host.controls.uses,
        &row_by_key,
        &mut rows,
        &mut activation_indices,
    )?;
    apply_disables(host.controls.disable, &row_by_key, &mut rows)?;

    let mut effective_order = Vec::with_capacity(rows.len());
    for tier in [
        ContributionTier::Preset,
        ContributionTier::Dependency,
        ContributionTier::HostDeclaration,
    ] {
        effective_order.extend(
            rows.iter()
                .enumerate()
                .filter(|(_, row)| row.effective_tier == tier)
                .map(|(index, _)| index),
        );
    }
    effective_order.extend(activation_indices);

    Ok(ExtensionRegistry {
        rows,
        effective_order,
        notices,
    })
}

fn dependency_key(provider: &DependencyProvider, id: &str) -> ExtensionKey {
    ExtensionKey::for_package(provider.id.group(), provider.id.name(), id)
}

fn host_key(provider: &HostProvider, id: &str) -> Result<ExtensionKey, CollectionError> {
    match &provider.identity {
        HostIdentity::UngroupedProject(name) => Ok(ExtensionKey::for_host(name, id)),
        HostIdentity::Coordinate(identity) => Ok(ExtensionKey::for_package(
            identity.group(),
            identity.name(),
            id,
        )),
        HostIdentity::VirtualWorkspace => {
            Err(CollectionError::VirtualHostDeclaration { id: id.to_owned() })
        }
    }
}

fn validate_declaration(
    key: &ExtensionKey,
    declaration: &vibe_core::manifest::ExtensionDecl,
) -> Result<(), CollectionError> {
    declaration
        .validate()
        .map_err(|reason| CollectionError::InvalidDeclaration {
            key: key.clone(),
            reason,
        })
}

fn compile_selector(
    key: &ExtensionKey,
    selector: Option<&vibe_core::manifest::ExtensionAppliesTo>,
) -> Result<CompiledSelector, CollectionError> {
    CompiledSelector::compile(selector).map_err(
        |SelectorCompileError {
             field,
             pattern,
             reason,
         }| CollectionError::MalformedSelector {
            key: key.clone(),
            field,
            pattern,
            reason,
        },
    )
}

fn push_unique_row(
    rows: &mut Vec<ExtensionRegistryRow>,
    row_by_key: &mut BTreeMap<ExtensionKey, usize>,
    row: ExtensionRegistryRow,
) -> Result<(), CollectionError> {
    if let Some(first_index) = row_by_key.get(&row.key).copied() {
        return Err(CollectionError::DuplicateDeclarationKey {
            key: row.key.clone(),
            first: declaration_site(&rows[first_index]),
            second: declaration_site(&row),
        });
    }
    let index = rows.len();
    row_by_key.insert(row.key.clone(), index);
    rows.push(row);
    Ok(())
}

fn declaration_site(row: &ExtensionRegistryRow) -> String {
    match row.provider_ordinal {
        Some(lock) => format!(
            "dependency {} at lock index {lock}, declaration index {}",
            row.provider, row.declaration_ordinal
        ),
        None => format!(
            "host {} at declaration index {}",
            row.provider, row.declaration_ordinal
        ),
    }
}

fn apply_activations(
    uses: Vec<ExtensionUse>,
    row_by_key: &BTreeMap<ExtensionKey, usize>,
    rows: &mut [ExtensionRegistryRow],
    activation_indices: &mut Vec<usize>,
) -> Result<(), CollectionError> {
    let mut first_use_by_key = BTreeMap::new();
    for (activation_ordinal, activation) in uses.into_iter().enumerate() {
        if let Some(first) =
            first_use_by_key.insert(activation.reference.clone(), activation_ordinal)
        {
            return Err(CollectionError::DuplicateUse {
                key: activation.reference,
                first,
                duplicate: activation_ordinal,
            });
        }
        let Some(row_index) = row_by_key.get(&activation.reference).copied() else {
            return Err(CollectionError::UnresolvedUse {
                key: activation.reference,
            });
        };
        let row = &mut rows[row_index];
        if !row.provider.is_dependency() {
            return Err(CollectionError::UseTargetsHost {
                key: activation.reference,
            });
        }
        row.activated = true;
        row.effective_tier = ContributionTier::HostActivation;
        row.activation_ordinal = Some(activation_ordinal);
        if let Some(config) = activation.config {
            row.effective_config = Some(config);
        }
        activation_indices.push(row_index);
    }
    Ok(())
}

fn apply_disables(
    disables: Vec<ExtensionKey>,
    row_by_key: &BTreeMap<ExtensionKey, usize>,
    rows: &mut [ExtensionRegistryRow],
) -> Result<(), CollectionError> {
    let mut seen = BTreeSet::new();
    for key in disables {
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(row_index) = row_by_key.get(&key).copied() else {
            return Err(CollectionError::UnknownDisable { key });
        };
        rows[row_index].disabled = true;
    }
    Ok(())
}
