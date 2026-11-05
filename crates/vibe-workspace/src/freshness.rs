//! Lockfile-freshness check — PROP-011 §2.2 / §5.1.
//!
//! Before `vibe install` runs the depsolver — a registry walk over the
//! network — it asks: is the current `vibe.lock` still a correct
//! resolution of every workspace node's `[requires]`? When it is, the
//! depsolver is skipped entirely; the resolution is exactly what the lock
//! records, and `vibe install` proceeds straight to application.
//!
//! ## The cargo model
//!
//! The check follows `cargo` (PROP-011 §5.1): no digest of the manifests
//! is stored anywhere. The lockfile *is* the baseline, and freshness is a
//! **satisfiability test** — every declared registry dependency must have
//! a `[[package]]` entry whose pinned version satisfies the current
//! constraint, and the declared root set must equal the lock's recorded
//! `meta.root_dependencies`. An added dependency has no locked entry; a
//! removed one leaves the root sets unequal; a tightened constraint leaves
//! the locked version outside it. Transitive packages are trusted: an
//! unchanged root set cannot have produced a different transitive closure
//! (a transitive `[requires]` lives inside a dependency slot, immutable
//! once materialised).
//!
//! ## Conservative by construction
//!
//! [`check`] never reports `Fresh` when it cannot *prove* freshness
//! cheaply. A git- or path-source dependency points at a mutable source,
//! a capability requirement is not recorded in the lock, an unresolved
//! `version.var` placeholder cannot be compared — each yields `Stale`, and
//! `vibe install` falls back to a full resolution (always correct, merely
//! slower). The fast path therefore triggers for the common case: a
//! workspace whose `[requires]` is purely registry-resolved packages.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#skip-resolution");

use std::collections::HashSet;

use vibe_core::manifest::{Lockfile, SourceKind};
use vibe_core::{Group, PackageRef, VersionSpec};

use crate::{Workspace, vibedeps};

mod configuration;
mod source;
pub use configuration::configuration_freshness;
pub use source::is_in_workspace_file_source;

/// The outcome of a freshness check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    /// The lock is a correct resolution of the current `[requires]` and
    /// every locked package is materialised — the depsolver can be
    /// skipped, application proceeds against the locked versions.
    Fresh,
    /// The lock cannot be proven fresh; the string explains why, for the
    /// `vibe install` report. A full resolution must run.
    Stale(String),
}

impl Freshness {
    /// `true` for [`Freshness::Fresh`].
    pub fn is_fresh(&self) -> bool {
        matches!(self, Freshness::Fresh)
    }
}

/// Decide whether `lockfile` is a fresh resolution of `workspace` — the
/// PROP-011 §2.2 freshness check. Pure over the in-memory inputs plus
/// dependency-slot presence on disk; runs no depsolver and no network.
pub fn check(workspace: &Workspace, lockfile: &Lockfile) -> Freshness {
    let configuration = configuration_freshness(workspace, lockfile);
    if !configuration.is_fresh() {
        return configuration;
    }
    // The declared registry root set, unioned across every node. Built
    // only once every node has been proven free of the source kinds the
    // check cannot cheaply reason about (below). Keyed by the
    // `(group, name)` identity (PROP-008 §2.3).
    let mut declared_roots: HashSet<(Group, String)> = HashSet::new();

    for (rel, manifest) in workspace.iter_nodes() {
        let req = &manifest.requires;

        // An unresolved `version.var` placeholder — the workspace loader
        // normally drains these into `packages`, so a non-empty one here
        // is unexpected; refuse to reason about it.
        if !req.var_packages.is_empty() {
            return stale(format!(
                "node `{rel}` carries unresolved version placeholders"
            ));
        }
        // A git- or path-source dependency points at a source whose
        // content can change with no `[requires]` edit (a moving branch,
        // a sibling directory). Freshness cannot be cheaply proven —
        // PROP-011 §5.1 leaves immutable-ref fast-pathing to a later
        // refinement.
        if !req.git_packages.is_empty() {
            return stale(format!("node `{rel}` has a git-source dependency"));
        }
        if !req.path_packages.is_empty() {
            return stale(format!("node `{rel}` has a path-source dependency"));
        }
        // An abstract capability requirement is satisfied by some package
        // in the graph, but the lock records no requested-capability set
        // to compare against — so an added requirement could not be
        // detected. Conservative: a node with capability requirements is
        // never fast-pathed.
        if !req.capabilities.is_empty() {
            return stale(format!("node `{rel}` has a capability requirement"));
        }

        // Registry-resolved dependencies: the locked version must still
        // satisfy the declared constraint. A `[requires.packages]` key is
        // group-qualified at parse time (PROP-008 §2.6), so `group` is
        // always present here.
        for pr in &req.packages {
            let Some(group) = pr.group.clone() else {
                return stale(format!(
                    "node `{rel}` declares an unqualified dependency `{}`",
                    pr.name
                ));
            };
            declared_roots.insert((group.clone(), pr.name.to_string()));
            let Some(locked) = lockfile.find(&group, pr.name.as_str()) else {
                return stale(format!(
                    "`{}/{}` is declared in `{rel}` but absent from vibe.lock",
                    group, pr.name
                ));
            };
            if !satisfies(&pr.version, &locked.version) {
                return stale(format!(
                    "`{}/{}` is locked at {}, outside the constraint declared in `{rel}`",
                    group, pr.name, locked.version
                ));
            }
            // A dependency resolved from a local `file://` source *inside the
            // workspace* — the in-repo self-hosting registry the author edits
            // in place — points at a
            // MUTABLE working tree: its content changes with no version or
            // `[requires]` edit, so the satisfiability test above cannot prove
            // the lock fresh. Treat it like a `path`/`git` source (above):
            // `Stale`, re-resolve, so a source edit is picked up. An *external*
            // local registry or mirror (a `file://` path outside the workspace)
            // is left immutable and keeps the fast path. `in-place` (PROP-022)
            // giants are excluded — re-hashing a giant tree every install is the
            // cost `in-place` exists to avoid; they refresh through
            // `vibe update`. PROP-011 §2.6.
            if is_in_workspace_file_source(locked.source_url.as_str(), &workspace.root)
                && !locked.materialization.is_in_place()
            {
                return stale(format!(
                    "`{}/{}` resolves from an in-workspace file:// source (a mutable working \
                     tree); re-resolving to pick up any source edit (PROP-011 §2.6)",
                    group, pr.name
                ));
            }
        }
    }

    // The declared root set must equal the lock's recorded roots. The
    // satisfiability loop above caught every *added* root (no locked
    // entry); this catches a *removed* one — still in `root_dependencies`,
    // no longer declared anywhere.
    let locked_roots: HashSet<(Group, String)> = lockfile
        .meta
        .root_dependencies
        .iter()
        .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
        .collect();
    if declared_roots != locked_roots {
        return stale(
            "the declared root set differs from vibe.lock meta.root_dependencies".to_string(),
        );
    }

    // Every locked package must be materialised in the dependency tree — the fast
    // path applies the lock without fetching, so missing content cannot
    // be tolerated. A fresh clone with a committed dependency tree satisfies
    // this; a gitignored or hand-deleted slot does not.
    for p in &lockfile.packages {
        if !vibedeps::is_materialised(&workspace.root, &p.group, p.name.as_str(), &p.version) {
            return stale(format!(
                "`{}/{}@{}` has no materialised {}/ slot",
                p.group,
                p.name,
                p.version,
                crate::layout_paths::vibedeps("")
            ));
        }
    }

    Freshness::Fresh
}

