//! The computed-view engine — per-node effective boot composition.
//!
//! PROP-009 §2.2 / §2.5 / §2.8. Every node has an **effective boot
//! sequence**, computed from the unified resolution:
//!
//! > inherited foundation (from ancestors) + the node's own authored boot
//! > + the boot of the node's transitive dependencies + user overrides
//!
//! [`compute_effective_boot`] is that computation for one node. It is a
//! pure function: it takes the already-discovered inputs ([`NodeBootInputs`])
//! and returns the ordered [`EffectiveBoot`]. It does not run the depsolver,
//! read disk, or generate artifacts — the workspace walk and the unified
//! resolution feed it (a later phase), and the `STATIC.md` / `INDEX.md`
//! artifacts are projected from its output (also a later phase, via
//! [`EffectiveBoot::static_entries`] / [`EffectiveBoot::dynamic_entries`]).
//!
//! ## Ordering — four bands (PROP-009 §2.5)
//!
//! The composed sequence is ordered: `foundation` → the node's own boot →
//! dependency boot (topological — a dependency before its dependents) →
//! `user-override`. The author-chosen `NN-` numeric prefix is gone; the
//! engine owns the order, keyed off each contribution's [`BootCategory`].
//!
//! ## Inclusion type — precedence (PROP-009 §2.4)
//!
//! Each dependency's [`LinkType`] is resolved by precedence: the consumer's
//! explicit per-dependency `link` wins; then the package's `[boot_snippet]`
//! suggestion; then the workspace `[boot].default_link`; then `dynamic`. A
//! node's own authored boot is always `dynamic` — it already lives in the
//! node's tree and is read by reference from `INDEX.md`, so there is
//! nothing to compile into the static lane.
//!
//! A dependency whose `[boot_snippet]` carries a `when` condition
//! (PROP-009 §2.6) is a conditional `dynamic` entry: the condition can only
//! be honoured by the gated INDEX form, never by the verbatim `static` lane.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-009#effective-boot");

use vibe_core::manifest::{BootCategory, LinkType, WhenCondition};
use vibe_core::{Group, PackageKind};

use crate::WorkspaceError;

/// The per-unit recursive compiler (PROP-038) — the hybrid linker that
/// compiles each compilation unit from its own edges. Lands alongside this
/// module's per-node composition during the migration (PROP-038 §4).
pub mod hybrid;

/// The band a boot entry sorts into within the computed sequence
/// (PROP-009 §2.5). Declaration order **is** the sort order — the
/// foundation leads, user overrides trail:
///
/// ```
/// use vibe_workspace::boot::BootBand;
/// assert!(BootBand::Foundation < BootBand::NodeOwn);
/// assert!(BootBand::Dependency < BootBand::UserOverride);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BootBand {
    /// Project-wide foundation — conventions, the four rules, technology
    /// choices. Inherited from ancestors and read first.
    Foundation,
    /// The node's own authored, non-foundation, non-override boot.
    NodeOwn,
    /// Boot contributed by the node's transitive dependencies, in
    /// topological order — a dependency before its dependents.
    Dependency,
    /// User-owned overrides — read last, so they win.
    UserOverride,
}

/// One authored boot file belonging to a node — a file the node's author
/// wrote in its `spec/boot/`. The engine receives these already
/// discovered; it never scans disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBoot {
    /// Workspace-root-relative, forward-slashed path of the boot file.
    pub path: String,
    /// The file's category. The user-owned `00-core.md` / `90-user.md`
    /// are `Foundation` / `UserOverride` by name convention; any other
    /// authored boot file is the node's own mid-band content (`None`).
    pub category: Option<BootCategory>,
    /// Provenance label — the node's `rel_path` (`"."` for the root, or a
    /// member path), used for diagnostics and artifact provenance.
    pub origin: String,
}

