//! Selective publish of a workspace's self-publishing members.
//!
//! Spec: [PROP-007 §2.7–§2.9](../../../spec/modules/vibe-workspace/PROP-007-workspace.md#selective-publish).
//!
//! `vibe workspace publish` walks the publishable nodes of a workspace in
//! dependency-first order and publishes each as its own repository, reusing
//! the per-package machinery in `vibe-publish`. This module owns the parts
//! that are pure and hermetically testable — *which* nodes publish, in *what*
//! order, and *what* a staged copy looks like — leaving the actual
//! repo-creation + push to the CLI (which threads `vibe-publish::Publisher`).
//!
//! Three concerns live here:
//!
//! - [`select_publishable_nodes`] — every node carrying `[package]` whose
//!   [`PublishPosture`] does not exclude it from the primary registry.
//! - [`topo_order`] — a stable dependency-first ordering of the selected
//!   nodes, derived from inter-member `path` dependencies. A cycle is a hard
//!   error per PROP-007 §2.7.
//! - [`stage_node`] — copy a node's directory into a fresh temp dir,
//!   excluding `.git/` and `.vibe/`, inject the `[origin]` provenance marker
//!   into the staged `vibe.toml`, prepend the "generated copy" README banner,
//!   write `.github/PULL_REQUEST_TEMPLATE.md`, and regenerate the boot
//!   artifacts for the published shape (PROP-009 §2.11) so they never
//!   dangle on the dev tree's workspace `vibedeps/` slots.
//!
//! Token discipline is not a concern of this module — no token, push URL, or
//! credential ever reaches it. That machinery is `vibe-publish`'s, reused
//! unchanged by the CLI.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish");

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageKind};

use crate::{Workspace, WorkspaceError};

type Result<T> = std::result::Result<T, WorkspaceError>;

mod staging;

pub use staging::{
    OriginInfo, StagedNode, generated_copy_description, generated_copy_readme_banner,
    pull_request_template, stage_node,
};

/// One node of a workspace selected for publishing — the root (when it
/// carries `[package]`) or a member.
///
/// `rel_path` is `"."` for the absolute root and the member's root-relative
/// forward-slashed path otherwise. The `(group, name)` pair is the node's
/// package identity, lifted out of its `[package]` table so the topological
/// sort and reporting code do not have to re-unwrap it; `kind` rides along
/// as metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishNode {
    /// `"."` for the absolute root, the member `rel_path` otherwise.
    pub rel_path: String,
    /// Package kind from the node's `[package].kind` — metadata only, not
    /// part of identity (PROP-008 §2.3).
    pub kind: PackageKind,
    /// Reverse-FQDN group from the node's `[package].group` — with `name`,
    /// the `(group, name)` identity.
    pub group: Group,
    /// Package name from the node's `[package].name`.
    pub name: String,
}

impl PublishNode {
    /// `<group>/<name>` — the human-facing pkgref for progress lines / JSON.
    pub fn pkgref(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }
}

/// A node skipped during selection, with the reason — surfaced to the
/// operator so a `publish = false` member never silently disappears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedNode {
    /// `"."` for the absolute root, the member `rel_path` otherwise.
    pub rel_path: String,
    /// Human-readable reason — e.g. `publish = false` or
    /// `publish posture excludes registry "vibespecs"`.
    pub reason: String,
}

/// Outcome of [`select_publishable_nodes`]: the publishable nodes plus the
/// `[package]`-carrying nodes deliberately skipped by their posture.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Nodes that will be published (before topological ordering).
    pub publishable: Vec<PublishNode>,
    /// `[package]` nodes excluded by their `publish` posture. Nodes with no
    /// `[package]` table at all are not packages and are not reported here.
    pub skipped: Vec<SkippedNode>,
}

