//! The installed-world capability: assemble the visibility graph of a
//! materialised project and explain it (PROP-050 §7 — `vibe why` /
//! `vibe friends`).
//!
//! The engine in the sibling modules is pure: it answers questions about a
//! [`VisibilityGraph`] it is handed. This module is the bridge from disk —
//! the root manifest, the lockfile, and each member's slot manifest — to
//! that pure input, plus the two explanatory queries the observability
//! facts (##VIBE-WHY, ##ALLOW-FRIENDS-EXHAUSTIVE) ask for.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use std::path::Path;

use crate::manifest::{AccessLevel, LockedPackage, Lockfile, Manifest, OverrideTarget};

use super::{
    Analysis, Diagnostic, EdgeDecl, NodeDecl, NodeId, Provenance, ProvenanceRule, VisibilityGraph,
    analyze,
};

/// The visibility graph of an INSTALLED world: the root manifest, the
/// lockfile members, and each member's manifest from its vibedeps slot
/// `vibedeps/<group>.<name>/<version>/vibe.toml` (the canonical slot shape —
/// vibe-workspace's vibedeps module is the authority; the formula is repeated
/// here to keep vibe-core dependency-free).
///
/// ```
/// use vibe_core::visibility::{EdgeDecl, InstalledWorld, NodeDecl, VisibilityGraph};
///
/// let mut graph = VisibilityGraph::default();
/// graph.nodes.insert("demo".into(), NodeDecl {
///     edges: vec![EdgeDecl { to: "org.x/api".into(), ..EdgeDecl::default() }],
///     ..NodeDecl::default()
/// });
/// let world = InstalledWorld { root_id: "demo".into(), graph, unread: Vec::new() };
/// assert_eq!(world.root_id, "demo");
/// let demo = world.graph.nodes.get("demo").unwrap();
/// assert_eq!(demo.edges[0].to, "org.x/api");
/// ```
pub struct InstalledWorld {
    pub root_id: NodeId,
    pub graph: VisibilityGraph,
    /// Members whose slot manifest was missing/unreadable — they become
    /// empty NodeDecls and are reported, never a hard error.
    pub unread: Vec<NodeId>,
}

/// Read the installed world of the project rooted at `project_root`: the
/// root `vibe.toml`, `vibe.lock`, and every member manifest reachable in
/// its vibedeps slot. A missing root manifest or lockfile is an honest
/// [`Err`]; a member whose slot manifest is missing or unreadable becomes
/// an empty node listed in [`InstalledWorld::unread`] instead.
///
/// ```
/// use std::fs;
/// use vibe_core::visibility::load_installed_world;
///
/// let root = tempfile::tempdir().unwrap();
/// fs::write(root.path().join("vibe.toml"), r#"
/// [project]
/// name = "demo"
/// version = "0.0.1"
/// "#).unwrap();
/// // No vibe.lock yet — the installed world cannot start.
/// assert!(load_installed_world(root.path()).is_err());
///
/// fs::write(root.path().join("vibe.lock"), format!(r#"
/// [meta]
/// generated_by = "vibe-test"
/// generated_at = "2026-08-23T00:00:00Z"
/// schema_version = {}
/// "#, vibe_core::manifest::CURRENT_SCHEMA_VERSION)).unwrap();
/// let world = load_installed_world(root.path()).unwrap();
/// assert_eq!(world.root_id, "demo");
/// assert!(world.unread.is_empty());
/// ```
pub fn load_installed_world(project_root: &Path) -> Result<InstalledWorld, String> {
    let manifest_path = project_root.join(Manifest::FILENAME);
    let root_manifest = Manifest::read(&manifest_path)
        .map_err(|error| format!("reading `{}`: {error}", manifest_path.display()))?;
    let Some(consumer) = root_manifest.consumer_node() else {
        return Err(format!(
            "root manifest `{}` declares no `[project]`/`[package]` consumer section",
            manifest_path.display()
        ));
    };
    let root_id = consumer.coordinate();
    let root_decl =
        node_decl_for(&root_manifest).map_err(|reason| format!("root `{root_id}`: {reason}"))?;
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(root_id.clone(), root_decl);

    let lock_path = project_root.join(Lockfile::FILENAME);
    if !lock_path.is_file() {
        return Err(format!(
            "no `{}` in `{}` — the installed world starts at a lock; run `vibe install`",
            Lockfile::FILENAME,
            project_root.display()
        ));
    }
    let lock = Lockfile::read(&lock_path)
        .map_err(|error| format!("reading `{}`: {error}", lock_path.display()))?;

    let mut unread = Vec::new();
    for member in &lock.packages {
        let node_id: NodeId = format!("{}/{}", member.group, member.name);
        let slot = slot_manifest_path(project_root, member);
        match Manifest::read(&slot) {
            Ok(manifest) => {
                let decl =
                    node_decl_for(&manifest).map_err(|reason| format!("`{node_id}`: {reason}"))?;
                graph.nodes.insert(node_id, decl);
            }
            Err(_) => {
                // A member without a readable slot manifest is reported,
                // never fatal: its edges are simply absent from the world.
                graph.nodes.insert(node_id.clone(), NodeDecl::default());
                unread.push(node_id);
            }
        }
    }
    Ok(InstalledWorld {
        root_id,
        graph,
        unread,
    })
}

