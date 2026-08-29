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

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#effective-boot");

use vibe_core::manifest::{BootCategory, LinkType, PackageFormat, WhenCondition};
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
/// wrote in its boot lane. The engine receives these already
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
/// dependency slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyBoot {
    /// The dependency's `kind` — metadata; used only for its dependency
    /// slot directory name, never for identity (PROP-008 §2.3).
    pub kind: PackageKind,
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity.
    pub group: Group,
    pub name: String,
    /// Workspace-root-relative path of the dependency's boot file inside
    /// its dependency slot — `None` when the package ships no boot
    /// snippet. A boot-less dependency still takes part in the
    /// topological order; it simply contributes no entry.
    pub boot_path: Option<String>,
    /// Additional independently conditional boot contributions from this
    /// package, already filtered for install-time predicates.
    pub fragments: Vec<BootContribution>,
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
    /// The dependency's PROP-035 §3 package format. A `normal` package whose
    /// entry resolves `static` is compiled to its `#use`-reachable,
    /// `#source`-merged closure by the boot linker (PROP-035 §8); a `simple`
    /// one is concatenated verbatim. Default `simple` (fail-safe).
    pub format: PackageFormat,
    /// B-006 (lane dedup): `true` when `node_dependency_boot` rewrote
    /// `boot_path` from the package's raw snippet to its compiled per-unit
    /// `STATIC.md` — the package statically links a child, so the whole zone
    /// is read (PROP-038 §2.1). The lane-dedup pass
    /// (`desubstitute_covered_units`) may roll such an entry back to its
    /// snippet — or elide it — once every boot-bearing member of its zone is
    /// present individually in the lane. `false` for every other construction.
    pub unit_substituted: bool,
}

/// One resolved boot contribution beyond a package's main snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootContribution {
    /// Workspace-root-relative, forward-slashed source path.
    pub path: String,
    /// Residual read-time predicate. Install-time predicates are resolved
    /// before this model is built, so only `os:*` may remain.
    pub when: Option<WhenCondition>,
}

/// The typed half of one boot entry's provenance.
///
/// [`BootEntry::origin`] is a DISPLAY string and PROP-054 keeps it that way:
/// it may carry a `[shared by …]` suffix, it is what a generated artifact
/// prints, and nothing may recover identity by parsing it. The compiler's
/// per-document selector subject nevertheless needs the typed provider that
/// declared each contribution, so the typed components travel BESIDE
/// `origin` — from the site that formats it, in the same expression — and
/// are never reconstructed from it.
///
/// The name half of the dependency arm is still the install model's bare
/// `String`; it is parsed through `PackageName`'s one grammar at the
/// adapter seam that needs the typed identity, and refused there rather
/// than being defaulted away. Retyping the install model itself is separate
/// hygiene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootProvenance {
    /// The node's own authored boot — the node itself is the provider.
    ///
    /// A single entry cannot carry the node's typed identity, because the
    /// same authored file is inherited into several nodes' lanes; the
    /// adapter names it from the node's own coordinate, which is exactly the
    /// authority `spec://` addressing already gives that lane.
    Node,
    /// A dependency package, by the `(group, name)` pair the resolution
    /// records for it.
    Dependency { group: Group, name: String },
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
    /// pkgref for a dependency. DISPLAY only: see [`BootProvenance`].
    pub origin: String,
    /// The typed half of the same provenance, carried beside `origin` so the
    /// compiler's document subject never parses identity out of a display
    /// string.
    pub provenance: BootProvenance,
    /// A soft-hoist reference (PROP-038 §2.5): a `static` entry hoisted out of
    /// this unit's zone into the global root `STATIC.md`. The renderer emits a
    /// `#use spec://<origin>` marker instead of the file's content, so the
    /// graph edge survives locally while the text lives once at the hoist
    /// point. `false` for an ordinary compiled-in entry.
    pub use_ref: bool,
    /// The contributing package's PROP-035 §3 format. `Normal` on a `static`
    /// entry tells the renderer to compile the `#use`/`#source` closure from
    /// this entry's contract rather than concatenate the file verbatim
    /// (PROP-035 §8). `Simple` (the default, and always for a node's own
    /// authored boot) keeps the verbatim path.
    pub format: PackageFormat,
    /// B-006 (lane dedup): carried from [`DependencyBoot::unit_substituted`]
    /// — `true` when this entry's `path` points at a compiled per-unit
    /// `STATIC.md` rather than the package's own snippet. See
    /// `desubstitute_covered_units`. `false` for authored boot and for any
    /// dependency the linker did not substitute.
    pub unit_substituted: bool,
    /// B-006 (lane dedup): `true` when this entry's whole static zone is
    /// emitted member-by-member elsewhere in the same lane, so the entry
    /// renders as a provenance stub (no `#use`, no body) instead of a second
    /// copy. Set only by `desubstitute_covered_units`, for a contentless
    /// umbrella whose every boot-bearing member is present individually.
    pub elided: bool,
}