/// Select the publishable nodes of `workspace`, honouring each node's
/// [`PublishPosture`] against `primary_registry`.
///
/// A node is publishable iff it carries `[package]` and its posture is not
/// `is_never()` and `includes(primary_registry)` (the latter narrows the
/// `publish = ["..."]` list form — PROP-007 §2.7).
///
/// `only_member` narrows the selection to a single node by `rel_path`
/// (`"."` selects the root). When it names a node that exists but whose
/// posture excludes it, that exclusion is still reported in `skipped` — the
/// operator asked for a node that will not publish and deserves to be told.
/// When it names no node at all, [`WorkspaceError::MemberNotFound`] is
/// raised so a typo'd `--member` fails loudly rather than silently
/// publishing nothing.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
pub fn select_publishable_nodes(
    workspace: &Workspace,
    primary_registry: &str,
    only_member: Option<&str>,
) -> Result<Selection> {
    // If `--member` was given, confirm the node exists at all (root or a
    // member). A non-existent rel_path is an operator error.
    if let Some(target) = only_member {
        let exists = target == "." || workspace.members.iter().any(|m| m.rel_path == target);
        if !exists {
            return Err(WorkspaceError::MemberNotFound {
                pattern: target.to_string(),
                declared_in: "the --member argument".to_string(),
            });
        }
    }

    let mut selection = Selection::default();
    for (rel_path, manifest) in workspace.iter_nodes() {
        if let Some(target) = only_member
            && target != rel_path
        {
            continue;
        }
        let Some(meta) = &manifest.package else {
            // Not a publishable package — `[project]`-only or a virtual
            // workspace coordinator. Not an error, not reported.
            continue;
        };
        if meta.publish.is_never() {
            selection.skipped.push(SkippedNode {
                rel_path: rel_path.to_string(),
                reason: "publish = false (workspace-internal)".to_string(),
            });
            continue;
        }
        if !meta.publish.includes(primary_registry) {
            selection.skipped.push(SkippedNode {
                rel_path: rel_path.to_string(),
                reason: format!("publish posture excludes registry `{primary_registry}`"),
            });
            continue;
        }
        selection.publishable.push(PublishNode {
            rel_path: rel_path.to_string(),
            kind: meta.kind,
            group: meta.group.clone(),
            name: meta.name.clone(),
        });
    }
    Ok(selection)
}