/// The member's slot manifest: the canonical versioned slot
/// `vibedeps/<group>.<name>/<version>/vibe.toml`, or the unversioned
/// `vibedeps/<group>.<name>/vibe.toml` for an `in-place` member whose
/// working tree carries no version directory (PROP-022 §2.4).
fn slot_manifest_path(project_root: &Path, member: &LockedPackage) -> std::path::PathBuf {
    let slot = project_root
        .join("vibedeps")
        .join(format!("{}.{}", member.group, member.name));
    if member.materialization.is_in_place() {
        slot.join(Manifest::FILENAME)
    } else {
        slot.join(member.version.to_string())
            .join(Manifest::FILENAME)
    }
}

/// Translate one parsed manifest into its pure graph declaration. Edge
/// fields are the RAW declared values — the engine applies the public /
/// `friend = false` defaults and the friends-only implication (F10).
fn node_decl_for(manifest: &Manifest) -> Result<NodeDecl, String> {
    let mut decl = NodeDecl::default();
    for (group, name) in manifest.requires.iter_pkgrefs() {
        // A `[requires]` entry without a group has no `<group>/<name>`
        // coordinate; a validated manifest never produces one, so the
        // branch is defensive only.
        let Some(group) = group else { continue };
        decl.edges.push(EdgeDecl {
            to: format!("{group}/{name}"),
            access: manifest.requires.declared_access(group, name),
            friend: manifest.requires.declared_friend(group, name),
            exclude: manifest.requires.excludes_for(group, name).to_vec(),
        });
    }
    if let Some(visibility) = &manifest.visibility {
        decl.friends = visibility.friends.clone();
        decl.unfriend = visibility.unfriend.clone();
        decl.allow_friends = visibility.allow_friends.clone();
    }
    if let Some(table) = &manifest.override_table {
        decl.overrides = table
            .targets()?
            .into_iter()
            .map(|(target, entry)| (target, entry.clone()))
            .collect();
    }
    Ok(decl)
}

/// Why is `target` in (or out of) the root's effective world.
///
/// ```
/// use vibe_core::visibility::{EdgeDecl, InstalledWorld, NodeDecl, VisibilityGraph, why, WhyVerdict};
///
/// let mut graph = VisibilityGraph::default();
/// graph.nodes.insert("demo".into(), NodeDecl {
///     edges: vec![EdgeDecl {
///         to: "org.x/wal".into(),
///         access: Some(vibe_core::manifest::AccessLevel::Private),
///         ..EdgeDecl::default()
///     }],
///     ..NodeDecl::default()
/// });
/// graph.nodes.insert("org.x/wal".into(), NodeDecl::default());
/// let world = InstalledWorld { root_id: "demo".into(), graph, unread: Vec::new() };
/// // The root's own edge always traverses (rule 1) — even a private one.
/// assert!(matches!(why(&world, "org.x/wal"), WhyVerdict::Present(_)));
/// assert!(matches!(why(&world, "org.nope/ghost"), WhyVerdict::UnknownCoordinate));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhyVerdict {
    Present(Provenance),
    Absent { blocked: Vec<BlockedEdge> },
    UnknownCoordinate,
}

