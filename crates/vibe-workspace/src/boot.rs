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
//! resolution feed it (a later phase), and the `INLINE.md` / `INDEX.md`
//! artifacts are projected from its output (also a later phase, via
//! [`EffectiveBoot::inline_entries`] / [`EffectiveBoot::indexed_entries`]).
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
//! suggestion; then the workspace `[boot].default_link`; then `static`. A
//! node's own authored boot is always `static` — it already lives in the
//! node's tree, so there is nothing to inline or defer.

use vibe_core::PackageKind;
use vibe_core::manifest::{BootCategory, LinkType};

use crate::WorkspaceError;

/// The band a boot entry sorts into within the computed sequence
/// (PROP-009 §2.5). Declaration order **is** the sort order.
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
    pub kind: PackageKind,
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
    /// The `(kind, name)` of every package this one directly requires —
    /// the edges of the topological order.
    pub requires: Vec<(PackageKind, String)>,
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
    /// Provenance — a node `rel_path` for authored boot, a `<kind>:<name>`
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

    /// The `inline`-linked entries, in composed order — the source for the
    /// generated `INLINE.md` (PROP-009 §2.3).
    pub fn inline_entries(&self) -> impl Iterator<Item = &BootEntry> {
        self.entries.iter().filter(|e| e.link == LinkType::Inline)
    }

    /// The `static` and `dynamic` entries, in composed order — the source
    /// for the generated `INDEX.md` (PROP-009 §2.3).
    pub fn indexed_entries(&self) -> impl Iterator<Item = &BootEntry> {
        self.entries.iter().filter(|e| e.link != LinkType::Inline)
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
pub fn compute_effective_boot(
    inputs: NodeBootInputs<'_>,
) -> Result<EffectiveBoot, WorkspaceError> {
    let order = topo_order(inputs.dependencies)?;

    let mut entries: Vec<BootEntry> = Vec::new();

    // Inherited foundation — from ancestors, absolute-root first. Always
    // the Foundation band; an authored boot file is always `static`.
    for boot in inputs.inherited_foundation {
        entries.push(BootEntry {
            path: boot.path.clone(),
            band: BootBand::Foundation,
            link: LinkType::Static,
            origin: boot.origin.clone(),
        });
    }

    // The node's own authored boot, in declared order — banded by
    // category, `static` link.
    for boot in inputs.own_boot {
        entries.push(BootEntry {
            path: boot.path.clone(),
            band: band_for(boot.category, BootBand::NodeOwn),
            link: LinkType::Static,
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
        entries.push(BootEntry {
            path: path.clone(),
            band: band_for(dep.category, BootBand::Dependency),
            link,
            origin: format!("{}:{}", dep.kind, dep.name),
        });
    }

    // Stable sort by band. The collection order above — inherited, then
    // own, then topo-ordered deps — is preserved within each band, so
    // inherited foundation precedes own foundation, and the node's own
    // overrides precede a dependency's inside the UserOverride band.
    entries.sort_by_key(|e| e.band);

    Ok(EffectiveBoot { entries })
}

/// Map a contribution's declared category to its band. A `flow` / `stack`
/// category, or none at all, falls to `default_band` — `NodeOwn` for the
/// node's own boot, `Dependency` for a dependency's.
fn band_for(category: Option<BootCategory>, default_band: BootBand) -> BootBand {
    match category {
        Some(BootCategory::Foundation) => BootBand::Foundation,
        Some(BootCategory::UserOverride) => BootBand::UserOverride,
        Some(BootCategory::Flow | BootCategory::Stack) | None => default_band,
    }
}

/// Topologically sort the dependency boot graph — a dependency before its
/// dependents. Ties break on the `<kind>:<name>` pkgref, so the order is
/// deterministic. Returns indices into `deps`; a cycle is an error.
fn topo_order(deps: &[DependencyBoot]) -> Result<Vec<usize>, WorkspaceError> {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap};

    let n = deps.len();
    let key = |i: usize| format!("{}:{}", deps[i].kind, deps[i].name);
    let index: HashMap<String, usize> = (0..n).map(|i| (key(i), i)).collect();

    // `in_degree[i]` counts the in-set packages `i` requires; `dependents`
    // is the reverse adjacency. An edge to a package outside the set
    // (never expected in a transitive closure) imposes no ordering.
    let mut in_degree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, dep) in deps.iter().enumerate() {
        for (rk, rn) in &dep.requires {
            if let Some(&j) = index.get(&format!("{rk}:{rn}")) {
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
        let mut stuck: Vec<String> = (0..n)
            .filter(|i| !order.contains(i))
            .map(key)
            .collect();
        stuck.sort();
        return Err(WorkspaceError::BootDependencyCycle {
            packages: stuck.join(", "),
        });
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authored(path: &str, category: Option<BootCategory>) -> AuthoredBoot {
        AuthoredBoot {
            path: path.to_string(),
            category,
            origin: ".".to_string(),
        }
    }

    /// A dependency with a boot snippet, no link declarations, given edges.
    fn dep(name: &str, has_boot: bool, requires: &[&str]) -> DependencyBoot {
        DependencyBoot {
            kind: PackageKind::Flow,
            name: name.to_string(),
            boot_path: has_boot.then(|| format!("vibedeps/flow-{name}/1.0.0/boot.md")),
            category: None,
            declared_link: None,
            suggested_link: None,
            requires: requires
                .iter()
                .map(|r| (PackageKind::Flow, r.to_string()))
                .collect(),
        }
    }

    fn compute(
        own: &[AuthoredBoot],
        inherited: &[AuthoredBoot],
        deps: &[DependencyBoot],
        default_link: Option<LinkType>,
    ) -> EffectiveBoot {
        compute_effective_boot(NodeBootInputs {
            own_boot: own,
            inherited_foundation: inherited,
            dependencies: deps,
            default_link,
        })
        .unwrap()
    }

    #[test]
    fn empty_inputs_yield_empty_boot() {
        let boot = compute(&[], &[], &[], None);
        assert!(boot.is_empty());
    }

    #[test]
    fn own_boot_bands_by_category() {
        let own = vec![
            authored("spec/boot/00-core.md", Some(BootCategory::Foundation)),
            authored("spec/boot/notes.md", None),
            authored("spec/boot/90-user.md", Some(BootCategory::UserOverride)),
        ];
        let boot = compute(&own, &[], &[], None);
        let bands: Vec<BootBand> = boot.entries.iter().map(|e| e.band).collect();
        assert_eq!(
            bands,
            vec![BootBand::Foundation, BootBand::NodeOwn, BootBand::UserOverride]
        );
        // Authored boot is always `static`.
        assert!(boot.entries.iter().all(|e| e.link == LinkType::Static));
    }

    #[test]
    fn declared_order_survives_even_when_input_is_shuffled() {
        // Override declared before foundation — the engine re-bands it.
        let own = vec![
            authored("spec/boot/90-user.md", Some(BootCategory::UserOverride)),
            authored("spec/boot/00-core.md", Some(BootCategory::Foundation)),
        ];
        let boot = compute(&own, &[], &[], None);
        assert_eq!(boot.entries[0].path, "spec/boot/00-core.md");
        assert_eq!(boot.entries[1].path, "spec/boot/90-user.md");
    }

    #[test]
    fn inherited_foundation_precedes_own_foundation() {
        let inherited = vec![authored("spec/boot/00-core.md", Some(BootCategory::Foundation))];
        let own = vec![authored(
            "packages/x/spec/boot/00-core.md",
            Some(BootCategory::Foundation),
        )];
        let boot = compute(&own, &inherited, &[], None);
        assert_eq!(boot.entries[0].path, "spec/boot/00-core.md");
        assert_eq!(boot.entries[1].path, "packages/x/spec/boot/00-core.md");
    }

    #[test]
    fn dependency_boot_is_topologically_ordered() {
        // `a` requires `b` — so `b` must come first.
        let deps = vec![dep("a", true, &["b"]), dep("b", true, &[])];
        let boot = compute(&[], &[], &deps, None);
        let origins: Vec<&str> = boot.entries.iter().map(|e| e.origin.as_str()).collect();
        assert_eq!(origins, vec!["flow:b", "flow:a"]);
        assert!(boot.entries.iter().all(|e| e.band == BootBand::Dependency));
    }

    #[test]
    fn bootless_dependency_contributes_no_entry_but_still_orders() {
        // `a` → `m` (no boot) → `b`. The topo order is b, m, a; filtered
        // to boot-bearing packages it is b, a — `m` still transmitted the
        // ordering between them.
        let deps = vec![
            dep("a", true, &["m"]),
            dep("m", false, &["b"]),
            dep("b", true, &[]),
        ];
        let boot = compute(&[], &[], &deps, None);
        let origins: Vec<&str> = boot.entries.iter().map(|e| e.origin.as_str()).collect();
        assert_eq!(origins, vec!["flow:b", "flow:a"]);
    }

    #[test]
    fn link_precedence_declared_beats_suggested_and_default() {
        let mut d = dep("x", true, &[]);
        d.declared_link = Some(LinkType::Dynamic);
        d.suggested_link = Some(LinkType::Inline);
        let boot = compute(&[], &[], &[d], Some(LinkType::Static));
        assert_eq!(boot.entries[0].link, LinkType::Dynamic);
    }

    #[test]
    fn link_precedence_suggested_beats_default() {
        let mut d = dep("x", true, &[]);
        d.suggested_link = Some(LinkType::Inline);
        let boot = compute(&[], &[], &[d], Some(LinkType::Dynamic));
        assert_eq!(boot.entries[0].link, LinkType::Inline);
    }

    #[test]
    fn link_precedence_falls_through_to_default() {
        let d = dep("x", true, &[]);
        let boot = compute(&[], &[], &[d], Some(LinkType::Dynamic));
        assert_eq!(boot.entries[0].link, LinkType::Dynamic);
    }

    #[test]
    fn link_precedence_defaults_to_static() {
        let d = dep("x", true, &[]);
        let boot = compute(&[], &[], &[d], None);
        assert_eq!(boot.entries[0].link, LinkType::Static);
    }

    #[test]
    fn dependency_with_foundation_category_joins_the_foundation_band() {
        let mut d = dep("x", true, &[]);
        d.category = Some(BootCategory::Foundation);
        let boot = compute(&[], &[], &[d], None);
        assert_eq!(boot.entries[0].band, BootBand::Foundation);
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let deps = vec![dep("a", true, &["b"]), dep("b", true, &["a"])];
        let err = compute_effective_boot(NodeBootInputs {
            own_boot: &[],
            inherited_foundation: &[],
            dependencies: &deps,
            default_link: None,
        })
        .unwrap_err();
        match err {
            WorkspaceError::BootDependencyCycle { packages } => {
                assert!(packages.contains("flow:a"), "{packages}");
                assert!(packages.contains("flow:b"), "{packages}");
            }
            other => panic!("expected a boot dependency cycle, got {other}"),
        }
    }

    #[test]
    fn inline_and_indexed_entries_split_by_link() {
        let mut inline = dep("crit", true, &[]);
        inline.declared_link = Some(LinkType::Inline);
        let mut dynamic = dep("rust", true, &[]);
        dynamic.declared_link = Some(LinkType::Dynamic);
        let plain = dep("wal", true, &[]); // static
        let boot = compute(&[], &[], &[inline, dynamic, plain], None);

        let inline_origins: Vec<&str> =
            boot.inline_entries().map(|e| e.origin.as_str()).collect();
        assert_eq!(inline_origins, vec!["flow:crit"]);

        let indexed_origins: Vec<&str> =
            boot.indexed_entries().map(|e| e.origin.as_str()).collect();
        // `static` and `dynamic` both land in the index, in composed order.
        assert_eq!(indexed_origins, vec!["flow:rust", "flow:wal"]);
    }

    #[test]
    fn full_composition_orders_all_four_bands() {
        let inherited = vec![authored("spec/boot/00-core.md", Some(BootCategory::Foundation))];
        let own = vec![
            authored("packages/x/spec/boot/intro.md", None),
            authored("packages/x/spec/boot/90-user.md", Some(BootCategory::UserOverride)),
        ];
        let deps = vec![dep("a", true, &["b"]), dep("b", true, &[])];
        let boot = compute(&own, &inherited, &deps, None);
        let bands: Vec<BootBand> = boot.entries.iter().map(|e| e.band).collect();
        assert_eq!(
            bands,
            vec![
                BootBand::Foundation,  // inherited 00-core.md
                BootBand::NodeOwn,     // intro.md
                BootBand::Dependency,  // flow:b
                BootBand::Dependency,  // flow:a
                BootBand::UserOverride // 90-user.md
            ]
        );
        let origins: Vec<&str> = boot.entries.iter().map(|e| e.origin.as_str()).collect();
        assert_eq!(origins, vec![".", ".", "flow:b", "flow:a", "."]);
    }
}
