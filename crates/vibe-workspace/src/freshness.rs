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
//! (a transitive `[requires]` lives inside a `vibedeps/` slot, immutable
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

use std::collections::HashSet;

use vibe_core::manifest::{Lockfile, SourceKind};
use vibe_core::{PackageKind, PackageRef, VersionSpec};

use crate::{Workspace, vibedeps};

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
/// `vibedeps/` slot presence on disk; runs no depsolver and no network.
pub fn check(workspace: &Workspace, lockfile: &Lockfile) -> Freshness {
    // The declared registry root set, unioned across every node. Built
    // only once every node has been proven free of the source kinds the
    // check cannot cheaply reason about (below).
    let mut declared_roots: HashSet<(PackageKind, String)> = HashSet::new();

    for (rel, manifest) in workspace.iter_nodes() {
        let req = &manifest.requires;

        // An unresolved `version.var` placeholder — the workspace loader
        // normally drains these into `packages`, so a non-empty one here
        // is unexpected; refuse to reason about it.
        if !req.var_packages.is_empty() {
            return stale(format!("node `{rel}` carries unresolved version placeholders"));
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
        // satisfy the declared constraint.
        for pr in &req.packages {
            declared_roots.insert((pr.kind, pr.name.clone()));
            let Some(locked) = lockfile.find(pr.kind, &pr.name) else {
                return stale(format!(
                    "`{}:{}` is declared in `{rel}` but absent from vibe.lock",
                    pr.kind, pr.name
                ));
            };
            if !satisfies(&pr.version, &locked.version) {
                return stale(format!(
                    "`{}:{}` is locked at {}, outside the constraint declared in `{rel}`",
                    pr.kind, pr.name, locked.version
                ));
            }
        }
    }

    // The declared root set must equal the lock's recorded roots. The
    // satisfiability loop above caught every *added* root (no locked
    // entry); this catches a *removed* one — still in `root_dependencies`,
    // no longer declared anywhere.
    let locked_roots: HashSet<(PackageKind, String)> = lockfile
        .meta
        .root_dependencies
        .iter()
        .map(|p| (p.kind, p.name.clone()))
        .collect();
    if declared_roots != locked_roots {
        return stale(
            "the declared root set differs from vibe.lock meta.root_dependencies".to_string(),
        );
    }

    // Every locked package must be materialised in `vibedeps/` — the fast
    // path applies the lock without fetching, so missing content cannot
    // be tolerated. A fresh clone with a committed `vibedeps/` satisfies
    // this; a gitignored or hand-deleted slot does not.
    for p in &lockfile.packages {
        if !vibedeps::is_materialised(&workspace.root, p.kind, &p.name, &p.version) {
            return stale(format!(
                "`{}:{}@{}` has no materialised vibedeps/ slot",
                p.kind, p.name, p.version
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
/// machinery (PROP-003 §2.1), deferred with the SAT solver.
pub fn hold_pins(declared_roots: &[PackageRef], lockfile: &Lockfile) -> Vec<PackageRef> {
    declared_roots
        .iter()
        .map(|root| match lockfile.find(root.kind, &root.name) {
            Some(locked)
                if locked.source_kind == Some(SourceKind::Registry)
                    && satisfies(&root.version, &locked.version) =>
            {
                let req = semver::VersionReq::parse(&format!("={}", locked.version))
                    .expect("`=<version>` always parses as a VersionReq");
                PackageRef {
                    kind: root.kind,
                    name: root.name.clone(),
                    version: VersionSpec::Req(req),
                }
            }
            _ => root.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    fn ver(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    /// A standalone workspace whose `vibe.toml` carries `body` as the
    /// `[requires]` section (already including the `[requires...]` header).
    fn workspace_with_requires(requires_toml: &str) -> (TempDir, Workspace) {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n{requires_toml}"),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        (tmp, ws)
    }

    /// Parse a `Lockfile` from a `[[package]]` body, prepending the meta
    /// block. `roots` becomes `meta.root_dependencies`.
    fn lockfile(roots: &[&str], packages_toml: &str) -> Lockfile {
        let roots_list = roots
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let text = format!(
            "[meta]\ngenerated_by = \"test\"\ngenerated_at = \"x\"\nschema_version = 4\n\
             root_dependencies = [{roots_list}]\n\n{packages_toml}"
        );
        toml::from_str(&text).unwrap()
    }

    /// Create an empty `vibedeps/` slot directory so `is_materialised`
    /// reports the package as present.
    fn materialise_slot(ws: &Workspace, kind: PackageKind, name: &str, version: &str) {
        fs::create_dir_all(ws.vibedeps_slot(kind, name, &ver(version))).unwrap();
    }

    /// A `[[package]]` table for a registry-resolved dependency.
    fn registry_pkg(kind: &str, name: &str, version: &str) -> String {
        format!(
            "[[package]]\nkind = \"{kind}\"\nname = \"{name}\"\nversion = \"{version}\"\n\
             source_url = \"https://example/{name}\"\ncontent_hash = \"sha256:x\"\n\
             source_kind = \"registry\"\n"
        )
    }

    #[test]
    fn fresh_when_lock_satisfies_and_slots_present() {
        let (_t, ws) = workspace_with_requires("[requires.packages]\n\"flow:wal\" = \"^0.3\"\n");
        let lf = lockfile(&["flow:wal"], &registry_pkg("flow", "wal", "0.3.2"));
        materialise_slot(&ws, PackageKind::Flow, "wal", "0.3.2");
        assert_eq!(check(&ws, &lf), Freshness::Fresh);
    }

    #[test]
    fn stale_when_a_declared_dep_is_absent_from_the_lock() {
        let (_t, ws) = workspace_with_requires("[requires.packages]\n\"flow:wal\" = \"^0.3\"\n");
        let lf = lockfile(&[], "");
        match check(&ws, &lf) {
            Freshness::Stale(r) => assert!(r.contains("absent from vibe.lock"), "{r}"),
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn stale_when_the_locked_version_is_outside_the_constraint() {
        // The constraint was tightened to `^0.4`; the lock still pins 0.3.2.
        let (_t, ws) = workspace_with_requires("[requires.packages]\n\"flow:wal\" = \"^0.4\"\n");
        let lf = lockfile(&["flow:wal"], &registry_pkg("flow", "wal", "0.3.2"));
        materialise_slot(&ws, PackageKind::Flow, "wal", "0.3.2");
        match check(&ws, &lf) {
            Freshness::Stale(r) => assert!(r.contains("outside the constraint"), "{r}"),
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn fresh_when_locked_version_still_satisfies_a_loosened_constraint() {
        // `^0.3` and the lock at 0.3.2 — no drift to a newer 0.3.x; the
        // locked version is honoured verbatim (the lockfile-respecting win).
        let (_t, ws) = workspace_with_requires("[requires.packages]\n\"flow:wal\" = \"^0.3\"\n");
        let lf = lockfile(&["flow:wal"], &registry_pkg("flow", "wal", "0.3.2"));
        materialise_slot(&ws, PackageKind::Flow, "wal", "0.3.2");
        assert!(check(&ws, &lf).is_fresh());
    }

    #[test]
    fn stale_when_a_root_was_removed() {
        // `[requires]` declares only `flow:wal`, but the lock still records
        // `feat:auth` as a root — a dependency was dropped.
        let (_t, ws) = workspace_with_requires("[requires.packages]\n\"flow:wal\" = \"^0.3\"\n");
        let lf = lockfile(
            &["flow:wal", "feat:auth"],
            &format!(
                "{}\n{}",
                registry_pkg("flow", "wal", "0.3.2"),
                registry_pkg("feat", "auth", "1.0.0"),
            ),
        );
        materialise_slot(&ws, PackageKind::Flow, "wal", "0.3.2");
        materialise_slot(&ws, PackageKind::Feat, "auth", "1.0.0");
        match check(&ws, &lf) {
            Freshness::Stale(r) => assert!(r.contains("root set"), "{r}"),
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn stale_when_a_locked_slot_is_not_materialised() {
        let (_t, ws) = workspace_with_requires("[requires.packages]\n\"flow:wal\" = \"^0.3\"\n");
        let lf = lockfile(&["flow:wal"], &registry_pkg("flow", "wal", "0.3.2"));
        // No materialise_slot call — the slot is absent.
        match check(&ws, &lf) {
            Freshness::Stale(r) => assert!(r.contains("no materialised"), "{r}"),
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn stale_when_a_git_source_dependency_is_present() {
        let (_t, ws) = workspace_with_requires(
            "[requires.packages]\n\
             \"flow:internal\" = { git = \"https://example/i\", tag = \"v1.0.0\" }\n",
        );
        let lf = lockfile(&[], "");
        match check(&ws, &lf) {
            Freshness::Stale(r) => assert!(r.contains("git-source"), "{r}"),
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn stale_when_a_capability_requirement_is_present() {
        let (_t, ws) =
            workspace_with_requires("[requires]\ncapabilities = [\"capability:wal-protocol\"]\n");
        let lf = lockfile(&[], "");
        match check(&ws, &lf) {
            Freshness::Stale(r) => assert!(r.contains("capability"), "{r}"),
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn fresh_for_an_empty_requires_and_empty_lock() {
        // A degenerate project with nothing declared and nothing locked is
        // trivially fresh — though `vibe install` bails earlier on "nothing
        // to install" before the check is ever reached.
        let (_t, ws) = workspace_with_requires("");
        let lf = lockfile(&[], "");
        assert!(check(&ws, &lf).is_fresh());
    }

    // --- PROP-011 §5.3 — minimum-churn re-resolution (`hold_pins`) --------

    #[test]
    fn hold_pins_pins_a_satisfied_registry_root() {
        let lf = lockfile(&["flow:wal"], &registry_pkg("flow", "wal", "0.3.2"));
        let declared = vec![PackageRef::parse("flow:wal@^0.3").unwrap()];
        let pinned = hold_pins(&declared, &lf);
        // `^0.3` becomes `=0.3.2` — the locked version is held.
        assert_eq!(pinned[0].to_string(), "flow:wal@=0.3.2");
    }

    #[test]
    fn hold_pins_leaves_a_changed_root_free() {
        // The constraint moved to `^0.4`; the lock at 0.3.2 no longer
        // satisfies it — this is the change, it must resolve freely.
        let lf = lockfile(&["flow:wal"], &registry_pkg("flow", "wal", "0.3.2"));
        let declared = vec![PackageRef::parse("flow:wal@^0.4").unwrap()];
        let pinned = hold_pins(&declared, &lf);
        assert_eq!(pinned[0], declared[0]);
    }

    #[test]
    fn hold_pins_leaves_an_unlocked_root_free() {
        let lf = lockfile(&[], "");
        let declared = vec![PackageRef::parse("flow:new@^1.0").unwrap()];
        let pinned = hold_pins(&declared, &lf);
        assert_eq!(pinned[0], declared[0]);
    }

    #[test]
    fn hold_pins_does_not_pin_a_git_sourced_root() {
        // A git-source root resolves by ref, not version — pinning it to
        // `=<version>` would be meaningless, so it is left at its declared
        // form and resolves freely.
        let lf = lockfile(
            &["flow:internal"],
            "[[package]]\nkind = \"flow\"\nname = \"internal\"\nversion = \"0.1.0\"\n\
             source_url = \"https://example/i\"\ncontent_hash = \"sha256:x\"\n\
             source_kind = \"git\"\n",
        );
        let declared = vec![PackageRef::parse("flow:internal").unwrap()];
        let pinned = hold_pins(&declared, &lf);
        assert_eq!(pinned[0], declared[0]);
    }

    #[test]
    fn hold_pins_mixes_held_and_free_roots() {
        // wal is satisfied (held); auth's constraint moved (free).
        let lf = lockfile(
            &["flow:wal", "feat:auth"],
            &format!(
                "{}\n{}",
                registry_pkg("flow", "wal", "0.3.2"),
                registry_pkg("feat", "auth", "1.0.0"),
            ),
        );
        let declared = vec![
            PackageRef::parse("flow:wal@^0.3").unwrap(),
            PackageRef::parse("feat:auth@^2.0").unwrap(),
        ];
        let pinned = hold_pins(&declared, &lf);
        assert_eq!(pinned[0].to_string(), "flow:wal@=0.3.2");
        assert_eq!(pinned[1], declared[1]);
    }
}