/// One declared edge toward an absent target and the best explanation of
/// why it did not open the target for this root.
///
/// ```
/// use vibe_core::visibility::{BlockReason, BlockedEdge};
///
/// let blocked = BlockedEdge {
///     from: "org.x/redbook".into(),
///     reason: BlockReason::Private,
/// };
/// assert_eq!(blocked.from, "org.x/redbook");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedEdge {
    pub from: NodeId,
    pub reason: BlockReason,
}

/// The best per-edge explanation for an absent target (PROP-050
/// ##VIBE-WHY): a classification, not an exhaustive theory — aimed at the
/// usefulness of the printed answer.
///
/// ```
/// use vibe_core::visibility::BlockReason;
///
/// assert_ne!(BlockReason::Private, BlockReason::Excluded);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockReason {
    /// The edge is `private` and its declarant is not the consumer root —
    /// a dev-world edge that seeps nowhere (rule 1 fails).
    Private,
    /// The edge is `friends-only` and its declarant never entered the
    /// root's friend closure (rule 3 fails); also the fallback when the
    /// edge itself would traverse but its declarant is absent from the
    /// world — the nearest blocked chain then sits upstream of it.
    NotAFriend,
    /// The target sits in the declarant's `unfriend` list — the grant that
    /// would have re-exported it through this declarant was pruned.
    Unfriended,
    /// The target is killed by an `exclude` — this edge's own deep
    /// subtree exclusion, or (named as `from`) another declarant's.
    Excluded,
    /// An override entry rewrote this edge with `exclude = true`.
    OverrideKilled,
    /// The declarant's grant was rejected by the target's `allow-friends`.
    AllowFriendsRejected,
}

/// Explain one coordinate's place in the root's effective world: the
/// admitting provenance when present, the classified blocked edges when
/// absent, or `UnknownCoordinate` when nothing in this world declares it.
///
/// ```
/// use vibe_core::manifest::AccessLevel;
/// use vibe_core::visibility::{
///     EdgeDecl, InstalledWorld, NodeDecl, VisibilityGraph, why, WhyVerdict,
/// };
///
/// let mut graph = VisibilityGraph::default();
/// graph.nodes.insert("demo".into(), NodeDecl {
///     edges: vec![EdgeDecl { to: "org.x/mid".into(), ..EdgeDecl::default() }],
///     ..NodeDecl::default()
/// });
/// graph.nodes.insert("org.x/mid".into(), NodeDecl {
///     edges: vec![EdgeDecl {
///         to: "org.x/wal".into(),
///         access: Some(AccessLevel::Private),
///         ..EdgeDecl::default()
///     }],
///     ..NodeDecl::default()
/// });
/// graph.nodes.insert("org.x/wal".into(), NodeDecl::default());
/// let world = InstalledWorld { root_id: "demo".into(), graph, unread: Vec::new() };
/// match why(&world, "org.x/wal") {
///     WhyVerdict::Absent { blocked } => {
///         assert_eq!(blocked[0].from, "org.x/mid");
///     }
///     other => panic!("expected Absent, got {other:?}"),
/// }
/// ```
pub fn why(world: &InstalledWorld, target: &str) -> WhyVerdict {
    let analysis = analyze(&world.graph, &world.root_id);
    if target == world.root_id {
        // The root trivially exists in its own world.
        return WhyVerdict::Present(Provenance {
            rule: ProvenanceRule::RootEdge,
            path: vec![world.root_id.clone()],
            via_override: None,
        });
    }
    if let Some(provenance) = analysis.effective.get(target) {
        return WhyVerdict::Present(provenance.clone());
    }
    let mut blocked: Vec<BlockedEdge> = Vec::new();
    let mut declared_anywhere = world.graph.nodes.contains_key(target);
    for (from, declaration) in &world.graph.nodes {
        for edge in &declaration.edges {
            if edge.to != target {
                continue;
            }
            declared_anywhere = true;
            blocked.push(blocked_edge(world, &analysis, from, edge));
        }
    }
    if !declared_anywhere {
        return WhyVerdict::UnknownCoordinate;
    }
    blocked.sort_by(|a, b| (&a.from, a.reason).cmp(&(&b.from, b.reason)));
    blocked.dedup_by(|a, b| a.from == b.from && a.reason == b.reason);
    WhyVerdict::Absent { blocked }
}

