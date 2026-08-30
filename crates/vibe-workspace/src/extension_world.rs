//! The durable extension-world adapter — one lock-ordered snapshot of the
//! selected node's world, and the owner-scoped views the one kernel collector
//! runs over.
//!
//! This is the *durable* world epoch (R4 architecture §4.2): the post-install
//! state the install just wrote — the absolute root `vibe.lock`, the slot each
//! locked package materialised into, and the manifest each of those slots
//! carries. The snapshot is taken once per run: every participating manifest is
//! parsed exactly once, and dependency order comes from the root lock and from
//! nothing else. Neither a name sort nor a directory listing may become
//! ordering input — an orphan slot is outside the extension world entirely
//! (§3), which is why no reader here enumerates the dependency root.
//!
//! The adapter owns the WORLD, never the collection semantics. It hands the
//! kernel already-typed identities, retains every package's own
//! [`ExtensionsControl`](vibe_core::manifest::ExtensionsControl) inert beside
//! its declarations — and, from that same single parse, its `[[mechanism]]`
//! provider declarations, so the mechanism plane never becomes a second read of
//! the same file — and then projects ONE owner-scoped view per lane owner:
//! the selected node for the node lane, and package P — through the kernel's
//! own dependency-seat→owner-seat projection — for P's unit lane. Collection
//! itself always goes through the single kernel entry
//! ([`collect_owner_view`]); this crate never re-reads a manifest row the
//! world already carries and never grows a second collector.
//!
//! The PROVISIONAL epoch (`from_lock_and_resolution`, §4.1: the pre-install
//! resolution a slot hook observes) is deliberately absent. It is a named
//! follow-up belonging to the orchestrator's migration onto this adapter, not
//! a stub: writing one here before its caller exists would fix an epoch
//! contract with no command to state which lock value is its authority.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, Materialization};
use vibe_core::{PackageKind, PackageName};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionRegistry,
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, MechanismRegistry,
    SyntheticPresetSource, collect_extensions_with_presets, collect_mechanisms, lane_owner_host,
};

use crate::vibedeps::{in_place_slot_abs_path, slot_abs_path};

mod errors;

pub use errors::ExtensionWorldError;

/// One installed package as the durable world retains it.
///
/// The kernel row is what the collector consumes; the two fields beside it are
/// package-scoped facts owner scoping needs and
/// [`DependencyExtensionSource`] has no seat for — they never reach the
/// kernel, and they are read only when this package is itself the lane owner.
#[derive(Debug, Clone)]
struct InstalledPackage {
    source: DependencyExtensionSource,
    /// This package's own locked dependency edges, in the order the lock
    /// records them.
    edges: Vec<DependencyProviderId>,
    /// This package's own `[active].stack` short name, if it declares one.
    active_stack: Option<String>,
}

/// One durable, lock-ordered snapshot of a selected node's extension world.
///
/// Built once per lifecycle/install run and then projected per lane owner.
/// The snapshot itself is inert data: it parses manifests, it never collects
/// them.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone)]
pub struct DurableExtensionWorld {
    /// Every locked package, in ROOT LOCK ORDER and in no other.
    installed: Vec<InstalledPackage>,
    /// Coordinate → position in `installed`, so a closure walk never scans.
    index: BTreeMap<DependencyProviderId, usize>,
    host: HostExtensionSource,
    host_edges: Vec<DependencyProviderId>,
    host_active_stack: Option<String>,
}

