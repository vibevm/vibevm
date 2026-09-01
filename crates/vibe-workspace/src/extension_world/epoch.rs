//! The exact ordered-resolution extension-world epoch.
//!
//! This cell owns only the command-supplied resolution constructor and the
//! public projections over the retained snapshot. Lock-backed compatibility,
//! shared closure/scoping rules and collectors stay in the parent module.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::manifest::Manifest;
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    lane_owner_host,
};

use crate::install::ResolvedDep;
use crate::vibedeps::{in_place_slot_abs_path, slot_abs_path};

use super::{
    ExtensionWorldEpoch, ExtensionWorldError, InstalledPackage, active_stack, checked_edges,
    declared_edges, host_source, owner_view, package,
};

/// Opaque identity of one exact ordered resolution and all composition facts
/// retained from it. Absolute materialisation roots never enter this value.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OrderedResolutionIdentity([u8; 32]);

impl fmt::Debug for OrderedResolutionIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OrderedResolutionIdentity(..)")
    }
}

fn ordered_resolution_identity(
    installed: &[InstalledPackage],
) -> Result<OrderedResolutionIdentity, ExtensionWorldError> {
    fn frame(hasher: &mut Sha256, bytes: &[u8]) {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }

    let mut hasher = ordered_resolution_hasher(installed.len());
    for entry in installed {
        let provider = &entry.source.provider;
        frame(&mut hasher, provider.id.group().as_str().as_bytes());
        frame(&mut hasher, provider.id.name().as_str().as_bytes());
        frame(&mut hasher, provider.version.as_bytes());
        frame(&mut hasher, provider.kind.as_str().as_bytes());
        frame(&mut hasher, provider.content_hash.to_string().as_bytes());
        hasher.update((entry.edges.len() as u64).to_be_bytes());
        for edge in &entry.edges {
            frame(&mut hasher, edge.group().as_str().as_bytes());
            frame(&mut hasher, edge.name().as_str().as_bytes());
        }
        let manifest = serde_json::to_vec(&entry.manifest).map_err(|source| {
            ExtensionWorldError::ResolutionIdentityEncoding {
                package: provider.id.to_string(),
                reason: source.to_string(),
            }
        })?;
        frame(&mut hasher, &manifest);
    }
    Ok(OrderedResolutionIdentity(hasher.finalize().into()))
}

fn ordered_resolution_hasher(rows: usize) -> Sha256 {
    const DOMAIN: &[u8] = b"vibe:extension-world:ordered-resolution:v1\0";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((rows as u64).to_be_bytes());
    hasher
}

fn empty_ordered_resolution_identity() -> OrderedResolutionIdentity {
    OrderedResolutionIdentity(ordered_resolution_hasher(0).finalize().into())
}

impl ExtensionWorldEpoch {
    /// Build the command-owned world from its exact ordered resolution.
    ///
    /// An empty slice is the explicit empty epoch. A nonempty slice is strict:
    /// every row must name one materialised slot, carry a package manifest
    /// agreeing with its resolved identity and retain a content hash.
    pub fn from_resolution(
        workspace_root: &Path,
        resolution: &[ResolvedDep],
    ) -> Result<Self, ExtensionWorldError> {
        let installed = resolution
            .iter()
            .map(|dep| resolved_package(workspace_root, dep))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_installed(installed)
    }

    /// The lawful empty installed world. Host declarations still enter a
    /// node-owner view supplied separately through [`Self::node_owner_view`].
    #[must_use]
    pub fn empty() -> Self {
        Self {
            installed: Vec::new(),
            index: BTreeMap::new(),
            resolution_identity: empty_ordered_resolution_identity(),
        }
    }

    pub(super) fn from_installed(
        installed: Vec<InstalledPackage>,
    ) -> Result<Self, ExtensionWorldError> {
        let mut index = BTreeMap::new();
        for (position, entry) in installed.iter().enumerate() {
            let id = entry.source.provider.id.clone();
            if index.insert(id.clone(), position).is_some() {
                return Err(ExtensionWorldError::DuplicatePackage {
                    package: id.to_string(),
                });
            }
        }
        let resolution_identity = ordered_resolution_identity(&installed)?;
        Ok(Self {
            installed,
            index,
            resolution_identity,
        })
    }

    #[must_use]
    pub(crate) const fn resolution_identity(&self) -> &super::OrderedResolutionIdentity {
        &self.resolution_identity
    }