/// Order `nodes` dependency-first.
///
/// Node M depends on node N when M's `[requires].path_packages` contains a
/// `path` that resolves — relative to M's directory — to N's directory.
/// Dependencies are published before their dependents (N before M); nodes
/// with no inter-member path dependency keep a stable `rel_path`-sorted
/// order. A cycle among the selected nodes is [`WorkspaceError::NestingCycle`]
/// — distributed publishing has no way to break one (PROP-007 §2.7).
///
/// `workspace` supplies the directory layout used to resolve each `path`
/// dependency back to a node `rel_path`. A `path` dependency that resolves
/// outside the selected set (an external sibling, a non-publishing member)
/// is simply not an edge — it imposes no ordering constraint here.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
pub fn topo_order(workspace: &Workspace, nodes: &[PublishNode]) -> Result<Vec<PublishNode>> {
    // Index the selected nodes by rel_path for O(1) membership tests, and
    // pre-sort by rel_path so the output is deterministic when the graph
    // imposes no constraint (Kahn's algorithm drains ready nodes in
    // insertion order).
    let mut sorted: Vec<&PublishNode> = nodes.iter().collect();
    sorted.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let selected: HashSet<&str> = sorted.iter().map(|n| n.rel_path.as_str()).collect();

    // Map an absolute directory back to a selected node's rel_path. Built
    // once; consulted per path-dependency.
    let mut dir_to_rel: HashMap<PathBuf, String> = HashMap::new();
    for node in &sorted {
        dir_to_rel.insert(
            node_abs_dir(workspace, &node.rel_path)?,
            node.rel_path.clone(),
        );
    }

    // Build the dependency edges: deps[m] = set of rel_paths m depends on.
    // Only edges between two selected nodes count.
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &sorted {
        deps.entry(node.rel_path.as_str()).or_default();
    }
    for node in &sorted {
        let manifest = node_manifest(workspace, &node.rel_path)?;
        let node_dir = node_abs_dir(workspace, &node.rel_path)?;
        for pd in &manifest.requires.path_packages {
            // Resolve the path-dep directory relative to the depending
            // node's directory, then normalise it (`..` / `.` segments).
            let dep_dir = normalise(&node_dir.join(rel_to_native(&pd.path)));
            let Some(dep_rel) = dir_to_rel.get(&dep_dir) else {
                continue; // not a selected node — imposes no ordering
            };
            if dep_rel.as_str() == node.rel_path.as_str() {
                continue; // self-edge — ignore
            }
            if !selected.contains(dep_rel.as_str()) {
                continue;
            }
            let m = node.rel_path.as_str();
            // De-dup edges — a node may path-dep the same member twice.
            let edge_list = deps.entry(m).or_default();
            if !edge_list.contains(&dep_rel.as_str()) {
                edge_list.push(dep_rel.as_str());
            }
        }
    }

    // Topological sort by repeated ready-node draining. A node is ready
    // once every dependency it has is already emitted; ties are broken by
    // rel_path order (`sorted` is rel_path-sorted, so the linear scan
    // emits the lexicographically-first ready node first — stable output
    // without a heap).
    let mut order: Vec<PublishNode> = Vec::with_capacity(sorted.len());
    let mut done: HashSet<&str> = HashSet::new();
    while order.len() < sorted.len() {
        let mut progressed = false;
        for node in &sorted {
            let rel = node.rel_path.as_str();
            if done.contains(rel) {
                continue;
            }
            // Ready iff every dependency is already emitted.
            let ready = deps
                .get(rel)
                .map(|ds| ds.iter().all(|d| done.contains(d)))
                .unwrap_or(true);
            if ready {
                order.push((*node).clone());
                done.insert(rel);
                progressed = true;
            }
        }
        if !progressed {
            // Remaining nodes form at least one cycle. Name one for the
            // error — the first node still not done.
            let stuck = sorted
                .iter()
                .find(|n| !done.contains(n.rel_path.as_str()))
                .map(|n| n.rel_path.clone())
                .unwrap_or_else(|| "<unknown>".to_string());
            return Err(WorkspaceError::NestingCycle { path: stuck });
        }
    }
    Ok(order)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// The manifest of a node by its `rel_path` (`"."` = the root).
fn node_manifest<'a>(workspace: &'a Workspace, rel_path: &str) -> Result<&'a Manifest> {
    if rel_path == "." {
        Ok(&workspace.root_manifest)
    } else {
        workspace
            .member_by_rel_path(rel_path)
            .map(|m| &m.manifest)
            .ok_or_else(|| WorkspaceError::UnknownPublishNode {
                rel_path: rel_path.to_string(),
            })
    }
}

/// The absolute directory of a node by its `rel_path` (`"."` = the root).
fn node_abs_dir(workspace: &Workspace, rel_path: &str) -> Result<PathBuf> {
    if rel_path == "." {
        Ok(workspace.root.clone())
    } else {
        let member = workspace.member_by_rel_path(rel_path).ok_or_else(|| {
            WorkspaceError::UnknownPublishNode {
                rel_path: rel_path.to_string(),
            }
        })?;
        Ok(workspace.member_abs_path(member))
    }
}

/// Turn a forward-slashed relative path into a native `PathBuf` (segment by
/// segment so Windows gets backslashes and `..` survives as a component).
fn rel_to_native(rel: &str) -> PathBuf {
    let mut p = PathBuf::new();
    for segment in rel.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        p.push(segment);
    }
    p
}

/// Lexically normalise a path — collapse `.` and resolve `..` against the
/// preceding component. No filesystem access (the directories may not all
/// exist yet under a `--dry-run`); a purely textual normalisation is enough
/// to compare a path-dep target against a node's directory.
fn normalise(path: &Path) -> PathBuf {
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    for comp in path.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                // Pop a normal component; keep `..` if there is nothing to
                // pop (path escapes its base — rare, but do not corrupt it).
                if matches!(out.last().map(|s| s.as_os_str()), Some(s) if s != "..") {
                    out.pop();
                } else {
                    out.push(std::ffi::OsString::from(".."));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                out.push(comp.as_os_str().to_os_string());
            }
            Component::Normal(s) => out.push(s.to_os_string()),
        }
    }
    let mut p = PathBuf::new();
    for seg in out {
        p.push(seg);
    }
    p
}

#[cfg(test)]
#[path = "publish/tests.rs"]
mod tests;