/// Classify one edge toward an absent target — the best explanation, most
/// specific authorial word first (an override kill or explicit exclusion
/// outranks a generic access default).
fn blocked_edge(
    world: &InstalledWorld,
    analysis: &Analysis,
    from: &NodeId,
    edge: &EdgeDecl,
) -> BlockedEdge {
    if override_kills(&world.graph, from, &edge.to) {
        return BlockedEdge {
            from: from.clone(),
            reason: BlockReason::OverrideKilled,
        };
    }
    if edge.exclude.iter().any(|excluded| excluded == &edge.to) {
        return BlockedEdge {
            from: from.clone(),
            reason: BlockReason::Excluded,
        };
    }
    if edge.effective_access() == AccessLevel::Private && from != &world.root_id {
        return BlockedEdge {
            from: from.clone(),
            reason: BlockReason::Private,
        };
    }
    let unfriended =
        world.graph.nodes.get(from).is_some_and(|declaration| {
            declaration.unfriend.iter().any(|pruned| pruned == &edge.to)
        });
    if edge.effective_friend() && unfriended {
        return BlockedEdge {
            from: from.clone(),
            reason: BlockReason::Unfriended,
        };
    }
    let rejected = analysis.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            Diagnostic::RejectedGrant { from: granter, to } if granter == from && to == &edge.to
        )
    });
    if edge.effective_friend() && rejected {
        return BlockedEdge {
            from: from.clone(),
            reason: BlockReason::AllowFriendsRejected,
        };
    }
    if edge.effective_access() == AccessLevel::FriendsOnly
        && from != &world.root_id
        && !analysis.closure.contains(from)
    {
        return BlockedEdge {
            from: from.clone(),
            reason: BlockReason::NotAFriend,
        };
    }
    // Conservative deep exclusion (Maven-style, F4): some other
    // declarant's edge.exclude names the target — name that declarant.
    if let Some(excluder) = deep_excluder(&world.graph, &edge.to) {
        return BlockedEdge {
            from: excluder,
            reason: BlockReason::Excluded,
        };
    }
    // The edge itself would traverse (its declarant is the root, or the
    // access is public) but the target is still absent: the declarant
    // never entered the world, so the nearest blocked chain sits
    // upstream. NotAFriend is the closest closed-set label.
    BlockedEdge {
        from: from.clone(),
        reason: BlockReason::NotAFriend,
    }
}

/// Whether any override table in the graph rewrites the edge `from -> to`
/// with `exclude = true` (PROP-050 §2.9 — the sanctioned kill).
fn override_kills(graph: &VisibilityGraph, from: &NodeId, to: &NodeId) -> bool {
    let target = OverrideTarget::Edge {
        from: from.clone(),
        to: to.clone(),
    };
    graph.nodes.values().any(|declaration| {
        declaration
            .overrides
            .iter()
            .any(|(candidate, entry)| candidate == &target && entry.exclude == Some(true))
    })
}

/// The first declarant (deterministic by node order) whose edge.exclude
/// names `target` — the conservative deep-exclusion witness.
fn deep_excluder(graph: &VisibilityGraph, target: &NodeId) -> Option<NodeId> {
    graph
        .nodes
        .iter()
        .filter(|(_, declaration)| {
            declaration
                .edges
                .iter()
                .any(|edge| edge.exclude.iter().any(|excluded| excluded == target))
        })
        .map(|(declarant, _)| declarant.clone())
        .next()
}

/// The sealed-circle state of one provider's `allow-friends` declaration
/// (PROP-050 ##ALLOW-FRIENDS-STATES).
///
/// ```
/// use vibe_core::visibility::AllowFriendsState;
///
/// assert_ne!(AllowFriendsState::Open, AllowFriendsState::Sealed);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowFriendsState {
    /// The field is absent — anyone's grant works.
    Open,
    /// An empty list — nobody enters.
    Sealed,
    /// The named circle (entries may be `<group>/*` patterns).
    Circle(Vec<NodeId>),
}