impl DurableExtensionWorld {
    /// Snapshot the durable world of one selected node.
    ///
    /// `workspace_root` is the absolute root that owns `vibe.lock` and the
    /// dependency slots; `node_root` and `node_manifest` are the selected
    /// node's own directory and already-parsed manifest; `lock` is that
    /// absolute root lock. Every locked package is visited exactly once, in
    /// lock order, and its slot manifest parsed exactly once.
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use vibe_core::manifest::{Lockfile, Manifest};
    /// use vibe_workspace::extension_world::DurableExtensionWorld;
    ///
    /// # fn snapshot(
    /// #     workspace_root: &Path,
    /// #     node_root: &Path,
    /// #     node_manifest: &Manifest,
    /// #     lock: &Lockfile,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// // One snapshot per run…
    /// let world = DurableExtensionWorld::from_lock(
    ///     workspace_root,
    ///     node_root,
    ///     node_manifest,
    ///     lock,
    /// )?;
    /// // …then one owner-scoped view per lane owner, each collected through
    /// // the one kernel entry.
    /// let node_lane = world.node_owner_view()?;
    /// assert_eq!(node_lane.host.provider.root.as_path(), node_root);
    /// # Ok(())
    /// # }
    /// ```
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
    pub fn from_lock(
        workspace_root: &Path,
        node_root: &Path,
        node_manifest: &Manifest,
        lock: &Lockfile,
    ) -> Result<Self, ExtensionWorldError> {
        let host = host_source(node_root, node_manifest)?;
        let owner = host.provider.identity.to_string();
        let mut installed = Vec::with_capacity(lock.packages.len());
        let mut index = BTreeMap::new();
        // ROOT LOCK ORDER, and nothing else: no sort, no directory listing.
        for package in &lock.packages {
            let entry = installed_package(workspace_root, package)?;
            index.insert(entry.source.provider.id.clone(), installed.len());
            installed.push(entry);
        }
        Ok(Self {
            host_edges: declared_edges(&owner, node_manifest)?,
            host_active_stack: active_stack(node_manifest),
            host,
            installed,
            index,
        })
    }

    /// The selected node's own host source — declarations, controls and typed
    /// identity, exactly as authored.
    #[must_use]
    pub const fn host(&self) -> &HostExtensionSource {
        &self.host
    }

    /// Every installed package's kernel row, in ROOT LOCK ORDER.
    pub fn installed(&self) -> impl Iterator<Item = &DependencyExtensionSource> {
        self.installed.iter().map(|entry| &entry.source)
    }

    /// Every installed coordinate that may own a unit lane, in ROOT LOCK
    /// ORDER.
    pub fn lane_owners(&self) -> impl Iterator<Item = &DependencyProviderId> {
        self.installed.iter().map(|entry| &entry.source.provider.id)
    }

    /// The NODE lane's owner-scoped view: the selected node IS the host, so
    /// its declarations and its controls are the live ones, and every package
    /// in its closure sits in the installed vector with its own controls
    /// retained inert.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
    pub fn node_owner_view(&self) -> Result<ExtensionWorld, ExtensionWorldError> {
        self.owner_view(
            self.host.clone(),
            &self.host_edges,
            None,
            self.host_active_stack.as_deref(),
        )
    }

    /// Package P's unit-lane owner-scoped view.
    ///
    /// P takes the host seat through the kernel's own
    /// [`lane_owner_host`] projection, so P's retained controls become that
    /// lane's live controls; the installed vector is P's own dependency
    /// closure in root lock order, with P itself excluded (it is the host
    /// now, and one coordinate cannot occupy both seats); and the real node's
    /// controls are absent, because nothing of the node enters this value.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
    pub fn package_owner_view(
        &self,
        owner: &DependencyProviderId,
    ) -> Result<ExtensionWorld, ExtensionWorldError> {
        let entry = self
            .index
            .get(owner)
            .map(|position| &self.installed[*position])
            .ok_or_else(|| ExtensionWorldError::UnknownOwner {
                owner: owner.to_string(),
            })?;
        self.owner_view(
            lane_owner_host(&entry.source),
            &entry.edges,
            Some(owner),
            entry.active_stack.as_deref(),
        )
    }

    /// The one shape both owner views are: a host seat, that owner's closure
    /// in root lock order, and that owner's own effective stack.
    fn owner_view(
        &self,
        host: HostExtensionSource,
        roots: &[DependencyProviderId],
        exclude: Option<&DependencyProviderId>,
        active_stack: Option<&str>,
    ) -> Result<ExtensionWorld, ExtensionWorldError> {
        let owner = host.provider.identity.to_string();
        let installed = self.closure(&owner, roots, exclude)?;
        let effective_stack = effective_stack(&owner, active_stack, &installed)?;
        Ok(ExtensionWorld {
            installed,
            host,
            effective_stack,
        })
    }