/// One resolved dependency contributing boot, as the engine sees it. The
/// caller builds this from the unified resolution and the materialised
/// `vibedeps/` slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyBoot {
    /// The dependency's `kind` — metadata; used only for its `vibedeps/`
    /// slot directory name, never for identity (PROP-008 §2.3).
    pub kind: PackageKind,
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity.
    pub group: Group,
    pub name: String,
    /// Workspace-root-relative path of the dependency's boot file inside
    /// its `vibedeps/` slot — `None` when the package ships no boot
    /// snippet. A boot-less dependency still takes part in the
    /// topological order; it simply contributes no entry.
    pub boot_path: Option<String>,
    /// The dependency's declared `[boot_snippet].category`, if any.
    pub category: Option<BootCategory>,
    /// The consumer's per-dependency `link` declaration
    /// (`[requires.packages].link`) — `None` for a transitive dependency
    /// or one the consumer left unspecified. Highest link precedence.
    pub declared_link: Option<LinkType>,
    /// The package's own suggested `link` (`[boot_snippet].link`) — a hint,
    /// below any consumer declaration.
    pub suggested_link: Option<LinkType>,
    /// The package's declared `[boot_snippet].when` activation condition,
    /// if any (PROP-009 §2.4 / §2.6). A snippet carrying a `when` is
    /// rendered `dynamic` irrespective of `link` — a condition implies the
    /// dynamic INCLUDE form.
    pub when: Option<WhenCondition>,
    /// The `(group, name)` of every package this one directly requires —
    /// the edges of the topological order.
    pub requires: Vec<(Group, String)>,
}

/// One entry in a node's computed effective boot sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootEntry {
    /// Workspace-root-relative, forward-slashed path of the boot file.
    pub path: String,
    /// The band this entry sorts into.
    pub band: BootBand,
    /// The resolved inclusion type.
    pub link: LinkType,
    /// The activation condition carried into a `dynamic` `INDEX.md` entry
    /// (PROP-009 §2.3). `None` for an unconditional entry. A `Some` here
    /// implies `link == LinkType::Dynamic` — the engine forces it.
    pub when: Option<WhenCondition>,
    /// Provenance — a node `rel_path` for authored boot, a `<group>/<name>`
    /// pkgref for a dependency.
    pub origin: String,
}

/// A node's computed effective boot sequence (PROP-009 §2.2) — every entry
/// in final composed order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveBoot {
    pub entries: Vec<BootEntry>,
}

impl EffectiveBoot {
    /// `true` when the node has no boot entries at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The `static`-linked entries, in composed order — the source for the
    /// generated `STATIC.md` (PROP-009 §2.3).
    pub fn static_entries(&self) -> impl Iterator<Item = &BootEntry> {
        self.entries.iter().filter(|e| e.link == LinkType::Static)
    }

    /// The `dynamic`-linked entries, in composed order — the source for the
    /// generated `INDEX.md` (PROP-009 §2.3).
    pub fn dynamic_entries(&self) -> impl Iterator<Item = &BootEntry> {
        self.entries.iter().filter(|e| e.link != LinkType::Static)
    }
}

/// The inputs the computed-view engine needs to compose one node's
/// effective boot sequence. The caller — workspace-aware `vibe install`,
/// a later phase — assembles this from the workspace walk and the unified
/// resolution.
#[derive(Debug, Clone, Copy)]
pub struct NodeBootInputs<'a> {
    /// The node's own authored boot files, in declared order.
    pub own_boot: &'a [AuthoredBoot],
    /// Foundation boot inherited from ancestors, absolute-root first
    /// (the most foundational layer leads).
    pub inherited_foundation: &'a [AuthoredBoot],
    /// The node's transitive dependency closure contributing boot.
    pub dependencies: &'a [DependencyBoot],
    /// The workspace `[boot].default_link`, if one is set — the fallback
    /// inclusion type for a dependency that declares none and whose
    /// package suggests none.
    pub default_link: Option<LinkType>,
}

