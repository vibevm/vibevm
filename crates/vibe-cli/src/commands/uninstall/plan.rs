//! Offline, identity-keyed uninstall graph planning.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#UNINSTALL-CLOSURE");

use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest};
use vibe_core::{Group, PackageRef};
use vibe_workspace::{Workspace, vibedeps};

pub(super) type Key = (Group, String);
pub(super) struct Plan {
    pub removed: Vec<LockedPackage>,
    pub removed_count: u32,
    pub roots: Vec<PackageRef>,
    pub manifest_changed: bool,
}
pub(super) fn key(group: &Group, name: &str) -> Key {
    (group.clone(), name.to_owned())
}
fn ref_key(p: &PackageRef) -> Result<Key> {
    let group = p
        .group
        .as_ref()
        .context("uninstall requires qualified lock graph identities")?;
    Ok(key(group, &p.name))
}
fn closure(seeds: impl IntoIterator<Item = Key>, edges: &BTreeMap<Key, Vec<Key>>) -> BTreeSet<Key> {
    let mut seen = BTreeSet::new();
    let mut pending: Vec<_> = seeds.into_iter().collect();
    while let Some(id) = pending.pop() {
        if seen.insert(id.clone()) {
            pending.extend(edges[&id].iter().cloned());
        }
    }
    seen
}
pub(super) fn build(
    workspace: &Workspace,
    project: &Path,
    manifest: &mut Manifest,
    lock: &Lockfile,
    group: &Group,
    name: &str,
) -> Result<Plan> {
    let selected = key(group, name);
    let mut rows = BTreeMap::new();
    for row in &lock.packages {
        if rows.insert(key(&row.group, &row.name), row).is_some() {
            bail!(
                "duplicate locked package identity `{}/{}`",
                row.group,
                row.name
            );
        }
    }
    let mut edges = BTreeMap::new();
    for (id, row) in &rows {
        let mut seen = BTreeSet::new();
        let mut children = Vec::new();
        for dep in &row.dependencies {
            let child = ref_key(dep)?;
            let target = rows
                .get(&child)
                .with_context(|| format!("locked dependency `{dep}` has no package row"))?;
            if !dep.version.is_exact_pin() || !dep.version.matches(&target.version) {
                bail!(
                    "locked dependency `{dep}` must pin its package row's exact version `{}`",
                    target.version
                );
            }
            if !seen.insert(child.clone()) {
                bail!("duplicate locked dependency `{dep}`");
            }
            children.push(child);
        }
        edges.insert(id.clone(), children);
    }
    let mut recorded = BTreeMap::new();
    for root in &lock.meta.root_dependencies {
        let id = ref_key(root)?;
        let row = rows
            .get(&id)
            .with_context(|| format!("recorded root `{root}` has no locked package row"))?;
        if !root.version.matches(&row.version) {
            bail!("recorded root `{root}` disagrees with its locked version");
        }
        if recorded.insert(id, root.clone()).is_some() {
            bail!("duplicate recorded root `{root}`");
        }
    }
    let declared = manifest
        .requires
        .iter_pkgrefs()
        .any(|(g, n)| g == Some(group) && n == name);
    let legacy = workspace.is_standalone()
        && manifest.requires.is_empty()
        && recorded.contains_key(&selected);
    if !declared && !legacy {
        bail!(
            "`{group}/{name}` is a pure transitive dependency in this node; uninstall a directly declared root instead"
        );
    }
    for (node, data) in workspace.iter_nodes() {
        if !data.requires.capabilities.is_empty()
            || !data.requires_any.is_empty()
            || !data.conditional_deps.is_empty()
        {
            bail!(
                "cannot safely uninstall: workspace node `{node}` has capability, disjunctive, or conditional requirements whose provider roots are not recorded"
            );
        }
    }
    let manifest_changed = drop_requires(manifest, group, name);
    let mut roots = BTreeSet::new();
    for (node, data) in workspace.iter_nodes() {
        let absolute = if node == "." {
            workspace.root.clone()
        } else {
            workspace.root.join(node)
        };
        let data = if absolute == project {
            &*manifest
        } else {
            data
        };
        for (g, n) in data.requires.iter_pkgrefs() {
            let id = key(g.context("workspace requires must be qualified")?, n);
            if !rows.contains_key(&id) {
                bail!(
                    "workspace root `{}/{}` has no locked package row",
                    id.0,
                    id.1
                );
            }
            roots.insert(id);
        }
    }
    if legacy {
        roots.extend(recorded.keys().filter(|id| **id != selected).cloned());
    }
    let candidate = closure([selected], &edges);
    // Keep unrelated historical rows and everything they still need. This is
    // root-local pruning, never an opportunistic orphan garbage collection.
    let seeds = roots
        .iter()
        .cloned()
        .chain(rows.keys().filter(|id| !candidate.contains(*id)).cloned());
    let retained = closure(seeds, &edges);
    let removed: Vec<_> = candidate
        .difference(&retained)
        .map(|id| (*rows[id]).clone())
        .collect();
    let removed_count =
        u32::try_from(removed.len()).context("uninstall removal count exceeds u32")?;
    let roots = roots
        .into_iter()
        .map(|id| {
            recorded
                .get(&id)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| rows[&id].as_package_ref().map_err(Into::into))
        })
        .collect::<Result<_>>()?;
    Ok(Plan {
        removed,
        removed_count,
        roots,
        manifest_changed,
    })
}
pub(super) fn slot(row: &LockedPackage) -> String {
    if row.materialization.is_in_place() {
        vibedeps::in_place_slot_rel_path(&row.group, &row.name)
    } else {
        vibedeps::slot_rel_path(&row.group, &row.name, &row.version)
    }
}
fn drop_requires(manifest: &mut Manifest, group: &Group, name: &str) -> bool {
    let before = manifest.requires.clone();
    let r = &mut manifest.requires;
    r.packages
        .retain(|p| !(p.group.as_ref() == Some(group) && p.name == name));
    r.git_packages
        .retain(|p| !(p.group == *group && p.name == name));
    r.path_packages
        .retain(|p| !(p.group == *group && p.name == name));
    r.var_packages
        .retain(|p| !(p.group == *group && p.name == name));
    let id = format!("{group}/{name}");
    r.links.remove(&id);
    r.accesses.remove(&id);
    r.friend_flags.remove(&id);
    r.excludes.remove(&id);
    r.optional.remove(&id);
    r.configurations.remove(&id);
    *r != before
}