    /// Every package reachable from `roots` through the lock's own dependency
    /// edges, projected back onto the snapshot in ROOT LOCK ORDER — the
    /// projection, not the walk, decides order, so reachability can never
    /// become an ordering input.
    fn closure(
        &self,
        owner: &str,
        roots: &[DependencyProviderId],
        exclude: Option<&DependencyProviderId>,
    ) -> Result<Vec<DependencyExtensionSource>, ExtensionWorldError> {
        let mut queue: VecDeque<DependencyProviderId> = roots.iter().cloned().collect();
        let mut reached = BTreeSet::new();
        while let Some(id) = queue.pop_front() {
            let Some(position) = self.index.get(&id).copied() else {
                return Err(ExtensionWorldError::UnlockedRequirement {
                    owner: owner.to_owned(),
                    requirement: id.to_string(),
                });
            };
            if !reached.insert(id) {
                continue;
            }
            queue.extend(self.installed[position].edges.iter().cloned());
        }
        Ok(self
            .installed
            .iter()
            .map(|entry| &entry.source)
            .filter(|source| {
                reached.contains(&source.provider.id) && Some(&source.provider.id) != exclude
            })
            .cloned()
            .collect())
    }
}

/// Collect one owner-scoped view through the ONE kernel entry.
///
/// The thin wrapper §5 of the R4 architecture allows: it adds no rule, reads
/// no manifest, and exists so every workspace-side collection is spelled once
/// and is findable by name. There is no second collector to find.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn collect_owner_view(
    view: ExtensionWorld,
    presets: Vec<SyntheticPresetSource>,
) -> Result<ExtensionRegistry, ExtensionWorldError> {
    Ok(collect_extensions_with_presets(view, presets)?)
}

/// Collect one owner-scoped view's MECHANISM plane through the ONE kernel
/// entry.
///
/// The sibling of [`collect_owner_view`], and a wrapper for the same reason:
/// every workspace-side collection is spelled once and is findable by name.
/// It borrows the view, so a caller that wants both planes of one snapshot
/// collects mechanisms first and then hands the same value to
/// [`collect_owner_view`] — one world, two registries, no second parse.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn collect_owner_mechanisms(
    view: &ExtensionWorld,
) -> Result<MechanismRegistry, ExtensionWorldError> {
    Ok(collect_mechanisms(view)?)
}

/// Parse one locked package's slot into the world's retained row.
fn installed_package(
    workspace_root: &Path,
    package: &LockedPackage,
) -> Result<InstalledPackage, ExtensionWorldError> {
    let root = slot_root(workspace_root, package);
    let id = DependencyProviderId::new(package.group.clone(), package.name.clone());
    let locked = format!("{}:{id}@{}", package.kind, package.version);
    if !root.is_dir() {
        return Err(ExtensionWorldError::MissingSlot {
            package: locked,
            slot: root,
        });
    }
    let manifest_path = root.join(Manifest::FILENAME);
    let manifest =
        Manifest::read(&manifest_path).map_err(|source| ExtensionWorldError::UnreadableSlot {
            manifest: manifest_path,
            source: Box::new(source),
        })?;
    let declared = manifest
        .package
        .as_ref()
        .ok_or_else(|| ExtensionWorldError::SlotWithoutPackage { slot: root.clone() })?;
    if declared.group != package.group
        || declared.name != package.name
        || declared.version != package.version
        || declared.kind != package.kind
    {
        return Err(ExtensionWorldError::SlotIdentityMismatch {
            slot: root,
            declared: format!(
                "{}:{}/{}@{}",
                declared.kind, declared.group, declared.name, declared.version
            ),
            locked,
        });
    }

    Ok(InstalledPackage {
        edges: declared_edges(&id.to_string(), &manifest)?,
        active_stack: active_stack(&manifest),
        source: DependencyExtensionSource {
            provider: DependencyProvider {
                id,
                root,
                version: package.version.to_string(),
                kind: package.kind,
                content_hash: package.content_hash.clone(),
            },
            // The package's own consumer controls, retained verbatim. They
            // are inert in every other owner's view and become live exactly
            // when this package takes its own lane's host seat.
            controls: manifest.extension_controls,
            declarations: manifest.extensions,
            // The mechanism plane rides the SAME parse: `[[mechanism]]` is a
            // sibling table of `[[extension]]` in the manifest this snapshot
            // already read, so carrying it costs no second read and can never
            // observe a different epoch.
            mechanisms: manifest.mechanism_decls,
        },
    })
}

/// The absolute slot a locked package materialised into, by the exact
/// representation the lock recorded for it.
fn slot_root(workspace_root: &Path, package: &LockedPackage) -> PathBuf {
    match package.materialization {
        Materialization::InPlace => {
            in_place_slot_abs_path(workspace_root, &package.group, &package.name)
        }
        Materialization::Copy | Materialization::Hardlink => slot_abs_path(
            workspace_root,
            &package.group,
            &package.name,
            &package.version,
        ),
    }
}