/// Compose one node's effective boot sequence (PROP-009 §2.2).
///
/// Errors only on a [`WorkspaceError::BootDependencyCycle`] — a cycle in
/// the dependency boot graph. A resolution from the depsolver is acyclic;
/// the check guards the engine against a malformed input.
pub fn compute_effective_boot(inputs: NodeBootInputs<'_>) -> Result<EffectiveBoot, WorkspaceError> {
    let order = topo_order(inputs.dependencies)?;

    let mut entries: Vec<BootEntry> = Vec::new();

    // Inherited foundation — from ancestors, absolute-root first. Always
    // the Foundation band; an authored boot file is always `static`.
    for boot in inputs.inherited_foundation {
        entries.push(BootEntry {
            path: boot.path.clone(),
            band: BootBand::Foundation,
            link: LinkType::Dynamic,
            when: None,
            origin: boot.origin.clone(),
        });
    }

    // The node's own authored boot, in declared order — banded by
    // category, `static` link.
    for boot in inputs.own_boot {
        entries.push(BootEntry {
            path: boot.path.clone(),
            band: band_for(boot.category, BootBand::NodeOwn),
            link: LinkType::Dynamic,
            when: None,
            origin: boot.origin.clone(),
        });
    }

    // Dependency boot, in topological order — a dependency before its
    // dependents. A dependency that ships no boot snippet contributes no
    // entry, but its position still threaded the ordering above.
    for &i in &order {
        let dep = &inputs.dependencies[i];
        let Some(path) = &dep.boot_path else {
            continue;
        };
        // PROP-009 §2.4 precedence: consumer's per-dep declaration, then
        // the package's suggestion, then the workspace default, then
        // `static`.
        let link = dep
            .declared_link
            .or(dep.suggested_link)
            .or(inputs.default_link)
            .unwrap_or_default();
        // `static-transitive` (PROP-035 §12) resolves to `static` at
        // emission — bootgen has already propagated the mode down the
        // closure, so here it is just a static entry.
        let link = match link {
            LinkType::StaticTransitive => LinkType::Static,
            other => other,
        };
        // A conditional snippet is `dynamic` by nature (PROP-009 §2.4): a
        // `when` cannot be honoured by the verbatim `static` lane or a
        // direct `static` read, so it forces the dynamic INCLUDE form
        // whatever the `link` precedence resolved to.
        let link = if dep.when.is_some() {
            LinkType::Dynamic
        } else {
            link
        };
        entries.push(BootEntry {
            path: path.clone(),
            band: band_for(dep.category, BootBand::Dependency),
            link,
            when: dep.when,
            origin: format!("{}/{}", dep.group, dep.name),
        });
    }

    // Stable sort by band. The collection order above — inherited, then
    // own, then topo-ordered deps — is preserved within each band, so
    // inherited foundation precedes own foundation, and the node's own
    // overrides precede a dependency's inside the UserOverride band.
    entries.sort_by_key(|e| e.band);

    Ok(EffectiveBoot { entries })
}

/// Map a category to its band: foundation and user-override get their own;
/// `flow` / `stack` / `tool` / `app` (or none at all) fall to `default_band`.
fn band_for(category: Option<BootCategory>, default_band: BootBand) -> BootBand {
    match category {
        Some(BootCategory::Foundation) => BootBand::Foundation,
        Some(BootCategory::UserOverride) => BootBand::UserOverride,
        Some(BootCategory::Flow | BootCategory::Stack | BootCategory::Tool | BootCategory::App)
        | None => default_band,
    }
}

/// Topologically sort the dependency boot graph — a dependency before its
/// dependents. Ties break on the `<group>/<name>` pkgref, so the order is
/// deterministic. Returns indices into `deps`; a cycle is an error.
fn topo_order(deps: &[DependencyBoot]) -> Result<Vec<usize>, WorkspaceError> {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap};

    let n = deps.len();
    let key = |i: usize| format!("{}/{}", deps[i].group, deps[i].name);
    let index: HashMap<String, usize> = (0..n).map(|i| (key(i), i)).collect();

    // `in_degree[i]` counts the in-set packages `i` requires; `dependents`
    // is the reverse adjacency. An edge to a package outside the set
    // (never expected in a transitive closure) imposes no ordering.
    let mut in_degree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, dep) in deps.iter().enumerate() {
        for (rg, rn) in &dep.requires {
            if let Some(&j) = index.get(&format!("{rg}/{rn}")) {
                // `i` requires `j` → `j` must precede `i`.
                in_degree[i] += 1;
                dependents[j].push(i);
            }
        }
    }

    // Kahn's algorithm. A min-heap keyed on the pkgref makes the choice
    // among ready packages deterministic.
    let mut ready: BinaryHeap<Reverse<(String, usize)>> = (0..n)
        .filter(|&i| in_degree[i] == 0)
        .map(|i| Reverse((key(i), i)))
        .collect();
    let mut order: Vec<usize> = Vec::with_capacity(n);
    while let Some(Reverse((_, i))) = ready.pop() {
        order.push(i);
        for &dependent in &dependents[i] {
            in_degree[dependent] -= 1;
            if in_degree[dependent] == 0 {
                ready.push(Reverse((key(dependent), dependent)));
            }
        }
    }

    if order.len() != n {
        let mut stuck: Vec<String> = (0..n).filter(|i| !order.contains(i)).map(key).collect();
        stuck.sort();
        return Err(WorkspaceError::BootDependencyCycle {
            packages: stuck.join(", "),
        });
    }
    Ok(order)
}

#[cfg(test)]
#[path = "boot/tests.rs"]
mod tests;