/// The typed provenance one authored `<group>/<name>` fixture spelling
/// stands for — a TEST FIXTURE BUILDER, never an identity recovery.
///
/// The distinction matters and is the whole reason this is `#[cfg(test)]`.
/// Production never reads a typed pair out of a display string: it authors
/// both halves from the same two values, in one expression. A fixture has
/// only one authored spelling to start from, so it states both halves from
/// that — the fixture is CHOOSING what to declare, which is exactly what a
/// manifest does. A spelling that is not a coordinate is a node's own boot,
/// which is what a bare `rel_path` origin means in every fixture here.
/// A fixture spelling may carry the `[shared by …]` display suffix, so the
/// coordinate is the leading whitespace-free token — the same reading
/// `ArtifactInput`'s own origin/target law already applies. Production
/// carries the typed pair past that suffix instead of reading around it.
#[cfg(test)]
pub(crate) fn fixture_provenance(origin: &str) -> BootProvenance {
    let coordinate = origin.split_whitespace().next().unwrap_or_default();
    match coordinate.split_once('/') {
        Some((group, name)) => match Group::parse(group) {
            Ok(group) => BootProvenance::Dependency {
                group,
                name: name.to_owned(),
            },
            Err(_) => BootProvenance::Node,
        },
        None => BootProvenance::Node,
    }
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
            // Inherited foundation is an ANCESTOR node's authored boot read
            // into this node's lane: still a node's own file, never a
            // package's, so the node arm is the honest one.
            provenance: BootProvenance::Node,
            use_ref: false,
            // A node's own authored boot is always carried verbatim.
            format: PackageFormat::Simple,
            unit_substituted: false,
            elided: false,
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
            provenance: BootProvenance::Node,
            use_ref: false,
            // A node's own authored boot is always carried verbatim.
            format: PackageFormat::Simple,
            unit_substituted: false,
            elided: false,
        });
    }

    // Dependency boot, in topological order — a dependency before its
    // dependents. A dependency that ships no boot snippet contributes no
    // entry, but its position still threaded the ordering above.
    for &i in &order {
        let dep = &inputs.dependencies[i];
        // PROP-009 §2.4 precedence: consumer's per-dep declaration, then
        // the package's suggestion, then the workspace default, then
        // `static`.
        let link = dep
            .declared_link
            .or(dep.suggested_link)
            .or(inputs.default_link)
            .unwrap_or_default();
        // `static-transitive` (PROP-035 §12) and `static-hard` (PROP-038
        // §2.3) both resolve to `static` at emission — the priority lane is
        // the same; the modes differ only in propagation (transitive) and
        // hoisting (hard opts out), decided before emission.
        let link = match link {
            LinkType::StaticTransitive | LinkType::StaticHard => LinkType::Static,
            other => other,
        };
        let mut push_contribution = |path: &str, when: Option<WhenCondition>, substituted: bool| {
            // A read-time conditional contribution is `dynamic` by nature:
            // `os:*` must remain in INDEX whatever link precedence resolved.
            let contribution_link = if when.is_some() {
                LinkType::Dynamic
            } else {
                link
            };
            entries.push(BootEntry {
                path: path.to_string(),
                band: band_for(dep.category, BootBand::Dependency),
                link: contribution_link,
                when,
                // The display spelling and its typed components, authored in
                // one place from the same two values — the pairing is local
                // and auditable rather than a later reconstruction.
                origin: format!("{}/{}", dep.group, dep.name),
                provenance: BootProvenance::Dependency {
                    group: dep.group.clone(),
                    name: dep.name.clone(),
                },
                use_ref: false,
                format: dep.format,
                unit_substituted: substituted,
                elided: false,
            });
        };
        if let Some(path) = &dep.boot_path {
            push_contribution(path, dep.when.clone(), dep.unit_substituted);
        }
        for fragment in &dep.fragments {
            push_contribution(&fragment.path, fragment.when.clone(), false);
        }
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