/// The sealed-circle report for one provider (##ALLOW-FRIENDS-EXHAUSTIVE):
/// who may befriend it, who actually does, which grants its circle
/// rejects, and whether it stands in the root's friend closure.
///
/// Computed from the graph's declarations WITHOUT override masks — the
/// basic picture; a rewritten permits list shows only in the closure math.
///
/// ```
/// use vibe_core::visibility::FriendsReport;
///
/// let report = FriendsReport {
///     state: vibe_core::visibility::AllowFriendsState::Open,
///     actual_friends: vec!["demo".into()],
///     rejected: Vec::new(),
///     in_root_closure: false,
/// };
/// assert_eq!(report.actual_friends, ["demo"]);
/// ```
pub struct FriendsReport {
    pub state: AllowFriendsState,
    /// Nodes whose grants on the target pass the gate: the target is in
    /// their grants (an effective `friend = true` edge — the friends-only
    /// implication included — or a `friends` entry, minus `unfriend`) and
    /// the target's allow-friends admits them.
    pub actual_friends: Vec<NodeId>,
    /// Grants rejected by the target's allow-friends gate.
    pub rejected: Vec<NodeId>,
    /// Whether the target is in `C(root)` for this world's root.
    pub in_root_closure: bool,
}

/// The sealed-circle report for one provider: `None` when the coordinate
/// names no node in this world.
///
/// ```
/// use vibe_core::manifest::AccessLevel;
/// use vibe_core::visibility::{
///     AllowFriendsState, EdgeDecl, InstalledWorld, NodeDecl, VisibilityGraph, friends,
/// };
///
/// let mut graph = VisibilityGraph::default();
/// // The root befriends the sealed provider with a friends-only edge.
/// graph.nodes.insert("demo".into(), NodeDecl {
///     edges: vec![EdgeDecl {
///         to: "org.x/g".into(),
///         access: Some(AccessLevel::FriendsOnly),
///         ..EdgeDecl::default()
///     }],
///     ..NodeDecl::default()
/// });
/// graph.nodes.insert("org.x/g".into(), NodeDecl {
///     allow_friends: Some(Vec::new()),
///     ..NodeDecl::default()
/// });
/// let world = InstalledWorld { root_id: "demo".into(), graph, unread: Vec::new() };
/// let report = friends(&world, "org.x/g").unwrap();
/// assert_eq!(report.state, AllowFriendsState::Sealed);
/// assert_eq!(report.rejected, ["demo"]);
/// assert!(friends(&world, "org.nope/ghost").is_none());
/// ```
pub fn friends(world: &InstalledWorld, target: &str) -> Option<FriendsReport> {
    let declaration = world.graph.nodes.get(target)?;
    let circle = declaration.allow_friends.clone();
    let state = match &circle {
        None => AllowFriendsState::Open,
        Some(entries) if entries.is_empty() => AllowFriendsState::Sealed,
        Some(entries) => AllowFriendsState::Circle(entries.clone()),
    };
    let mut actual_friends = Vec::new();
    let mut rejected = Vec::new();
    for (granter, granter_decl) in &world.graph.nodes {
        if !granter_decl.grants().contains(target) {
            continue;
        }
        if circle
            .as_ref()
            .is_none_or(|entries| circle_admits(entries, granter))
        {
            actual_friends.push(granter.clone());
        } else {
            rejected.push(granter.clone());
        }
    }
    let analysis = analyze(&world.graph, &world.root_id);
    Some(FriendsReport {
        state,
        actual_friends,
        rejected,
        in_root_closure: analysis.closure.contains(target),
    })
}

/// Whether the permits list covers `candidate`: an exact coordinate or a
/// `<group>/*` group pattern (the same match the closure engine applies).
fn circle_admits(circle: &[NodeId], candidate: &str) -> bool {
    circle.iter().any(|allowed| {
        allowed == candidate
            || allowed.strip_suffix("/*").is_some_and(|group| {
                candidate
                    .strip_prefix(group)
                    .is_some_and(|name| name.starts_with('/') && !name[1..].contains('/'))
            })
    })
}

#[cfg(test)]
#[path = "query/tests.rs"]
mod tests;