/// One manifest's declared package requirements as typed coordinates, in
/// authored order. The name arrives as a bare string, so it is parsed here
/// through the one existing grammar and refused typed on failure.
fn declared_edges(
    owner: &str,
    manifest: &Manifest,
) -> Result<Vec<DependencyProviderId>, ExtensionWorldError> {
    manifest
        .requires
        .iter_pkgrefs()
        .map(|(group, name)| {
            let group = group
                .cloned()
                .ok_or_else(|| ExtensionWorldError::UngroupedEdge {
                    owner: owner.to_owned(),
                    edge: name.to_owned(),
                })?;
            Ok(DependencyProviderId::new(
                group,
                typed_name("[requires.packages] name", name)?,
            ))
        })
        .collect()
}

/// One manifest's `[active].stack` short name, if it declares one.
fn active_stack(manifest: &Manifest) -> Option<String> {
    manifest
        .active
        .as_ref()
        .and_then(|active| active.stack.clone())
}

/// Resolve one owner's `[active].stack` against that owner's own closure.
///
/// The same rule for every lane owner: a node's active stack is resolved in
/// the node's closure, a package's in the package's. Silence is legal;
/// naming a stack that is not there, or naming two, is not.
fn effective_stack(
    owner: &str,
    short_name: Option<&str>,
    installed: &[DependencyExtensionSource],
) -> Result<Option<DependencyProviderId>, ExtensionWorldError> {
    let Some(short_name) = short_name else {
        return Ok(None);
    };
    let candidates: Vec<_> = installed
        .iter()
        .filter(|source| {
            source.provider.kind == PackageKind::Stack
                && source.provider.id.name().as_str() == short_name
        })
        .map(|source| source.provider.id.clone())
        .collect();
    match candidates.as_slice() {
        [id] => Ok(Some(id.clone())),
        [] => Err(ExtensionWorldError::UnresolvedActiveStack {
            owner: owner.to_owned(),
            stack: short_name.to_owned(),
        }),
        many => Err(ExtensionWorldError::AmbiguousActiveStack {
            owner: owner.to_owned(),
            stack: short_name.to_owned(),
            candidates: many
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

/// The selected node's own host source, with its identity typed at this seam.
fn host_source(
    node_root: &Path,
    manifest: &Manifest,
) -> Result<HostExtensionSource, ExtensionWorldError> {
    let (identity, version, kind) = if let Some(package) = &manifest.package {
        (
            HostIdentity::coordinate(DependencyProviderId::new(
                package.group.clone(),
                typed_name("[package].name", &package.name)?,
            )),
            package.version.to_string(),
            Some(package.kind),
        )
    } else if let Some(project) = &manifest.project {
        let identity = match &project.group {
            Some(group) => HostIdentity::coordinate(DependencyProviderId::new(
                group.clone(),
                typed_name("[project].name", &project.name)?,
            )),
            // An ungrouped project declares no coordinate: its authored name
            // reaches the kernel through the shared host-owner codec, not
            // through the package-name grammar.
            None => HostIdentity::ungrouped_project(project.name.clone()),
        };
        (identity, project.version.clone(), None)
    } else {
        (HostIdentity::virtual_workspace(), String::new(), None)
    };

    Ok(HostExtensionSource {
        provider: HostProvider {
            identity,
            root: node_root.to_path_buf(),
            version,
            kind,
            content_hash: None,
        },
        declarations: manifest.extensions.clone(),
        controls: manifest.extension_controls.clone(),
        // The selected node's own providers, from the same parse. Its
        // `[mechanisms]` ROUTES stay on the manifest: a route is an argument
        // to selection, not a property of the world.
        mechanisms: manifest.mechanism_decls.clone(),
    })
}

/// Parse one bare-string component through `PackageName`'s own grammar,
/// refusing typed and by component name on failure.
fn typed_name(component: &'static str, spelling: &str) -> Result<PackageName, ExtensionWorldError> {
    PackageName::parse(spelling).map_err(|error| ExtensionWorldError::UntypedComponent {
        component,
        spelling: spelling.to_owned(),
        reason: error.to_string(),
    })
}

#[cfg(test)]
#[path = "extension_world/test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "extension_world/cycle_tests.rs"]
mod cycle_tests;

#[cfg(test)]
#[path = "extension_world/mechanism_tests.rs"]
mod mechanism_tests;

#[cfg(test)]
#[path = "extension_world/tests.rs"]
mod tests;