/// `true` iff `version` satisfies `spec`. `VersionSpec::Latest` (`*`)
/// accepts any version.
fn satisfies(spec: &VersionSpec, version: &semver::Version) -> bool {
    match spec {
        VersionSpec::Latest => true,
        VersionSpec::Req(req) => req.matches(version),
    }
}

fn stale(reason: String) -> Freshness {
    Freshness::Stale(reason)
}

/// Pin every registry-resolved declared root the lockfile still satisfies
/// to its exact locked version — the minimum-churn re-resolution of
/// PROP-011 §5.3.
///
/// When [`check`] reports `Stale`, `vibe install` must re-resolve. A
/// *free* re-resolve re-picks every root within its constraint, drifting
/// dependencies the operator never touched. Instead, every root the
/// current lock still satisfies is pinned to `=<locked>`, so only the
/// changed root and its subtree move — `vibe install` stays
/// lockfile-respecting even when it re-resolves. A git-, path-, or
/// override-sourced root keeps its declared form (its version is not the
/// resolution key); so does a root the lock no longer satisfies — that is
/// the change being installed.
///
/// The pinned set can over-constrain: a changed root may be incompatible
/// with a held pin. The caller treats a depsolver error on the pinned set
/// as the signal to fall back to a full, free re-resolve (PROP-011 §5.3).
///
/// This *holds* the pins; it does not *skip* the registry walk. Skipping
/// the walk for an unchanged subtree needs the depsolver's pin-preference
/// machinery — `pin_preferences` on `DepSolver` (PROP-003 §2.1), once
/// deferred "with the SAT solver". SatDepSolver shipped, resolvo shipped as the
/// production default, and neither brought the method; the walk stays,
/// and preference inside the solve would be resolvo's `sort_candidates`
/// (PROP-017 §3).
pub fn hold_pins(declared_roots: &[PackageRef], lockfile: &Lockfile) -> Vec<PackageRef> {
    declared_roots
        .iter()
        .map(|root| {
            // A root with no group cannot be matched against the lock's
            // `(group, name)` identity — leave it at its declared form.
            let Some(group) = root.group.as_ref() else {
                return root.clone();
            };
            match lockfile.find(group, root.name.as_str()) {
                Some(locked)
                    if locked.source_kind == Some(SourceKind::Registry)
                        && satisfies(&root.version, &locked.version) =>
                {
                    // The `=` pin built structurally rather than via a
                    // string round-trip: `VersionReq::parse("={version}")`
                    // rejects versions carrying build metadata (a req has
                    // no build-metadata grammar), and build metadata never
                    // participates in pinning anyway.
                    let req = semver::VersionReq {
                        comparators: vec![semver::Comparator {
                            op: semver::Op::Exact,
                            major: locked.version.major,
                            minor: Some(locked.version.minor),
                            patch: Some(locked.version.patch),
                            pre: locked.version.pre.clone(),
                        }],
                    };
                    PackageRef {
                        kind: root.kind,
                        group: root.group.clone(),
                        name: root.name.clone(),
                        version: VersionSpec::Req(req),
                    }
                }
                _ => root.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "freshness/tests.rs"]
mod tests;