    /// Every installed package's kernel row, in supplied resolution order.
    pub fn installed(&self) -> impl Iterator<Item = &DependencyExtensionSource> {
        self.installed.iter().map(|entry| &entry.source)
    }

    /// Every installed coordinate that may own a unit lane, in supplied
    /// resolution order.
    pub fn lane_owners(&self) -> impl Iterator<Item = &DependencyProviderId> {
        self.installed.iter().map(|entry| &entry.source.provider.id)
    }

    /// The parsed manifest retained for a package owner, including its exact
    /// `[mechanisms]` routes.
    pub fn package_manifest(
        &self,
        owner: &DependencyProviderId,
    ) -> Result<&Manifest, ExtensionWorldError> {
        Ok(&package(self, owner)?.manifest)
    }

    /// The materialised slot root retained for a package owner.
    pub fn package_root(&self, owner: &DependencyProviderId) -> Result<&Path, ExtensionWorldError> {
        Ok(&package(self, owner)?.source.provider.root)
    }

    /// Project a workspace root/member as the host of its own lane without
    /// reparsing any installed package.
    pub fn node_owner_view(
        &self,
        node_root: &Path,
        node_manifest: &Manifest,
    ) -> Result<ExtensionWorld, ExtensionWorldError> {
        let host = host_source(node_root, node_manifest)?;
        let owner = host.provider.identity.to_string();
        let roots = declared_edges(&owner, node_manifest)?;
        owner_view(
            self,
            host,
            &roots,
            None,
            active_stack(node_manifest).as_deref(),
        )
    }

    /// Package P's package-owned unit-lane view from the same epoch.
    pub fn package_owner_view(
        &self,
        owner: &DependencyProviderId,
    ) -> Result<ExtensionWorld, ExtensionWorldError> {
        let entry = package(self, owner)?;
        owner_view(
            self,
            lane_owner_host(&entry.source),
            &entry.edges,
            Some(owner),
            entry.active_stack.as_deref(),
        )
    }
}

/// Retain one supplied resolution row without reparsing its slot manifest.
fn resolved_package(
    workspace_root: &Path,
    dep: &ResolvedDep,
) -> Result<InstalledPackage, ExtensionWorldError> {
    let manifest = dep.manifest.clone();
    let declared =
        manifest
            .package
            .as_ref()
            .ok_or_else(|| ExtensionWorldError::SlotWithoutPackage {
                slot: resolved_slot_root(workspace_root, dep),
            })?;
    let root = resolved_slot_root(workspace_root, dep);
    let id = DependencyProviderId::new(
        dep.group.clone(),
        super::typed_name("resolved package name", &dep.name)?,
    );
    let resolved = format!("{}:{id}@{}", dep.kind, dep.version);
    if declared.group != dep.group
        || declared.name != dep.name
        || declared.version != dep.version
        || declared.kind != dep.kind
    {
        return Err(ExtensionWorldError::ResolutionIdentityMismatch {
            package: resolved,
            declared: format!(
                "{}:{}/{}@{}",
                declared.kind, declared.group, declared.name, declared.version
            ),
        });
    }
    if !root.is_dir() {
        return Err(ExtensionWorldError::MissingSlot {
            package: resolved,
            slot: root,
        });
    }
    let content_hash = dep.source_hash.clone().ok_or_else(|| {
        ExtensionWorldError::ResolutionWithoutContentHash {
            package: id.to_string(),
        }
    })?;

    Ok(InstalledPackage {
        edges: checked_edges(
            &id.to_string(),
            dep.requires
                .iter()
                .map(|(group, name)| (Some(group), name.as_str())),
            "resolved dependency name",
        )?,
        active_stack: active_stack(&manifest),
        source: DependencyExtensionSource {
            provider: DependencyProvider {
                id,
                root,
                version: dep.version.to_string(),
                kind: dep.kind,
                content_hash,
            },
            controls: manifest.extension_controls.clone(),
            declarations: manifest.extensions.clone(),
            mechanisms: manifest.mechanism_decls.clone(),
        },
        manifest,
    })
}

fn resolved_slot_root(workspace_root: &Path, dep: &ResolvedDep) -> PathBuf {
    let in_place = dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|package| package.materialization.is_in_place());
    if in_place {
        in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
    } else {
        slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
    }
}
